//! # Endpoint Module
//!
//! This module defines endpoint structures and endpoint management.

use std::fmt;
use std::hash::{Hash, Hasher};
use crate::protocol::TransportProtocol;

/// Weight type for load balancing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[repr(i16)]
pub enum WeightType {
    /// Round-robin (no weight)
    #[default]
    Loop = 0,
    /// Static weight
    StaticWeight = 1,
}

impl WeightType {
    pub fn from_i16(value: i16) -> Option<Self> {
        match value {
            0 => Some(WeightType::Loop),
            1 => Some(WeightType::StaticWeight),
            _ => None,
        }
    }

    pub fn as_i16(self) -> i16 {
        self as i16
    }
}

/// Service endpoint information
#[derive(Debug, Clone, Default)]
pub struct Endpoint {
    /// Host address
    pub host: String,
    /// Port number
    pub port: u16,
    /// Timeout in milliseconds
    pub timeout: u64,
    /// Transport type: 0=UDP, 1=TCP, 2=SSL
    pub istcp: i32,
    /// Grid identifier
    pub grid: i32,
    /// QoS level
    pub qos: i32,
    /// Weight value
    pub weight: u32,
    /// Weight type
    pub weight_type: i16,
    /// Auth type
    pub auth_type: i32,
    /// SET ID (format: setname.setarea.setgroup)
    pub set_id: String,
}

impl Endpoint {
    /// Create a new endpoint
    pub fn new(host: impl Into<String>, port: u16) -> Self {
        Self {
            host: host.into(),
            port,
            timeout: 3000,
            istcp: TransportProtocol::Tcp.as_i32(),
            grid: 0,
            qos: 0,
            weight: 100,
            weight_type: WeightType::Loop.as_i16(),
            auth_type: 0,
            set_id: String::new(),
        }
    }

    /// Create a TCP endpoint
    pub fn tcp(host: impl Into<String>, port: u16) -> Self {
        let mut ep = Self::new(host, port);
        ep.istcp = TransportProtocol::Tcp.as_i32();
        ep
    }

    /// Create a UDP endpoint
    pub fn udp(host: impl Into<String>, port: u16) -> Self {
        let mut ep = Self::new(host, port);
        ep.istcp = TransportProtocol::Udp.as_i32();
        ep
    }

    /// Create an SSL endpoint
    pub fn ssl(host: impl Into<String>, port: u16) -> Self {
        let mut ep = Self::new(host, port);
        ep.istcp = TransportProtocol::Ssl.as_i32();
        ep
    }

    /// Get transport protocol
    pub fn protocol(&self) -> TransportProtocol {
        TransportProtocol::from_i32(self.istcp).unwrap_or(TransportProtocol::Tcp)
    }

    /// Check if TCP
    pub fn is_tcp(&self) -> bool {
        self.istcp == TransportProtocol::Tcp.as_i32()
    }

    /// Check if UDP
    pub fn is_udp(&self) -> bool {
        self.istcp == TransportProtocol::Udp.as_i32()
    }

    /// Check if SSL
    pub fn is_ssl(&self) -> bool {
        self.istcp == TransportProtocol::Ssl.as_i32()
    }

    /// Get weight type
    pub fn get_weight_type(&self) -> WeightType {
        WeightType::from_i16(self.weight_type).unwrap_or(WeightType::Loop)
    }

    /// Check if static weight is enabled
    pub fn is_static_weight(&self) -> bool {
        self.get_weight_type() == WeightType::StaticWeight
    }

    /// Get address string "host:port"
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    /// Format as endpoint string
    pub fn to_endpoint_string(&self) -> String {
        format!(
            "{} -h {} -p {} -t {}",
            self.protocol(),
            self.host,
            self.port,
            self.timeout
        )
    }

    /// Parse from endpoint string
    pub fn from_string(s: &str) -> Option<Self> {
        crate::util::parse_endpoint_string(s)
    }
}

impl fmt::Display for Endpoint {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.to_endpoint_string())
    }
}

impl PartialEq for Endpoint {
    fn eq(&self, other: &Self) -> bool {
        self.host == other.host && self.port == other.port && self.istcp == other.istcp
    }
}

impl Eq for Endpoint {}

impl Hash for Endpoint {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.host.hash(state);
        self.port.hash(state);
        self.istcp.hash(state);
    }
}

/// Servant instance information (for service registration)
#[derive(Debug, Clone, Default)]
pub struct ServantInstance {
    /// Tars version
    pub tars_version: String,
    /// Application name
    pub app: String,
    /// Server name
    pub server: String,
    /// Enable SET routing
    pub enable_set: bool,
    /// SET division
    pub set_division: String,
    /// Protocol type
    pub protocol: String,
    /// Servant name
    pub servant: String,
    /// Endpoint
    pub endpoint: Endpoint,
}

impl ServantInstance {
    /// Create a new servant instance
    pub fn new(app: &str, server: &str, servant: &str, endpoint: Endpoint) -> Self {
        Self {
            tars_version: "1.0.0".to_string(),
            app: app.to_string(),
            server: server.to_string(),
            enable_set: false,
            set_division: String::new(),
            protocol: "tars".to_string(),
            servant: servant.to_string(),
            endpoint,
        }
    }

    /// Get full object name
    pub fn object_name(&self) -> String {
        format!("{}.{}.{}", self.app, self.server, self.servant)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_endpoint_new() {
        let ep = Endpoint::new("127.0.0.1", 10000);
        assert_eq!(ep.host, "127.0.0.1");
        assert_eq!(ep.port, 10000);
        assert!(ep.is_tcp());
    }

    #[test]
    fn test_endpoint_protocols() {
        let tcp = Endpoint::tcp("127.0.0.1", 10000);
        assert!(tcp.is_tcp());
        assert!(!tcp.is_udp());
        assert!(!tcp.is_ssl());

        let udp = Endpoint::udp("127.0.0.1", 10000);
        assert!(!udp.is_tcp());
        assert!(udp.is_udp());

        let ssl = Endpoint::ssl("127.0.0.1", 10000);
        assert!(!ssl.is_tcp());
        assert!(ssl.is_ssl());
    }

    #[test]
    fn test_endpoint_equality() {
        let ep1 = Endpoint::tcp("127.0.0.1", 10000);
        let ep2 = Endpoint::tcp("127.0.0.1", 10000);
        let ep3 = Endpoint::tcp("127.0.0.1", 10001);

        assert_eq!(ep1, ep2);
        assert_ne!(ep1, ep3);
    }

    #[test]
    fn test_endpoint_string() {
        let ep = Endpoint::tcp("127.0.0.1", 10000);
        let s = ep.to_endpoint_string();
        assert!(s.contains("tcp"));
        assert!(s.contains("127.0.0.1"));
        assert!(s.contains("10000"));
    }

    #[test]
    fn test_endpoint_address() {
        let ep = Endpoint::tcp("127.0.0.1", 10000);
        assert_eq!(ep.address(), "127.0.0.1:10000");
    }

    #[test]
    fn test_servant_instance() {
        let ep = Endpoint::tcp("127.0.0.1", 10000);
        let instance = ServantInstance::new("Test", "HelloServer", "HelloObj", ep);
        assert_eq!(instance.object_name(), "Test.HelloServer.HelloObj");
    }
}
