//! Python-specific parsing using tree-sitter.
//!
//! Extracts functions, classes, imports, docstrings, and async patterns.

use std::path::Path;

use crate::types::{AcbResult, CodeUnitType, Language, Visibility};

use super::treesitter::{count_complexity, get_node_text, node_to_span};
use super::{LanguageParser, RawCodeUnit, RawReference, ReferenceKind};

/// Python language parser.
pub struct PythonParser;

impl Default for PythonParser {
    fn default() -> Self {
        Self::new()
    }
}

impl PythonParser {
    /// Create a new Python parser.
    pub fn new() -> Self {
        Self
    }

    fn extract_from_node(
        &self,
        node: tree_sitter::Node,
        source: &str,
        file_path: &Path,
        units: &mut Vec<RawCodeUnit>,
        next_id: &mut u64,
        parent_qname: &str,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "function_definition" => {
                    if let Some(unit) = self.extract_function(
                        child,
                        source,
                        file_path,
                        false,
                        parent_qname,
                        next_id,
                    ) {
                        let qname = unit.qualified_name.clone();
                        units.push(unit);
                        // Recurse into function body for nested definitions
                        if let Some(body) = child.child_by_field_name("body") {
                            self.extract_from_node(body, source, file_path, units, next_id, &qname);
                        }
                    }
                }
                "async_function_definition" | "async function_definition" => {
                    // tree-sitter-python uses "function_definition" inside decorated nodes
                    // but async functions may appear differently
                }
                "class_definition" => {
                    if let Some(unit) =
                        self.extract_class(child, source, file_path, parent_qname, next_id)
                    {
                        let qname = unit.qualified_name.clone();
                        units.push(unit);
                        if let Some(body) = child.child_by_field_name("body") {
                            self.extract_from_node(body, source, file_path, units, next_id, &qname);
                        }
                    }
                }
                "import_statement" | "import_from_statement" => {
                    if let Some(unit) =
                        self.extract_import(child, source, file_path, parent_qname, next_id)
                    {
                        units.push(unit);
                    }
                }
                "decorated_definition" => {
                    // Look inside the decorated definition for the actual def/class
                    self.extract_from_node(child, source, file_path, units, next_id, parent_qname);
                }
                _ => {
                    // Check for assignments at module level (constants)
                    // and recurse into other compound statements
                }
            }
        }
    }

    fn extract_function(
        &self,
        node: tree_sitter::Node,
        source: &str,
        file_path: &Path,
        _is_nested: bool,
        parent_qname: &str,
        next_id: &mut u64,
    ) -> Option<RawCodeUnit> {
        let name_node = node.child_by_field_name("name")?;
        let name = get_node_text(name_node, source).to_string();

        let qname = if parent_qname.is_empty() {
            name.clone()
        } else {
            format!("{}.{}", parent_qname, name)
        };

        let is_async = node.kind() == "async_function_definition"
            || node
                .parent()
                .map(|p| {
                    let mut c = p.walk();
                    let result = p
                        .children(&mut c)
                        .any(|ch| ch.kind() == "async" || get_node_text(ch, source) == "async");
                    result
                })
                .unwrap_or(false);

        let span = node_to_span(node);

        // Extract signature from parameters
        let sig = node.child_by_field_name("parameters").map(|params| {
            let params_text = get_node_text(params, source);
            let ret = node
                .child_by_field_name("return_type")
                .map(|r| format!(" -> {}", get_node_text(r, source)))
                .unwrap_or_default();
            format!("{}{}", params_text, ret)
        });

        // Extract docstring
        let doc = self.extract_docstring(node, source);

        // Visibility from name convention
        let vis = python_visibility(&name);

        // Complexity
        let complexity_kinds = &[
            "if_statement",
            "elif_clause",
            "for_statement",
            "while_statement",
            "try_statement",
            "except_clause",
            "with_statement",
            "boolean_operator",
            "conditional_expression",
        ];
        let complexity = count_complexity(node, complexity_kinds);

        // Check for yield (generator)
        let is_generator = source[node.byte_range()].contains("yield");

        let id = *next_id;
        *next_id += 1;

        // Determine if this is a test function
        let unit_type = if name.starts_with("test_") || name.starts_with("test") {
            CodeUnitType::Test
        } else {
            CodeUnitType::Function
        };

        let mut unit = RawCodeUnit::new(
            unit_type,
            Language::Python,
            name,
            file_path.to_path_buf(),
            span,
        );
        unit.temp_id = id;
        unit.qualified_name = qname;
        unit.signature = sig;
        unit.doc = doc;
        unit.visibility = vis;
        unit.is_async = is_async;
        unit.is_generator = is_generator;
        unit.complexity = complexity;

        // Extract call references from function body
        if let Some(body) = node.child_by_field_name("body") {
            self.extract_call_refs(body, source, &mut unit.references);
        }

        Some(unit)
    }

    fn extract_class(
        &self,
        node: tree_sitter::Node,
        source: &str,
        file_path: &Path,
        parent_qname: &str,
        next_id: &mut u64,
    ) -> Option<RawCodeUnit> {
        let name_node = node.child_by_field_name("name")?;
        let name = get_node_text(name_node, source).to_string();

        let qname = if parent_qname.is_empty() {
            name.clone()
        } else {
            format!("{}.{}", parent_qname, name)
        };

        let span = node_to_span(node);
        let doc = self.extract_docstring(node, source);
        let vis = python_visibility(&name);

        let id = *next_id;
        *next_id += 1;

        let mut unit = RawCodeUnit::new(
            CodeUnitType::Type,
            Language::Python,
            name,
            file_path.to_path_buf(),
            span,
        );
        unit.temp_id = id;
        unit.qualified_name = qname;
        unit.doc = doc;
        unit.visibility = vis;

        // Extract base classes as inheritance references
        if let Some(args) = node.child_by_field_name("superclasses") {
            let mut cursor = args.walk();
            for child in args.children(&mut cursor) {
                if child.kind() == "identifier" || child.kind() == "attribute" {
                    let base_name = get_node_text(child, source).to_string();
                    unit.references.push(RawReference {
                        name: base_name,
                        kind: ReferenceKind::Inherit,
                        span: node_to_span(child),
                    });
                }
            }
        }

        Some(unit)
    }

    fn extract_import(
        &self,
        node: tree_sitter::Node,
        source: &str,
        file_path: &Path,
        parent_qname: &str,
        next_id: &mut u64,
    ) -> Option<RawCodeUnit> {
        let text = get_node_text(node, source).to_string();
        let span = node_to_span(node);

        // Derive a name from the import text
        let import_name = text
            .trim_start_matches("from ")
            .trim_start_matches("import ")
            .split_whitespace()
            .next()
            .unwrap_or("unknown")
            .to_string();

        let id = *next_id;
        *next_id += 1;

        let mut unit = RawCodeUnit::new(
            CodeUnitType::Import,
            Language::Python,
            import_name.clone(),
            file_path.to_path_buf(),
            span,
        );
        unit.temp_id = id;
        unit.qualified_name = if parent_qname.is_empty() {
            import_name.clone()
        } else {
            format!("{}.{}", parent_qname, import_name)
        };

        unit.references.push(RawReference {
            name: import_name,
            kind: ReferenceKind::Import,
            span,
        });

        Some(unit)
    }

    fn extract_docstring(&self, node: tree_sitter::Node, source: &str) -> Option<String> {
        let body = node.child_by_field_name("body")?;
        let mut cursor = body.walk();
        let first_stmt = body.children(&mut cursor).next()?;

        if first_stmt.kind() == "expression_statement" {
            let mut c2 = first_stmt.walk();
            let expr = first_stmt.children(&mut c2).next()?;
            if expr.kind() == "string" {
                let text = get_node_text(expr, source);
                return Some(clean_docstring(text));
            }
        }
        None
    }

    #[allow(clippy::only_used_in_recursion)]
    fn extract_call_refs(
        &self,
        node: tree_sitter::Node,
        source: &str,
        refs: &mut Vec<RawReference>,
    ) {
        if node.kind() == "call" {
            if let Some(func) = node.child_by_field_name("function") {
                let name = get_node_text(func, source).to_string();
                refs.push(RawReference {
                    name,
                    kind: ReferenceKind::Call,
                    span: node_to_span(node),
                });
            }
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.extract_call_refs(child, source, refs);
        }
    }
}

impl LanguageParser for PythonParser {
    fn extract_units(
        &self,
        tree: &tree_sitter::Tree,
        source: &str,
        file_path: &Path,
    ) -> AcbResult<Vec<RawCodeUnit>> {
        let mut units = Vec::new();
        let mut next_id = 0u64;

        // Create module unit for the file
        let module_name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let root_span = node_to_span(tree.root_node());
        let mut module_unit = RawCodeUnit::new(
            CodeUnitType::Module,
            Language::Python,
            module_name.clone(),
            file_path.to_path_buf(),
            root_span,
        );
        module_unit.temp_id = next_id;
        module_unit.qualified_name = module_name.clone();
        next_id += 1;
        units.push(module_unit);

        // Extract all definitions
        self.extract_from_node(
            tree.root_node(),
            source,
            file_path,
            &mut units,
            &mut next_id,
            &module_name,
        );

        Ok(units)
    }

    fn is_test_file(&self, path: &Path, source: &str) -> bool {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        name.starts_with("test_")
            || name.ends_with("_test.py")
            || path.components().any(|c| c.as_os_str() == "tests")
            || source.contains("import pytest")
            || source.contains("import unittest")
    }
}

fn python_visibility(name: &str) -> Visibility {
    if name.starts_with("__") && !name.ends_with("__") {
        Visibility::Private
    } else if name.starts_with('_') {
        Visibility::Internal
    } else {
        Visibility::Public
    }
}

fn clean_docstring(raw: &str) -> String {
    let trimmed = raw
        .trim_start_matches("\"\"\"")
        .trim_end_matches("\"\"\"")
        .trim_start_matches("'''")
        .trim_end_matches("'''")
        .trim_start_matches('"')
        .trim_end_matches('"')
        .trim_start_matches('\'')
        .trim_end_matches('\'');
    trimmed.lines().next().unwrap_or("").trim().to_string()
}
