//! # Transport Module
//!
//! This module handles low-level network communication for TCP/UDP/SSL connections.
//!
//! ## Components
//!
//! - **TarsClient**: Client-side connection management
//! - **TarsServer**: Server-side listener management
//! - **Connection**: Single TCP/UDP connection handling
//! - **TLS**: TLS utilities for secure connections

mod client;
mod server;
mod config;
mod simple_client;
pub mod tls;

pub use client::TarsClient;
pub use server::TarsServer;
pub use config::{TarsClientConfig, TarsServerConfig};
pub use simple_client::{SimpleTarsClient, AsyncSimpleTarsClient};
pub use tls::{
    load_certs, load_private_key,
    create_client_config, create_client_config_with_native_roots, create_insecure_client_config,
    create_mtls_client_config, create_server_config, create_mtls_server_config,
    create_tls_connector, create_tls_acceptor, parse_server_name,
};

use crate::codec::PackageStatus;

/// Client protocol interface for handling incoming data
pub trait ClientProtocol: Send + Sync {
    /// Parse package boundary
    /// Returns (package_length, status)
    fn parse_package(&self, buff: &[u8]) -> (usize, PackageStatus);

    /// Handle received package
    fn recv(&self, pkg: Vec<u8>);
}

/// Server protocol interface for request handling
#[async_trait::async_trait]
pub trait ServerProtocolHandler: Send + Sync {
    /// Parse package boundary
    fn parse_package(&self, buff: &[u8]) -> (usize, PackageStatus);

    /// Handle request and return response
    async fn invoke(&self, ctx: &mut crate::util::Context, pkg: &[u8]) -> Vec<u8>;

    /// Handle timeout
    fn invoke_timeout(&self, pkg: &[u8]) -> Vec<u8>;

    /// Get close message (for graceful shutdown)
    fn get_close_msg(&self) -> Vec<u8>;

    /// Handle connection close
    fn do_close(&self, ctx: &crate::util::Context);
}

/// Connection status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionStatus {
    /// Connection is active
    Active,
    /// Connection is idle
    Idle,
    /// Connection is closed
    Closed,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_connection_status() {
        let status = ConnectionStatus::Active;
        assert_eq!(status, ConnectionStatus::Active);
    }
}
