//! Round Robin selector implementation

use std::sync::atomic::{AtomicUsize, Ordering};
use parking_lot::RwLock;
use crate::{Endpoint, Result, TarsError};
use super::{Selector, Message};

/// Round Robin selector - cycles through endpoints in order
pub struct RoundRobin {
    nodes: RwLock<Vec<Endpoint>>,
    index: AtomicUsize,
}

impl Default for RoundRobin {
    fn default() -> Self {
        Self::new()
    }
}

impl RoundRobin {
    /// Create a new Round Robin selector
    pub fn new() -> Self {
        Self {
            nodes: RwLock::new(Vec::new()),
            index: AtomicUsize::new(0),
        }
    }

    /// Create with initial endpoints
    pub fn with_nodes(nodes: Vec<Endpoint>) -> Self {
        Self {
            nodes: RwLock::new(nodes),
            index: AtomicUsize::new(0),
        }
    }
}

impl Selector for RoundRobin {
    fn select(&self, _msg: &dyn Message) -> Result<Endpoint> {
        let nodes = self.nodes.read();
        if nodes.is_empty() {
            return Err(TarsError::NoEndpoint);
        }

        let idx = self.index.fetch_add(1, Ordering::SeqCst) % nodes.len();
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

    #[test]
    fn test_roundrobin_empty() {
        let selector = RoundRobin::new();
        let msg = DefaultMessage::new();
        assert!(selector.select(&msg).is_err());
    }

    #[test]
    fn test_roundrobin_single() {
        let nodes = vec![Endpoint::tcp("127.0.0.1", 10000)];
        let selector = RoundRobin::with_nodes(nodes);
        let msg = DefaultMessage::new();

        let ep1 = selector.select(&msg).unwrap();
        let ep2 = selector.select(&msg).unwrap();
        assert_eq!(ep1.port, 10000);
        assert_eq!(ep2.port, 10000);
    }

    #[test]
    fn test_roundrobin_multiple() {
        let nodes = vec![
            Endpoint::tcp("127.0.0.1", 10000),
            Endpoint::tcp("127.0.0.1", 10001),
            Endpoint::tcp("127.0.0.1", 10002),
        ];
        let selector = RoundRobin::with_nodes(nodes);
        let msg = DefaultMessage::new();

        let ports: Vec<u16> = (0..6).map(|_| selector.select(&msg).unwrap().port).collect();

        // Should cycle through in order
        assert_eq!(ports[0], ports[3]); // Same position in cycle
        assert_eq!(ports[1], ports[4]);
        assert_eq!(ports[2], ports[5]);
    }

    #[test]
    fn test_roundrobin_add_remove() {
        let selector = RoundRobin::new();

        let ep1 = Endpoint::tcp("127.0.0.1", 10000);
        let ep2 = Endpoint::tcp("127.0.0.1", 10001);

        selector.add(ep1.clone()).unwrap();
        selector.add(ep2.clone()).unwrap();
        assert_eq!(selector.len(), 2);

        selector.remove(&ep1).unwrap();
        assert_eq!(selector.len(), 1);
    }

    #[test]
    fn test_roundrobin_refresh() {
        let nodes = vec![Endpoint::tcp("127.0.0.1", 10000)];
        let selector = RoundRobin::with_nodes(nodes);
        assert_eq!(selector.len(), 1);

        let new_nodes = vec![
            Endpoint::tcp("127.0.0.1", 10001),
            Endpoint::tcp("127.0.0.1", 10002),
        ];
        selector.refresh(new_nodes);
        assert_eq!(selector.len(), 2);
    }
}
