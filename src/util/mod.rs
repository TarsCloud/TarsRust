//! # Utility Module
//!
//! Common utilities used across the framework.

mod context;
mod config;

pub use context::Context;
pub use config::*;

use std::sync::atomic::{AtomicI32, Ordering};

/// Global request ID generator
static REQUEST_ID: AtomicI32 = AtomicI32::new(0);

/// Generate a unique request ID
pub fn gen_request_id() -> i32 {
    loop {
        let current = REQUEST_ID.load(Ordering::SeqCst);
        let next = if current >= i32::MAX - 1 { 1 } else { current + 1 };

        if REQUEST_ID.compare_exchange(current, next, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
            if next != 0 {
                return next;
            }
        }
    }
}

/// Convert bytes to i8 slice (for compatibility with Go version)
pub fn bytes_to_int8_slice(data: &[u8]) -> Vec<i8> {
    data.iter().map(|&b| b as i8).collect()
}

/// Convert i8 slice to bytes
pub fn int8_slice_to_bytes(data: &[i8]) -> Vec<u8> {
    data.iter().map(|&b| b as u8).collect()
}

/// Parse endpoint string like "tcp -h 127.0.0.1 -p 10000 -t 3000"
pub fn parse_endpoint_string(s: &str) -> Option<crate::Endpoint> {
    use crate::endpoint::Endpoint;
    use crate::protocol::TransportProtocol;

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
pub fn parse_obj_name(obj_name: &str) -> (String, Vec<crate::Endpoint>) {
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
    fn test_gen_request_id() {
        let id1 = gen_request_id();
        let id2 = gen_request_id();
        assert_ne!(id1, id2);
        assert_ne!(id1, 0);
        assert_ne!(id2, 0);
    }

    #[test]
    fn test_bytes_conversion() {
        let bytes: Vec<u8> = vec![1, 2, 128, 255];
        let int8s = bytes_to_int8_slice(&bytes);
        let back = int8_slice_to_bytes(&int8s);
        assert_eq!(bytes, back);
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
