//! CodeUnit — the atomic element of the code graph.
//!
//! A code unit represents any identifiable piece of code: a function, class,
//! module, import, test, or documentation block.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use super::language::Language;
use super::span::Span;
use super::DEFAULT_DIMENSION;

/// The type of code unit stored in a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum CodeUnitType {
    /// A logical grouping (file, package, namespace, module).
    Module = 0,
    /// A named entity (function, class, variable, constant).
    Symbol = 1,
    /// A type definition (class, struct, interface, enum, type alias).
    Type = 2,
    /// A callable unit (function, method, closure).
    Function = 3,
    /// A function parameter or struct/class field.
    Parameter = 4,
    /// A dependency declaration (import, require, use).
    Import = 5,
    /// A test case or test suite.
    Test = 6,
    /// Documentation block (docstring, JSDoc, comment block).
    Doc = 7,
    /// Configuration value or constant.
    Config = 8,
    /// An identified design pattern (Singleton, Factory, etc.).
    Pattern = 9,
    /// A trait, interface, or protocol definition.
    Trait = 10,
    /// An implementation block (impl, class body).
    Impl = 11,
    /// A macro definition or invocation.
    Macro = 12,
}

impl CodeUnitType {
    /// Convert from raw byte value.
    ///
    /// Returns `None` for values that don't correspond to a known variant.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Module),
            1 => Some(Self::Symbol),
            2 => Some(Self::Type),
            3 => Some(Self::Function),
            4 => Some(Self::Parameter),
            5 => Some(Self::Import),
            6 => Some(Self::Test),
            7 => Some(Self::Doc),
            8 => Some(Self::Config),
            9 => Some(Self::Pattern),
            10 => Some(Self::Trait),
            11 => Some(Self::Impl),
            12 => Some(Self::Macro),
            _ => None,
        }
    }

    /// Returns true if this type represents a callable.
    pub fn is_callable(&self) -> bool {
        matches!(self, Self::Function | Self::Macro)
    }

    /// Returns true if this type can have children.
    pub fn is_container(&self) -> bool {
        matches!(self, Self::Module | Self::Type | Self::Trait | Self::Impl)
    }

    /// Returns a human-readable label for this type.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Module => "module",
            Self::Symbol => "symbol",
            Self::Type => "type",
            Self::Function => "function",
            Self::Parameter => "parameter",
            Self::Import => "import",
            Self::Test => "test",
            Self::Doc => "doc",
            Self::Config => "config",
            Self::Pattern => "pattern",
            Self::Trait => "trait",
            Self::Impl => "impl",
            Self::Macro => "macro",
        }
    }
}

impl std::fmt::Display for CodeUnitType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// Symbol visibility/accessibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum Visibility {
    /// Accessible from anywhere.
    Public = 0,
    /// Accessible within module/file.
    Private = 1,
    /// Accessible within package/crate.
    Internal = 2,
    /// Protected (subclass access).
    Protected = 3,
    /// Unknown visibility.
    Unknown = 255,
}

impl Visibility {
    /// Convert from raw byte value.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Public),
            1 => Some(Self::Private),
            2 => Some(Self::Internal),
            3 => Some(Self::Protected),
            255 => Some(Self::Unknown),
            _ => None,
        }
    }
}

impl std::fmt::Display for Visibility {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Public => write!(f, "public"),
            Self::Private => write!(f, "private"),
            Self::Internal => write!(f, "internal"),
            Self::Protected => write!(f, "protected"),
            Self::Unknown => write!(f, "unknown"),
        }
    }
}

/// A single code unit — the atomic element of the code graph.
///
/// Code units are the nodes of the semantic graph. Each represents an
/// identifiable piece of code: a function, class, module, import, etc.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeUnit {
    /// Unique identifier (assigned sequentially during compilation).
    pub id: u64,

    /// Type of code unit.
    pub unit_type: CodeUnitType,

    /// Programming language.
    pub language: Language,

    /// Simple name (e.g., "process_payment").
    pub name: String,

    /// Fully qualified name (e.g., "payments.stripe.process_payment").
    pub qualified_name: String,

    /// Source file path (relative to repo root).
    pub file_path: PathBuf,

    /// Location in source file.
    pub span: Span,

    /// Type signature if applicable (e.g., "(amount: Decimal) -> bool").
    pub signature: Option<String>,

    /// First line of documentation.
    pub doc_summary: Option<String>,

    // === Semantic metadata ===
    /// Visibility level.
    pub visibility: Visibility,

    /// Cyclomatic complexity (0 for non-functions).
    pub complexity: u32,

    /// Is this async/await?
    pub is_async: bool,

    /// Is this a generator/iterator?
    pub is_generator: bool,

    // === Temporal metadata ===
    /// First seen timestamp (git commit time, or compile time if no git).
    pub created_at: u64,

    /// Last modified timestamp.
    pub last_modified: u64,

    /// Total changes in git history.
    pub change_count: u32,

    /// Stability score: 0.0 = constantly changing, 1.0 = never changes.
    pub stability_score: f32,

    // === Collective metadata ===
    /// Global usage count from collective (0 if private code).
    pub collective_usage: u64,

    /// Content hash for deduplication (Blake3).
    pub content_hash: [u8; 32],

    // === Vector for semantic search ===
    /// Feature vector for similarity (dimension = DEFAULT_DIMENSION).
    pub feature_vec: Vec<f32>,

    // === Graph position (set by graph builder) ===
    /// Byte offset into edge table.
    pub edge_offset: u64,

    /// Number of outgoing edges.
    pub edge_count: u32,
}

impl CodeUnit {
    /// Create a new code unit with required fields only.
    ///
    /// Optional fields are initialized to their defaults:
    /// - `id` is 0 (set by the graph on insertion)
    /// - `visibility` is `Unknown`
    /// - `stability_score` is 1.0 (stable by default)
    /// - `feature_vec` is zero-filled with `DEFAULT_DIMENSION` elements
    pub fn new(
        unit_type: CodeUnitType,
        language: Language,
        name: String,
        qualified_name: String,
        file_path: PathBuf,
        span: Span,
    ) -> Self {
        let now = crate::types::now_micros();
        Self {
            id: 0,
            unit_type,
            language,
            name,
            qualified_name,
            file_path,
            span,
            signature: None,
            doc_summary: None,
            visibility: Visibility::Unknown,
            complexity: 0,
            is_async: false,
            is_generator: false,
            created_at: now,
            last_modified: now,
            change_count: 0,
            stability_score: 1.0,
            collective_usage: 0,
            content_hash: [0u8; 32],
            feature_vec: vec![0.0; DEFAULT_DIMENSION],
            edge_offset: 0,
            edge_count: 0,
        }
    }
}

/// Builder for constructing [`CodeUnit`] instances with optional fields.
///
/// # Examples
///
/// ```
/// use agentic_codebase::types::*;
/// use std::path::PathBuf;
///
/// let unit = CodeUnitBuilder::new(
///     CodeUnitType::Function,
///     Language::Python,
///     "my_func",
///     "mymodule.my_func",
///     PathBuf::from("src/mymodule.py"),
///     Span::new(10, 0, 20, 0),
/// )
/// .signature("(x: int) -> bool")
/// .doc("Checks if x is valid")
/// .visibility(Visibility::Public)
/// .complexity(3)
/// .build();
/// ```
pub struct CodeUnitBuilder {
    inner: CodeUnit,
}

impl CodeUnitBuilder {
    /// Create a new builder with required fields.
    pub fn new(
        unit_type: CodeUnitType,
        language: Language,
        name: impl Into<String>,
        qualified_name: impl Into<String>,
        file_path: impl Into<PathBuf>,
        span: Span,
    ) -> Self {
        Self {
            inner: CodeUnit::new(
                unit_type,
                language,
                name.into(),
                qualified_name.into(),
                file_path.into(),
                span,
            ),
        }
    }

    /// Set the type signature.
    pub fn signature(mut self, sig: impl Into<String>) -> Self {
        self.inner.signature = Some(sig.into());
        self
    }

    /// Set the documentation summary.
    pub fn doc(mut self, doc: impl Into<String>) -> Self {
        self.inner.doc_summary = Some(doc.into());
        self
    }

    /// Set the visibility level.
    pub fn visibility(mut self, vis: Visibility) -> Self {
        self.inner.visibility = vis;
        self
    }

    /// Set the cyclomatic complexity.
    pub fn complexity(mut self, c: u32) -> Self {
        self.inner.complexity = c;
        self
    }

    /// Mark this unit as async.
    pub fn async_fn(mut self) -> Self {
        self.inner.is_async = true;
        self
    }

    /// Mark this unit as a generator.
    pub fn generator(mut self) -> Self {
        self.inner.is_generator = true;
        self
    }

    /// Set the feature vector.
    pub fn feature_vec(mut self, vec: Vec<f32>) -> Self {
        self.inner.feature_vec = vec;
        self
    }

    /// Set the content hash.
    pub fn content_hash(mut self, hash: [u8; 32]) -> Self {
        self.inner.content_hash = hash;
        self
    }

    /// Set timestamps.
    pub fn timestamps(mut self, created: u64, modified: u64) -> Self {
        self.inner.created_at = created;
        self.inner.last_modified = modified;
        self
    }

    /// Consume the builder and produce a [`CodeUnit`].
    pub fn build(self) -> CodeUnit {
        self.inner
    }
}
