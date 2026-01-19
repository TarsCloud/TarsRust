//! Random selector implementation

use parking_lot::RwLock;
use rand::Rng;
use crate::{Endpoint, Result, TarsError};
use super::{Selector, Message};

/// Random selector - randomly selects an endpoint
pub struct Random {
    nodes: RwLock<Vec<Endpoint>>,
}

impl Default for Random {
    fn default() -> Self {
        Self::new()
    }
}

impl Random {
    /// Create a new Random selector
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

impl Selector for Random {
    fn select(&self, _msg: &dyn Message) -> Result<Endpoint> {
        let nodes = self.nodes.read();
        if nodes.is_empty() {
            return Err(TarsError::NoEndpoint);
        }

        let idx = rand::thread_rng().gen_range(0..nodes.len());
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
    use crate::selector::DefaultMessage;
    use std::collections::HashSet;

    #[test]
    fn test_random_empty() {
        let selector = Random::new();
        let msg = DefaultMessage::new();
        assert!(selector.select(&msg).is_err());
    }

    #[test]
    fn test_random_single() {
        let nodes = vec![Endpoint::tcp("127.0.0.1", 10000)];
        let selector = Random::with_nodes(nodes);
        let msg = DefaultMessage::new();

        let ep = selector.select(&msg).unwrap();
        assert_eq!(ep.port, 10000);
    }

    #[test]
    fn test_random_multiple() {
        let nodes = vec![
            Endpoint::tcp("127.0.0.1", 10000),
            Endpoint::tcp("127.0.0.1", 10001),
            Endpoint::tcp("127.0.0.1", 10002),
        ];
        let selector = Random::with_nodes(nodes);
        let msg = DefaultMessage::new();

        // Run multiple selections and verify we get different results
        let ports: HashSet<u16> = (0..100)
            .map(|_| selector.select(&msg).unwrap().port)
            .collect();

        // With 100 tries on 3 endpoints, we should get all 3
        assert!(ports.len() > 1);
    }
}
