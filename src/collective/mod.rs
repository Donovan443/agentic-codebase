//! Collective intelligence layer.
//!
//! Delta sync, pattern aggregation, privacy-preserving extraction.
//! Network-optional — works fully offline.

pub mod delta;
pub mod patterns;
pub mod privacy;
pub mod registry;

pub use delta::{CollectiveDelta, DeltaPattern, MistakeCategory, MistakeReport};
pub use patterns::{
    ComplexityBucket, PatternCategory, PatternExtractor, PatternQuality, PatternSignature,
    UsagePattern,
};
pub use privacy::{filter_shareable, is_shareable, NonShareable, Shareable, ShareableCheck};
pub use registry::{CollectiveCache, RegistryClient, RegistryMode};

/// High-level manager for collective intelligence operations.
///
/// Coordinates pattern extraction, privacy filtering, delta creation,
/// and registry communication. Supports both online and offline modes.
#[derive(Debug)]
pub struct CollectiveManager {
    /// The registry client for syncing data.
    registry: RegistryClient,
    /// The pattern extractor.
    extractor: PatternExtractor,
}

impl CollectiveManager {
    /// Create a new collective manager in offline mode.
    pub fn offline() -> Self {
        Self {
            registry: RegistryClient::offline(),
            extractor: PatternExtractor::new(),
        }
    }

    /// Create a new collective manager in online mode (stub).
    pub fn online(endpoint: String) -> Self {
        Self {
            registry: RegistryClient::online(endpoint),
            extractor: PatternExtractor::new(),
        }
    }

    /// Get the current operating mode.
    pub fn mode(&self) -> &RegistryMode {
        self.registry.mode()
    }

    /// Check if the manager is in offline mode.
    pub fn is_offline(&self) -> bool {
        *self.registry.mode() == RegistryMode::Offline
    }

    /// Extract patterns from a code graph.
    pub fn extract_patterns(&self, graph: &crate::graph::CodeGraph) -> Vec<UsagePattern> {
        self.extractor.extract(graph)
    }

    /// Query the registry for patterns.
    pub fn query_patterns(&mut self, language: &str, category: &str) -> Vec<UsagePattern> {
        self.registry.query_patterns(language, category)
    }

    /// Publish a delta to the registry.
    pub fn publish_delta(&mut self, delta: &CollectiveDelta) -> bool {
        self.registry.publish_delta(delta)
    }

    /// Access the registry client.
    pub fn registry(&self) -> &RegistryClient {
        &self.registry
    }

    /// Access the registry client mutably.
    pub fn registry_mut(&mut self) -> &mut RegistryClient {
        &mut self.registry
    }
}
