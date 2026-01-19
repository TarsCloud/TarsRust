//! Mod Hash selector implementation

use parking_lot::RwLock;
use crate::{Endpoint, Result, TarsError};
use super::{Selector, Message};

/// Mod Hash selector - selects endpoint based on hash % node_count
pub struct ModHash {
    nodes: RwLock<Vec<Endpoint>>,
}

impl Default for ModHash {
    fn default() -> Self {
        Self::new()
    }
}

impl ModHash {
    /// Create a new Mod Hash selector
    pub fn new() -> Self {
        Self {
            nodes: RwLock::new(Vec::new()),
        }
    }

    /// Create with initial endpoints
    pub fn with_nodes(nodes: Vec<Endpoint>) -> Self {
        Self {
            nodes: RwLock::new(nodes),
        }
    }
}

impl Selector for ModHash {
    fn select(&self, msg: &dyn Message) -> Result<Endpoint> {
        let nodes = self.nodes.read();
        if nodes.is_empty() {
            return Err(TarsError::NoEndpoint);
        }

        let hash_code = msg.hash_code();
        let idx = (hash_code as usize) % nodes.len();
        Ok(nodes[idx].clone())
    }

    fn refresh(&self, nodes: Vec<Endpoint>) {
        let mut current = self.nodes.write();
        *current = nodes;
    }

    fn add(&self, node: Endpoint) -> Result<()> {
        let mut nodes = self.nodes.write();
        if !nodes.contains(&node) {
            nodes.push(node);
        }
        Ok(())
    }

    fn remove(&self, node: &Endpoint) -> Result<()> {
        let mut nodes = self.nodes.write();
        nodes.retain(|n| n != node);
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
    fn test_modhash_empty() {
        let selector = ModHash::new();
        let msg = DefaultMessage::with_hash(123, HashType::ModHash);
        assert!(selector.select(&msg).is_err());
    }

    #[test]
    fn test_modhash_single() {
        let nodes = vec![Endpoint::tcp("127.0.0.1", 10000)];
        let selector = ModHash::with_nodes(nodes);
        let msg = DefaultMessage::with_hash(123, HashType::ModHash);

        let ep = selector.select(&msg).unwrap();
        assert_eq!(ep.port, 10000);
    }

    #[test]
    fn test_modhash_consistent() {
        let nodes = vec![
            Endpoint::tcp("127.0.0.1", 10000),
            Endpoint::tcp("127.0.0.1", 10001),
            Endpoint::tcp("127.0.0.1", 10002),
        ];
        let selector = ModHash::with_nodes(nodes);
        let msg = DefaultMessage::with_hash(12345, HashType::ModHash);

        // Same hash should always select same endpoint
        let ep1 = selector.select(&msg).unwrap();
        let ep2 = selector.select(&msg).unwrap();
        assert_eq!(ep1.port, ep2.port);

        // Expected: 12345 % 3 = 0, so port 10000
        assert_eq!(ep1.port, 10000);
    }

    #[test]
    fn test_modhash_distribution() {
        let nodes = vec![
            Endpoint::tcp("127.0.0.1", 10000),
            Endpoint::tcp("127.0.0.1", 10001),
            Endpoint::tcp("127.0.0.1", 10002),
        ];
        let selector = ModHash::with_nodes(nodes);

        let msg0 = DefaultMessage::with_hash(0, HashType::ModHash);
        let msg1 = DefaultMessage::with_hash(1, HashType::ModHash);
        let msg2 = DefaultMessage::with_hash(2, HashType::ModHash);

        assert_eq!(selector.select(&msg0).unwrap().port, 10000);
        assert_eq!(selector.select(&msg1).unwrap().port, 10001);
        assert_eq!(selector.select(&msg2).unwrap().port, 10002);
    }
}
