//! # Adapter Module
//!
//! AdapterProxy manages a connection to a single service endpoint.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI32, AtomicI64, Ordering};
use dashmap::DashMap;
use tokio::sync::oneshot;
use tracing::debug;

use crate::{Endpoint, Result};
use crate::protocol::{RequestPacket, ResponsePacket, Protocol, TarsProtocol};
use crate::transport::{TarsClient, TarsClientConfig, ClientProtocol};
use crate::codec::PackageStatus;
use crate::consts;

/// AdapterProxy manages connection to a single endpoint
pub struct AdapterProxy {
    /// Endpoint information
    endpoint: Endpoint,
    /// Transport client
    client: Arc<TarsClient>,
    /// Protocol handler
    protocol: Arc<TarsProtocol>,
    /// Response channels: request_id -> response sender
    responses: DashMap<i32, oneshot::Sender<ResponsePacket>>,
    /// Fail count
    fail_count: AtomicI32,
    /// Last fail count (consecutive)
    last_fail_count: AtomicI32,
    /// Send count
    send_count: AtomicI32,
    /// Success count
    success_count: AtomicI32,
    /// Last success time (unix seconds)
    last_success_time: AtomicI64,
    /// Last block time
    last_block_time: AtomicI64,
    /// Last check time
    last_check_time: AtomicI64,
    /// Status: true = active
    status: AtomicBool,
    /// Closed flag
    closed: AtomicBool,
    /// Push callback
    push_callback: Option<Box<dyn Fn(Vec<u8>) + Send + Sync>>,
}

impl AdapterProxy {
    /// Create a new AdapterProxy
    pub fn new(endpoint: Endpoint, config: TarsClientConfig) -> Arc<Self> {
        let protocol = Arc::new(TarsProtocol::new());
        let address = endpoint.address();

        let adapter = Arc::new(Self {
            endpoint,
            client: TarsClient::new(
                &address,
                Arc::new(AdapterProtocolHandler::new()),
                config,
            ),
            protocol,
            responses: DashMap::new(),
            fail_count: AtomicI32::new(0),
            last_fail_count: AtomicI32::new(0),
            send_count: AtomicI32::new(0),
            success_count: AtomicI32::new(0),
            last_success_time: AtomicI64::new(now_secs()),
            last_block_time: AtomicI64::new(now_secs()),
            last_check_time: AtomicI64::new(now_secs()),
            status: AtomicBool::new(true),
            closed: AtomicBool::new(false),
            push_callback: None,
        });

        adapter
    }

    /// Get endpoint
    pub fn endpoint(&self) -> &Endpoint {
        &self.endpoint
    }

    /// Check if adapter is active
    pub fn is_active(&self) -> bool {
        self.status.load(Ordering::SeqCst)
    }

    /// Check if adapter is closed
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    /// Send a request
    pub async fn send(&self, req: &RequestPacket) -> Result<()> {
        self.send_count.fetch_add(1, Ordering::SeqCst);

        let data = self.protocol.request_pack(req)?;
        self.client.send(data).await?;

        Ok(())
    }

    /// Register response channel
    pub fn register_response(&self, request_id: i32) -> oneshot::Receiver<ResponsePacket> {
        let (tx, rx) = oneshot::channel();
        self.responses.insert(request_id, tx);
        rx
    }

    /// Unregister response channel
    pub fn unregister_response(&self, request_id: i32) {
        self.responses.remove(&request_id);
    }

    /// Handle received response
    pub fn handle_response(&self, response: ResponsePacket) {
        if response.i_request_id == 0 {
            // Server push
            self.handle_push(&response);
            return;
        }

        if let Some((_, tx)) = self.responses.remove(&response.i_request_id) {
            let _ = tx.send(response);
        } else {
            debug!("No handler for request {}", response.i_request_id);
        }
    }

    /// Handle server push
    fn handle_push(&self, response: &ResponsePacket) {
        if response.s_result_desc == consts::RECONNECT_MSG {
            debug!("Received reconnect message");
            // TODO: Handle reconnect
            return;
        }

        if let Some(ref callback) = self.push_callback {
            callback(response.s_buffer.clone());
        }
    }

    /// Record success
    pub fn success_add(&self) {
        self.last_success_time.store(now_secs(), Ordering::SeqCst);
        self.success_count.fetch_add(1, Ordering::SeqCst);
        self.last_fail_count.store(0, Ordering::SeqCst);
    }

    /// Record failure
    pub fn fail_add(&self) {
        self.last_fail_count.fetch_add(1, Ordering::SeqCst);
        self.fail_count.fetch_add(1, Ordering::SeqCst);
    }

    /// Check and update active status
    /// Returns (first_time_inactive, need_check)
    pub fn check_active(&self) -> (bool, bool) {
        if self.closed.load(Ordering::SeqCst) {
            return (false, false);
        }

        let now = now_secs();

        if self.status.load(Ordering::SeqCst) {
            // Active status
            let last_success = self.last_success_time.load(Ordering::SeqCst);
            let last_fail_count = self.last_fail_count.load(Ordering::SeqCst);

            // Check consecutive failures within interval
            if (now - last_success) >= consts::FAIL_INTERVAL as i64
                && last_fail_count >= consts::FAIL_N
            {
                self.status.store(false, Ordering::SeqCst);
                self.last_block_time.store(now, Ordering::SeqCst);
                return (true, false);
            }

            // Periodic check
            let last_check = self.last_check_time.load(Ordering::SeqCst);
            if (now - last_check) >= consts::CHECK_TIME as i64 {
                self.last_check_time.store(now, Ordering::SeqCst);

                let fail_count = self.fail_count.load(Ordering::SeqCst);
                let send_count = self.send_count.load(Ordering::SeqCst);

                // Check failure ratio
                if fail_count >= consts::OVER_N
                    && send_count > 0
                    && (fail_count as f32 / send_count as f32) >= consts::FAIL_RATIO
                {
                    self.status.store(false, Ordering::SeqCst);
                    self.last_block_time.store(now, Ordering::SeqCst);
                    return (true, false);
                }
            }

            return (false, false);
        }

        // Inactive status - check if we should try to reactivate
        let last_block = self.last_block_time.load(Ordering::SeqCst);
        if (now - last_block) >= consts::TRY_TIME_INTERVAL as i64 {
            self.last_block_time.store(now, Ordering::SeqCst);
            return (false, true);
        }

        (false, false)
    }

    /// Reset statistics
    pub fn reset(&self) {
        let now = now_secs();
        self.send_count.store(0, Ordering::SeqCst);
        self.success_count.store(0, Ordering::SeqCst);
        self.fail_count.store(0, Ordering::SeqCst);
        self.last_fail_count.store(0, Ordering::SeqCst);
        self.last_block_time.store(now, Ordering::SeqCst);
        self.last_check_time.store(now, Ordering::SeqCst);
        self.status.store(true, Ordering::SeqCst);
    }

    /// Close the adapter
    pub fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
        self.client.close();
    }
}

/// Get current time in seconds
fn now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64
}

/// Protocol handler for adapter
struct AdapterProtocolHandler;

impl AdapterProtocolHandler {
    fn new() -> Self {
        Self
    }
}

impl ClientProtocol for AdapterProtocolHandler {
    fn parse_package(&self, buff: &[u8]) -> (usize, PackageStatus) {
        crate::codec::parse_package(buff)
    }

    fn recv(&self, _pkg: Vec<u8>) {
        // Response handling is done through the responses map
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_adapter_proxy_creation() {
        let endpoint = Endpoint::tcp("127.0.0.1", 10000);
        let config = TarsClientConfig::tcp();
        let adapter = AdapterProxy::new(endpoint, config);

        assert!(adapter.is_active());
        assert!(!adapter.is_closed());
    }

    #[tokio::test]
    async fn test_adapter_statistics() {
        let endpoint = Endpoint::tcp("127.0.0.1", 10000);
        let config = TarsClientConfig::tcp();
        let adapter = AdapterProxy::new(endpoint, config);

        adapter.success_add();
        adapter.success_add();
        adapter.fail_add();

        assert_eq!(adapter.success_count.load(Ordering::SeqCst), 2);
        assert_eq!(adapter.fail_count.load(Ordering::SeqCst), 1);
    }
}
