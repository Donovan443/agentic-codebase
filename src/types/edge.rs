//! Edge types and structures for relationships between code units.

use serde::{Deserialize, Serialize};

/// The type of relationship between two code units.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[repr(u8)]
pub enum EdgeType {
    /// Runtime invocation: source calls target.
    Calls = 0,
    /// Static dependency: source imports/uses target.
    Imports = 1,
    /// Type hierarchy: source extends/inherits target.
    Inherits = 2,
    /// Interface conformance: source implements target trait/interface.
    Implements = 3,
    /// Method override: source overrides target method.
    Overrides = 4,
    /// Structural containment: source contains target (module contains function).
    Contains = 5,
    /// Non-call reference: source references target without calling.
    References = 6,
    /// Test coverage: source test covers target code.
    Tests = 7,
    /// Documentation: source doc describes target.
    Documents = 8,
    /// Configuration: source configures target.
    Configures = 9,
    /// Hidden coupling: changes together >70% of time (from history).
    CouplesWith = 10,
    /// Breaking relationship: changing source historically breaks target.
    BreaksWith = 11,
    /// Pattern instance: source is an instance of target pattern.
    PatternOf = 12,
    /// Temporal: source is newer version of target.
    VersionOf = 13,
    /// Cross-language: source binds to target across FFI.
    FfiBinds = 14,
    /// Type relationship: source uses target as a type.
    UsesType = 15,
    /// Return type: source returns target type.
    Returns = 16,
    /// Parameter type: source has parameter of target type.
    ParamType = 17,
}

impl EdgeType {
    /// Convert from raw byte value.
    ///
    /// Returns `None` for values that don't correspond to a known variant.
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Calls),
            1 => Some(Self::Imports),
            2 => Some(Self::Inherits),
            3 => Some(Self::Implements),
            4 => Some(Self::Overrides),
            5 => Some(Self::Contains),
            6 => Some(Self::References),
            7 => Some(Self::Tests),
            8 => Some(Self::Documents),
            9 => Some(Self::Configures),
            10 => Some(Self::CouplesWith),
            11 => Some(Self::BreaksWith),
            12 => Some(Self::PatternOf),
            13 => Some(Self::VersionOf),
            14 => Some(Self::FfiBinds),
            15 => Some(Self::UsesType),
            16 => Some(Self::Returns),
            17 => Some(Self::ParamType),
            _ => None,
        }
    }

    /// Returns true if this edge type indicates a dependency.
    pub fn is_dependency(&self) -> bool {
        matches!(
            self,
            Self::Calls
                | Self::Imports
                | Self::Inherits
                | Self::Implements
                | Self::UsesType
                | Self::FfiBinds
        )
    }

    /// Returns true if this edge is derived from history analysis.
    pub fn is_temporal(&self) -> bool {
        matches!(self, Self::CouplesWith | Self::BreaksWith | Self::VersionOf)
    }

    /// Returns a human-readable label for this edge type.
    pub fn label(&self) -> &'static str {
        match self {
            Self::Calls => "calls",
            Self::Imports => "imports",
            Self::Inherits => "inherits",
            Self::Implements => "implements",
            Self::Overrides => "overrides",
            Self::Contains => "contains",
            Self::References => "references",
            Self::Tests => "tests",
            Self::Documents => "documents",
            Self::Configures => "configures",
            Self::CouplesWith => "couples_with",
            Self::BreaksWith => "breaks_with",
            Self::PatternOf => "pattern_of",
            Self::VersionOf => "version_of",
            Self::FfiBinds => "ffi_binds",
            Self::UsesType => "uses_type",
            Self::Returns => "returns",
            Self::ParamType => "param_type",
        }
    }

    /// The total number of edge type variants.
    pub const COUNT: usize = 18;
}

impl std::fmt::Display for EdgeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.label())
    }
}

/// A directed relationship between two code units.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Edge {
    /// Source code unit ID.
    pub source_id: u64,

    /// Target code unit ID.
    pub target_id: u64,

    /// Type of relationship.
    pub edge_type: EdgeType,

    /// Relationship strength (0.0 = weak, 1.0 = strong).
    /// For temporal edges, this is the confidence/frequency.
    pub weight: f32,

    /// When this edge was established.
    pub created_at: u64,

    /// Additional context (e.g., call site line number).
    pub context: u32,
}

impl Edge {
    /// Create a new edge with default weight of 1.0.
    pub fn new(source_id: u64, target_id: u64, edge_type: EdgeType) -> Self {
        Self {
            source_id,
            target_id,
            edge_type,
            weight: 1.0,
            created_at: crate::types::now_micros(),
            context: 0,
        }
    }

    /// Set the weight (clamped to [0.0, 1.0]).
    pub fn with_weight(mut self, weight: f32) -> Self {
        self.weight = weight.clamp(0.0, 1.0);
        self
    }

    /// Set the context value.
    pub fn with_context(mut self, context: u32) -> Self {
        self.context = context;
        self
    }

    /// Returns true if this is a self-edge (source == target).
    pub fn is_self_edge(&self) -> bool {
        self.source_id == self.target_id
    }
}
