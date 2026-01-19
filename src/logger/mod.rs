//! Remote Logging Module
//!
//! Provides async buffered logging with remote reporting to tars.tarslog service.

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use crate::protocol::logf::{LogInfo, encode_log_buffer, LOG_LOGGER_BY_INFO};
use crate::protocol::RequestPacket;
use crate::codec::Buffer;
use crate::transport::AsyncSimpleTarsClient;

/// Configuration for remote logging
pub struct RemoteLogConfig {
    /// Queue size for buffering logs
    pub queue_size: usize,
    /// Max logs to send in one batch
    pub max_batch_size: usize,
    /// Interval for flushing logs (milliseconds)
    pub flush_interval_ms: u64,
    /// Remote log server address
    pub server_addr: String,
}

impl Default for RemoteLogConfig {
    fn default() -> Self {
        Self {
            queue_size: 500_000,
            max_batch_size: 2000,
            flush_interval_ms: 1000,
            server_addr: String::new(),
        }
    }
}

/// Remote log writer that buffers and sends logs asynchronously
pub struct RemoteTimeWriter {
    log_info: LogInfo,
    sender: mpsc::Sender<String>,
    config: RemoteLogConfig,
}

impl RemoteTimeWriter {
    /// Create a new remote time writer
    pub fn new(config: RemoteLogConfig) -> (Self, RemoteLogHandle) {
        let (tx, rx) = mpsc::channel(config.queue_size);

        let writer = Self {
            log_info: LogInfo::default(),
            sender: tx,
            config,
        };

        let handle = RemoteLogHandle { receiver: rx };

        (writer, handle)
    }

    /// Initialize server info
    pub fn init_server_info(&mut self, app: &str, server: &str, filename: &str, set_division: &str) {
        self.log_info = LogInfo {
            appname: app.to_string(),
            servername: server.to_string(),
            filename: filename.to_string(),
            format: "%Y%m%d".to_string(),
            setdivision: set_division.to_string(),
            has_suffix: true,
            has_app_prefix: true,
            has_square_bracket: false,
            concat_str: "_".to_string(),
            separator: "|".to_string(),
            log_type: String::new(),
        };
    }

    /// Write a log message (non-blocking)
    pub fn write(&self, msg: &str) {
        if let Err(_) = self.sender.try_send(msg.to_string()) {
            // Channel is full, log dropped
            eprintln!("Remote log channel is full, dropping log");
        }
    }

    /// Get log info
    pub fn log_info(&self) -> &LogInfo {
        &self.log_info
    }

    /// Get server address
    pub fn server_addr(&self) -> &str {
        &self.config.server_addr
    }
}

/// Handle for the background log sender task
pub struct RemoteLogHandle {
    receiver: mpsc::Receiver<String>,
}

impl RemoteLogHandle {
    /// Run the background log sender
    pub async fn run(
        mut self,
        log_info: LogInfo,
        server_addr: String,
        max_batch_size: usize,
        flush_interval_ms: u64,
    ) {
        let flush_interval = Duration::from_millis(flush_interval_ms);
        let mut buffer: Vec<String> = Vec::with_capacity(max_batch_size);
        let mut client: Option<Arc<AsyncSimpleTarsClient>> = None;

        // Try to connect to log server
        if !server_addr.is_empty() {
            match AsyncSimpleTarsClient::connect(&server_addr).await {
                Ok(c) => {
                    info!("Connected to remote log server: {}", server_addr);
                    client = Some(Arc::new(c));
                }
                Err(e) => {
                    error!("Failed to connect to remote log server: {}", e);
                }
            }
        }

        let mut interval = tokio::time::interval(flush_interval);

        loop {
            tokio::select! {
                // Receive log messages
                msg = self.receiver.recv() => {
                    match msg {
                        Some(log) => {
                            buffer.push(log);
                            if buffer.len() >= max_batch_size {
                                if let Some(ref mut c) = client {
                                    Self::flush_logs(c, &log_info, &mut buffer).await;
                                } else {
                                    buffer.clear();
                                }
                            }
                        }
                        None => {
                            // Channel closed, flush remaining and exit
                            if !buffer.is_empty() {
                                if let Some(ref mut c) = client {
                                    Self::flush_logs(c, &log_info, &mut buffer).await;
                                }
                            }
                            break;
                        }
                    }
                }
                // Periodic flush
                _ = interval.tick() => {
                    if !buffer.is_empty() {
                        if let Some(ref mut c) = client {
                            Self::flush_logs(c, &log_info, &mut buffer).await;
                        } else {
                            buffer.clear();
                        }
                    }
                }
            }
        }
    }

    async fn flush_logs(client: &Arc<AsyncSimpleTarsClient>, log_info: &LogInfo, buffer: &mut Vec<String>) {
        if buffer.is_empty() {
            return;
        }

        // Encode request
        let mut body_buf = Buffer::new();

        // Encode LogInfo at tag 0
        body_buf.write_struct_begin(0).ok();
        log_info.encode(&mut body_buf).ok();
        body_buf.write_struct_end().ok();

        // Encode log buffer at tag 1
        encode_log_buffer(&mut body_buf, buffer, 1).ok();

        let mut req = RequestPacket::new();
        req.s_servant_name = "tars.tarslog.LogObj".to_string();
        req.s_func_name = LOG_LOGGER_BY_INFO.to_string();
        req.s_buffer = body_buf.to_bytes();
        req.i_timeout = 3000;
        req.c_packet_type = crate::consts::TARS_ONEWAY;

        // Send (one-way, don't wait for response)
        match client.send_oneway(&req).await {
            Ok(_) => {
                debug!("Flushed {} logs to remote server", buffer.len());
            }
            Err(e) => {
                error!("Failed to flush logs: {}", e);
            }
        }

        buffer.clear();
    }
}

/// Log level
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
    Trace,
    Debug,
    Info,
    Warn,
    Error,
}

impl LogLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            LogLevel::Trace => "TRACE",
            LogLevel::Debug => "DEBUG",
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        }
    }
}

/// Tars logger that writes to both local and remote destinations
pub struct TarsLogger {
    name: String,
    remote_writer: Option<Arc<RemoteTimeWriter>>,
    level: LogLevel,
}

impl TarsLogger {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            remote_writer: None,
            level: LogLevel::Info,
        }
    }

    pub fn with_remote(mut self, writer: Arc<RemoteTimeWriter>) -> Self {
        self.remote_writer = Some(writer);
        self
    }

    pub fn set_level(&mut self, level: LogLevel) {
        self.level = level;
    }

    fn log(&self, level: LogLevel, msg: &str) {
        if level < self.level {
            return;
        }

        let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let formatted = format!("{}|{}|{}|{}", timestamp, level.as_str(), self.name, msg);

        // Write to remote if available
        if let Some(ref writer) = self.remote_writer {
            writer.write(&formatted);
        }

        // Also write to local tracing
        match level {
            LogLevel::Trace => tracing::trace!("{}", formatted),
            LogLevel::Debug => tracing::debug!("{}", formatted),
            LogLevel::Info => tracing::info!("{}", formatted),
            LogLevel::Warn => tracing::warn!("{}", formatted),
            LogLevel::Error => tracing::error!("{}", formatted),
        }
    }

    pub fn trace(&self, msg: &str) { self.log(LogLevel::Trace, msg); }
    pub fn debug(&self, msg: &str) { self.log(LogLevel::Debug, msg); }
    pub fn info(&self, msg: &str) { self.log(LogLevel::Info, msg); }
    pub fn warn(&self, msg: &str) { self.log(LogLevel::Warn, msg); }
    pub fn error(&self, msg: &str) { self.log(LogLevel::Error, msg); }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_log_level_ordering() {
        assert!(LogLevel::Trace < LogLevel::Debug);
        assert!(LogLevel::Debug < LogLevel::Info);
        assert!(LogLevel::Info < LogLevel::Warn);
        assert!(LogLevel::Warn < LogLevel::Error);
    }
}
