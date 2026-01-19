//! # Registry Module
//!
//! Service registration and discovery interface with full QueryF protocol support.

use async_trait::async_trait;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{debug, error, info, warn};

use crate::protocol::queryf::{
    EndpointF, decode_endpoint_list,
    QUERY_FIND_OBJECT_BY_ID_4_ALL, QUERY_FIND_OBJECT_BY_ID_IN_SAME_SET
};
use crate::protocol::RequestPacket;
use crate::codec::{Buffer, Reader};
use crate::transport::AsyncSimpleTarsClient;
use crate::{Endpoint, Result, TarsError};
use crate::endpoint::ServantInstance;

/// Registrar trait for service registration and discovery
#[async_trait]
pub trait Registrar: Send + Sync {
    /// Register a servant instance
    async fn register(&self, servant: &ServantInstance) -> Result<()>;

    /// Deregister a servant instance
    async fn deregister(&self, servant: &ServantInstance) -> Result<()>;

    /// Query servant endpoints by object ID
    /// Returns (active_endpoints, inactive_endpoints)
    async fn query_servant(&self, id: &str) -> Result<(Vec<Endpoint>, Vec<Endpoint>)>;

    /// Query servant endpoints by object ID and SET division
    async fn query_servant_by_set(&self, id: &str, set: &str) -> Result<(Vec<Endpoint>, Vec<Endpoint>)>;
}

/// Direct registrar implementation (no service discovery)
pub struct DirectRegistrar {
    endpoints: Vec<Endpoint>,
}

impl DirectRegistrar {
    pub fn new(endpoints: Vec<Endpoint>) -> Self {
        Self { endpoints }
    }
}

#[async_trait]
impl Registrar for DirectRegistrar {
    async fn register(&self, _servant: &ServantInstance) -> Result<()> {
        // Direct mode doesn't support registration
        Ok(())
    }

    async fn deregister(&self, _servant: &ServantInstance) -> Result<()> {
        // Direct mode doesn't support deregistration
        Ok(())
    }

    async fn query_servant(&self, _id: &str) -> Result<(Vec<Endpoint>, Vec<Endpoint>)> {
        Ok((self.endpoints.clone(), vec![]))
    }

    async fn query_servant_by_set(&self, _id: &str, _set: &str) -> Result<(Vec<Endpoint>, Vec<Endpoint>)> {
        Ok((self.endpoints.clone(), vec![]))
    }
}

/// Tars registry client (communicates with tars.tarsregistry.QueryObj)
pub struct TarsRegistry {
    /// Locator string (e.g., "tars.tarsregistry.QueryObj@tcp -h 127.0.0.1 -p 17890")
    locator: String,
    /// Cached client connection
    client: RwLock<Option<AsyncSimpleTarsClient>>,
    /// Query timeout in milliseconds
    timeout: i32,
}

impl TarsRegistry {
    pub fn new(locator: &str) -> Self {
        Self {
            locator: locator.to_string(),
            client: RwLock::new(None),
            timeout: 5000,
        }
    }

    pub fn with_timeout(mut self, timeout_ms: i32) -> Self {
        self.timeout = timeout_ms;
        self
    }

    pub fn locator(&self) -> &str {
        &self.locator
    }

    /// Parse locator string to get endpoint
    fn parse_locator(&self) -> Option<String> {
        // Parse format: "ServiceName@tcp -h host -p port"
        let parts: Vec<&str> = self.locator.split('@').collect();
        if parts.len() < 2 {
            return None;
        }

        let endpoint_str = parts[1];
        let mut host = "127.0.0.1";
        let mut port = "17890";

        let tokens: Vec<&str> = endpoint_str.split_whitespace().collect();
        let mut i = 0;
        while i < tokens.len() {
            match tokens[i] {
                "-h" if i + 1 < tokens.len() => {
                    host = tokens[i + 1];
                    i += 2;
                }
                "-p" if i + 1 < tokens.len() => {
                    port = tokens[i + 1];
                    i += 2;
                }
                _ => i += 1,
            }
        }

        Some(format!("{}:{}", host, port))
    }

    /// Get or create client connection
    async fn get_client(&self) -> Result<Arc<AsyncSimpleTarsClient>> {
        // Check if we have a valid client
        {
            let guard = self.client.read().await;
            if let Some(ref client) = *guard {
                // Return a reference wrapped in Arc (we'll need to restructure)
                // For simplicity, we'll always create a new connection per query
            }
        }

        // Need to create a new client
        let addr = self.parse_locator()
            .ok_or_else(|| TarsError::Config("Invalid locator format".into()))?;

        let client = AsyncSimpleTarsClient::connect(&addr).await?;

        info!("Connected to registry: {}", addr);
        Ok(Arc::new(client))
    }

    /// Convert EndpointF to Endpoint
    fn convert_endpoint(epf: &EndpointF) -> Endpoint {
        let mut ep = if epf.istcp == 1 {
            Endpoint::tcp(&epf.host, epf.port as u16)
        } else if epf.istcp == 2 {
            Endpoint::ssl(&epf.host, epf.port as u16)
        } else {
            Endpoint::udp(&epf.host, epf.port as u16)
        };

        ep.timeout = epf.timeout as u64;
        ep.weight = epf.weight as u32;
        ep.set_id = epf.set_id.clone();
        ep
    }

    /// Query endpoints using findObjectById4All
    async fn do_query(&self, id: &str, func: &str, set: Option<&str>) -> Result<(Vec<Endpoint>, Vec<Endpoint>)> {
        let client = self.get_client().await?;

        // Build request body
        let mut body_buf = Buffer::new();
        body_buf.write_string(id, 1)?;  // id at tag 1

        if let Some(set_id) = set {
            body_buf.write_string(set_id, 2)?;  // setId at tag 2
        }

        // Build request packet
        let mut req = RequestPacket::new();
        req.s_servant_name = "tars.tarsregistry.QueryObj".to_string();
        req.s_func_name = func.to_string();
        req.s_buffer = body_buf.to_bytes();
        req.i_timeout = self.timeout;

        // Invoke
        let rsp = client.invoke(&req).await?;

        if rsp.i_ret != 0 {
            return Err(TarsError::ServerError {
                code: rsp.i_ret,
                message: rsp.s_result_desc.clone(),
            });
        }

        // Parse response
        let mut reader = Reader::new(&rsp.s_buffer);

        // Return value at tag 0
        let _ret = reader.read_int32(0, true)?;

        // Active endpoints at tag 2
        let active_epf = decode_endpoint_list(&mut reader, 2, true)?;

        // Inactive endpoints at tag 3
        let inactive_epf = decode_endpoint_list(&mut reader, 3, false)?;

        let active: Vec<Endpoint> = active_epf.iter().map(Self::convert_endpoint).collect();
        let inactive: Vec<Endpoint> = inactive_epf.iter().map(Self::convert_endpoint).collect();

        debug!("Query {} returned {} active, {} inactive endpoints",
               id, active.len(), inactive.len());

        Ok((active, inactive))
    }
}

#[async_trait]
impl Registrar for TarsRegistry {
    async fn register(&self, _servant: &ServantInstance) -> Result<()> {
        // Registration is typically handled by tarsnode
        // This is a placeholder for future implementation
        warn!("TarsRegistry::register is not yet implemented");
        Ok(())
    }

    async fn deregister(&self, _servant: &ServantInstance) -> Result<()> {
        // Deregistration is typically handled by tarsnode
        warn!("TarsRegistry::deregister is not yet implemented");
        Ok(())
    }

    async fn query_servant(&self, id: &str) -> Result<(Vec<Endpoint>, Vec<Endpoint>)> {
        self.do_query(id, QUERY_FIND_OBJECT_BY_ID_4_ALL, None).await
    }

    async fn query_servant_by_set(&self, id: &str, set: &str) -> Result<(Vec<Endpoint>, Vec<Endpoint>)> {
        self.do_query(id, QUERY_FIND_OBJECT_BY_ID_IN_SAME_SET, Some(set)).await
    }
}

/// Endpoint manager for caching and load balancing
pub struct EndpointManager {
    obj_name: String,
    registrar: Arc<dyn Registrar>,
    active_endpoints: RwLock<Vec<Endpoint>>,
    inactive_endpoints: RwLock<Vec<Endpoint>>,
    refresh_interval_ms: u64,
}

impl EndpointManager {
    pub fn new(obj_name: &str, registrar: Arc<dyn Registrar>) -> Self {
        Self {
            obj_name: obj_name.to_string(),
            registrar,
            active_endpoints: RwLock::new(vec![]),
            inactive_endpoints: RwLock::new(vec![]),
            refresh_interval_ms: 60_000,  // 60 seconds default
        }
    }

    pub fn with_refresh_interval(mut self, interval_ms: u64) -> Self {
        self.refresh_interval_ms = interval_ms;
        self
    }

    /// Refresh endpoints from registry
    pub async fn refresh(&self) -> Result<()> {
        let (active, inactive) = self.registrar.query_servant(&self.obj_name).await?;

        {
            let mut guard = self.active_endpoints.write().await;
            *guard = active;
        }
        {
            let mut guard = self.inactive_endpoints.write().await;
            *guard = inactive;
        }

        Ok(())
    }

    /// Get active endpoints
    pub async fn get_active(&self) -> Vec<Endpoint> {
        self.active_endpoints.read().await.clone()
    }

    /// Get inactive endpoints
    pub async fn get_inactive(&self) -> Vec<Endpoint> {
        self.inactive_endpoints.read().await.clone()
    }

    /// Start background refresh task
    pub fn start_refresh_task(self: Arc<Self>) {
        let interval = std::time::Duration::from_millis(self.refresh_interval_ms);
        let manager = self.clone();

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);
            loop {
                interval_timer.tick().await;
                if let Err(e) = manager.refresh().await {
                    error!("Failed to refresh endpoints for {}: {}", manager.obj_name, e);
                }
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_direct_registrar() {
        let endpoints = vec![
            Endpoint::tcp("127.0.0.1", 10000),
            Endpoint::tcp("127.0.0.1", 10001),
        ];
        let registrar = DirectRegistrar::new(endpoints);

        let (active, inactive) = registrar.query_servant("Test.HelloObj").await.unwrap();
        assert_eq!(active.len(), 2);
        assert!(inactive.is_empty());
    }

    #[test]
    fn test_parse_locator() {
        let registry = TarsRegistry::new("tars.tarsregistry.QueryObj@tcp -h 192.168.1.1 -p 17890");
        let addr = registry.parse_locator().unwrap();
        assert_eq!(addr, "192.168.1.1:17890");
    }

    #[test]
    fn test_convert_endpoint() {
        let epf = EndpointF {
            host: "10.0.0.1".to_string(),
            port: 8080,
            timeout: 3000,
            istcp: 1,
            grid: 0,
            groupworkid: 0,
            grouprealid: 0,
            set_id: "test.1.1".to_string(),
            qos: 0,
            bak_flag: 0,
            weight: 100,
            weight_type: 0,
            auth_type: 0,
        };

        let ep = TarsRegistry::convert_endpoint(&epf);
        assert_eq!(ep.host, "10.0.0.1");
        assert_eq!(ep.port, 8080);
        assert_eq!(ep.timeout, 3000);
        assert_eq!(ep.weight, 100);
        assert_eq!(ep.set_id, "test.1.1");
    }
}
