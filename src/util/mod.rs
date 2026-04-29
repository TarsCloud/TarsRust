//! # Utility Module
//!
//! Common utilities used across the framework.

mod config;

pub use config::*;

use std::sync::atomic::{AtomicI32, Ordering};

/// Global request ID generator
static REQUEST_ID: AtomicI32 = AtomicI32::new(0);

/// Generate a unique request ID
pub fn gen_request_id() -> i32 {
    loop {
        let current = REQUEST_ID.load(Ordering::SeqCst);
        let next = if current >= i32::MAX - 1 { 1 } else { current + 1 };

        if REQUEST_ID.compare_exchange(current, next, Ordering::SeqCst, Ordering::SeqCst).is_ok() {
            if next != 0 {
                return next;
            }
        }
    }
}

/// Convert bytes to i8 slice (for compatibility with Go version)
pub fn bytes_to_int8_slice(data: &[u8]) -> Vec<i8> {
    data.iter().map(|&b| b as i8).collect()
}

/// Convert i8 slice to bytes
pub fn int8_slice_to_bytes(data: &[i8]) -> Vec<u8> {
    data.iter().map(|&b| b as u8).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gen_request_id() {
        let id1 = gen_request_id();
        let id2 = gen_request_id();
        assert_ne!(id1, id2);
        assert_ne!(id1, 0);
        assert_ne!(id2, 0);
    }

    #[test]
    fn test_bytes_conversion() {
        let bytes: Vec<u8> = vec![1, 2, 128, 255];
        let int8s = bytes_to_int8_slice(&bytes);
        let back = int8_slice_to_bytes(&int8s);
        assert_eq!(bytes, back);
    }

}
