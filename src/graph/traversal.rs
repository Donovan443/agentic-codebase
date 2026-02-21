//! Graph traversal algorithms for the code graph.
//!
//! BFS, DFS, and specialized traversals for dependency analysis,
//! impact analysis, and concept grouping.

use std::collections::{HashSet, VecDeque};

use crate::types::EdgeType;

use super::code_graph::CodeGraph;

/// Direction of traversal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// Follow edges from source to target.
    Forward,
    /// Follow edges from target to source (reverse).
    Backward,
}

/// Options for graph traversal.
#[derive(Debug, Clone)]
pub struct TraversalOptions {
    /// Maximum depth to traverse (-1 = unlimited).
    pub max_depth: i32,
    /// Only traverse edges of these types (empty = all).
    pub edge_types: Vec<EdgeType>,
    /// Direction of traversal.
    pub direction: Direction,
}

impl Default for TraversalOptions {
    fn default() -> Self {
        Self {
            max_depth: -1,
            edge_types: Vec::new(),
            direction: Direction::Forward,
        }
    }
}

/// Result of a traversal: a list of (unit_id, depth) pairs.
pub type TraversalResult = Vec<(u64, u32)>;

/// Perform a breadth-first traversal from the given start node.
///
/// Returns all reachable nodes with their distance from the start.
pub fn bfs(graph: &CodeGraph, start_id: u64, options: &TraversalOptions) -> TraversalResult {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut result = Vec::new();

    visited.insert(start_id);
    queue.push_back((start_id, 0u32));

    while let Some((current_id, depth)) = queue.pop_front() {
        result.push((current_id, depth));

        // Check depth limit
        if options.max_depth >= 0 && depth >= options.max_depth as u32 {
            continue;
        }

        let neighbors = match options.direction {
            Direction::Forward => graph.edges_from(current_id),
            Direction::Backward => graph.edges_to(current_id),
        };

        for edge in neighbors {
            // Filter by edge type if specified
            if !options.edge_types.is_empty() && !options.edge_types.contains(&edge.edge_type) {
                continue;
            }

            let next_id = match options.direction {
                Direction::Forward => edge.target_id,
                Direction::Backward => edge.source_id,
            };

            if visited.insert(next_id) {
                queue.push_back((next_id, depth + 1));
            }
        }
    }

    result
}

/// Perform a depth-first traversal from the given start node.
///
/// Returns all reachable nodes with their distance from the start.
pub fn dfs(graph: &CodeGraph, start_id: u64, options: &TraversalOptions) -> TraversalResult {
    let mut visited = HashSet::new();
    let mut result = Vec::new();
    dfs_inner(graph, start_id, 0, options, &mut visited, &mut result);
    result
}

fn dfs_inner(
    graph: &CodeGraph,
    current_id: u64,
    depth: u32,
    options: &TraversalOptions,
    visited: &mut HashSet<u64>,
    result: &mut TraversalResult,
) {
    if !visited.insert(current_id) {
        return;
    }

    result.push((current_id, depth));

    if options.max_depth >= 0 && depth >= options.max_depth as u32 {
        return;
    }

    let neighbors = match options.direction {
        Direction::Forward => graph.edges_from(current_id),
        Direction::Backward => graph.edges_to(current_id),
    };

    for edge in neighbors {
        if !options.edge_types.is_empty() && !options.edge_types.contains(&edge.edge_type) {
            continue;
        }

        let next_id = match options.direction {
            Direction::Forward => edge.target_id,
            Direction::Backward => edge.source_id,
        };

        dfs_inner(graph, next_id, depth + 1, options, visited, result);
    }
}

/// Find all paths between two nodes up to a maximum depth.
pub fn find_paths(
    graph: &CodeGraph,
    from: u64,
    to: u64,
    max_depth: u32,
    edge_types: &[EdgeType],
) -> Vec<Vec<u64>> {
    let mut paths = Vec::new();
    let mut current_path = vec![from];
    let mut visited = HashSet::new();
    visited.insert(from);
    find_paths_inner(
        graph,
        to,
        max_depth,
        edge_types,
        &mut current_path,
        &mut visited,
        &mut paths,
    );
    paths
}

fn find_paths_inner(
    graph: &CodeGraph,
    target: u64,
    max_depth: u32,
    edge_types: &[EdgeType],
    current_path: &mut Vec<u64>,
    visited: &mut HashSet<u64>,
    results: &mut Vec<Vec<u64>>,
) {
    let current = *current_path.last().unwrap();

    if current == target && current_path.len() > 1 {
        results.push(current_path.clone());
        return;
    }

    if current_path.len() > max_depth as usize {
        return;
    }

    for edge in graph.edges_from(current) {
        if !edge_types.is_empty() && !edge_types.contains(&edge.edge_type) {
            continue;
        }

        if visited.insert(edge.target_id) {
            current_path.push(edge.target_id);
            find_paths_inner(
                graph,
                target,
                max_depth,
                edge_types,
                current_path,
                visited,
                results,
            );
            current_path.pop();
            visited.remove(&edge.target_id);
        }
    }
}

/// Find the shortest path between two nodes using BFS.
///
/// Returns `None` if no path exists.
pub fn shortest_path(
    graph: &CodeGraph,
    from: u64,
    to: u64,
    edge_types: &[EdgeType],
) -> Option<Vec<u64>> {
    if from == to {
        return Some(vec![from]);
    }

    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut parent: std::collections::HashMap<u64, u64> = std::collections::HashMap::new();

    visited.insert(from);
    queue.push_back(from);

    while let Some(current) = queue.pop_front() {
        for edge in graph.edges_from(current) {
            if !edge_types.is_empty() && !edge_types.contains(&edge.edge_type) {
                continue;
            }

            if visited.insert(edge.target_id) {
                parent.insert(edge.target_id, current);

                if edge.target_id == to {
                    // Reconstruct path
                    let mut path = vec![to];
                    let mut node = to;
                    while let Some(&p) = parent.get(&node) {
                        path.push(p);
                        node = p;
                    }
                    path.reverse();
                    return Some(path);
                }

                queue.push_back(edge.target_id);
            }
        }
    }

    None
}
