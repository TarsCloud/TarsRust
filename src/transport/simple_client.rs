//! Simple synchronous-style RPC client for framework services
//!
//! This is a simplified client for registry, logging, and statistics services.

use std::io::{Read, Write};
use std::net::TcpStream;
use std::time::Duration;

use crate::{Result, TarsError};
use crate::protocol::{RequestPacket, ResponsePacket};

/// Simple Tars client for framework services
#[derive(Debug)]
pub struct SimpleTarsClient {
    address: String,
    stream: Option<TcpStream>,
    connect_timeout: Duration,
    read_timeout: Duration,
    write_timeout: Duration,
}

impl Clone for SimpleTarsClient {
    fn clone(&self) -> Self {
        Self {
            address: self.address.clone(),
            stream: None,  // Don't clone the connection
            connect_timeout: self.connect_timeout,
            read_timeout: self.read_timeout,
            write_timeout: self.write_timeout,
        }
    }
}

impl SimpleTarsClient {
    /// Create a new client and connect
    pub fn connect(address: &str) -> Result<Self> {
        Self::connect_with_timeout(address, Duration::from_secs(5))
    }

    /// Create a new client with custom timeout
    pub fn connect_with_timeout(address: &str, timeout: Duration) -> Result<Self> {
        let stream = TcpStream::connect_timeout(
            &address.parse().map_err(|e| TarsError::Config(format!("Invalid address: {}", e)))?,
            timeout,
        )?;

        stream.set_nodelay(true)?;
        stream.set_read_timeout(Some(Duration::from_secs(30)))?;
        stream.set_write_timeout(Some(Duration::from_secs(30)))?;

        Ok(Self {
            address: address.to_string(),
            stream: Some(stream),
            connect_timeout: timeout,
            read_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(30),
        })
    }

    /// Ensure connection is established
    fn ensure_connected(&mut self) -> Result<&mut TcpStream> {
        if self.stream.is_none() {
            let stream = TcpStream::connect_timeout(
                &self.address.parse().map_err(|e| TarsError::Config(format!("Invalid address: {}", e)))?,
                self.connect_timeout,
            )?;
            stream.set_nodelay(true)?;
            stream.set_read_timeout(Some(self.read_timeout))?;
            stream.set_write_timeout(Some(self.write_timeout))?;
            self.stream = Some(stream);
        }
        Ok(self.stream.as_mut().unwrap())
    }

    /// Invoke a remote method
    pub fn invoke(&mut self, req: &RequestPacket) -> Result<ResponsePacket> {
        self.ensure_connected()?;

        // Encode request
        let data = req.encode()?;

        // Send
        let send_result = self.stream.as_mut().unwrap().write_all(&data);
        if let Err(e) = send_result {
            self.stream = None;
            return Err(TarsError::Transport(e));
        }

        // Receive response header
        let mut header = [0u8; 4];
        let read_result = self.stream.as_mut().unwrap().read_exact(&mut header);
        if let Err(e) = read_result {
            self.stream = None;
            return Err(TarsError::Transport(e));
        }

        let pkg_len = u32::from_be_bytes(header) as usize;
        if pkg_len > 100 * 1024 * 1024 {
            self.stream = None;
            return Err(TarsError::Protocol("Package too large".into()));
        }

        // Receive response body
        let mut body = vec![0u8; pkg_len - 4];
        let read_result = self.stream.as_mut().unwrap().read_exact(&mut body);
        if let Err(e) = read_result {
            self.stream = None;
            return Err(TarsError::Transport(e));
        }

        // Combine header and body
        let mut full_packet = header.to_vec();
        full_packet.extend(body);

        // Decode response
        ResponsePacket::decode(&full_packet)
    }

    /// Send a one-way request (no response expected)
    pub fn send_oneway(&mut self, req: &RequestPacket) -> Result<()> {
        self.ensure_connected()?;

        // Encode request
        let data = req.encode()?;

        // Send
        let send_result = self.stream.as_mut().unwrap().write_all(&data);
        if let Err(e) = send_result {
            self.stream = None;
            return Err(TarsError::Transport(e));
        }

        Ok(())
    }

    /// Close the connection
    pub fn close(&mut self) {
        self.stream = None;
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.stream.is_some()
    }

    /// Get the server address
    pub fn address(&self) -> &str {
        &self.address
    }
}

/// Async wrapper for SimpleTarsClient
pub struct AsyncSimpleTarsClient {
    inner: tokio::sync::Mutex<SimpleTarsClient>,
}

impl AsyncSimpleTarsClient {
    /// Connect to address
    pub async fn connect(address: &str) -> Result<Self> {
        let addr = address.to_string();
        let client = tokio::task::spawn_blocking(move || {
            SimpleTarsClient::connect(&addr)
        })
        .await
        .map_err(|e| TarsError::Transport(std::io::Error::other(format!("Join error: {}", e))))??;

        Ok(Self {
            inner: tokio::sync::Mutex::new(client),
        })
    }

    /// Invoke a remote method
    pub async fn invoke(&self, req: &RequestPacket) -> Result<ResponsePacket> {
        let req_clone = req.clone();
        let mut guard = self.inner.lock().await;

        // Run blocking I/O in a separate thread
        let mut client = std::mem::replace(&mut *guard, SimpleTarsClient {
            address: String::new(),
            stream: None,
            connect_timeout: Duration::from_secs(5),
            read_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(30),
        });

        let (result, client) = tokio::task::spawn_blocking(move || {
            let result = client.invoke(&req_clone);
            (result, client)
        })
        .await
        .map_err(|e| TarsError::Transport(std::io::Error::other(format!("Join error: {}", e))))?;

        *guard = client;
        result
    }

    /// Send a one-way request
    pub async fn send_oneway(&self, req: &RequestPacket) -> Result<()> {
        let req_clone = req.clone();
        let mut guard = self.inner.lock().await;

        let mut client = std::mem::replace(&mut *guard, SimpleTarsClient {
            address: String::new(),
            stream: None,
            connect_timeout: Duration::from_secs(5),
            read_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(30),
        });

        let (result, client) = tokio::task::spawn_blocking(move || {
            let result = client.send_oneway(&req_clone);
            (result, client)
        })
        .await
        .map_err(|e| TarsError::Transport(std::io::Error::other(format!("Join error: {}", e))))?;

        *guard = client;
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_clone() {
        // Just test that clone compiles
        let client = SimpleTarsClient {
            address: "127.0.0.1:10000".to_string(),
            stream: None,
            connect_timeout: Duration::from_secs(5),
            read_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(30),
        };
        let _cloned = client.clone();
    }
}
