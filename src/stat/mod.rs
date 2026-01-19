//! Statistics Reporting Module
//!
//! Provides call statistics collection and reporting to tars.tarsstat service.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, error, info};

use crate::protocol::statf::{
    StatMicMsgHead, StatMicMsgBody, StatInfo, encode_stat_map, STAT_REPORT_MIC_MSG
};
use crate::protocol::RequestPacket;
use crate::codec::Buffer;
use crate::transport::AsyncSimpleTarsClient;

/// Configuration for statistics reporting
pub struct StatConfig {
    /// Report interval in milliseconds
    pub report_interval_ms: u64,
    /// Channel buffer size
    pub channel_buf_size: usize,
    /// Statistics server address
    pub server_addr: String,
    /// Tars version string
    pub tars_version: String,
}

impl Default for StatConfig {
    fn default() -> Self {
        Self {
            report_interval_ms: 10_000,  // 10 seconds
            channel_buf_size: 100_000,
            server_addr: String::new(),
            tars_version: "1.0.0".to_string(),
        }
    }
}

/// Statistics reporter that collects and reports call metrics
pub struct StatReporter {
    sender: mpsc::Sender<StatInfo>,
    local_ip: String,
    tars_version: String,
}

impl StatReporter {
    /// Create a new statistics reporter
    pub fn new(config: StatConfig) -> (Self, StatReportHandle) {
        let (tx, rx) = mpsc::channel(config.channel_buf_size);

        let reporter = Self {
            sender: tx,
            local_ip: Self::get_local_ip(),
            tars_version: config.tars_version.clone(),
        };

        let handle = StatReportHandle {
            receiver: rx,
            config,
        };

        (reporter, handle)
    }

    fn get_local_ip() -> String {
        // Try to get local IP (simplified)
        "127.0.0.1".to_string()
    }

    /// Report a successful call from client side
    pub fn report_success(&self, servant: &str, func: &str, slave_ip: &str, slave_port: i32, cost_ms: i64) {
        let head = StatMicMsgHead {
            master_name: "".to_string(),
            slave_name: servant.to_string(),
            interface_name: func.to_string(),
            master_ip: self.local_ip.clone(),
            slave_ip: slave_ip.to_string(),
            slave_port,
            return_value: 0,
            slave_set_name: String::new(),
            slave_set_area: String::new(),
            slave_set_id: String::new(),
            tars_version: self.tars_version.clone(),
        };

        let mut body = StatMicMsgBody::new();
        body.count = 1;
        body.add_response_time(cost_ms);

        let _ = self.sender.try_send(StatInfo::new(head, body));
    }

    /// Report a timeout call from client side
    pub fn report_timeout(&self, servant: &str, func: &str, slave_ip: &str, slave_port: i32, cost_ms: i64) {
        let head = StatMicMsgHead {
            master_name: "".to_string(),
            slave_name: servant.to_string(),
            interface_name: func.to_string(),
            master_ip: self.local_ip.clone(),
            slave_ip: slave_ip.to_string(),
            slave_port,
            return_value: -3,  // TARS_INVOKE_TIMEOUT
            slave_set_name: String::new(),
            slave_set_area: String::new(),
            slave_set_id: String::new(),
            tars_version: self.tars_version.clone(),
        };

        let mut body = StatMicMsgBody::new();
        body.timeout_count = 1;
        body.add_response_time(cost_ms);

        let _ = self.sender.try_send(StatInfo::new(head, body));
    }

    /// Report an exception call from client side
    pub fn report_exception(&self, servant: &str, func: &str, slave_ip: &str, slave_port: i32, ret: i32, cost_ms: i64) {
        let head = StatMicMsgHead {
            master_name: "".to_string(),
            slave_name: servant.to_string(),
            interface_name: func.to_string(),
            master_ip: self.local_ip.clone(),
            slave_ip: slave_ip.to_string(),
            slave_port,
            return_value: ret,
            slave_set_name: String::new(),
            slave_set_area: String::new(),
            slave_set_id: String::new(),
            tars_version: self.tars_version.clone(),
        };

        let mut body = StatMicMsgBody::new();
        body.exec_count = 1;
        body.add_response_time(cost_ms);

        let _ = self.sender.try_send(StatInfo::new(head, body));
    }

    /// Report from server side
    pub fn report_from_server(&self, func: &str, client_ip: &str, ret: i32, cost_ms: i64) {
        let head = StatMicMsgHead {
            master_name: client_ip.to_string(),
            slave_name: "stat_from_server".to_string(),
            interface_name: func.to_string(),
            master_ip: client_ip.to_string(),
            slave_ip: self.local_ip.clone(),
            slave_port: 0,
            return_value: ret,
            slave_set_name: String::new(),
            slave_set_area: String::new(),
            slave_set_id: String::new(),
            tars_version: self.tars_version.clone(),
        };

        let mut body = StatMicMsgBody::new();
        if ret == 0 {
            body.count = 1;
        } else {
            body.exec_count = 1;
        }
        body.add_response_time(cost_ms);

        let _ = self.sender.try_send(StatInfo::new(head, body));
    }
}

/// Handle for the background statistics sender task
pub struct StatReportHandle {
    receiver: mpsc::Receiver<StatInfo>,
    config: StatConfig,
}

impl StatReportHandle {
    /// Run the background statistics sender
    pub async fn run(mut self) {
        let report_interval = Duration::from_millis(self.config.report_interval_ms);

        // Aggregated stats: client stats and server stats
        let mut client_stats: HashMap<StatMicMsgHead, StatMicMsgBody> = HashMap::new();
        let mut server_stats: HashMap<StatMicMsgHead, StatMicMsgBody> = HashMap::new();

        let mut client: Option<Arc<AsyncSimpleTarsClient>> = None;

        // Try to connect to stat server
        if !self.config.server_addr.is_empty() {
            match AsyncSimpleTarsClient::connect(&self.config.server_addr).await {
                Ok(c) => {
                    info!("Connected to stat server: {}", self.config.server_addr);
                    client = Some(Arc::new(c));
                }
                Err(e) => {
                    error!("Failed to connect to stat server: {}", e);
                }
            }
        }

        let mut interval = tokio::time::interval(report_interval);

        loop {
            tokio::select! {
                // Receive stat info
                msg = self.receiver.recv() => {
                    match msg {
                        Some(info) => {
                            // Determine if it's client or server stat
                            let stats = if info.head.slave_name == "stat_from_server" {
                                &mut server_stats
                            } else {
                                &mut client_stats
                            };

                            // Aggregate
                            if let Some(existing) = stats.get_mut(&info.head) {
                                existing.merge(&info.body);
                            } else {
                                stats.insert(info.head, info.body);
                            }
                        }
                        None => {
                            // Channel closed, report remaining and exit
                            if let Some(ref c) = client {
                                if !client_stats.is_empty() {
                                    Self::report_stats(c, &client_stats, true).await;
                                }
                                if !server_stats.is_empty() {
                                    Self::report_stats(c, &server_stats, false).await;
                                }
                            }
                            break;
                        }
                    }
                }
                // Periodic report
                _ = interval.tick() => {
                    if let Some(ref c) = client {
                        if !client_stats.is_empty() {
                            Self::report_stats(c, &client_stats, true).await;
                            client_stats.clear();
                        }
                        if !server_stats.is_empty() {
                            Self::report_stats(c, &server_stats, false).await;
                            server_stats.clear();
                        }
                    } else {
                        // No client, just clear stats
                        client_stats.clear();
                        server_stats.clear();
                    }
                }
            }
        }
    }

    async fn report_stats(
        client: &Arc<AsyncSimpleTarsClient>,
        stats: &HashMap<StatMicMsgHead, StatMicMsgBody>,
        from_client: bool,
    ) {
        if stats.is_empty() {
            return;
        }

        // Encode request
        let mut body_buf = Buffer::new();

        // Encode stats map at tag 0
        encode_stat_map(&mut body_buf, stats, 0).ok();

        // Encode bFromClient at tag 1
        body_buf.write_bool(from_client, 1).ok();

        let mut req = RequestPacket::new();
        req.s_servant_name = "tars.tarsstat.StatObj".to_string();
        req.s_func_name = STAT_REPORT_MIC_MSG.to_string();
        req.s_buffer = body_buf.to_bytes();
        req.i_timeout = 3000;

        // Send and get response
        match client.invoke(&req).await {
            Ok(_rsp) => {
                debug!("Reported {} stat entries (from_client={})", stats.len(), from_client);
            }
            Err(e) => {
                error!("Failed to report stats: {}", e);
            }
        }
    }
}

/// Global stat reporter instance
pub struct GlobalStatReporter {
    reporter: Option<Arc<StatReporter>>,
}

impl GlobalStatReporter {
    pub fn new() -> Self {
        Self { reporter: None }
    }

    pub fn init(&mut self, reporter: Arc<StatReporter>) {
        self.reporter = Some(reporter);
    }

    pub fn reporter(&self) -> Option<Arc<StatReporter>> {
        self.reporter.clone()
    }
}

// Convenience functions for reporting
impl StatReporter {
    /// Create a call timer for measuring latency
    pub fn start_call(&self) -> CallTimer {
        CallTimer {
            start: Instant::now(),
        }
    }
}

/// Timer for measuring call latency
pub struct CallTimer {
    start: Instant,
}

impl CallTimer {
    pub fn elapsed_ms(&self) -> i64 {
        self.start.elapsed().as_millis() as i64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stat_body_merge() {
        let mut body1 = StatMicMsgBody::new();
        body1.count = 5;
        body1.total_rsp_time = 100;
        body1.max_rsp_time = 30;
        body1.min_rsp_time = 10;

        let mut body2 = StatMicMsgBody::new();
        body2.count = 3;
        body2.total_rsp_time = 60;
        body2.max_rsp_time = 40;
        body2.min_rsp_time = 5;

        body1.merge(&body2);

        assert_eq!(body1.count, 8);
        assert_eq!(body1.total_rsp_time, 160);
        assert_eq!(body1.max_rsp_time, 40);
        assert_eq!(body1.min_rsp_time, 5);
    }

    #[test]
    fn test_call_timer() {
        let timer = CallTimer { start: Instant::now() };
        std::thread::sleep(std::time::Duration::from_millis(10));
        let elapsed = timer.elapsed_ms();
        assert!(elapsed >= 10);
    }
}
