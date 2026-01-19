//! # Application Module
//!
//! Application lifecycle management for Tars services.

use std::sync::Arc;
use std::collections::HashMap;
use parking_lot::RwLock;
use tokio::sync::broadcast;
use tokio::signal;
use tracing::{info, error};

use crate::Result;
use crate::transport::{TarsServer, TarsServerConfig, ServerProtocolHandler};
use crate::util::{ServerConfig, ClientConfig};
use crate::communicator::Communicator;
use crate::filter::Filters;

/// Application state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppState {
    /// Not yet started
    Init,
    /// Running
    Running,
    /// Shutting down
    ShuttingDown,
    /// Stopped
    Stopped,
}

/// Application is the main entry point for Tars services
pub struct Application {
    /// Server configuration
    server_config: RwLock<ServerConfig>,
    /// Client configuration
    client_config: RwLock<ClientConfig>,
    /// Communicator
    communicator: Arc<Communicator>,
    /// Registered servers
    servers: RwLock<HashMap<String, Arc<TarsServer>>>,
    /// Object run list
    obj_run_list: RwLock<Vec<String>>,
    /// All filters
    filters: RwLock<Filters>,
    /// Application state
    state: RwLock<AppState>,
    /// Shutdown sender
    shutdown_tx: broadcast::Sender<()>,
}

impl Default for Application {
    fn default() -> Self {
        Self::new()
    }
}

impl Application {
    /// Create a new Application
    pub fn new() -> Self {
        let (shutdown_tx, _) = broadcast::channel(1);

        Self {
            server_config: RwLock::new(ServerConfig::default()),
            client_config: RwLock::new(ClientConfig::default()),
            communicator: Arc::new(Communicator::new()),
            servers: RwLock::new(HashMap::new()),
            obj_run_list: RwLock::new(Vec::new()),
            filters: RwLock::new(Filters::new()),
            state: RwLock::new(AppState::Init),
            shutdown_tx,
        }
    }

    /// Get server configuration
    pub fn server_config(&self) -> ServerConfig {
        self.server_config.read().clone()
    }

    /// Set server configuration
    pub fn set_server_config(&self, config: ServerConfig) {
        *self.server_config.write() = config;
    }

    /// Get client configuration
    pub fn client_config(&self) -> ClientConfig {
        self.client_config.read().clone()
    }

    /// Set client configuration
    pub fn set_client_config(&self, config: ClientConfig) {
        *self.client_config.write() = config;
    }

    /// Get communicator
    pub fn communicator(&self) -> Arc<Communicator> {
        Arc::clone(&self.communicator)
    }

    /// Get current state
    pub fn state(&self) -> AppState {
        *self.state.read()
    }

    /// Add a servant with protocol handler
    pub fn add_servant<H: ServerProtocolHandler + 'static>(
        &self,
        obj_name: &str,
        handler: H,
        address: &str,
    ) -> Result<()> {
        let server_config = self.server_config.read();

        let config = TarsServerConfig::tcp(address)
            .with_max_invoke(server_config.max_invoke)
            .with_accept_timeout(server_config.accept_timeout_duration())
            .with_read_timeout(server_config.read_timeout_duration())
            .with_write_timeout(server_config.write_timeout_duration())
            .with_handle_timeout(server_config.handle_timeout_duration())
            .with_idle_timeout(server_config.idle_timeout_duration())
            .with_queue_cap(server_config.queue_cap)
            .with_tcp_no_delay(server_config.tcp_no_delay);

        let server = TarsServer::new(Arc::new(handler), config);

        self.servers.write().insert(obj_name.to_string(), server);
        self.obj_run_list.write().push(obj_name.to_string());

        info!("Added servant: {} at {}", obj_name, address);
        Ok(())
    }

    /// Run the application
    pub async fn run(&self) -> Result<()> {
        *self.state.write() = AppState::Running;
        info!("Application starting...");

        // Start all servers
        let servers = self.servers.read().clone();
        let mut handles = Vec::new();

        for (name, server) in servers {
            info!("Starting server: {}", name);
            let handle = tokio::spawn(async move {
                if let Err(e) = server.serve().await {
                    error!("Server {} error: {}", name, e);
                }
            });
            handles.push(handle);
        }

        // Wait for shutdown signal
        self.wait_for_shutdown().await;

        // Graceful shutdown
        self.shutdown().await?;

        // Wait for all servers to stop
        for handle in handles {
            let _ = handle.await;
        }

        *self.state.write() = AppState::Stopped;
        info!("Application stopped");

        Ok(())
    }

    /// Wait for shutdown signal
    async fn wait_for_shutdown(&self) {
        let mut shutdown_rx = self.shutdown_tx.subscribe();

        tokio::select! {
            _ = signal::ctrl_c() => {
                info!("Received Ctrl+C, shutting down...");
            }
            _ = shutdown_rx.recv() => {
                info!("Received shutdown signal");
            }
        }
    }

    /// Shutdown the application gracefully
    pub async fn shutdown(&self) -> Result<()> {
        *self.state.write() = AppState::ShuttingDown;
        info!("Shutting down application...");

        // Shutdown all servers
        let servers = self.servers.read().clone();
        for (name, server) in servers {
            info!("Shutting down server: {}", name);
            server.shutdown().await;
        }

        Ok(())
    }

    /// Send shutdown signal
    pub fn signal_shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }

    /// Register client filter middleware
    pub fn use_client_filter_middleware(&self, middleware: crate::filter::ClientFilterMiddleware) {
        self.filters.write().use_client_middleware(middleware);
    }

    /// Register server filter middleware
    pub fn use_server_filter_middleware(&self, middleware: crate::filter::ServerFilterMiddleware) {
        self.filters.write().use_server_middleware(middleware);
    }
}

/// Builder for Application
pub struct ApplicationBuilder {
    app: Application,
}

impl Default for ApplicationBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl ApplicationBuilder {
    pub fn new() -> Self {
        Self {
            app: Application::new(),
        }
    }

    pub fn server_config(self, config: ServerConfig) -> Self {
        self.app.set_server_config(config);
        self
    }

    pub fn client_config(self, config: ClientConfig) -> Self {
        self.app.set_client_config(config);
        self
    }

    pub fn build(self) -> Application {
        self.app
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_application_creation() {
        let app = Application::new();
        assert_eq!(app.state(), AppState::Init);
    }

    #[test]
    fn test_application_builder() {
        let config = ServerConfig {
            app: "Test".to_string(),
            server: "HelloServer".to_string(),
            ..Default::default()
        };

        let app = ApplicationBuilder::new()
            .server_config(config)
            .build();

        assert_eq!(app.server_config().app, "Test");
        assert_eq!(app.server_config().server, "HelloServer");
    }

    #[test]
    fn test_application_communicator() {
        let app = Application::new();
        let comm = app.communicator();
        assert!(comm.locator().is_empty());
    }
}
