//! # tars-util
//!
//! Configs, request-id generator, byte helpers and load-balancing selectors.

pub mod selector;
pub mod util;

pub use selector::{create_selector, HashType, Selector};
pub use util::{
    bytes_to_int8_slice, gen_request_id, int8_slice_to_bytes, ClientConfig, ServerConfig,
};
