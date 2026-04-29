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
        parse_endpoint_string(s)
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

/// Parse endpoint string like "tcp -h 127.0.0.1 -p 10000 -t 3000"
pub fn parse_endpoint_string(s: &str) -> Option<Endpoint> {
    let parts: Vec<&str> = s.trim().split_whitespace().collect();
    if parts.is_empty() {
        return None;
    }

    let proto = match parts[0].to_lowercase().as_str() {
        "tcp" => TransportProtocol::Tcp,
        "udp" => TransportProtocol::Udp,
        "ssl" => TransportProtocol::Ssl,
        _ => return None,
    };

    let mut host = String::new();
    let mut port: u16 = 0;
    let mut timeout: u64 = 3000;

    let mut i = 1;
    while i < parts.len() {
        match parts[i] {
            "-h" if i + 1 < parts.len() => {
                host = parts[i + 1].to_string();
                i += 2;
            }
            "-p" if i + 1 < parts.len() => {
                port = parts[i + 1].parse().unwrap_or(0);
                i += 2;
            }
            "-t" if i + 1 < parts.len() => {
                timeout = parts[i + 1].parse().unwrap_or(3000);
                i += 2;
            }
            _ => {
                i += 1;
            }
        }
    }

    if host.is_empty() || port == 0 {
        return None;
    }

    Some(Endpoint {
        host,
        port,
        timeout,
        istcp: proto.as_i32(),
        ..Default::default()
    })
}

/// Parse object name with endpoints
/// Format: "App.Server.Obj" or "App.Server.Obj@tcp -h 127.0.0.1 -p 10000"
pub fn parse_obj_name(obj_name: &str) -> (String, Vec<Endpoint>) {
    let parts: Vec<&str> = obj_name.splitn(2, '@').collect();
    let name = parts[0].to_string();

    let endpoints = if parts.len() > 1 {
        parts[1]
            .split(':')
            .filter_map(|s| parse_endpoint_string(s.trim()))
            .collect()
    } else {
        Vec::new()
    };

    (name, endpoints)
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

    #[test]
    fn test_parse_endpoint_string() {
        let ep = parse_endpoint_string("tcp -h 127.0.0.1 -p 10000 -t 5000").unwrap();
        assert_eq!(ep.host, "127.0.0.1");
        assert_eq!(ep.port, 10000);
        assert_eq!(ep.timeout, 5000);
        assert_eq!(ep.istcp, 1); // TCP

        let ep = parse_endpoint_string("udp -h 192.168.1.1 -p 8080").unwrap();
        assert_eq!(ep.host, "192.168.1.1");
        assert_eq!(ep.port, 8080);
        assert_eq!(ep.istcp, 0); // UDP
    }

    #[test]
    fn test_parse_obj_name() {
        let (name, eps) = parse_obj_name("Test.HelloServer.HelloObj");
        assert_eq!(name, "Test.HelloServer.HelloObj");
        assert!(eps.is_empty());

        let (name, eps) = parse_obj_name("Test.HelloServer.HelloObj@tcp -h 127.0.0.1 -p 10000");
        assert_eq!(name, "Test.HelloServer.HelloObj");
        assert_eq!(eps.len(), 1);
        assert_eq!(eps[0].host, "127.0.0.1");
        assert_eq!(eps[0].port, 10000);

        let (name, eps) = parse_obj_name("Test.HelloServer.HelloObj@tcp -h 127.0.0.1 -p 10000:tcp -h 127.0.0.1 -p 10001");
        assert_eq!(name, "Test.HelloServer.HelloObj");
        assert_eq!(eps.len(), 2);
    }
}
