//! # Tars RPC Framework for Rust
//!
//! Tars is a high-performance RPC framework that supports multiple programming languages.
//! This is the Rust implementation providing the same functionality as other TARS language implementations.
//!
//! ## Architecture
//!
//! The framework is organized into the following layers:
//!
//! - **Application Layer**: Application lifecycle management, configuration parsing
//! - **Proxy Layer**: Service proxy, protocol handling, filters
//! - **Endpoint Layer**: Endpoint management, load balancing, service discovery
//! - **Transport Layer**: TCP/UDP connection management, data transmission
//! - **Protocol Layer**: Tars protocol encoding/decoding
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use tars::{Application, Communicator};
//!
//! #[tokio::main]
//! async fn main() {
//!     // Create communicator
//!     let comm = Communicator::new();
//!
//!     // Create proxy and call remote service
//!     // ...
//! }
//! ```

pub mod codec;
pub mod protocol;
pub mod endpoint;
pub mod selector;
pub mod transport;
pub mod registry;
pub mod adapter;
pub mod filter;
pub mod servant;
pub mod communicator;
pub mod application;
pub mod util;
pub mod logger;
pub mod stat;

// Re-export commonly used types
pub use codec::{Buffer, Reader};
pub use protocol::{RequestPacket, ResponsePacket, PacketType, TarsVersion};
pub use protocol::{EndpointF, LogInfo, StatMicMsgHead, StatMicMsgBody, StatInfo};
pub use endpoint::Endpoint;
pub use selector::{Selector, HashType};
pub use transport::{TarsClient, TarsServer, TarsClientConfig, TarsServerConfig};
pub use registry::{Registrar, TarsRegistry, DirectRegistrar, EndpointManager, RegistryCircuitBreaker, NodeCircuitBreaker};
pub use adapter::AdapterProxy;
pub use filter::{ClientFilter, ServerFilter, ClientFilterMiddleware, ServerFilterMiddleware};
pub use servant::ServantProxy;
pub use communicator::Communicator;
pub use application::Application;
pub use logger::{RemoteTimeWriter, RemoteLogConfig, TarsLogger, LogLevel};
pub use stat::{StatReporter, StatConfig, CallTimer};

/// Error types for the Tars framework
pub mod error {
    use thiserror::Error;

    #[derive(Error, Debug)]
    pub enum TarsError {
        #[error("Codec error: {0}")]
        Codec(String),

        #[error("Protocol error: {0}")]
        Protocol(String),

        #[error("Transport error: {0}")]
        Transport(#[from] std::io::Error),

        #[error("Timeout error: operation timed out after {0}ms")]
        Timeout(u64),

        #[error("No available endpoint")]
        NoEndpoint,

        #[error("Service not found: {0}")]
        ServiceNotFound(String),

        #[error("Server error: code={code}, message={message}")]
        ServerError { code: i32, message: String },

        #[error("Configuration error: {0}")]
        Config(String),

        #[error("Queue full")]
        QueueFull,

        #[error("Connection closed")]
        ConnectionClosed,

        #[error("Invalid argument: {0}")]
        InvalidArgument(String),
    }

    pub type Result<T> = std::result::Result<T, TarsError>;
}

pub use error::{TarsError, Result};

/// Constants used throughout the framework
pub mod consts {
    /// Protocol versions
    pub const TARS_VERSION: i16 = 1;
    pub const TUP_VERSION: i16 = 2;
    pub const JSON_VERSION: i16 = 3;

    /// Packet types
    pub const TARS_NORMAL: i8 = 0;
    pub const TARS_ONEWAY: i8 = 1;

    /// Message types
    pub const TARS_MESSAGE_TYPE_NULL: i32 = 0;
    pub const TARS_MESSAGE_TYPE_DYED: i32 = 4;
    pub const TARS_MESSAGE_TYPE_TRACE: i32 = 8;

    /// Return codes
    pub const TARS_SERVER_SUCCESS: i32 = 0;
    pub const TARS_SERVER_DECODE_ERR: i32 = -1;
    pub const TARS_SERVER_QUEUE_TIMEOUT: i32 = -2;
    pub const TARS_INVOKE_TIMEOUT: i32 = -3;
    pub const TARS_SERVER_UNKNOWN_ERR: i32 = -99;

    /// Transport protocols
    pub const PROTO_TCP: i32 = 1;
    pub const PROTO_UDP: i32 = 0;
    pub const PROTO_SSL: i32 = 2;

    /// Health check parameters
    pub const FAIL_INTERVAL: u64 = 5;      // seconds
    pub const FAIL_N: i32 = 5;             // consecutive failures
    pub const CHECK_TIME: u64 = 60;        // seconds
    pub const OVER_N: i32 = 2;             // minimum failures
    pub const FAIL_RATIO: f32 = 0.5;       // failure ratio threshold
    pub const TRY_TIME_INTERVAL: u64 = 30; // seconds

    /// Default timeouts (milliseconds)
    pub const DEFAULT_ASYNC_TIMEOUT: u64 = 3000;
    pub const DEFAULT_SYNC_TIMEOUT: u64 = 3000;
    pub const DEFAULT_CONNECT_TIMEOUT: u64 = 3000;
    pub const DEFAULT_IDLE_TIMEOUT: u64 = 600000;

    /// Queue limits
    pub const DEFAULT_QUEUE_LEN: usize = 10000;
    pub const DEFAULT_MAX_INVOKE: i32 = 200000;

    /// Max package length
    pub const MAX_PACKAGE_LENGTH: u32 = 100 * 1024 * 1024; // 100MB

    /// Reconnect message
    pub const RECONNECT_MSG: &str = "_reconnect_";

    /// Status keys
    pub const STATUS_DYED_KEY: &str = "STATUS_DYED_KEY";
    pub const STATUS_TRACE_KEY: &str = "STATUS_TRACE_KEY";

    /// Consistent hash virtual nodes
    pub const CON_HASH_VIRTUAL_NODES: usize = 100;
}
