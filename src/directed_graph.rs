use std::fmt::Debug;

use std::fmt::Formatter;
use std::hash::Hash;
use std::collections::{HashMap, HashSet};


#[derive(Debug, PartialEq)]
pub enum DGError {
    EdgeCreatesCycle,
}

impl std::fmt::Display for DGError {
    fn fmt(&self, w: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            DGError::EdgeCreatesCycle => w.write_str("Edge creates cycle"),
        }
    }
}

 

// The graph should support the following operations:
// - add_node(node)
// - add_edge(src, dst)
// - contains(node) -> bool
// - is_reachable(src, dst) -> bool

// add_edge should return an error if the edge creates a cycle

// You may use any data structures you like, but you may not use any graph
// libraries or external crates. You may also assume that the node type
// implements Eq, Hash, Debug, and Clone.

// You may write additional functions, structs, or traits to help you solve the
// problem. Please include these in your solution.


pub (crate) struct DirectedGraph<T> {
    adj_list: HashMap<T, Vec<T>>,
}

impl<T: Hash + PartialEq + Eq + Clone> DirectedGraph<T> {
    pub (crate) fn new() -> Self {
        Self {
            adj_list: HashMap::new(),
        }
    }

    pub (crate) fn add_edge_with_check(&mut self, src: T, dest: T) -> Result<(), DGError> {
        // Temporarily add the edge
        self.adj_list.entry(src.clone()).or_insert(vec![]).push(dest.clone());

        if self.is_cyclic() {
            // If a cycle is detected, remove the edge and return an error
            if let Some(edges) = self.adj_list.get_mut(&src) {
                edges.retain(|x| *x != dest);
            }
            Err(DGError::EdgeCreatesCycle)
        } else {
            // If no cycle is detected, keep the edge and return Ok
            Ok(())
        }
    }

    // Method to check if the graph is cyclic
    pub (crate) fn is_cyclic(&self) -> bool {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();

        for node in self.adj_list.keys() {
            if !visited.contains(node) && self.is_cyclic_util(node, &mut visited, &mut rec_stack) {
                return true;
            }
        }
        false
    }

    pub (crate) fn is_cyclic_util(&self, node: &T, visited: &mut HashSet<T>, rec_stack: &mut HashSet<T>) -> bool {
        if rec_stack.contains(&node) {
            return true;
        }
        if visited.contains(&node) {
            return false;
        }

        visited.insert(node.clone());
        rec_stack.insert(node.clone());

        if let Some(neighbors) = self.adj_list.get(&node) {
            for neighbor in neighbors {
                if self.is_cyclic_util(&neighbor, visited, rec_stack) {
                    return true;
                }
            }
        }

        rec_stack.remove(&node);
        false
    }
}

#[cfg(test)]
mod directed_graph_tests {
    use super::*;

    #[test]
    fn test_adding_edge_no_cycle() {
        let mut graph = DirectedGraph::new();
        assert!(graph.add_edge_with_check(0, 1).is_ok());
        assert!(graph.add_edge_with_check(1, 2).is_ok());
    }

    #[test]
    fn test_adding_edge_creates_cycle() {
        let mut graph = DirectedGraph::new();
        graph.add_edge_with_check(0, 1).unwrap();
        graph.add_edge_with_check(1, 2).unwrap();
        assert!(graph.add_edge_with_check(2, 0).is_err());
    }

    #[test]
    fn test_empty_graph() {
        let graph = DirectedGraph::<i32>::new();
        assert!(graph.adj_list.is_empty());
    }

    #[test]
    fn test_single_node_self_loop() {
        let mut graph = DirectedGraph::new();
        assert!(graph.add_edge_with_check(0, 0).is_err());
    }
}
