//! Request/Response context

use std::collections::HashMap;
use std::sync::Arc;
use parking_lot::RwLock;
use std::time::{Duration, Instant};

/// Context for request processing
#[derive(Debug, Clone)]
pub struct Context {
    /// Request start time
    start_time: Instant,
    /// Deadline for the request
    deadline: Option<Instant>,
    /// Key-value pairs
    values: Arc<RwLock<HashMap<String, String>>>,
    /// Server IP (for client-side context)
    server_ip: Option<String>,
    /// Server port
    server_port: Option<u16>,
    /// Client IP (for server-side context)
    client_ip: Option<String>,
    /// Client port
    client_port: Option<u16>,
    /// Dyeing key
    dyeing_key: Option<String>,
    /// Trace key
    trace_key: Option<String>,
    /// Packet type
    packet_type: i8,
    /// Receive package timestamp (ms)
    recv_pkg_ts: i64,
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

impl Context {
    /// Create a new context
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
            deadline: None,
            values: Arc::new(RwLock::new(HashMap::new())),
            server_ip: None,
            server_port: None,
            client_ip: None,
            client_port: None,
            dyeing_key: None,
            trace_key: None,
            packet_type: 0,
            recv_pkg_ts: chrono::Utc::now().timestamp_millis(),
        }
    }

    /// Create context with timeout
    pub fn with_timeout(timeout: Duration) -> Self {
        let mut ctx = Self::new();
        ctx.deadline = Some(Instant::now() + timeout);
        ctx
    }

    /// Create context with deadline
    pub fn with_deadline(deadline: Instant) -> Self {
        let mut ctx = Self::new();
        ctx.deadline = Some(deadline);
        ctx
    }

    /// Set timeout
    pub fn set_timeout(&mut self, timeout: Duration) {
        self.deadline = Some(Instant::now() + timeout);
    }

    /// Get remaining time until deadline
    pub fn remaining(&self) -> Option<Duration> {
        self.deadline.map(|d| {
            let now = Instant::now();
            if d > now {
                d - now
            } else {
                Duration::ZERO
            }
        })
    }

    /// Check if deadline has passed
    pub fn is_expired(&self) -> bool {
        self.deadline.map(|d| Instant::now() >= d).unwrap_or(false)
    }

    /// Get elapsed time since context creation
    pub fn elapsed(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Set a value
    pub fn set(&self, key: impl Into<String>, value: impl Into<String>) {
        self.values.write().insert(key.into(), value.into());
    }

    /// Get a value
    pub fn get(&self, key: &str) -> Option<String> {
        self.values.read().get(key).cloned()
    }

    /// Remove a value
    pub fn remove(&self, key: &str) -> Option<String> {
        self.values.write().remove(key)
    }

    /// Get all values
    pub fn values(&self) -> HashMap<String, String> {
        self.values.read().clone()
    }

    /// Set server IP
    pub fn set_server_ip(&mut self, ip: impl Into<String>) {
        self.server_ip = Some(ip.into());
    }

    /// Get server IP
    pub fn server_ip(&self) -> Option<&str> {
        self.server_ip.as_deref()
    }

    /// Set server port
    pub fn set_server_port(&mut self, port: u16) {
        self.server_port = Some(port);
    }

    /// Get server port
    pub fn server_port(&self) -> Option<u16> {
        self.server_port
    }

    /// Set client IP
    pub fn set_client_ip(&mut self, ip: impl Into<String>) {
        self.client_ip = Some(ip.into());
    }

    /// Get client IP
    pub fn client_ip(&self) -> Option<&str> {
        self.client_ip.as_deref()
    }

    /// Set client port
    pub fn set_client_port(&mut self, port: u16) {
        self.client_port = Some(port);
    }

    /// Get client port
    pub fn client_port(&self) -> Option<u16> {
        self.client_port
    }

    /// Set dyeing key
    pub fn set_dyeing_key(&mut self, key: impl Into<String>) {
        self.dyeing_key = Some(key.into());
    }

    /// Get dyeing key
    pub fn dyeing_key(&self) -> Option<&str> {
        self.dyeing_key.as_deref()
    }

    /// Check if request is dyed
    pub fn is_dyed(&self) -> bool {
        self.dyeing_key.is_some()
    }

    /// Set trace key
    pub fn set_trace_key(&mut self, key: impl Into<String>) {
        self.trace_key = Some(key.into());
    }

    /// Get trace key
    pub fn trace_key(&self) -> Option<&str> {
        self.trace_key.as_deref()
    }

    /// Check if request is traced
    pub fn is_traced(&self) -> bool {
        self.trace_key.is_some()
    }

    /// Set packet type
    pub fn set_packet_type(&mut self, packet_type: i8) {
        self.packet_type = packet_type;
    }

    /// Get packet type
    pub fn packet_type(&self) -> i8 {
        self.packet_type
    }

    /// Set receive package timestamp
    pub fn set_recv_pkg_ts(&mut self, ts: i64) {
        self.recv_pkg_ts = ts;
    }

    /// Get receive package timestamp
    pub fn recv_pkg_ts(&self) -> i64 {
        self.recv_pkg_ts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_context_timeout() {
        let ctx = Context::with_timeout(Duration::from_secs(1));
        assert!(!ctx.is_expired());
        assert!(ctx.remaining().unwrap() <= Duration::from_secs(1));
    }

    #[test]
    fn test_context_values() {
        let ctx = Context::new();
        ctx.set("key", "value");
        assert_eq!(ctx.get("key"), Some("value".to_string()));
        assert_eq!(ctx.get("nonexistent"), None);

        ctx.remove("key");
        assert_eq!(ctx.get("key"), None);
    }

    #[test]
    fn test_context_dyeing() {
        let mut ctx = Context::new();
        assert!(!ctx.is_dyed());

        ctx.set_dyeing_key("dye-123");
        assert!(ctx.is_dyed());
        assert_eq!(ctx.dyeing_key(), Some("dye-123"));
    }

    #[test]
    fn test_context_clone() {
        let ctx = Context::new();
        ctx.set("key", "value");

        let ctx2 = ctx.clone();
        assert_eq!(ctx2.get("key"), Some("value".to_string()));

        // Modifications to cloned context should be visible to original
        // because they share the same Arc<RwLock<>>
        ctx2.set("key2", "value2");
        assert_eq!(ctx.get("key2"), Some("value2".to_string()));
    }
}
