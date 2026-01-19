//! # Selector Module
//!
//! This module implements load balancing strategies for endpoint selection.
//!
//! ## Strategies
//!
//! - **Round Robin**: Default strategy, cycles through endpoints in order
//! - **Random**: Randomly selects an endpoint
//! - **Mod Hash**: Selects endpoint based on hash code modulo
//! - **Consistent Hash**: Uses consistent hashing with virtual nodes

mod roundrobin;
mod random;
mod modhash;
mod consistenthash;
mod weight;

pub use roundrobin::RoundRobin;
pub use random::Random;
pub use modhash::ModHash;
pub use consistenthash::ConsistentHash;
pub use weight::build_static_weight_list;

use crate::{Endpoint, Result};

/// Hash type for hash-based routing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum HashType {
    /// Mod hash (hash % node_count)
    #[default]
    ModHash,
    /// Consistent hash with virtual nodes
    ConsistentHash,
}

impl HashType {
    pub fn as_str(&self) -> &'static str {
        match self {
            HashType::ModHash => "ModHash",
            HashType::ConsistentHash => "ConsistentHash",
        }
    }
}

impl std::fmt::Display for HashType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Message trait for selector
pub trait Message {
    /// Get hash code for hash-based routing
    fn hash_code(&self) -> u32;
    /// Get hash type
    fn hash_type(&self) -> HashType;
    /// Check if this is a hash-based request
    fn is_hash(&self) -> bool;
}

/// Selector trait for load balancing
pub trait Selector: Send + Sync {
    /// Select an endpoint for the given message
    fn select(&self, msg: &dyn Message) -> Result<Endpoint>;

    /// Refresh endpoint list
    fn refresh(&self, nodes: Vec<Endpoint>);

    /// Add an endpoint
    fn add(&self, node: Endpoint) -> Result<()>;

    /// Remove an endpoint
    fn remove(&self, node: &Endpoint) -> Result<()>;

    /// Get all endpoints
    fn all(&self) -> Vec<Endpoint>;

    /// Get endpoint count
    fn len(&self) -> usize;

    /// Check if empty
    fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// Default message implementation for testing
#[derive(Debug, Clone, Default)]
pub struct DefaultMessage {
    pub hash_code: u32,
    pub hash_type: HashType,
    pub is_hash: bool,
}

impl Message for DefaultMessage {
    fn hash_code(&self) -> u32 {
        self.hash_code
    }

    fn hash_type(&self) -> HashType {
        self.hash_type
    }

    fn is_hash(&self) -> bool {
        self.is_hash
    }
}

impl DefaultMessage {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_hash(hash_code: u32, hash_type: HashType) -> Self {
        Self {
            hash_code,
            hash_type,
            is_hash: true,
        }
    }
}

use std::sync::Arc;

/// Create a selector by type
pub fn create_selector(selector_type: &str) -> Arc<dyn Selector> {
    match selector_type.to_lowercase().as_str() {
        "roundrobin" | "rr" => Arc::new(RoundRobin::new()),
        "random" => Arc::new(Random::new()),
        "modhash" => Arc::new(ModHash::new()),
        "consistenthash" | "ch" => Arc::new(ConsistentHash::new()),
        _ => Arc::new(RoundRobin::new()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_type() {
        assert_eq!(HashType::ModHash.as_str(), "ModHash");
        assert_eq!(HashType::ConsistentHash.as_str(), "ConsistentHash");
    }

    #[test]
    fn test_default_message() {
        let msg = DefaultMessage::new();
        assert!(!msg.is_hash());

        let msg = DefaultMessage::with_hash(123, HashType::ModHash);
        assert!(msg.is_hash());
        assert_eq!(msg.hash_code(), 123);
    }

    #[test]
    fn test_create_selector() {
        let _ = create_selector("roundrobin");
        let _ = create_selector("random");
        let _ = create_selector("modhash");
        let _ = create_selector("consistenthash");
        let _ = create_selector("unknown"); // Should default to roundrobin
    }
}
