//! # tars-registry
//!
//! Service registry/adapter and remote logging/stat reporting.

pub mod adapter;
pub mod logger;
pub mod registry;
pub mod stat;

pub use adapter::AdapterProxy;
pub use logger::{LogLevel, RemoteLogConfig, RemoteTimeWriter, TarsLogger};
pub use registry::{
    DirectRegistrar, EndpointManager, NodeCircuitBreaker, Registrar, RegistryCircuitBreaker,
    TarsRegistry,
};
pub use stat::{CallTimer, StatConfig, StatReporter};
