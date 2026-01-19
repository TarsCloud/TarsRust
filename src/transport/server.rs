//! Tars server transport implementation

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::time::Instant;
use std::net::SocketAddr;
use tokio::net::{TcpListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use parking_lot::Mutex;
use tracing::{debug, error, info, warn};

use crate::{Result, TarsError};
use crate::codec::PackageStatus;
use crate::util::Context;
use super::{TarsServerConfig, ServerProtocolHandler};

/// Tars server for handling incoming connections
pub struct TarsServer {
    /// Server configuration
    config: TarsServerConfig,
    /// Protocol handler
    protocol: Arc<dyn ServerProtocolHandler>,
    /// Close flag
    closed: AtomicBool,
    /// Number of active connections
    num_conn: AtomicI32,
    /// Number of active invokes
    num_invoke: AtomicI32,
    /// Last invoke time
    last_invoke: Mutex<Instant>,
}

impl TarsServer {
    /// Create a new TarsServer
    pub fn new(protocol: Arc<dyn ServerProtocolHandler>, config: TarsServerConfig) -> Arc<Self> {
        Arc::new(Self {
            config,
            protocol,
            closed: AtomicBool::new(false),
            num_conn: AtomicI32::new(0),
            num_invoke: AtomicI32::new(0),
            last_invoke: Mutex::new(Instant::now()),
        })
    }

    /// Start listening and serving
    pub async fn serve(self: Arc<Self>) -> Result<()> {
        let listener = TcpListener::bind(&self.config.address).await?;
        info!("Server listening on {}", self.config.address);

        loop {
            if self.closed.load(Ordering::SeqCst) {
                break;
            }

            match tokio::time::timeout(self.config.accept_timeout, listener.accept()).await {
                Ok(Ok((stream, addr))) => {
                    self.num_conn.fetch_add(1, Ordering::SeqCst);
                    let server = Arc::clone(&self);
                    tokio::spawn(async move {
                        if let Err(e) = server.handle_connection(stream, addr).await {
                            debug!("Connection error: {}", e);
                        }
                        server.num_conn.fetch_sub(1, Ordering::SeqCst);
                    });
                }
                Ok(Err(e)) => {
                    warn!("Accept error: {}", e);
                }
                Err(_) => {
                    // Timeout, continue
                    continue;
                }
            }
        }

        Ok(())
    }

    /// Handle a single connection
    async fn handle_connection(&self, stream: TcpStream, addr: SocketAddr) -> Result<()> {
        debug!("New connection from {}", addr);

        // Set TCP options
        stream.set_nodelay(self.config.tcp_no_delay)?;

        let (mut read_half, mut write_half) = stream.into_split();

        let mut buffer = vec![0u8; self.config.tcp_read_buffer];
        let mut accumulated = Vec::new();

        loop {
            if self.closed.load(Ordering::SeqCst) {
                // Send close message
                let close_msg = self.protocol.get_close_msg();
                let _ = write_half.write_all(&close_msg).await;
                break;
            }

            match tokio::time::timeout(self.config.read_timeout, read_half.read(&mut buffer)).await {
                Ok(Ok(0)) => {
                    debug!("Connection closed by {}", addr);
                    break;
                }
                Ok(Ok(n)) => {
                    accumulated.extend_from_slice(&buffer[..n]);

                    // Parse and handle complete packages
                    loop {
                        let (pkg_len, status) = self.protocol.parse_package(&accumulated);
                        match status {
                            PackageStatus::Full => {
                                let pkg: Vec<u8> = accumulated.drain(..pkg_len).collect();

                                // Check concurrent limit
                                if self.num_invoke.load(Ordering::SeqCst) >= self.config.max_invoke {
                                    warn!("Max invoke limit reached");
                                    continue;
                                }

                                self.num_invoke.fetch_add(1, Ordering::SeqCst);
                                *self.last_invoke.lock() = Instant::now();

                                // Handle request
                                let protocol = Arc::clone(&self.protocol);
                                let handle_timeout = self.config.handle_timeout;
                                let num_invoke = &self.num_invoke;

                                let mut ctx = Context::new();
                                ctx.set_client_ip(addr.ip().to_string());
                                ctx.set_client_port(addr.port());

                                let response = tokio::time::timeout(
                                    handle_timeout,
                                    protocol.invoke(&mut ctx, &pkg),
                                )
                                .await
                                .unwrap_or_else(|_| protocol.invoke_timeout(&pkg));

                                num_invoke.fetch_sub(1, Ordering::SeqCst);

                                // Send response
                                if !response.is_empty() {
                                    if let Err(e) = tokio::time::timeout(
                                        self.config.write_timeout,
                                        write_half.write_all(&response),
                                    )
                                    .await
                                    {
                                        error!("Write error: {:?}", e);
                                        break;
                                    }
                                }
                            }
                            PackageStatus::Less => break,
                            PackageStatus::Error => {
                                error!("Package parse error from {}", addr);
                                return Err(TarsError::Protocol("package parse error".into()));
                            }
                        }
                    }
                }
                Ok(Err(e)) => {
                    debug!("Read error from {}: {}", addr, e);
                    break;
                }
                Err(_) => {
                    // Read timeout - check idle
                    if self.last_invoke.lock().elapsed() > self.config.idle_timeout {
                        debug!("Connection idle from {}, closing", addr);
                        break;
                    }
                }
            }
        }

        // Call close handler
        let ctx = Context::new();
        self.protocol.do_close(&ctx);

        Ok(())
    }

    /// Check if server is closed
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    /// Get connection count
    pub fn connection_count(&self) -> i32 {
        self.num_conn.load(Ordering::SeqCst)
    }

    /// Get invoke count
    pub fn invoke_count(&self) -> i32 {
        self.num_invoke.load(Ordering::SeqCst)
    }

    /// Shutdown the server gracefully
    pub async fn shutdown(&self) {
        info!("Shutting down server...");
        self.closed.store(true, Ordering::SeqCst);

        // Wait for pending invokes to complete
        let timeout = std::time::Duration::from_secs(30);
        let start = Instant::now();

        while self.num_invoke.load(Ordering::SeqCst) > 0 {
            if start.elapsed() > timeout {
                warn!("Shutdown timeout, forcing close");
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        }

        info!("Server shutdown complete");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    struct MockHandler;

    #[async_trait::async_trait]
    impl ServerProtocolHandler for MockHandler {
        fn parse_package(&self, buff: &[u8]) -> (usize, PackageStatus) {
            if buff.len() < 4 {
                return (0, PackageStatus::Less);
            }
            let len = u32::from_be_bytes([buff[0], buff[1], buff[2], buff[3]]) as usize;
            if buff.len() >= len {
                (len, PackageStatus::Full)
            } else {
                (0, PackageStatus::Less)
            }
        }

        async fn invoke(&self, _ctx: &mut Context, _pkg: &[u8]) -> Vec<u8> {
            vec![0, 0, 0, 4]
        }

        fn invoke_timeout(&self, _pkg: &[u8]) -> Vec<u8> {
            vec![0, 0, 0, 4]
        }

        fn get_close_msg(&self) -> Vec<u8> {
            vec![]
        }

        fn do_close(&self, _ctx: &Context) {}
    }

    #[tokio::test]
    async fn test_server_creation() {
        let handler = Arc::new(MockHandler);
        let config = TarsServerConfig::tcp("127.0.0.1:0");
        let server = TarsServer::new(handler, config);

        assert!(!server.is_closed());
        assert_eq!(server.connection_count(), 0);
        assert_eq!(server.invoke_count(), 0);
    }
}
