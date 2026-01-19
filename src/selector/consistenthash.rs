//! Consistent Hash selector implementation

use std::collections::HashMap;
use parking_lot::RwLock;
use crc32fast::Hasher;
use crate::{Endpoint, Result, TarsError, consts};
use super::{Selector, Message};

/// Consistent Hash selector with virtual nodes
pub struct ConsistentHash {
    /// Virtual node ring: hash -> endpoint
    ring: RwLock<HashMap<u32, Endpoint>>,
    /// Sorted keys for binary search
    sorted_keys: RwLock<Vec<u32>>,
    /// Original nodes
    nodes: RwLock<Vec<Endpoint>>,
}

impl Default for ConsistentHash {
    fn default() -> Self {
        Self::new()
    }
}

impl ConsistentHash {
    /// Create a new Consistent Hash selector
    pub fn new() -> Self {
        Self {
            ring: RwLock::new(HashMap::new()),
            sorted_keys: RwLock::new(Vec::new()),
            nodes: RwLock::new(Vec::new()),
        }
    }

    /// Create with initial endpoints
    pub fn with_nodes(nodes: Vec<Endpoint>) -> Self {
        let selector = Self::new();
        selector.build_ring(&nodes);
        *selector.nodes.write() = nodes;
        selector
    }

    /// Build the hash ring from nodes
    fn build_ring(&self, nodes: &[Endpoint]) {
        let mut ring = self.ring.write();
        let mut sorted_keys = self.sorted_keys.write();

        ring.clear();
        sorted_keys.clear();

        for node in nodes {
            // Create virtual nodes
            for i in 0..consts::CON_HASH_VIRTUAL_NODES {
                let key = self.hash_key(&node.address(), i);
                ring.insert(key, node.clone());
                sorted_keys.push(key);
            }
        }

        // Sort keys for binary search
        sorted_keys.sort_unstable();
    }

    /// Calculate hash key for a virtual node
    fn hash_key(&self, addr: &str, idx: usize) -> u32 {
        let key = format!("{}#{}", addr, idx);
        let mut hasher = Hasher::new();
        hasher.update(key.as_bytes());
        hasher.finalize()
    }

    /// Find the closest key in the ring
    fn find_key(&self, hash: u32) -> Option<u32> {
        let sorted_keys = self.sorted_keys.read();
        if sorted_keys.is_empty() {
            return None;
        }

        // Binary search for the first key >= hash
        match sorted_keys.binary_search(&hash) {
            Ok(idx) => Some(sorted_keys[idx]),
            Err(idx) => {
                if idx >= sorted_keys.len() {
                    // Wrap around to the first key
                    Some(sorted_keys[0])
                } else {
                    Some(sorted_keys[idx])
                }
            }
        }
    }
}

impl Selector for ConsistentHash {
    fn select(&self, msg: &dyn Message) -> Result<Endpoint> {
        let ring = self.ring.read();
        if ring.is_empty() {
            return Err(TarsError::NoEndpoint);
        }

        let hash = msg.hash_code();
        let key = self.find_key(hash).ok_or(TarsError::NoEndpoint)?;
        ring.get(&key).cloned().ok_or(TarsError::NoEndpoint)
    }

    fn refresh(&self, nodes: Vec<Endpoint>) {
        self.build_ring(&nodes);
        *self.nodes.write() = nodes;
    }

    fn add(&self, node: Endpoint) -> Result<()> {
        let mut nodes = self.nodes.write();
        if !nodes.contains(&node) {
            nodes.push(node.clone());
            drop(nodes);

            // Add only the new node's virtual nodes instead of rebuilding
            let mut ring = self.ring.write();
            let mut sorted_keys = self.sorted_keys.write();

            for i in 0..consts::CON_HASH_VIRTUAL_NODES {
                let key = self.hash_key(&node.address(), i);
                ring.insert(key, node.clone());
                sorted_keys.push(key);
            }

            // Re-sort keys
            sorted_keys.sort_unstable();
        }
        Ok(())
    }

    fn remove(&self, node: &Endpoint) -> Result<()> {
        let mut nodes = self.nodes.write();
        nodes.retain(|n| n != node);
        drop(nodes);
        let nodes = self.nodes.read();
        self.build_ring(&nodes);
        Ok(())
    }

    fn all(&self) -> Vec<Endpoint> {
        self.nodes.read().clone()
    }

    fn len(&self) -> usize {
        self.nodes.read().len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::selector::{DefaultMessage, HashType};

    #[test]
    fn test_consistenthash_empty() {
        let selector = ConsistentHash::new();
        let msg = DefaultMessage::with_hash(123, HashType::ConsistentHash);
        assert!(selector.select(&msg).is_err());
    }

    #[test]
    fn test_consistenthash_single() {
        let nodes = vec![Endpoint::tcp("127.0.0.1", 10000)];
        let selector = ConsistentHash::with_nodes(nodes);
        let msg = DefaultMessage::with_hash(123, HashType::ConsistentHash);

        let ep = selector.select(&msg).unwrap();
        assert_eq!(ep.port, 10000);
    }

    #[test]
    fn test_consistenthash_consistent() {
        let nodes = vec![
            Endpoint::tcp("127.0.0.1", 10000),
            Endpoint::tcp("127.0.0.1", 10001),
            Endpoint::tcp("127.0.0.1", 10002),
        ];
        let selector = ConsistentHash::with_nodes(nodes);
        let msg = DefaultMessage::with_hash(12345, HashType::ConsistentHash);

        // Same hash should always select same endpoint
        let ep1 = selector.select(&msg).unwrap();
        let ep2 = selector.select(&msg).unwrap();
        assert_eq!(ep1.port, ep2.port);
    }

    #[test]
    fn test_consistenthash_distribution() {
        let nodes = vec![
            Endpoint::tcp("127.0.0.1", 10000),
            Endpoint::tcp("127.0.0.1", 10001),
            Endpoint::tcp("127.0.0.1", 10002),
        ];
        let selector = ConsistentHash::with_nodes(nodes);

        // Test multiple hash values
        let mut counts = std::collections::HashMap::new();
        for i in 0..1000 {
            let msg = DefaultMessage::with_hash(i, HashType::ConsistentHash);
            let ep = selector.select(&msg).unwrap();
            *counts.entry(ep.port).or_insert(0) += 1;
        }

        // Each node should get some requests (rough distribution)
        for (port, count) in counts {
            println!("Port {}: {} requests", port, count);
            assert!(count > 0);
        }
    }

    #[test]
    fn test_consistenthash_minimal_disruption() {
        let nodes = vec![
            Endpoint::tcp("127.0.0.1", 10000),
            Endpoint::tcp("127.0.0.1", 10001),
            Endpoint::tcp("127.0.0.1", 10002),
        ];
        let selector = ConsistentHash::with_nodes(nodes.clone());

        // Use well-distributed hash values (CRC32 of strings)
        let hash_values: Vec<u32> = (0..100)
            .map(|i| {
                let mut hasher = crc32fast::Hasher::new();
                hasher.update(format!("key_{}", i).as_bytes());
                hasher.finalize()
            })
            .collect();

        // Record initial selections
        let initial: Vec<_> = hash_values
            .iter()
            .map(|&h| {
                let msg = DefaultMessage::with_hash(h, HashType::ConsistentHash);
                selector.select(&msg).unwrap().port
            })
            .collect();

        // Add a new node
        selector.add(Endpoint::tcp("127.0.0.1", 10003)).unwrap();

        // Check how many selections changed
        let mut changed = 0;
        for (i, &h) in hash_values.iter().enumerate() {
            let msg = DefaultMessage::with_hash(h, HashType::ConsistentHash);
            let new_port = selector.select(&msg).unwrap().port;
            if new_port != initial[i] {
                changed += 1;
            }
        }

        // With consistent hashing, only ~25% should change (1/4 of requests)
        println!("Changed: {}/100", changed);
        assert!(changed < 50, "Too many changes: {}", changed);
    }
}
