//! Transport configuration structures

use std::time::Duration;
use std::sync::Arc;
use tokio_rustls::rustls;

/// Client transport configuration
#[derive(Clone)]
pub struct TarsClientConfig {
    /// Protocol type: "tcp", "udp", "ssl"
    pub proto: String,
    /// Send queue length
    pub queue_len: usize,
    /// Connection idle timeout
    pub idle_timeout: Duration,
    /// Read timeout
    pub read_timeout: Duration,
    /// Write timeout
    pub write_timeout: Duration,
    /// Connection dial timeout
    pub dial_timeout: Duration,
    /// TLS configuration (for SSL)
    pub tls_config: Option<Arc<rustls::ClientConfig>>,
}

impl Default for TarsClientConfig {
    fn default() -> Self {
        Self {
            proto: "tcp".to_string(),
            queue_len: 10000,
            idle_timeout: Duration::from_secs(600),
            read_timeout: Duration::from_secs(3),
            write_timeout: Duration::from_secs(3),
            dial_timeout: Duration::from_secs(3),
            tls_config: None,
        }
    }
}

impl TarsClientConfig {
    /// Create a new TCP client config
    pub fn tcp() -> Self {
        Self::default()
    }

    /// Create a new UDP client config
    pub fn udp() -> Self {
        Self {
            proto: "udp".to_string(),
            ..Self::default()
        }
    }

    /// Create a new SSL client config
    pub fn ssl(tls_config: Arc<rustls::ClientConfig>) -> Self {
        Self {
            proto: "ssl".to_string(),
            tls_config: Some(tls_config),
            ..Self::default()
        }
    }

    /// Set queue length
    pub fn with_queue_len(mut self, len: usize) -> Self {
        self.queue_len = len;
        self
    }

    /// Set idle timeout
    pub fn with_idle_timeout(mut self, timeout: Duration) -> Self {
        self.idle_timeout = timeout;
        self
    }

    /// Set read timeout
    pub fn with_read_timeout(mut self, timeout: Duration) -> Self {
        self.read_timeout = timeout;
        self
    }

    /// Set write timeout
    pub fn with_write_timeout(mut self, timeout: Duration) -> Self {
        self.write_timeout = timeout;
        self
    }

    /// Set dial timeout
    pub fn with_dial_timeout(mut self, timeout: Duration) -> Self {
        self.dial_timeout = timeout;
        self
    }

    /// Check if TCP
    pub fn is_tcp(&self) -> bool {
        self.proto == "tcp"
    }

    /// Check if UDP
    pub fn is_udp(&self) -> bool {
        self.proto == "udp"
    }

    /// Check if SSL
    pub fn is_ssl(&self) -> bool {
        self.proto == "ssl"
    }
}

/// Server transport configuration
#[derive(Clone)]
pub struct TarsServerConfig {
    /// Protocol type: "tcp", "udp", "ssl"
    pub proto: String,
    /// Bind address (e.g., "0.0.0.0:10000")
    pub address: String,
    /// Max concurrent invokes
    pub max_invoke: i32,
    /// Accept timeout
    pub accept_timeout: Duration,
    /// Read timeout
    pub read_timeout: Duration,
    /// Write timeout
    pub write_timeout: Duration,
    /// Handle timeout
    pub handle_timeout: Duration,
    /// Idle connection timeout
    pub idle_timeout: Duration,
    /// Queue capacity
    pub queue_cap: usize,
    /// TCP read buffer size
    pub tcp_read_buffer: usize,
    /// TCP write buffer size
    pub tcp_write_buffer: usize,
    /// TCP no delay
    pub tcp_no_delay: bool,
    /// TLS configuration (for SSL)
    pub tls_config: Option<Arc<rustls::ServerConfig>>,
}

impl Default for TarsServerConfig {
    fn default() -> Self {
        Self {
            proto: "tcp".to_string(),
            address: "0.0.0.0:10000".to_string(),
            max_invoke: 200000,
            accept_timeout: Duration::from_secs(10),
            read_timeout: Duration::from_secs(60),
            write_timeout: Duration::from_secs(60),
            handle_timeout: Duration::from_secs(60),
            idle_timeout: Duration::from_secs(600),
            queue_cap: 10000,
            tcp_read_buffer: 128 * 1024,
            tcp_write_buffer: 128 * 1024,
            tcp_no_delay: false,
            tls_config: None,
        }
    }
}

impl TarsServerConfig {
    /// Create a new TCP server config
    pub fn tcp(address: &str) -> Self {
        Self {
            address: address.to_string(),
            ..Self::default()
        }
    }

    /// Create a new UDP server config
    pub fn udp(address: &str) -> Self {
        Self {
            proto: "udp".to_string(),
            address: address.to_string(),
            ..Self::default()
        }
    }

    /// Create a new SSL server config
    pub fn ssl(address: &str, tls_config: Arc<rustls::ServerConfig>) -> Self {
        Self {
            proto: "ssl".to_string(),
            address: address.to_string(),
            tls_config: Some(tls_config),
            ..Self::default()
        }
    }

    /// Set max concurrent invokes
    pub fn with_max_invoke(mut self, max: i32) -> Self {
        self.max_invoke = max;
        self
    }

    /// Set accept timeout
    pub fn with_accept_timeout(mut self, timeout: Duration) -> Self {
        self.accept_timeout = timeout;
        self
    }

    /// Set read timeout
    pub fn with_read_timeout(mut self, timeout: Duration) -> Self {
        self.read_timeout = timeout;
        self
    }

    /// Set write timeout
    pub fn with_write_timeout(mut self, timeout: Duration) -> Self {
        self.write_timeout = timeout;
        self
    }

    /// Set handle timeout
    pub fn with_handle_timeout(mut self, timeout: Duration) -> Self {
        self.handle_timeout = timeout;
        self
    }

    /// Set idle timeout
    pub fn with_idle_timeout(mut self, timeout: Duration) -> Self {
        self.idle_timeout = timeout;
        self
    }

    /// Set queue capacity
    pub fn with_queue_cap(mut self, cap: usize) -> Self {
        self.queue_cap = cap;
        self
    }

    /// Set TCP no delay
    pub fn with_tcp_no_delay(mut self, no_delay: bool) -> Self {
        self.tcp_no_delay = no_delay;
        self
    }

    /// Check if TCP
    pub fn is_tcp(&self) -> bool {
        self.proto == "tcp"
    }

    /// Check if UDP
    pub fn is_udp(&self) -> bool {
        self.proto == "udp"
    }

    /// Check if SSL
    pub fn is_ssl(&self) -> bool {
        self.proto == "ssl"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_config_default() {
        let config = TarsClientConfig::default();
        assert!(config.is_tcp());
        assert_eq!(config.queue_len, 10000);
    }

    #[test]
    fn test_client_config_builder() {
        let config = TarsClientConfig::tcp()
            .with_queue_len(5000)
            .with_idle_timeout(Duration::from_secs(300));

        assert!(config.is_tcp());
        assert_eq!(config.queue_len, 5000);
        assert_eq!(config.idle_timeout, Duration::from_secs(300));
    }

    #[test]
    fn test_server_config_default() {
        let config = TarsServerConfig::tcp("0.0.0.0:10000");
        assert!(config.is_tcp());
        assert_eq!(config.address, "0.0.0.0:10000");
    }

    #[test]
    fn test_server_config_builder() {
        let config = TarsServerConfig::tcp("0.0.0.0:10000")
            .with_max_invoke(100000)
            .with_tcp_no_delay(true);

        assert_eq!(config.max_invoke, 100000);
        assert!(config.tcp_no_delay);
    }
}
