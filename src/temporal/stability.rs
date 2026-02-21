//! Stability score calculation.
//!
//! Analyses change history to compute a stability score for code units.
//! Higher scores indicate more stable (less frequently changing) code.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::history::ChangeHistory;

/// Options for stability analysis.
#[derive(Debug, Clone)]
pub struct StabilityOptions {
    /// Weight for change frequency factor (default 0.25).
    pub change_frequency_weight: f32,
    /// Weight for bugfix ratio factor (default 0.25).
    pub bugfix_ratio_weight: f32,
    /// Weight for recent activity factor (default 0.20).
    pub recent_activity_weight: f32,
    /// Weight for author concentration factor (default 0.15).
    pub author_concentration_weight: f32,
    /// Weight for churn factor (default 0.15).
    pub churn_weight: f32,
    /// Timestamp considered "now" for recency calculations (0 = use current time).
    pub now_timestamp: u64,
    /// Window (in seconds) for "recent" activity (default: 30 days).
    pub recent_window_secs: u64,
}

impl Default for StabilityOptions {
    fn default() -> Self {
        Self {
            change_frequency_weight: 0.25,
            bugfix_ratio_weight: 0.25,
            recent_activity_weight: 0.20,
            author_concentration_weight: 0.15,
            churn_weight: 0.15,
            now_timestamp: 0,
            recent_window_secs: 30 * 24 * 3600,
        }
    }
}

/// A single factor contributing to the stability score.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StabilityFactor {
    /// Factor name.
    pub name: String,
    /// Factor value (0.0 = unstable, 1.0 = stable).
    pub value: f32,
    /// Weight applied to this factor in the overall score.
    pub weight: f32,
    /// Human-readable description.
    pub description: String,
}

/// A recommendation based on stability analysis.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StabilityRecommendation {
    /// Recommendation priority (lower = more urgent).
    pub priority: u32,
    /// Short summary.
    pub summary: String,
    /// Detailed explanation.
    pub detail: String,
}

/// Result of a stability analysis for a single file or code unit.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StabilityResult {
    /// The file path analysed.
    pub path: String,
    /// Overall stability score (0.0 = very unstable, 1.0 = very stable).
    pub overall_score: f32,
    /// Individual contributing factors.
    pub factors: Vec<StabilityFactor>,
    /// Recommendations for improving stability.
    pub recommendations: Vec<StabilityRecommendation>,
}

/// Analyses code unit stability based on change history patterns.
#[derive(Debug, Clone)]
pub struct StabilityAnalyzer {
    /// Configuration options.
    options: StabilityOptions,
}

impl StabilityAnalyzer {
    /// Create a new stability analyser with default options.
    pub fn new() -> Self {
        Self {
            options: StabilityOptions::default(),
        }
    }

    /// Create a new stability analyser with custom options.
    pub fn with_options(options: StabilityOptions) -> Self {
        Self { options }
    }

    /// Calculate stability for a file path given its change history.
    ///
    /// Returns a [`StabilityResult`] with the overall score and contributing factors.
    /// If the path has no history, returns a perfect stability score of 1.0.
    pub fn calculate_stability(&self, path: &Path, history: &ChangeHistory) -> StabilityResult {
        let change_count = history.change_count(path);

        // No history means perfectly stable.
        if change_count == 0 {
            return StabilityResult {
                path: path.display().to_string(),
                overall_score: 1.0,
                factors: vec![StabilityFactor {
                    name: "no_history".to_string(),
                    value: 1.0,
                    weight: 1.0,
                    description: "No change history recorded; assumed stable.".to_string(),
                }],
                recommendations: Vec::new(),
            };
        }

        let mut factors = Vec::new();

        // Factor 1: Change frequency — fewer changes = more stable.
        // Normalise: score = 1 / (1 + log2(change_count)).
        let freq_score = 1.0 / (1.0 + (change_count as f32).log2());
        factors.push(StabilityFactor {
            name: "change_frequency".to_string(),
            value: freq_score,
            weight: self.options.change_frequency_weight,
            description: format!(
                "{} total changes recorded; frequency score {:.2}.",
                change_count, freq_score
            ),
        });

        // Factor 2: Bugfix ratio — fewer bugfixes = more stable.
        let bugfix_count = history.bugfix_count(path);
        let bugfix_ratio = if change_count > 0 {
            bugfix_count as f32 / change_count as f32
        } else {
            0.0
        };
        let bugfix_score = 1.0 - bugfix_ratio;
        factors.push(StabilityFactor {
            name: "bugfix_ratio".to_string(),
            value: bugfix_score,
            weight: self.options.bugfix_ratio_weight,
            description: format!(
                "{} of {} changes were bugfixes ({:.0}%); bugfix score {:.2}.",
                bugfix_count,
                change_count,
                bugfix_ratio * 100.0,
                bugfix_score
            ),
        });

        // Factor 3: Recent activity — less recent activity = more stable.
        let now = if self.options.now_timestamp > 0 {
            self.options.now_timestamp
        } else {
            crate::types::now_micros() / 1_000_000
        };
        let cutoff = now.saturating_sub(self.options.recent_window_secs);
        let changes = history.changes_for_path(path);
        let recent_count = changes.iter().filter(|c| c.timestamp >= cutoff).count();
        let recent_ratio = if change_count > 0 {
            recent_count as f32 / change_count as f32
        } else {
            0.0
        };
        let recent_score = 1.0 - recent_ratio.min(1.0);
        factors.push(StabilityFactor {
            name: "recent_activity".to_string(),
            value: recent_score,
            weight: self.options.recent_activity_weight,
            description: format!(
                "{} of {} changes were recent (within window); recency score {:.2}.",
                recent_count, change_count, recent_score
            ),
        });

        // Factor 4: Author concentration — more authors = less stable.
        let authors = history.authors_for_path(path);
        let author_count = authors.len().max(1);
        let author_score = 1.0 / (author_count as f32);
        factors.push(StabilityFactor {
            name: "author_concentration".to_string(),
            value: author_score,
            weight: self.options.author_concentration_weight,
            description: format!(
                "{} unique authors; concentration score {:.2}.",
                author_count, author_score
            ),
        });

        // Factor 5: Churn — less churn = more stable.
        let churn = history.total_churn(path);
        let churn_score = 1.0 / (1.0 + (churn as f32).log2().max(0.0));
        factors.push(StabilityFactor {
            name: "churn".to_string(),
            value: churn_score,
            weight: self.options.churn_weight,
            description: format!(
                "{} total lines churned; churn score {:.2}.",
                churn, churn_score
            ),
        });

        // Compute weighted average.
        let overall_score: f32 = factors.iter().map(|f| f.value * f.weight).sum::<f32>()
            / factors.iter().map(|f| f.weight).sum::<f32>().max(0.001);
        let overall_score = overall_score.clamp(0.0, 1.0);

        // Generate recommendations.
        let mut recommendations = Vec::new();
        if bugfix_ratio > 0.5 {
            recommendations.push(StabilityRecommendation {
                priority: 1,
                summary: "High bugfix ratio".to_string(),
                detail: format!(
                    "Over {:.0}% of changes are bugfixes. Consider refactoring for reliability.",
                    bugfix_ratio * 100.0
                ),
            });
        }
        if recent_count > 5 {
            recommendations.push(StabilityRecommendation {
                priority: 2,
                summary: "High recent activity".to_string(),
                detail: format!(
                    "{} changes in the recent window. This file may be in active flux.",
                    recent_count
                ),
            });
        }
        if author_count > 5 {
            recommendations.push(StabilityRecommendation {
                priority: 3,
                summary: "Many authors".to_string(),
                detail: format!(
                    "{} authors have modified this file. Consider assigning ownership.",
                    author_count
                ),
            });
        }

        StabilityResult {
            path: path.display().to_string(),
            overall_score,
            factors,
            recommendations,
        }
    }
}

impl Default for StabilityAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}
