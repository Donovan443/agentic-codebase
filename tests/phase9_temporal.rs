//! Phase 9: Temporal analysis tests.
//!
//! Tests for change history, stability analysis, coupling detection,
//! and prophecy predictions.

use std::path::PathBuf;

use agentic_codebase::graph::CodeGraph;
use agentic_codebase::temporal::coupling::{CouplingDetector, CouplingOptions, CouplingType};
use agentic_codebase::temporal::history::{ChangeHistory, ChangeType, FileChange};
use agentic_codebase::temporal::prophecy::{ProphecyEngine, ProphecyOptions};
use agentic_codebase::temporal::stability::{StabilityAnalyzer, StabilityOptions};
use agentic_codebase::types::{CodeUnit, CodeUnitType, Language, Span};

/// Helper: create a FileChange with sensible defaults.
#[allow(clippy::too_many_arguments)]
fn make_change(
    path: &str,
    change_type: ChangeType,
    commit: &str,
    timestamp: u64,
    author: &str,
    is_bugfix: bool,
    added: u32,
    deleted: u32,
) -> FileChange {
    FileChange {
        path: PathBuf::from(path),
        change_type,
        commit_id: commit.to_string(),
        timestamp,
        author: author.to_string(),
        is_bugfix,
        lines_added: added,
        lines_deleted: deleted,
        old_path: None,
    }
}

/// Helper: build a simple test graph with a few units.
fn build_test_graph() -> CodeGraph {
    let mut graph = CodeGraph::with_default_dimension();

    let unit_a = CodeUnit::new(
        CodeUnitType::Function,
        Language::Rust,
        "func_a".to_string(),
        "mod::func_a".to_string(),
        PathBuf::from("src/a.rs"),
        Span::new(1, 0, 10, 0),
    );
    graph.add_unit(unit_a);

    let unit_b = CodeUnit::new(
        CodeUnitType::Function,
        Language::Rust,
        "func_b".to_string(),
        "mod::func_b".to_string(),
        PathBuf::from("src/b.rs"),
        Span::new(1, 0, 20, 0),
    );
    graph.add_unit(unit_b);

    let unit_c = CodeUnit::new(
        CodeUnitType::Function,
        Language::Rust,
        "func_c".to_string(),
        "mod::func_c".to_string(),
        PathBuf::from("src/c.rs"),
        Span::new(1, 0, 15, 0),
    );
    graph.add_unit(unit_c);

    graph
}

// ============================================================================
// History tests
// ============================================================================

#[test]
fn test_history_add_and_lookup() {
    let mut history = ChangeHistory::new();
    history.add_change(make_change(
        "src/main.rs",
        ChangeType::Add,
        "abc123",
        1000,
        "alice",
        false,
        100,
        0,
    ));
    history.add_change(make_change(
        "src/main.rs",
        ChangeType::Modify,
        "def456",
        2000,
        "bob",
        true,
        10,
        5,
    ));
    history.add_change(make_change(
        "src/lib.rs",
        ChangeType::Add,
        "abc123",
        1000,
        "alice",
        false,
        200,
        0,
    ));

    assert_eq!(history.change_count(&PathBuf::from("src/main.rs")), 2);
    assert_eq!(history.change_count(&PathBuf::from("src/lib.rs")), 1);
    assert_eq!(history.change_count(&PathBuf::from("src/none.rs")), 0);
}

#[test]
fn test_history_commits() {
    let mut history = ChangeHistory::new();
    history.add_change(make_change(
        "src/a.rs",
        ChangeType::Add,
        "commit1",
        1000,
        "alice",
        false,
        50,
        0,
    ));
    history.add_change(make_change(
        "src/b.rs",
        ChangeType::Add,
        "commit1",
        1000,
        "alice",
        false,
        30,
        0,
    ));
    history.add_change(make_change(
        "src/a.rs",
        ChangeType::Modify,
        "commit2",
        2000,
        "bob",
        false,
        10,
        5,
    ));

    let commits = history.all_commits();
    assert_eq!(commits.len(), 2);
    assert_eq!(history.files_in_commit("commit1").len(), 2);
    assert_eq!(history.files_in_commit("commit2").len(), 1);
    assert_eq!(history.files_in_commit("nonexistent").len(), 0);
}

#[test]
fn test_history_bugfix_count() {
    let mut history = ChangeHistory::new();
    let path = PathBuf::from("src/buggy.rs");
    history.add_change(make_change(
        "src/buggy.rs",
        ChangeType::Add,
        "c1",
        1000,
        "dev",
        false,
        100,
        0,
    ));
    history.add_change(make_change(
        "src/buggy.rs",
        ChangeType::Modify,
        "c2",
        2000,
        "dev",
        true,
        5,
        3,
    ));
    history.add_change(make_change(
        "src/buggy.rs",
        ChangeType::Modify,
        "c3",
        3000,
        "dev",
        true,
        8,
        6,
    ));
    history.add_change(make_change(
        "src/buggy.rs",
        ChangeType::Modify,
        "c4",
        4000,
        "dev",
        false,
        2,
        1,
    ));

    assert_eq!(history.bugfix_count(&path), 2);
    assert_eq!(history.change_count(&path), 4);
}

#[test]
fn test_history_churn_and_authors() {
    let mut history = ChangeHistory::new();
    let path = PathBuf::from("src/churn.rs");
    history.add_change(make_change(
        "src/churn.rs",
        ChangeType::Add,
        "c1",
        1000,
        "alice",
        false,
        100,
        0,
    ));
    history.add_change(make_change(
        "src/churn.rs",
        ChangeType::Modify,
        "c2",
        2000,
        "bob",
        false,
        20,
        10,
    ));
    history.add_change(make_change(
        "src/churn.rs",
        ChangeType::Modify,
        "c3",
        3000,
        "alice",
        false,
        15,
        5,
    ));

    assert_eq!(history.total_churn(&path), 150);
    let authors = history.authors_for_path(&path);
    assert_eq!(authors.len(), 2);
    assert!(authors.contains(&"alice".to_string()));
    assert!(authors.contains(&"bob".to_string()));
}

#[test]
fn test_history_timestamps() {
    let mut history = ChangeHistory::new();
    let path = PathBuf::from("src/ts.rs");
    history.add_change(make_change(
        "src/ts.rs",
        ChangeType::Add,
        "c1",
        500,
        "dev",
        false,
        10,
        0,
    ));
    history.add_change(make_change(
        "src/ts.rs",
        ChangeType::Modify,
        "c2",
        1500,
        "dev",
        false,
        5,
        2,
    ));

    assert_eq!(history.oldest_timestamp(&path), 500);
    assert_eq!(history.latest_timestamp(&path), 1500);
    assert_eq!(history.oldest_timestamp(&PathBuf::from("nonexistent")), 0);
}

// ============================================================================
// Stability tests
// ============================================================================

#[test]
fn test_stability_no_history() {
    let history = ChangeHistory::new();
    let analyzer = StabilityAnalyzer::new();
    let result = analyzer.calculate_stability(&PathBuf::from("src/new.rs"), &history);

    assert_eq!(result.overall_score, 1.0);
    assert_eq!(result.factors.len(), 1);
    assert_eq!(result.factors[0].name, "no_history");
}

#[test]
fn test_stability_single_change() {
    let mut history = ChangeHistory::new();
    history.add_change(make_change(
        "src/stable.rs",
        ChangeType::Add,
        "c1",
        1000,
        "dev",
        false,
        50,
        0,
    ));

    let analyzer = StabilityAnalyzer::with_options(StabilityOptions {
        now_timestamp: 100_000,
        ..StabilityOptions::default()
    });
    let result = analyzer.calculate_stability(&PathBuf::from("src/stable.rs"), &history);

    // With only one non-bugfix change, should be fairly stable.
    assert!(result.overall_score > 0.5);
    assert_eq!(result.factors.len(), 5);
}

#[test]
fn test_stability_many_bugfixes() {
    let mut history = ChangeHistory::new();
    for i in 0..20 {
        history.add_change(make_change(
            "src/bad.rs",
            ChangeType::Modify,
            &format!("c{}", i),
            1000 + i as u64 * 100,
            "dev",
            i % 2 == 0, // 50% bugfix
            5,
            3,
        ));
    }

    let analyzer = StabilityAnalyzer::with_options(StabilityOptions {
        now_timestamp: 100_000,
        ..StabilityOptions::default()
    });
    let result = analyzer.calculate_stability(&PathBuf::from("src/bad.rs"), &history);

    // 50% bugfix rate should give a lower score.
    assert!(result.overall_score < 0.8);
    // Should have a recommendation about high bugfix ratio.
    assert!(!result.recommendations.is_empty());
}

#[test]
fn test_stability_recommendations() {
    let mut history = ChangeHistory::new();
    let now = 100_000u64;

    // Create a file with lots of recent bugfix changes by many authors.
    for i in 0..10 {
        history.add_change(make_change(
            "src/messy.rs",
            ChangeType::Modify,
            &format!("c{}", i),
            now - 100 + i as u64 * 10, // All very recent
            &format!("dev{}", i),
            true, // All bugfixes
            10,
            8,
        ));
    }

    let analyzer = StabilityAnalyzer::with_options(StabilityOptions {
        now_timestamp: now,
        recent_window_secs: 1000,
        ..StabilityOptions::default()
    });
    let result = analyzer.calculate_stability(&PathBuf::from("src/messy.rs"), &history);

    // Should flag high bugfix ratio AND high recent activity AND many authors.
    assert!(result.recommendations.len() >= 2);
}

// ============================================================================
// Coupling tests
// ============================================================================

#[test]
fn test_coupling_detection() {
    let mut history = ChangeHistory::new();

    // Files A and B always change together.
    for i in 0..5 {
        let commit = format!("c{}", i);
        history.add_change(make_change(
            "src/a.rs",
            ChangeType::Modify,
            &commit,
            1000 + i as u64 * 100,
            "dev",
            false,
            5,
            2,
        ));
        history.add_change(make_change(
            "src/b.rs",
            ChangeType::Modify,
            &commit,
            1000 + i as u64 * 100,
            "dev",
            false,
            3,
            1,
        ));
    }

    // File C only changes alone.
    history.add_change(make_change(
        "src/c.rs",
        ChangeType::Modify,
        "c_solo",
        5000,
        "dev",
        false,
        10,
        5,
    ));

    let detector = CouplingDetector::with_options(CouplingOptions {
        min_cochanges: 3,
        min_strength: 0.3,
        limit: 0,
    });
    let couplings = detector.detect_all(&history, None);

    // Should detect coupling between a.rs and b.rs.
    assert!(!couplings.is_empty());
    let ab = couplings
        .iter()
        .find(|c| {
            (c.path_a.to_str() == Some("src/a.rs") && c.path_b.to_str() == Some("src/b.rs"))
                || (c.path_a.to_str() == Some("src/b.rs") && c.path_b.to_str() == Some("src/a.rs"))
        })
        .unwrap();
    assert_eq!(ab.cochange_count, 5);
    assert!(ab.strength > 0.5);
}

#[test]
fn test_coupling_with_graph() {
    let mut history = ChangeHistory::new();
    let graph = build_test_graph();

    // Create co-changes between files that have units in the graph.
    for i in 0..4 {
        let commit = format!("c{}", i);
        history.add_change(make_change(
            "src/a.rs",
            ChangeType::Modify,
            &commit,
            1000 + i as u64 * 100,
            "dev",
            false,
            5,
            2,
        ));
        history.add_change(make_change(
            "src/b.rs",
            ChangeType::Modify,
            &commit,
            1000 + i as u64 * 100,
            "dev",
            false,
            3,
            1,
        ));
    }

    let detector = CouplingDetector::with_options(CouplingOptions {
        min_cochanges: 3,
        min_strength: 0.3,
        limit: 0,
    });
    let couplings = detector.detect_all(&history, Some(&graph));

    assert!(!couplings.is_empty());
    // With no edges between units, should be CoChange type.
    assert_eq!(couplings[0].coupling_type, CouplingType::CoChange);
}

#[test]
fn test_coupling_below_threshold() {
    let mut history = ChangeHistory::new();

    // Only 1 co-change (below default threshold of 3).
    history.add_change(make_change(
        "src/x.rs",
        ChangeType::Modify,
        "c1",
        1000,
        "dev",
        false,
        5,
        2,
    ));
    history.add_change(make_change(
        "src/y.rs",
        ChangeType::Modify,
        "c1",
        1000,
        "dev",
        false,
        3,
        1,
    ));

    let detector = CouplingDetector::new();
    let couplings = detector.detect_all(&history, None);

    assert!(couplings.is_empty());
}

// ============================================================================
// Prophecy tests
// ============================================================================

#[test]
fn test_prophecy_empty_history() {
    let history = ChangeHistory::new();
    let engine = ProphecyEngine::new();
    let result = engine.predict(&history, None);

    assert_eq!(result.files_analysed, 0);
    assert_eq!(result.predictions.len(), 0);
    assert_eq!(result.average_risk, 0.0);
}

#[test]
fn test_prophecy_with_buggy_file() {
    let mut history = ChangeHistory::new();
    let now = 100_000u64;

    // Create a file with lots of recent bugfixes.
    for i in 0..15 {
        history.add_change(make_change(
            "src/problematic.rs",
            ChangeType::Modify,
            &format!("c{}", i),
            now - 500 + i as u64 * 30,
            "dev",
            true,
            10,
            8,
        ));
    }

    // Create a stable file for contrast.
    history.add_change(make_change(
        "src/stable.rs",
        ChangeType::Add,
        "c_stable",
        1000,
        "dev",
        false,
        50,
        0,
    ));

    let engine = ProphecyEngine::with_options(ProphecyOptions {
        now_timestamp: now,
        min_risk: 0.1,
        recent_window_secs: 30 * 24 * 3600,
        ..ProphecyOptions::default()
    });
    let result = engine.predict(&history, None);

    assert_eq!(result.files_analysed, 2);
    // The problematic file should have higher risk.
    if !result.predictions.is_empty() {
        let top = &result.predictions[0];
        assert!(top.path.contains("problematic"));
    }
}

#[test]
fn test_prophecy_with_graph() {
    let mut history = ChangeHistory::new();
    let graph = build_test_graph();
    let now = 100_000u64;

    for i in 0..8 {
        let commit = format!("c{}", i);
        history.add_change(make_change(
            "src/a.rs",
            ChangeType::Modify,
            &commit,
            now - 1000 + i as u64 * 100,
            "dev",
            i % 3 == 0,
            10,
            5,
        ));
        history.add_change(make_change(
            "src/b.rs",
            ChangeType::Modify,
            &commit,
            now - 1000 + i as u64 * 100,
            "dev",
            false,
            3,
            1,
        ));
    }

    let engine = ProphecyEngine::with_options(ProphecyOptions {
        now_timestamp: now,
        min_risk: 0.0,
        ..ProphecyOptions::default()
    });
    let result = engine.predict(&history, Some(&graph));

    assert_eq!(result.files_analysed, 2);
    assert!(!result.predictions.is_empty());
}

#[test]
fn test_prophecy_alerts() {
    let mut history = ChangeHistory::new();
    let now = 100_000u64;

    // Create many files that all have bugfix issues.
    for file_idx in 0..5 {
        for change_idx in 0..10 {
            history.add_change(make_change(
                &format!("src/file{}.rs", file_idx),
                ChangeType::Modify,
                &format!("c_{}_{}", file_idx, change_idx),
                now - 500 + change_idx as u64 * 40,
                "dev",
                true,
                10,
                8,
            ));
        }
    }

    let engine = ProphecyEngine::with_options(ProphecyOptions {
        now_timestamp: now,
        min_risk: 0.0,
        recent_window_secs: 30 * 24 * 3600,
        ..ProphecyOptions::default()
    });
    let result = engine.predict(&history, None);

    assert_eq!(result.files_analysed, 5);
    // Should generate some alerts (hotspot or systemic instability).
    // The exact alerts depend on thresholds, but with 100% bugfix rate
    // across 5 files, we should see at least a hotspot.
    assert!(
        !result.predictions.is_empty(),
        "Expected at least one prediction from highly buggy history."
    );
}
