//! Phase 8: Collective intelligence tests.
//!
//! Tests for delta compression, pattern extraction, privacy filtering,
//! and the offline-capable registry client.

use agentic_codebase::collective::delta::{
    CollectiveDelta, DeltaPattern, MistakeCategory, MistakeReport,
};
use agentic_codebase::collective::patterns::{
    ComplexityBucket, PatternCategory, PatternExtractor, PatternQuality,
};
use agentic_codebase::collective::privacy::{filter_shareable, is_shareable, ShareableCheck};
use agentic_codebase::collective::registry::{RegistryClient, RegistryMode};
use agentic_codebase::collective::CollectiveManager;
use agentic_codebase::graph::CodeGraph;

// ============================================================================
// Delta compression tests
// ============================================================================

#[test]
fn test_delta_creation() {
    let mut delta = CollectiveDelta::new("test-source".to_string());
    assert!(delta.is_empty());

    delta.add_pattern(DeltaPattern {
        name: "singleton".to_string(),
        language: "rust".to_string(),
        signature: "struct { static instance }".to_string(),
        occurrence_count: 5,
        confidence: 0.9,
    });

    assert!(!delta.is_empty());
    assert_eq!(delta.patterns.len(), 1);
    assert_eq!(delta.patterns[0].name, "singleton");
}

#[test]
fn test_delta_with_mistakes() {
    let mut delta = CollectiveDelta::new("test-source".to_string());
    delta.add_mistake(MistakeReport {
        category: MistakeCategory::BugPattern,
        description: "Unchecked null dereference".to_string(),
        pattern_signature: "deref(nullable)".to_string(),
        suggestion: "Use Option::map or if-let".to_string(),
        severity: 0.8,
    });
    delta.add_mistake(MistakeReport {
        category: MistakeCategory::SecurityVulnerability,
        description: "SQL injection via string concatenation".to_string(),
        pattern_signature: "format!(\"SELECT {} ...\")".to_string(),
        suggestion: "Use parameterised queries".to_string(),
        severity: 1.0,
    });

    assert_eq!(delta.mistakes.len(), 2);
    assert_eq!(delta.mistakes[0].category, MistakeCategory::BugPattern);
    assert_eq!(
        delta.mistakes[1].category,
        MistakeCategory::SecurityVulnerability
    );
}

#[test]
fn test_delta_compress_decompress() {
    let mut delta = CollectiveDelta::new("compress-test".to_string());
    delta.add_pattern(DeltaPattern {
        name: "factory".to_string(),
        language: "python".to_string(),
        signature: "class Factory { create() }".to_string(),
        occurrence_count: 12,
        confidence: 0.85,
    });
    delta.add_mistake(MistakeReport {
        category: MistakeCategory::CodeSmell,
        description: "God class detected".to_string(),
        pattern_signature: "class { methods > 30 }".to_string(),
        suggestion: "Split into smaller classes".to_string(),
        severity: 0.6,
    });

    let compressed = delta.compress().unwrap();
    assert!(!compressed.is_empty());

    let decompressed = CollectiveDelta::decompress(&compressed).unwrap();
    assert_eq!(decompressed.source_id, "compress-test");
    assert_eq!(decompressed.patterns.len(), 1);
    assert_eq!(decompressed.patterns[0].name, "factory");
    assert_eq!(decompressed.mistakes.len(), 1);
    assert_eq!(
        decompressed.mistakes[0].category,
        MistakeCategory::CodeSmell
    );
}

#[test]
fn test_delta_finalize_sets_id() {
    let mut delta = CollectiveDelta::new("finalize-test".to_string());
    delta.add_pattern(DeltaPattern {
        name: "observer".to_string(),
        language: "java".to_string(),
        signature: "interface Observer { update() }".to_string(),
        occurrence_count: 3,
        confidence: 0.7,
    });

    assert!(delta.delta_id.is_empty());
    delta.finalize().unwrap();
    assert!(!delta.delta_id.is_empty());

    // Finalize again should produce a different ID since delta_id changed.
    let first_id = delta.delta_id.clone();
    delta.finalize().unwrap();
    // ID changes because the content (including delta_id) changed.
    assert_ne!(first_id, delta.delta_id);
}

#[test]
fn test_delta_empty_compress() {
    let delta = CollectiveDelta::new("empty".to_string());
    let compressed = delta.compress().unwrap();
    let decompressed = CollectiveDelta::decompress(&compressed).unwrap();
    assert!(decompressed.is_empty());
    assert_eq!(decompressed.source_id, "empty");
}

#[test]
fn test_delta_decompress_invalid_data() {
    let result = CollectiveDelta::decompress(&[0xFF, 0xFF, 0xFF]);
    assert!(result.is_err());
}

// ============================================================================
// Pattern extraction tests
// ============================================================================

#[test]
fn test_pattern_extractor_basic() {
    let graph = CodeGraph::with_default_dimension();
    let extractor = PatternExtractor::new();
    let patterns = extractor.extract(&graph);

    // Placeholder implementation returns empty.
    assert!(patterns.is_empty());
}

#[test]
fn test_pattern_extractor_min_occurrences() {
    let extractor = PatternExtractor::with_min_occurrences(5);
    assert_eq!(extractor.min_occurrences(), 5);
}

#[test]
fn test_complexity_bucket() {
    assert_eq!(ComplexityBucket::from_complexity(0), ComplexityBucket::Low);
    assert_eq!(ComplexityBucket::from_complexity(5), ComplexityBucket::Low);
    assert_eq!(
        ComplexityBucket::from_complexity(10),
        ComplexityBucket::Medium
    );
    assert_eq!(
        ComplexityBucket::from_complexity(20),
        ComplexityBucket::High
    );
    assert_eq!(
        ComplexityBucket::from_complexity(50),
        ComplexityBucket::VeryHigh
    );
}

#[test]
fn test_pattern_category_display() {
    assert_eq!(PatternCategory::DesignPattern.to_string(), "design-pattern");
    assert_eq!(PatternCategory::ApiUsage.to_string(), "api-usage");
    assert_eq!(PatternCategory::ErrorHandling.to_string(), "error-handling");
}

#[test]
fn test_pattern_quality_display() {
    assert_eq!(PatternQuality::Established.to_string(), "established");
    assert_eq!(PatternQuality::Emerging.to_string(), "emerging");
    assert_eq!(PatternQuality::Declining.to_string(), "declining");
    assert_eq!(PatternQuality::Unknown.to_string(), "unknown");
}

// ============================================================================
// Privacy filter tests
// ============================================================================

#[test]
fn test_privacy_shareable_pattern() {
    let check = ShareableCheck::PatternSignature("struct { static instance }".to_string());
    assert!(is_shareable(&check));
}

#[test]
fn test_privacy_shareable_stats() {
    let check = ShareableCheck::AggregateStats {
        name: "avg_complexity".to_string(),
        value: 4.5,
    };
    assert!(is_shareable(&check));
}

#[test]
fn test_privacy_non_shareable_source() {
    let check = ShareableCheck::SourceCode("fn main() { }".to_string());
    assert!(!is_shareable(&check));
}

#[test]
fn test_privacy_non_shareable_path() {
    let check = ShareableCheck::FilePath("/home/user/secret/project/main.rs".to_string());
    assert!(!is_shareable(&check));
}

#[test]
fn test_privacy_non_shareable_symbol() {
    let check = ShareableCheck::SymbolName("process_payment".to_string());
    assert!(!is_shareable(&check));
}

#[test]
fn test_privacy_non_shareable_author() {
    let check = ShareableCheck::AuthorIdentity("alice@example.com".to_string());
    assert!(!is_shareable(&check));
}

#[test]
fn test_privacy_raw_text_credentials() {
    let check = ShareableCheck::RawText("API_KEY=sk-1234567890".to_string());
    assert!(!is_shareable(&check));

    let safe = ShareableCheck::RawText("Hello, world!".to_string());
    assert!(is_shareable(&safe));
}

#[test]
fn test_privacy_filter_batch() {
    let items = vec![
        ShareableCheck::PatternSignature("sig1".to_string()),
        ShareableCheck::SourceCode("fn secret() {}".to_string()),
        ShareableCheck::AggregateStats {
            name: "count".to_string(),
            value: 42.0,
        },
        ShareableCheck::FilePath("/secret/path.rs".to_string()),
        ShareableCheck::MistakeCount {
            category: "bug".to_string(),
            count: 5,
        },
    ];

    let shareable = filter_shareable(&items);
    assert_eq!(shareable.len(), 3); // PatternSignature, AggregateStats, MistakeCount
}

// ============================================================================
// Registry client tests
// ============================================================================

#[test]
fn test_registry_offline_mode() {
    let mut client = RegistryClient::offline();
    assert_eq!(*client.mode(), RegistryMode::Offline);
    assert!(client.endpoint().is_none());

    let patterns = client.query_patterns("rust", "design-pattern");
    assert!(patterns.is_empty());
}

#[test]
fn test_registry_online_stub() {
    let mut client = RegistryClient::online("https://registry.example.com".to_string());
    assert_eq!(*client.mode(), RegistryMode::Online);
    assert_eq!(client.endpoint(), Some("https://registry.example.com"));

    // Online mode is a stub, still returns empty.
    let patterns = client.query_patterns("python", "api-usage");
    assert!(patterns.is_empty());
}

#[test]
fn test_registry_publish_delta() {
    let mut client = RegistryClient::offline();
    let delta = CollectiveDelta::new("pub-test".to_string());
    let result = client.publish_delta(&delta);
    assert!(result);
}

#[test]
fn test_registry_cache() {
    let client = RegistryClient::offline();
    assert!(client.cache().is_empty());
}

// ============================================================================
// CollectiveManager tests
// ============================================================================

#[test]
fn test_collective_manager_offline() {
    let mut manager = CollectiveManager::offline();
    assert!(manager.is_offline());
    assert_eq!(*manager.mode(), RegistryMode::Offline);

    let patterns = manager.query_patterns("rust", "any");
    assert!(patterns.is_empty());
}

#[test]
fn test_collective_manager_extract() {
    let manager = CollectiveManager::offline();
    let graph = CodeGraph::with_default_dimension();
    let patterns = manager.extract_patterns(&graph);
    assert!(patterns.is_empty());
}

#[test]
fn test_collective_manager_publish() {
    let mut manager = CollectiveManager::offline();
    let delta = CollectiveDelta::new("mgr-test".to_string());
    assert!(manager.publish_delta(&delta));
}
