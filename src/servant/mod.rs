//! # Servant Module
//!
//! ServantProxy is the client-side RPC proxy for calling remote services.

use std::sync::Arc;
use std::sync::atomic::{AtomicI32, AtomicI64, Ordering};
use std::time::Duration;
use std::collections::HashMap;
use parking_lot::RwLock;

use crate::{Result, TarsError, Endpoint};
use crate::protocol::{RequestPacket, ResponsePacket, TarsProtocol};
use crate::selector::{Selector, HashType, create_selector};
use crate::adapter::AdapterProxy;
use crate::transport::TarsClientConfig;
use crate::filter::Message;
use crate::util::Context;
use crate::consts;

/// Global request ID counter
static REQUEST_ID: AtomicI32 = AtomicI32::new(0);

/// Generate unique request ID
fn gen_request_id() -> i32 {
    loop {
        let current = REQUEST_ID.load(Ordering::SeqCst);
        let next = if current >= i32::MAX - 1 { 1 } else { current + 1 };
        if REQUEST_ID
            .compare_exchange(current, next, Ordering::SeqCst, Ordering::SeqCst)
            .is_ok()
        {
            if next != 0 {
                return next;
            }
        }
    }
}

/// ServantProxy for client-side RPC calls
pub struct ServantProxy {
    /// Service name (e.g., "Test.HelloServer.HelloObj")
    name: String,
    /// Protocol handler
    #[allow(dead_code)]
    protocol: Arc<TarsProtocol>,
    /// Endpoint selector
    selector: Arc<dyn Selector>,
    /// Active adapters
    adapters: RwLock<HashMap<Endpoint, Arc<AdapterProxy>>>,
    /// Active endpoints
    active_endpoints: RwLock<Vec<Endpoint>>,
    /// Timeout in milliseconds
    timeout: AtomicI64,
    /// Protocol version
    version: i16,
    /// Queue length
    queue_len: AtomicI32,
    /// Client config
    client_config: TarsClientConfig,
}

impl ServantProxy {
    /// Create a new ServantProxy
    pub fn new(name: &str, endpoints: Vec<Endpoint>, config: TarsClientConfig) -> Self {
        let selector = create_selector("roundrobin");
        selector.refresh(endpoints.clone());

        let proxy = Self {
            name: name.to_string(),
            protocol: Arc::new(TarsProtocol::new()),
            selector,
            adapters: RwLock::new(HashMap::new()),
            active_endpoints: RwLock::new(endpoints.clone()),
            timeout: AtomicI64::new(consts::DEFAULT_ASYNC_TIMEOUT as i64),
            version: consts::TARS_VERSION,
            queue_len: AtomicI32::new(0),
            client_config: config,
        };

        // Initialize adapters
        for ep in endpoints {
            proxy.get_or_create_adapter(&ep);
        }

        proxy
    }

    /// Get service name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Set timeout in milliseconds
    pub fn set_timeout(&self, timeout_ms: u64) {
        self.timeout.store(timeout_ms as i64, Ordering::SeqCst);
    }

    /// Get timeout
    pub fn timeout(&self) -> Duration {
        Duration::from_millis(self.timeout.load(Ordering::SeqCst) as u64)
    }

    /// Refresh endpoints
    pub fn refresh_endpoints(&self, endpoints: Vec<Endpoint>) {
        self.selector.refresh(endpoints.clone());

        let mut adapters = self.adapters.write();
        let mut active = self.active_endpoints.write();

        // Remove old adapters
        let new_set: std::collections::HashSet<_> = endpoints.iter().collect();
        adapters.retain(|ep, adapter| {
            if new_set.contains(ep) {
                true
            } else {
                adapter.close();
                false
            }
        });

        // Add new adapters
        for ep in &endpoints {
            if !adapters.contains_key(ep) {
                let adapter = AdapterProxy::new(ep.clone(), self.client_config.clone());
                adapters.insert(ep.clone(), adapter);
            }
        }

        *active = endpoints;
    }

    /// Get or create adapter for endpoint
    fn get_or_create_adapter(&self, endpoint: &Endpoint) -> Arc<AdapterProxy> {
        let adapters = self.adapters.read();
        if let Some(adapter) = adapters.get(endpoint) {
            return Arc::clone(adapter);
        }
        drop(adapters);

        let mut adapters = self.adapters.write();
        if let Some(adapter) = adapters.get(endpoint) {
            return Arc::clone(adapter);
        }

        let adapter = AdapterProxy::new(endpoint.clone(), self.client_config.clone());
        adapters.insert(endpoint.clone(), Arc::clone(&adapter));
        adapter
    }

    /// Select an adapter for the request
    fn select_adapter(&self, msg: &Message) -> Result<Arc<AdapterProxy>> {
        let endpoint = self.selector.select(msg)?;
        Ok(self.get_or_create_adapter(&endpoint))
    }

    /// Invoke a remote method
    pub async fn invoke(
        &self,
        ctx: Context,
        func_name: &str,
        buffer: Vec<u8>,
        status: HashMap<String, String>,
        context: HashMap<String, String>,
    ) -> Result<ResponsePacket> {
        let mut msg = Message::new();

        // Build request
        msg.req.i_version = self.version;
        msg.req.c_packet_type = consts::TARS_NORMAL;
        msg.req.i_request_id = gen_request_id();
        msg.req.s_servant_name = self.name.clone();
        msg.req.s_func_name = func_name.to_string();
        msg.req.s_buffer = buffer;
        msg.req.i_timeout = self.timeout.load(Ordering::SeqCst) as i32;
        msg.req.status = status;
        msg.req.context = context;

        // Handle dyeing
        if let Some(dye_key) = ctx.dyeing_key() {
            msg.req
                .status
                .insert(consts::STATUS_DYED_KEY.to_string(), dye_key.to_string());
            msg.req.add_message_type(consts::TARS_MESSAGE_TYPE_DYED);
        }

        // Handle tracing
        if let Some(trace_key) = ctx.trace_key() {
            msg.req
                .status
                .insert(consts::STATUS_TRACE_KEY.to_string(), trace_key.to_string());
            msg.req.add_message_type(consts::TARS_MESSAGE_TYPE_TRACE);
        }

        self.do_invoke(ctx, msg).await
    }

    /// Invoke with oneway (no response)
    pub async fn invoke_oneway(
        &self,
        _ctx: Context,
        func_name: &str,
        buffer: Vec<u8>,
        status: HashMap<String, String>,
        context: HashMap<String, String>,
    ) -> Result<()> {
        let mut req = RequestPacket::new();
        req.i_version = self.version;
        req.c_packet_type = consts::TARS_ONEWAY;
        req.i_request_id = gen_request_id();
        req.s_servant_name = self.name.clone();
        req.s_func_name = func_name.to_string();
        req.s_buffer = buffer;
        req.i_timeout = self.timeout.load(Ordering::SeqCst) as i32;
        req.status = status;
        req.context = context;

        let msg = Message::with_request(req);
        let adapter = self.select_adapter(&msg)?;
        adapter.send(&msg.req).await?;
        adapter.success_add();

        Ok(())
    }

    /// Invoke with hash routing
    pub async fn invoke_hash(
        &self,
        ctx: Context,
        func_name: &str,
        buffer: Vec<u8>,
        hash_code: u32,
        hash_type: HashType,
    ) -> Result<ResponsePacket> {
        let mut msg = Message::new();

        msg.req.i_version = self.version;
        msg.req.c_packet_type = consts::TARS_NORMAL;
        msg.req.i_request_id = gen_request_id();
        msg.req.s_servant_name = self.name.clone();
        msg.req.s_func_name = func_name.to_string();
        msg.req.s_buffer = buffer;
        msg.req.i_timeout = self.timeout.load(Ordering::SeqCst) as i32;

        msg.is_hash = true;
        msg.hash_code = hash_code;
        msg.hash_type = hash_type;

        self.do_invoke(ctx, msg).await
    }

    /// Internal invoke implementation
    async fn do_invoke(&self, mut ctx: Context, msg: Message) -> Result<ResponsePacket> {
        // Check queue limit
        let queue_len = self.queue_len.fetch_add(1, Ordering::SeqCst);
        if queue_len > DEFAULT_OBJ_QUEUE_MAX {
            self.queue_len.fetch_sub(1, Ordering::SeqCst);
            return Err(TarsError::QueueFull);
        }

        // Set timeout context
        let timeout = self.timeout();
        ctx.set_timeout(timeout);

        // Select adapter
        let adapter = match self.select_adapter(&msg) {
            Ok(adp) => adp,
            Err(e) => {
                self.queue_len.fetch_sub(1, Ordering::SeqCst);
                return Err(e);
            }
        };

        // Update context with server info
        ctx.set_server_ip(adapter.endpoint().host.clone());
        ctx.set_server_port(adapter.endpoint().port as u16);

        // Register response channel
        let request_id = msg.req.i_request_id;
        let rx = adapter.register_response(request_id);

        // Send request
        if let Err(e) = adapter.send(&msg.req).await {
            adapter.unregister_response(request_id);
            adapter.fail_add();
            self.queue_len.fetch_sub(1, Ordering::SeqCst);
            return Err(e);
        }

        // Wait for response
        let result = tokio::time::timeout(timeout, rx).await;

        self.queue_len.fetch_sub(1, Ordering::SeqCst);
        adapter.unregister_response(request_id);

        match result {
            Ok(Ok(resp)) => {
                adapter.success_add();
                if resp.is_success() {
                    Ok(resp)
                } else {
                    Err(TarsError::ServerError {
                        code: resp.i_ret,
                        message: resp.s_result_desc,
                    })
                }
            }
            Ok(Err(_)) => {
                adapter.fail_add();
                Err(TarsError::ConnectionClosed)
            }
            Err(_) => {
                adapter.fail_add();
                Err(TarsError::Timeout(timeout.as_millis() as u64))
            }
        }
    }
}

/// Default max queue size per object
const DEFAULT_OBJ_QUEUE_MAX: i32 = 10000;

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

    #[tokio::test]
    async fn test_servant_proxy_creation() {
        let endpoints = vec![Endpoint::tcp("127.0.0.1", 10000)];
        let config = TarsClientConfig::tcp();
        let proxy = ServantProxy::new("Test.HelloServer.HelloObj", endpoints, config);

        assert_eq!(proxy.name(), "Test.HelloServer.HelloObj");
    }

    #[tokio::test]
    async fn test_servant_proxy_timeout() {
        let endpoints = vec![Endpoint::tcp("127.0.0.1", 10000)];
        let config = TarsClientConfig::tcp();
        let proxy = ServantProxy::new("Test.HelloServer.HelloObj", endpoints, config);

        proxy.set_timeout(5000);
        assert_eq!(proxy.timeout(), Duration::from_millis(5000));
    }
}
