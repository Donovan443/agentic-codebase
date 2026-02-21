//! TypeScript and JavaScript parsing using tree-sitter.
//!
//! Extracts functions, classes, interfaces, type aliases, imports, methods.

use std::path::Path;

use crate::types::{AcbResult, CodeUnitType, Language, Visibility};

use super::treesitter::{get_node_text, node_to_span};
use super::{LanguageParser, RawCodeUnit, RawReference, ReferenceKind};

/// TypeScript and JavaScript language parser.
pub struct TypeScriptParser;

impl Default for TypeScriptParser {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeScriptParser {
    /// Create a new TypeScript parser.
    pub fn new() -> Self {
        Self
    }

    fn detect_language(file_path: &Path) -> Language {
        match file_path.extension().and_then(|e| e.to_str()) {
            Some("ts") | Some("tsx") => Language::TypeScript,
            Some("js") | Some("jsx") | Some("mjs") | Some("cjs") => Language::JavaScript,
            _ => Language::TypeScript,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn extract_from_node(
        &self,
        node: tree_sitter::Node,
        source: &str,
        file_path: &Path,
        units: &mut Vec<RawCodeUnit>,
        next_id: &mut u64,
        parent_qname: &str,
        lang: Language,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            match child.kind() {
                "function_declaration" => {
                    if let Some(unit) =
                        self.extract_function(child, source, file_path, parent_qname, next_id, lang)
                    {
                        units.push(unit);
                    }
                }
                "class_declaration" => {
                    if let Some(unit) =
                        self.extract_class(child, source, file_path, parent_qname, next_id, lang)
                    {
                        let qname = unit.qualified_name.clone();
                        units.push(unit);
                        if let Some(body) = child.child_by_field_name("body") {
                            self.extract_from_node(
                                body, source, file_path, units, next_id, &qname, lang,
                            );
                        }
                    }
                }
                "interface_declaration" => {
                    if let Some(unit) = self.extract_interface(
                        child,
                        source,
                        file_path,
                        parent_qname,
                        next_id,
                        lang,
                    ) {
                        units.push(unit);
                    }
                }
                "type_alias_declaration" => {
                    if let Some(unit) = self.extract_type_alias(
                        child,
                        source,
                        file_path,
                        parent_qname,
                        next_id,
                        lang,
                    ) {
                        units.push(unit);
                    }
                }
                "import_statement" => {
                    if let Some(unit) =
                        self.extract_import(child, source, file_path, parent_qname, next_id, lang)
                    {
                        units.push(unit);
                    }
                }
                "export_statement" => {
                    // Look inside export for the actual declaration
                    self.extract_from_node(
                        child,
                        source,
                        file_path,
                        units,
                        next_id,
                        parent_qname,
                        lang,
                    );
                }
                "method_definition" => {
                    if let Some(unit) =
                        self.extract_method(child, source, file_path, parent_qname, next_id, lang)
                    {
                        units.push(unit);
                    }
                }
                "lexical_declaration" | "variable_declaration" => {
                    // Check for arrow function assignments: const foo = () => {}
                    self.extract_arrow_functions(
                        child,
                        source,
                        file_path,
                        units,
                        next_id,
                        parent_qname,
                        lang,
                    );
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
        lang: Language,
    ) -> Option<RawCodeUnit> {
        let name_node = node.child_by_field_name("name")?;
        let name = get_node_text(name_node, source).to_string();
        let qname = ts_qname(parent_qname, &name);
        let span = node_to_span(node);

        let sig = node
            .child_by_field_name("parameters")
            .map(|p| get_node_text(p, source).to_string());
        let is_async = get_node_text(node, source)
            .trim_start()
            .starts_with("async ");

        let id = *next_id;
        *next_id += 1;

        let mut unit = RawCodeUnit::new(
            CodeUnitType::Function,
            lang,
            name,
            file_path.to_path_buf(),
            span,
        );
        unit.temp_id = id;
        unit.qualified_name = qname;
        unit.signature = sig;
        unit.is_async = is_async;
        unit.visibility = Visibility::Public;

        Some(unit)
    }

    fn extract_class(
        &self,
        node: tree_sitter::Node,
        source: &str,
        file_path: &Path,
        parent_qname: &str,
        next_id: &mut u64,
        lang: Language,
    ) -> Option<RawCodeUnit> {
        let name_node = node.child_by_field_name("name")?;
        let name = get_node_text(name_node, source).to_string();
        let qname = ts_qname(parent_qname, &name);
        let span = node_to_span(node);

        let id = *next_id;
        *next_id += 1;

        let mut unit = RawCodeUnit::new(
            CodeUnitType::Type,
            lang,
            name,
            file_path.to_path_buf(),
            span,
        );
        unit.temp_id = id;
        unit.qualified_name = qname;
        unit.visibility = Visibility::Public;

        // Extract heritage (extends, implements)
        let mut c = node.walk();
        for child in node.children(&mut c) {
            if child.kind() == "class_heritage" {
                let heritage_text = get_node_text(child, source);
                if heritage_text.contains("extends") || heritage_text.contains("implements") {
                    unit.references.push(RawReference {
                        name: heritage_text.trim().to_string(),
                        kind: ReferenceKind::Inherit,
                        span: node_to_span(child),
                    });
                }
            }
        }

        Some(unit)
    }

    fn extract_interface(
        &self,
        node: tree_sitter::Node,
        source: &str,
        file_path: &Path,
        parent_qname: &str,
        next_id: &mut u64,
        lang: Language,
    ) -> Option<RawCodeUnit> {
        let name_node = node.child_by_field_name("name")?;
        let name = get_node_text(name_node, source).to_string();
        let qname = ts_qname(parent_qname, &name);
        let span = node_to_span(node);

        let id = *next_id;
        *next_id += 1;

        let mut unit = RawCodeUnit::new(
            CodeUnitType::Trait,
            lang,
            name,
            file_path.to_path_buf(),
            span,
        );
        unit.temp_id = id;
        unit.qualified_name = qname;
        unit.visibility = Visibility::Public;

        Some(unit)
    }

    fn extract_type_alias(
        &self,
        node: tree_sitter::Node,
        source: &str,
        file_path: &Path,
        parent_qname: &str,
        next_id: &mut u64,
        lang: Language,
    ) -> Option<RawCodeUnit> {
        let name_node = node.child_by_field_name("name")?;
        let name = get_node_text(name_node, source).to_string();
        let qname = ts_qname(parent_qname, &name);
        let span = node_to_span(node);

        let id = *next_id;
        *next_id += 1;

        let mut unit = RawCodeUnit::new(
            CodeUnitType::Type,
            lang,
            name,
            file_path.to_path_buf(),
            span,
        );
        unit.temp_id = id;
        unit.qualified_name = qname;
        unit.visibility = Visibility::Public;

        Some(unit)
    }

    fn extract_import(
        &self,
        node: tree_sitter::Node,
        source: &str,
        file_path: &Path,
        parent_qname: &str,
        next_id: &mut u64,
        lang: Language,
    ) -> Option<RawCodeUnit> {
        let text = get_node_text(node, source).to_string();
        let span = node_to_span(node);

        // Extract module name from import statement
        let import_name = text
            .split("from")
            .last()
            .unwrap_or(&text)
            .trim()
            .trim_matches(|c: char| c == '\'' || c == '"' || c == ';')
            .to_string();

        let id = *next_id;
        *next_id += 1;

        let mut unit = RawCodeUnit::new(
            CodeUnitType::Import,
            lang,
            import_name.clone(),
            file_path.to_path_buf(),
            span,
        );
        unit.temp_id = id;
        unit.qualified_name = ts_qname(parent_qname, &import_name);
        unit.references.push(RawReference {
            name: import_name,
            kind: ReferenceKind::Import,
            span,
        });

        Some(unit)
    }

    fn extract_method(
        &self,
        node: tree_sitter::Node,
        source: &str,
        file_path: &Path,
        parent_qname: &str,
        next_id: &mut u64,
        lang: Language,
    ) -> Option<RawCodeUnit> {
        let name_node = node.child_by_field_name("name")?;
        let name = get_node_text(name_node, source).to_string();
        let qname = ts_qname(parent_qname, &name);
        let span = node_to_span(node);

        let is_async = get_node_text(node, source)
            .trim_start()
            .starts_with("async ");

        let id = *next_id;
        *next_id += 1;

        let mut unit = RawCodeUnit::new(
            CodeUnitType::Function,
            lang,
            name,
            file_path.to_path_buf(),
            span,
        );
        unit.temp_id = id;
        unit.qualified_name = qname;
        unit.is_async = is_async;
        unit.visibility = Visibility::Public;

        Some(unit)
    }

    #[allow(clippy::too_many_arguments)]
    fn extract_arrow_functions(
        &self,
        node: tree_sitter::Node,
        source: &str,
        file_path: &Path,
        units: &mut Vec<RawCodeUnit>,
        next_id: &mut u64,
        parent_qname: &str,
        lang: Language,
    ) {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if child.kind() == "variable_declarator" {
                let name_node = child.child_by_field_name("name");
                let value_node = child.child_by_field_name("value");
                if let (Some(name_n), Some(val_n)) = (name_node, value_node) {
                    if val_n.kind() == "arrow_function" {
                        let name = get_node_text(name_n, source).to_string();
                        let qname = ts_qname(parent_qname, &name);
                        let span = node_to_span(child);

                        let id = *next_id;
                        *next_id += 1;

                        let mut unit = RawCodeUnit::new(
                            CodeUnitType::Function,
                            lang,
                            name,
                            file_path.to_path_buf(),
                            span,
                        );
                        unit.temp_id = id;
                        unit.qualified_name = qname;
                        unit.visibility = Visibility::Public;
                        units.push(unit);
                    }
                }
            }
        }
    }
}

impl LanguageParser for TypeScriptParser {
    fn extract_units(
        &self,
        tree: &tree_sitter::Tree,
        source: &str,
        file_path: &Path,
    ) -> AcbResult<Vec<RawCodeUnit>> {
        let lang = Self::detect_language(file_path);
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
            lang,
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
            lang,
        );

        Ok(units)
    }

    fn is_test_file(&self, path: &Path, source: &str) -> bool {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        name.ends_with(".test.ts")
            || name.ends_with(".test.tsx")
            || name.ends_with(".spec.ts")
            || name.ends_with(".spec.tsx")
            || name.ends_with(".test.js")
            || name.ends_with(".spec.js")
            || path.components().any(|c| {
                let s = c.as_os_str().to_str().unwrap_or("");
                s == "__tests__" || s == "tests" || s == "test"
            })
            || source.contains("describe(")
            || source.contains("it(")
    }
}

fn ts_qname(parent: &str, name: &str) -> String {
    if parent.is_empty() {
        name.to_string()
    } else {
        format!("{}.{}", parent, name)
    }
}
