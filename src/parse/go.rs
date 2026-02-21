//! Go parsing using tree-sitter.
//!
//! Extracts functions, methods, types, imports, packages.

use std::path::Path;

use crate::types::{AcbResult, CodeUnitType, Language, Visibility};

use super::treesitter::{get_node_text, node_to_span};
use super::{LanguageParser, RawCodeUnit};

/// Go language parser.
pub struct GoParser;

impl Default for GoParser {
    fn default() -> Self {
        Self::new()
    }
}

impl GoParser {
    /// Create a new Go parser.
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
                "function_declaration" => {
                    if let Some(unit) =
                        self.extract_function(child, source, file_path, parent_qname, next_id)
                    {
                        units.push(unit);
                    }
                }
                "method_declaration" => {
                    if let Some(unit) =
                        self.extract_method(child, source, file_path, parent_qname, next_id)
                    {
                        units.push(unit);
                    }
                }
                "type_declaration" => {
                    self.extract_type_decl(child, source, file_path, units, next_id, parent_qname);
                }
                "import_declaration" => {
                    if let Some(unit) =
                        self.extract_import(child, source, file_path, parent_qname, next_id)
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
        let qname = go_qname(parent_qname, &name);
        let span = node_to_span(node);

        let vis = if name
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
        {
            Visibility::Public
        } else {
            Visibility::Private
        };

        let is_test = name.starts_with("Test") || name.starts_with("Benchmark");

        let id = *next_id;
        *next_id += 1;

        let unit_type = if is_test {
            CodeUnitType::Test
        } else {
            CodeUnitType::Function
        };
        let mut unit =
            RawCodeUnit::new(unit_type, Language::Go, name, file_path.to_path_buf(), span);
        unit.temp_id = id;
        unit.qualified_name = qname;
        unit.visibility = vis;

        Some(unit)
    }

    fn extract_method(
        &self,
        node: tree_sitter::Node,
        source: &str,
        file_path: &Path,
        parent_qname: &str,
        next_id: &mut u64,
    ) -> Option<RawCodeUnit> {
        let name_node = node.child_by_field_name("name")?;
        let name = get_node_text(name_node, source).to_string();
        let qname = go_qname(parent_qname, &name);
        let span = node_to_span(node);

        let vis = if name
            .chars()
            .next()
            .map(|c| c.is_uppercase())
            .unwrap_or(false)
        {
            Visibility::Public
        } else {
            Visibility::Private
        };

        let id = *next_id;
        *next_id += 1;

        let mut unit = RawCodeUnit::new(
            CodeUnitType::Function,
            Language::Go,
            name,
            file_path.to_path_buf(),
            span,
        );
        unit.temp_id = id;
        unit.qualified_name = qname;
        unit.visibility = vis;

        Some(unit)
    }

    fn extract_type_decl(
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
            if child.kind() == "type_spec" {
                if let Some(name_node) = child.child_by_field_name("name") {
                    let name = get_node_text(name_node, source).to_string();
                    let qname = go_qname(parent_qname, &name);
                    let span = node_to_span(child);

                    let vis = if name
                        .chars()
                        .next()
                        .map(|c| c.is_uppercase())
                        .unwrap_or(false)
                    {
                        Visibility::Public
                    } else {
                        Visibility::Private
                    };

                    let id = *next_id;
                    *next_id += 1;

                    // Determine if it's an interface
                    let type_def = child.child_by_field_name("type");
                    let unit_type = if type_def
                        .map(|t| t.kind() == "interface_type")
                        .unwrap_or(false)
                    {
                        CodeUnitType::Trait
                    } else {
                        CodeUnitType::Type
                    };

                    let mut unit = RawCodeUnit::new(
                        unit_type,
                        Language::Go,
                        name,
                        file_path.to_path_buf(),
                        span,
                    );
                    unit.temp_id = id;
                    unit.qualified_name = qname;
                    unit.visibility = vis;
                    units.push(unit);
                }
            }
        }
    }

    fn extract_import(
        &self,
        node: tree_sitter::Node,
        source: &str,
        file_path: &Path,
        parent_qname: &str,
        next_id: &mut u64,
    ) -> Option<RawCodeUnit> {
        let _text = get_node_text(node, source);
        let span = node_to_span(node);

        let id = *next_id;
        *next_id += 1;

        let mut unit = RawCodeUnit::new(
            CodeUnitType::Import,
            Language::Go,
            "import".to_string(),
            file_path.to_path_buf(),
            span,
        );
        unit.temp_id = id;
        unit.qualified_name = go_qname(parent_qname, "import");

        Some(unit)
    }
}

impl LanguageParser for GoParser {
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
            Language::Go,
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

    fn is_test_file(&self, path: &Path, _source: &str) -> bool {
        path.file_name()
            .and_then(|n| n.to_str())
            .map(|n| n.ends_with("_test.go"))
            .unwrap_or(false)
    }
}

fn go_qname(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_string()
    } else {
        format!("{}.{}", parent, name)
    }
}
