//! # Communicator Module
//!
//! Communicator is the client-side communication manager.

use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use once_cell::sync::OnceCell;

use crate::{Result, TarsError, Endpoint};
use crate::servant::ServantProxy;
use crate::transport::TarsClientConfig;
use crate::util::{ClientConfig, parse_obj_name};

/// Global communicator instance
static GLOBAL_COMMUNICATOR: OnceCell<Arc<Communicator>> = OnceCell::new();

/// Communicator manages client-side communication
pub struct Communicator {
    /// Client configuration
    config: RwLock<ClientConfig>,
    /// Servant proxies cache
    proxies: RwLock<HashMap<String, Arc<ServantProxy>>>,
    /// Properties
    properties: RwLock<HashMap<String, String>>,
}

impl Default for Communicator {
    fn default() -> Self {
        Self::new()
    }
}

impl Communicator {
    /// Create a new Communicator
    pub fn new() -> Self {
        Self {
            config: RwLock::new(ClientConfig::default()),
            proxies: RwLock::new(HashMap::new()),
            properties: RwLock::new(HashMap::new()),
        }
    }

    /// Create with configuration
    pub fn with_config(config: ClientConfig) -> Self {
        let comm = Self::new();
        *comm.config.write() = config;
        comm
    }

    /// Get the global communicator instance
    pub fn global() -> Arc<Communicator> {
        GLOBAL_COMMUNICATOR
            .get_or_init(|| Arc::new(Communicator::new()))
            .clone()
    }

    /// Get client configuration
    pub fn config(&self) -> ClientConfig {
        self.config.read().clone()
    }

    /// Set locator
    pub fn set_locator(&self, locator: &str) {
        self.config.write().locator = locator.to_string();
        self.set_property("locator", locator);
    }

    /// Get locator
    pub fn locator(&self) -> String {
        self.config.read().locator.clone()
    }

    /// Set a property
    pub fn set_property(&self, key: &str, value: &str) {
        self.properties.write().insert(key.to_string(), value.to_string());
    }

    /// Get a property
    pub fn get_property(&self, key: &str) -> Option<String> {
        self.properties.read().get(key).cloned()
    }

    /// Create servant proxy from object name string
    ///
    /// # Arguments
    ///
    /// * `obj_name` - Object name, can be:
    ///   - "App.Server.Obj" - Query from registry
    ///   - "App.Server.Obj@tcp -h 127.0.0.1 -p 10000" - Direct connection
    ///
    /// # Returns
    ///
    /// Arc<ServantProxy> that can be used for RPC calls
    pub fn string_to_proxy(&self, obj_name: &str) -> Result<Arc<ServantProxy>> {
        if obj_name.is_empty() {
            return Err(TarsError::InvalidArgument("empty object name".into()));
        }

        // Check cache
        {
            let proxies = self.proxies.read();
            if let Some(proxy) = proxies.get(obj_name) {
                return Ok(Arc::clone(proxy));
            }
        }

        // Parse object name
        let (name, endpoints) = parse_obj_name(obj_name);

        let endpoints = if endpoints.is_empty() {
            // TODO: Query from registry
            // For now, return error if no direct endpoints
            return Err(TarsError::ServiceNotFound(name));
        } else {
            endpoints
        };

        // Create client config
        let config = self.config.read();
        let client_config = TarsClientConfig::tcp()
            .with_queue_len(config.queue_len)
            .with_idle_timeout(config.idle_timeout_duration())
            .with_read_timeout(config.read_timeout_duration())
            .with_write_timeout(config.write_timeout_duration())
            .with_dial_timeout(config.dial_timeout_duration());

        // Create proxy
        let proxy = Arc::new(ServantProxy::new(&name, endpoints, client_config));
        proxy.set_timeout(config.async_invoke_timeout);

        // Cache proxy
        self.proxies.write().insert(obj_name.to_string(), Arc::clone(&proxy));

        Ok(proxy)
    }

    /// Get or create servant proxy
    pub fn get_servant_proxy(&self, obj_name: &str) -> Result<Arc<ServantProxy>> {
        self.string_to_proxy(obj_name)
    }

    /// Refresh servant endpoints
    pub fn refresh_servant(&self, obj_name: &str, endpoints: Vec<Endpoint>) -> Result<()> {
        let proxies = self.proxies.read();
        if let Some(proxy) = proxies.get(obj_name) {
            proxy.refresh_endpoints(endpoints);
        }
        Ok(())
    }

    /// Calculate hash key for this communicator (for endpoint manager)
    pub fn hash_key(&self) -> String {
        use md5::{Md5, Digest};

        let mut hasher = Md5::new();
        let props = self.properties.read();

        for key in &["locator", "enableset", "setdivision"] {
            if let Some(value) = props.get(*key) {
                hasher.update(format!("{}:{}", key, value).as_bytes());
            }
        }

        hex::encode(hasher.finalize())
    }
}

/// Get the global communicator
pub fn get_communicator() -> Arc<Communicator> {
    Communicator::global()
}

/// Create a new communicator
pub fn new_communicator() -> Communicator {
    Communicator::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_communicator_creation() {
        let comm = Communicator::new();
        assert!(comm.locator().is_empty());
    }

    #[test]
    fn test_communicator_properties() {
        let comm = Communicator::new();
        comm.set_property("key", "value");
        assert_eq!(comm.get_property("key"), Some("value".to_string()));
        assert_eq!(comm.get_property("nonexistent"), None);
    }

    #[test]
    fn test_communicator_locator() {
        let comm = Communicator::new();
        comm.set_locator("tars.tarsregistry.QueryObj@tcp -h 127.0.0.1 -p 17890");
        assert!(comm.locator().contains("QueryObj"));
    }

    #[tokio::test]
    async fn test_string_to_proxy_direct() {
        let comm = Communicator::new();
        let result = comm.string_to_proxy("Test.HelloServer.HelloObj@tcp -h 127.0.0.1 -p 10000");
        assert!(result.is_ok());

        let proxy = result.unwrap();
        assert_eq!(proxy.name(), "Test.HelloServer.HelloObj");
    }

    #[test]
    fn test_string_to_proxy_empty() {
        let comm = Communicator::new();
        let result = comm.string_to_proxy("");
        assert!(result.is_err());
    }

    #[test]
    fn test_hash_key() {
        let comm = Communicator::new();
        comm.set_property("locator", "test");
        let key1 = comm.hash_key();

        comm.set_property("locator", "test2");
        let key2 = comm.hash_key();

        assert_ne!(key1, key2);
    }
}
