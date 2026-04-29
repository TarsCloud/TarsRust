//! # Tars RPC Framework for Rust
//!
//! Tars is a high-performance RPC framework that supports multiple programming languages.
//! This is the Rust implementation; the crate is a facade that re-exports the public API
//! from the underlying workspace members (`tars-core`, `tars-protocol`, `tars-util`,
//! `tars-transport`, `tars-registry`, `tars-rpc`).
//!
//! ## Quick Start
//!
//! ```rust,ignore
//! use tars::{Application, Communicator};
//!
//! #[tokio::main]
//! async fn main() {
//!     let comm = Communicator::new();
//!     // ...
//! }
//! ```

// Sub-module re-exports — preserve `tars::<module>::...` paths from before the split.
pub use tars_core::{codec, consts, error, Context};
pub use tars_protocol::{endpoint, protocol};
pub use tars_util::{selector, util};
pub use tars_transport::{filter, transport};
pub use tars_registry::{adapter, logger, registry, stat};
pub use tars_rpc::{application, communicator, servant};

// Flat re-exports — preserve `tars::Foo` paths from the previous src/lib.rs.
pub use tars_core::error::{Result, TarsError};
pub use tars_core::codec::{Buffer, Reader};
pub use tars_protocol::endpoint::Endpoint;
pub use tars_protocol::protocol::{
    EndpointF, LogInfo, PacketType, RequestPacket, ResponsePacket, StatInfo, StatMicMsgBody,
    StatMicMsgHead, TarsVersion,
};
pub use tars_util::selector::{HashType, Selector};
pub use tars_transport::filter::{
    ClientFilter, ClientFilterMiddleware, ServerFilter, ServerFilterMiddleware,
};
pub use tars_transport::transport::{TarsClient, TarsClientConfig, TarsServer, TarsServerConfig};
pub use tars_registry::adapter::AdapterProxy;
pub use tars_registry::logger::{LogLevel, RemoteLogConfig, RemoteTimeWriter, TarsLogger};
pub use tars_registry::registry::{
    DirectRegistrar, EndpointManager, NodeCircuitBreaker, Registrar, RegistryCircuitBreaker,
    TarsRegistry,
};
pub use tars_registry::stat::{CallTimer, StatConfig, StatReporter};
pub use tars_rpc::application::Application;
pub use tars_rpc::communicator::Communicator;
pub use tars_rpc::servant::ServantProxy;
