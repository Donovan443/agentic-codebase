//! Tree-sitter wrapper utilities shared across language parsers.

use std::path::PathBuf;

use crate::types::{AcbError, AcbResult, Span};

/// Get the text content of a tree-sitter node.
pub fn get_node_text<'a>(node: tree_sitter::Node, source: &'a str) -> &'a str {
    &source[node.byte_range()]
}

/// Convert a tree-sitter node to a Span.
pub fn node_to_span(node: tree_sitter::Node) -> Span {
    let start = node.start_position();
    let end = node.end_position();
    Span::new(
        start.row as u32 + 1,
        start.column as u32,
        end.row as u32 + 1,
        end.column as u32,
    )
}

/// Find the first child of a node with the given kind.
pub fn find_child_by_kind<'a>(
    node: tree_sitter::Node<'a>,
    kind: &str,
) -> Option<tree_sitter::Node<'a>> {
    let mut cursor = node.walk();
    let result = node
        .children(&mut cursor)
        .find(|child| child.kind() == kind);
    result
}

/// Collect all direct children of a node with the given kind.
pub fn collect_children_by_kind<'a>(
    node: tree_sitter::Node<'a>,
    kind: &str,
) -> Vec<tree_sitter::Node<'a>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .filter(|c| c.kind() == kind)
        .collect()
}

/// Parse source code with error recovery using tree-sitter.
pub fn parse_with_language(
    source: &str,
    language: tree_sitter::Language,
) -> AcbResult<tree_sitter::Tree> {
    let mut parser = tree_sitter::Parser::new();
    parser
        .set_language(&language)
        .map_err(|e| AcbError::ParseError {
            path: PathBuf::new(),
            message: format!("Failed to set tree-sitter language: {}", e),
        })?;
    parser
        .parse(source, None)
        .ok_or_else(|| AcbError::ParseError {
            path: PathBuf::new(),
            message: "Failed to parse source".into(),
        })
}

/// Count decision points in a subtree for cyclomatic complexity.
pub fn count_complexity(node: tree_sitter::Node, decision_kinds: &[&str]) -> u32 {
    let mut complexity = 1u32;
    count_complexity_inner(node, decision_kinds, &mut complexity);
    complexity
}

fn count_complexity_inner(node: tree_sitter::Node, decision_kinds: &[&str], count: &mut u32) {
    if decision_kinds.contains(&node.kind()) {
        *count += 1;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        count_complexity_inner(child, decision_kinds, count);
    }
}
