//! Configuration structures

use std::collections::HashMap;
use std::time::Duration;
use serde::{Deserialize, Serialize};

/// Server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    /// Application name
    pub app: String,
    /// Server name
    pub server: String,
    /// Base path
    pub base_path: String,
    /// Data path
    pub data_path: String,
    /// Log path
    pub log_path: String,
    /// Log level
    pub log_level: String,
    /// Local endpoint (admin port)
    pub local: String,
    /// Node endpoint
    pub node: String,
    /// Log server endpoint
    pub log: String,
    /// Config server endpoint
    pub config: String,
    /// Notify server endpoint
    pub notify: String,
    /// Enable SET routing
    pub enable_set: bool,
    /// SET division (e.g., "sz.app.1")
    pub set_division: String,
    /// Accept timeout (ms)
    #[serde(default = "default_accept_timeout")]
    pub accept_timeout: u64,
    /// Read timeout (ms)
    #[serde(default = "default_read_timeout")]
    pub read_timeout: u64,
    /// Write timeout (ms)
    #[serde(default = "default_write_timeout")]
    pub write_timeout: u64,
    /// Handle timeout (ms)
    #[serde(default = "default_handle_timeout")]
    pub handle_timeout: u64,
    /// Idle timeout (ms)
    #[serde(default = "default_idle_timeout")]
    pub idle_timeout: u64,
    /// Max concurrent invokes
    #[serde(default = "default_max_invoke")]
    pub max_invoke: i32,
    /// Queue capacity
    #[serde(default = "default_queue_cap")]
    pub queue_cap: usize,
    /// TCP read buffer size
    #[serde(default = "default_tcp_read_buffer")]
    pub tcp_read_buffer: usize,
    /// TCP write buffer size
    #[serde(default = "default_tcp_write_buffer")]
    pub tcp_write_buffer: usize,
    /// TCP no delay
    #[serde(default = "default_tcp_no_delay")]
    pub tcp_no_delay: bool,
    /// Adapter configurations
    #[serde(default)]
    pub adapters: HashMap<String, AdapterConfig>,
}

fn default_accept_timeout() -> u64 { 10000 }
fn default_read_timeout() -> u64 { 60000 }
fn default_write_timeout() -> u64 { 60000 }
fn default_handle_timeout() -> u64 { 60000 }
fn default_idle_timeout() -> u64 { 600000 }
fn default_max_invoke() -> i32 { 200000 }
fn default_queue_cap() -> usize { 10000 }
fn default_tcp_read_buffer() -> usize { 128 * 1024 }
fn default_tcp_write_buffer() -> usize { 128 * 1024 }
fn default_tcp_no_delay() -> bool { false }

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            app: String::new(),
            server: String::new(),
            base_path: String::new(),
            data_path: String::new(),
            log_path: String::new(),
            log_level: "INFO".to_string(),
            local: String::new(),
            node: String::new(),
            log: String::new(),
            config: String::new(),
            notify: String::new(),
            enable_set: false,
            set_division: String::new(),
            accept_timeout: default_accept_timeout(),
            read_timeout: default_read_timeout(),
            write_timeout: default_write_timeout(),
            handle_timeout: default_handle_timeout(),
            idle_timeout: default_idle_timeout(),
            max_invoke: default_max_invoke(),
            queue_cap: default_queue_cap(),
            tcp_read_buffer: default_tcp_read_buffer(),
            tcp_write_buffer: default_tcp_write_buffer(),
            tcp_no_delay: default_tcp_no_delay(),
            adapters: HashMap::new(),
        }
    }
}

impl ServerConfig {
    pub fn accept_timeout_duration(&self) -> Duration {
        Duration::from_millis(self.accept_timeout)
    }

    pub fn read_timeout_duration(&self) -> Duration {
        Duration::from_millis(self.read_timeout)
    }

    pub fn write_timeout_duration(&self) -> Duration {
        Duration::from_millis(self.write_timeout)
    }

    pub fn handle_timeout_duration(&self) -> Duration {
        Duration::from_millis(self.handle_timeout)
    }

    pub fn idle_timeout_duration(&self) -> Duration {
        Duration::from_millis(self.idle_timeout)
    }
}

/// Adapter (servant) configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdapterConfig {
    /// Servant name (e.g., "App.Server.Obj")
    pub servant: String,
    /// Endpoint (e.g., "tcp -h 0.0.0.0 -p 10000")
    pub endpoint: String,
    /// Protocol type
    pub protocol: String,
    /// Max connections
    #[serde(default)]
    pub max_conn: i32,
    /// Thread pool size
    #[serde(default)]
    pub threads: i32,
    /// Queue capacity
    #[serde(default)]
    pub queue_cap: i32,
    /// Queue timeout (ms)
    #[serde(default)]
    pub queue_timeout: i32,
}

impl Default for AdapterConfig {
    fn default() -> Self {
        Self {
            servant: String::new(),
            endpoint: String::new(),
            protocol: "tars".to_string(),
            max_conn: 0,
            threads: 0,
            queue_cap: 0,
            queue_timeout: 0,
        }
    }
}

/// Client configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClientConfig {
    /// Locator endpoint
    pub locator: String,
    /// Stat server endpoint
    pub stat: String,
    /// Property server endpoint
    pub property: String,
    /// Async invoke timeout (ms)
    #[serde(default = "default_async_timeout")]
    pub async_invoke_timeout: u64,
    /// Refresh endpoint interval (ms)
    #[serde(default = "default_refresh_interval")]
    pub refresh_endpoint_interval: u64,
    /// Report interval (ms)
    #[serde(default = "default_report_interval")]
    pub report_interval: u64,
    /// Sample rate
    #[serde(default = "default_sample_rate")]
    pub sample_rate: u32,
    /// Max sample count
    #[serde(default = "default_max_sample_count")]
    pub max_sample_count: u32,
    /// Client dial timeout (ms)
    #[serde(default = "default_dial_timeout")]
    pub dial_timeout: u64,
    /// Client idle timeout (ms)
    #[serde(default = "default_client_idle_timeout")]
    pub idle_timeout: u64,
    /// Client read timeout (ms)
    #[serde(default = "default_client_read_timeout")]
    pub read_timeout: u64,
    /// Client write timeout (ms)
    #[serde(default = "default_client_write_timeout")]
    pub write_timeout: u64,
    /// Client queue length
    #[serde(default = "default_client_queue_len")]
    pub queue_len: usize,
    /// Max queue size per object
    #[serde(default = "default_obj_queue_max")]
    pub obj_queue_max: i32,
    /// Keep alive interval (ms)
    #[serde(default = "default_keep_alive_interval")]
    pub keep_alive_interval: u64,
}

fn default_async_timeout() -> u64 { 3000 }
fn default_refresh_interval() -> u64 { 60000 }
fn default_report_interval() -> u64 { 10000 }
fn default_sample_rate() -> u32 { 1000 }
fn default_max_sample_count() -> u32 { 100 }
fn default_dial_timeout() -> u64 { 3000 }
fn default_client_idle_timeout() -> u64 { 600000 }
fn default_client_read_timeout() -> u64 { 3000 }
fn default_client_write_timeout() -> u64 { 3000 }
fn default_client_queue_len() -> usize { 10000 }
fn default_obj_queue_max() -> i32 { 10000 }
fn default_keep_alive_interval() -> u64 { 60000 }

impl Default for ClientConfig {
    fn default() -> Self {
        Self {
            locator: String::new(),
            stat: String::new(),
            property: String::new(),
            async_invoke_timeout: default_async_timeout(),
            refresh_endpoint_interval: default_refresh_interval(),
            report_interval: default_report_interval(),
            sample_rate: default_sample_rate(),
            max_sample_count: default_max_sample_count(),
            dial_timeout: default_dial_timeout(),
            idle_timeout: default_client_idle_timeout(),
            read_timeout: default_client_read_timeout(),
            write_timeout: default_client_write_timeout(),
            queue_len: default_client_queue_len(),
            obj_queue_max: default_obj_queue_max(),
            keep_alive_interval: default_keep_alive_interval(),
        }
    }
}

impl ClientConfig {
    pub fn async_timeout_duration(&self) -> Duration {
        Duration::from_millis(self.async_invoke_timeout)
    }

    pub fn dial_timeout_duration(&self) -> Duration {
        Duration::from_millis(self.dial_timeout)
    }

    pub fn idle_timeout_duration(&self) -> Duration {
        Duration::from_millis(self.idle_timeout)
    }

    pub fn read_timeout_duration(&self) -> Duration {
        Duration::from_millis(self.read_timeout)
    }

    pub fn write_timeout_duration(&self) -> Duration {
        Duration::from_millis(self.write_timeout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.accept_timeout, 10000);
        assert_eq!(config.max_invoke, 200000);
    }

    #[test]
    fn test_client_config_default() {
        let config = ClientConfig::default();
        assert_eq!(config.async_invoke_timeout, 3000);
        assert_eq!(config.dial_timeout, 3000);
    }

    #[test]
    fn test_timeout_duration() {
        let config = ClientConfig::default();
        assert_eq!(config.async_timeout_duration(), Duration::from_millis(3000));
    }
}
