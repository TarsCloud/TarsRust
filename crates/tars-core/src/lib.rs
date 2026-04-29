//! # tars-core
//!
//! Core types (error / Result / consts), request Context, and the Tars binary codec.

pub mod codec;
pub mod consts;
pub mod context;
pub mod error;

pub use context::Context;
pub use error::{Result, TarsError};
