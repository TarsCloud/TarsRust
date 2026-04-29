//! # tars-rpc
//!
//! Top-level RPC abstractions: servant proxy, communicator, and application.

pub mod application;
pub mod communicator;
pub mod servant;

pub use application::Application;
pub use communicator::Communicator;
pub use servant::ServantProxy;
