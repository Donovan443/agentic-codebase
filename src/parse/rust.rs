//! Rust-specific parsing using tree-sitter.
//!
//! Extracts functions, structs, enums, traits, impls, mods, use declarations, macros.

use std::path::Path;

use crate::types::{AcbResult, CodeUnitType, Language, Visibility};

use super::treesitter::{count_complexity, get_node_text, node_to_span};
use super::{LanguageParser, RawCodeUnit, RawReference, ReferenceKind};

/// Rust language parser.
pub struct RustParser;

impl Default for RustParser {
    fn default() -> Self {
        Self::new()
    }
}

impl RustParser {
    /// Create a new Rust parser.
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
                "function_item" => {
                    if let Some(unit) =
                        self.extract_function(child, source, file_path, parent_qname, next_id)
                    {
                        units.push(unit);
                    }
                }
                "struct_item" => {
                    if let Some(unit) = self.extract_type_def(
                        child,
                        source,
                        file_path,
                        CodeUnitType::Type,
                        parent_qname,
                        next_id,
                    ) {
                        units.push(unit);
                    }
                }
                "enum_item" => {
                    if let Some(unit) = self.extract_type_def(
                        child,
                        source,
                        file_path,
                        CodeUnitType::Type,
                        parent_qname,
                        next_id,
                    ) {
                        units.push(unit);
                    }
                }
                "trait_item" => {
                    if let Some(unit) = self.extract_type_def(
                        child,
                        source,
                        file_path,
                        CodeUnitType::Trait,
                        parent_qname,
                        next_id,
                    ) {
                        let qname = unit.qualified_name.clone();
                        units.push(unit);
                        if let Some(body) = child.child_by_field_name("body") {
                            self.extract_from_node(body, source, file_path, units, next_id, &qname);
                        }
                    }
                }
                "impl_item" => {
                    if let Some(unit) =
                        self.extract_impl(child, source, file_path, parent_qname, next_id)
                    {
                        let qname = unit.qualified_name.clone();
                        units.push(unit);
                        if let Some(body) = child.child_by_field_name("body") {
                            self.extract_from_node(body, source, file_path, units, next_id, &qname);
                        }
                    }
                }
                "mod_item" => {
                    if let Some(unit) =
                        self.extract_mod(child, source, file_path, parent_qname, next_id)
                    {
                        let qname = unit.qualified_name.clone();
                        units.push(unit);
                        if let Some(body) = child.child_by_field_name("body") {
                            self.extract_from_node(body, source, file_path, units, next_id, &qname);
                        }
                    }
                }
                "use_declaration" => {
                    if let Some(unit) =
                        self.extract_use(child, source, file_path, parent_qname, next_id)
                    {
                        units.push(unit);
                    }
                }
                "macro_definition" => {
                    if let Some(unit) =
                        self.extract_macro(child, source, file_path, parent_qname, next_id)
                    {
                        units.push(unit);
                    }
                }
                _ => {}
            }
        }
    }

    fn extract_function(
        &self,
        node: tree_sitter::Node,
        source: &str,
        file_path: &Path,
        parent_qname: &str,
        next_id: &mut u64,
    ) -> Option<RawCodeUnit> {
        let name_node = node.child_by_field_name("name")?;
        let name = get_node_text(name_node, source).to_string();
        let qname = make_qname(parent_qname, &name);
        let span = node_to_span(node);

        // Signature
        let sig = node.child_by_field_name("parameters").map(|params| {
            let params_text = get_node_text(params, source);
            let ret = node
                .child_by_field_name("return_type")
                .map(|r| format!(" -> {}", get_node_text(r, source)))
                .unwrap_or_default();
            format!("fn {}{}{}", name, params_text, ret)
        });

        let vis = rust_visibility(node, source);
        let fn_text = &source[node.byte_range()];
        let is_async = fn_text.contains("async fn ") || fn_text.trim_start().starts_with("async ");

        let complexity_kinds = &[
            "if_expression",
            "else_clause",
            "for_expression",
            "while_expression",
            "loop_expression",
            "match_arm",
            "binary_expression",
        ];
        let complexity = count_complexity(node, complexity_kinds);

        let doc = extract_rust_doc(node, source);

        let is_test = name.starts_with("test_") || source[node.byte_range()].contains("#[test]");

        let unit_type = if is_test {
            CodeUnitType::Test
        } else {
            CodeUnitType::Function
        };

        let id = *next_id;
        *next_id += 1;

        let mut unit = RawCodeUnit::new(
            unit_type,
            Language::Rust,
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
        unit.complexity = complexity;

        Some(unit)
    }

    fn extract_type_def(
        &self,
        node: tree_sitter::Node,
        source: &str,
        file_path: &Path,
        unit_type: CodeUnitType,
        parent_qname: &str,
        next_id: &mut u64,
    ) -> Option<RawCodeUnit> {
        let name_node = node.child_by_field_name("name")?;
        let name = get_node_text(name_node, source).to_string();
        let qname = make_qname(parent_qname, &name);
        let span = node_to_span(node);
        let vis = rust_visibility(node, source);
        let doc = extract_rust_doc(node, source);

        let id = *next_id;
        *next_id += 1;

        let mut unit = RawCodeUnit::new(
            unit_type,
            Language::Rust,
            name,
            file_path.to_path_buf(),
            span,
        );
        unit.temp_id = id;
        unit.qualified_name = qname;
        unit.doc = doc;
        unit.visibility = vis;

        Some(unit)
    }

    fn extract_impl(
        &self,
        node: tree_sitter::Node,
        source: &str,
        file_path: &Path,
        parent_qname: &str,
        next_id: &mut u64,
    ) -> Option<RawCodeUnit> {
        let type_node = node.child_by_field_name("type")?;
        let type_name = get_node_text(type_node, source).to_string();

        let trait_name = node
            .child_by_field_name("trait")
            .map(|t| get_node_text(t, source).to_string());

        let name = if let Some(ref tr) = trait_name {
            format!("impl {} for {}", tr, type_name)
        } else {
            format!("impl {}", type_name)
        };

        let qname = make_qname(parent_qname, &name);
        let span = node_to_span(node);

        let id = *next_id;
        *next_id += 1;

        let mut unit = RawCodeUnit::new(
            CodeUnitType::Impl,
            Language::Rust,
            name,
            file_path.to_path_buf(),
            span,
        );
        unit.temp_id = id;
        unit.qualified_name = qname;

        if let Some(tr) = trait_name {
            unit.references.push(RawReference {
                name: tr,
                kind: ReferenceKind::Implement,
                span: node_to_span(type_node),
            });
        }

        Some(unit)
    }

    fn extract_mod(
        &self,
        node: tree_sitter::Node,
        source: &str,
        file_path: &Path,
        parent_qname: &str,
        next_id: &mut u64,
    ) -> Option<RawCodeUnit> {
        let name_node = node.child_by_field_name("name")?;
        let name = get_node_text(name_node, source).to_string();
        let qname = make_qname(parent_qname, &name);
        let span = node_to_span(node);
        let vis = rust_visibility(node, source);

        let id = *next_id;
        *next_id += 1;

        let mut unit = RawCodeUnit::new(
            CodeUnitType::Module,
            Language::Rust,
            name,
            file_path.to_path_buf(),
            span,
        );
        unit.temp_id = id;
        unit.qualified_name = qname;
        unit.visibility = vis;

        Some(unit)
    }

    fn extract_use(
        &self,
        node: tree_sitter::Node,
        source: &str,
        file_path: &Path,
        parent_qname: &str,
        next_id: &mut u64,
    ) -> Option<RawCodeUnit> {
        let text = get_node_text(node, source).to_string();
        let span = node_to_span(node);
        let import_name = text
            .trim_start_matches("use ")
            .trim_end_matches(';')
            .trim()
            .to_string();

        let id = *next_id;
        *next_id += 1;

        let mut unit = RawCodeUnit::new(
            CodeUnitType::Import,
            Language::Rust,
            import_name.clone(),
            file_path.to_path_buf(),
            span,
        );
        unit.temp_id = id;
        unit.qualified_name = make_qname(parent_qname, &import_name);
        unit.references.push(RawReference {
            name: import_name,
            kind: ReferenceKind::Import,
            span,
        });

        Some(unit)
    }

    fn extract_macro(
        &self,
        node: tree_sitter::Node,
        source: &str,
        file_path: &Path,
        parent_qname: &str,
        next_id: &mut u64,
    ) -> Option<RawCodeUnit> {
        let name_node = node.child_by_field_name("name")?;
        let name = get_node_text(name_node, source).to_string();
        let qname = make_qname(parent_qname, &name);
        let span = node_to_span(node);

        let id = *next_id;
        *next_id += 1;

        let mut unit = RawCodeUnit::new(
            CodeUnitType::Macro,
            Language::Rust,
            name,
            file_path.to_path_buf(),
            span,
        );
        unit.temp_id = id;
        unit.qualified_name = qname;
        unit.visibility = rust_visibility(node, source);

        Some(unit)
    }
}

impl LanguageParser for RustParser {
    fn extract_units(
        &self,
        tree: &tree_sitter::Tree,
        source: &str,
        file_path: &Path,
    ) -> AcbResult<Vec<RawCodeUnit>> {
        let mut units = Vec::new();
        let mut next_id = 0u64;

        let module_name = file_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("unknown")
            .to_string();

        let root_span = node_to_span(tree.root_node());
        let mut module_unit = RawCodeUnit::new(
            CodeUnitType::Module,
            Language::Rust,
            module_name.clone(),
            file_path.to_path_buf(),
            root_span,
        );
        module_unit.temp_id = next_id;
        module_unit.qualified_name = module_name.clone();
        next_id += 1;
        units.push(module_unit);

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
        path.components().any(|c| c.as_os_str() == "tests")
            || name.ends_with("_test.rs")
            || source.contains("#[cfg(test)]")
            || source.contains("#[test]")
    }
}

fn make_qname(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_string()
    } else {
        format!("{}::{}", parent, name)
    }
}

fn rust_visibility(node: tree_sitter::Node, source: &str) -> Visibility {
    let text = get_node_text(node, source);
    if text.starts_with("pub(crate)") {
        Visibility::Internal
    } else if text.starts_with("pub(super)") {
        Visibility::Protected
    } else if text.starts_with("pub ") || text.starts_with("pub(") {
        Visibility::Public
    } else {
        Visibility::Private
    }
}

fn extract_rust_doc(node: tree_sitter::Node, source: &str) -> Option<String> {
    let mut prev = node.prev_sibling();
    while let Some(p) = prev {
        if p.kind() == "line_comment" {
            let text = get_node_text(p, source);
            if let Some(stripped) = text.strip_prefix("///") {
                return Some(stripped.trim().to_string());
            }
            if let Some(stripped) = text.strip_prefix("//!") {
                return Some(stripped.trim().to_string());
            }
        } else {
            break;
        }
        prev = p.prev_sibling();
    }
    None
}
