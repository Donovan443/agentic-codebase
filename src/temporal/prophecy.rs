//! Predictive analysis — the prophecy engine.
//!
//! Uses change history, stability scores, and coupling data to predict
//! which files are likely to cause problems (bugs, test failures, etc.)
//! in the near future.

use std::path::Path;

use serde::{Deserialize, Serialize};

use super::coupling::{CouplingDetector, CouplingOptions};
use super::history::ChangeHistory;
use super::stability::StabilityAnalyzer;
use crate::graph::CodeGraph;

/// Options for prophecy prediction.
#[derive(Debug, Clone)]
pub struct ProphecyOptions {
    /// Maximum number of predictions to return.
    pub top_k: usize,
    /// Minimum risk score threshold (0.0 to 1.0).
    pub min_risk: f32,
    /// Timestamp considered "now" (0 = use current time).
    pub now_timestamp: u64,
    /// Window (in seconds) for "recent" calculations (default 30 days).
    pub recent_window_secs: u64,
}

impl Default for ProphecyOptions {
    fn default() -> Self {
        Self {
            top_k: 20,
            min_risk: 0.3,
            now_timestamp: 0,
            recent_window_secs: 30 * 24 * 3600,
        }
    }
}

/// The type of prediction made.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PredictionType {
    /// File is likely to have a bug introduced.
    BugRisk,
    /// File is likely to need changes soon.
    ChangeVelocity,
    /// File complexity is growing unsustainably.
    ComplexityGrowth,
    /// File has dangerous coupling with other files.
    CouplingRisk,
}

impl std::fmt::Display for PredictionType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::BugRisk => write!(f, "bug-risk"),
            Self::ChangeVelocity => write!(f, "change-velocity"),
            Self::ComplexityGrowth => write!(f, "complexity-growth"),
            Self::CouplingRisk => write!(f, "coupling-risk"),
        }
    }
}

/// A single prediction about a file.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Prediction {
    /// The file path this prediction is about.
    pub path: String,
    /// Risk score (0.0 = low risk, 1.0 = high risk).
    pub risk_score: f32,
    /// The type of prediction.
    pub prediction_type: PredictionType,
    /// Human-readable reason for the prediction.
    pub reason: String,
    /// Contributing factors and their values.
    pub factors: Vec<(String, f32)>,
}

/// The type of ecosystem alert.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AlertType {
    /// A hotspot file that changes too often.
    Hotspot,
    /// A file that many other files are coupled with.
    CouplingHub,
    /// Systemic instability across the codebase.
    SystemicInstability,
}

impl std::fmt::Display for AlertType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Hotspot => write!(f, "hotspot"),
            Self::CouplingHub => write!(f, "coupling-hub"),
            Self::SystemicInstability => write!(f, "systemic-instability"),
        }
    }
}

/// An ecosystem-level alert about codebase health.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemAlert {
    /// Alert type.
    pub alert_type: AlertType,
    /// Severity (0.0 = informational, 1.0 = critical).
    pub severity: f32,
    /// Human-readable message.
    pub message: String,
    /// Affected file paths.
    pub affected_paths: Vec<String>,
}

/// Result of a prophecy prediction run.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProphecyResult {
    /// Predictions about individual files, sorted by risk descending.
    pub predictions: Vec<Prediction>,
    /// Ecosystem-level alerts.
    pub alerts: Vec<EcosystemAlert>,
    /// Average risk across all analysed files.
    pub average_risk: f32,
    /// Number of files analysed.
    pub files_analysed: usize,
}

/// The prophecy engine: predicts future problems based on historical patterns.
#[derive(Debug, Clone)]
pub struct ProphecyEngine {
    /// Configuration options.
    options: ProphecyOptions,
}

impl ProphecyEngine {
    /// Create a new prophecy engine with default options.
    pub fn new() -> Self {
        Self {
            options: ProphecyOptions::default(),
        }
    }

    /// Create a new prophecy engine with custom options.
    pub fn with_options(options: ProphecyOptions) -> Self {
        Self { options }
    }

    /// Run predictions on the codebase.
    ///
    /// Analyses change history and the code graph to produce predictions
    /// and ecosystem alerts.
    pub fn predict(&self, history: &ChangeHistory, graph: Option<&CodeGraph>) -> ProphecyResult {
        let all_paths = history.all_paths();
        let mut predictions = Vec::new();
        let mut total_risk = 0.0_f32;

        let stability_analyzer = StabilityAnalyzer::new();
        let coupling_detector = CouplingDetector::with_options(CouplingOptions {
            min_cochanges: 2,
            min_strength: 0.3,
            limit: 0,
        });
        let couplings = coupling_detector.detect_all(history, graph);

        for path in &all_paths {
            let stability = stability_analyzer.calculate_stability(path, history);

            // Factor 1: Change velocity.
            let velocity = self.calculate_velocity(path, history);

            // Factor 2: Bugfix trend.
            let bugfix_trend = self.calculate_bugfix_trend(path, history);

            // Factor 3: Complexity growth proxy (churn as surrogate).
            let complexity_growth = self.calculate_complexity_growth(path, history);

            // Factor 4: Coupling risk.
            let coupling_risk = self.calculate_coupling_risk(path, &couplings);

            // Combine into a final risk score.
            let risk_score = (velocity * 0.30
                + bugfix_trend * 0.30
                + complexity_growth * 0.15
                + coupling_risk * 0.25)
                .clamp(0.0, 1.0);

            total_risk += risk_score;

            // Select the dominant prediction type.
            let factors = vec![
                ("velocity".to_string(), velocity),
                ("bugfix_trend".to_string(), bugfix_trend),
                ("complexity_growth".to_string(), complexity_growth),
                ("coupling_risk".to_string(), coupling_risk),
            ];

            let prediction_type = if bugfix_trend >= velocity
                && bugfix_trend >= complexity_growth
                && bugfix_trend >= coupling_risk
            {
                PredictionType::BugRisk
            } else if coupling_risk >= velocity && coupling_risk >= complexity_growth {
                PredictionType::CouplingRisk
            } else if complexity_growth >= velocity {
                PredictionType::ComplexityGrowth
            } else {
                PredictionType::ChangeVelocity
            };

            let reason = match &prediction_type {
                PredictionType::BugRisk => format!(
                    "High bugfix trend ({:.2}) with stability score {:.2}.",
                    bugfix_trend, stability.overall_score
                ),
                PredictionType::ChangeVelocity => format!(
                    "High change velocity ({:.2}); file changes frequently.",
                    velocity
                ),
                PredictionType::ComplexityGrowth => format!(
                    "Complexity growth signal ({:.2}) from increasing churn.",
                    complexity_growth
                ),
                PredictionType::CouplingRisk => format!(
                    "Coupling risk ({:.2}); many co-changing dependencies.",
                    coupling_risk
                ),
            };

            if risk_score >= self.options.min_risk {
                predictions.push(Prediction {
                    path: path.display().to_string(),
                    risk_score,
                    prediction_type,
                    reason,
                    factors,
                });
            }
        }

        // Sort by risk descending.
        predictions.sort_by(|a, b| {
            b.risk_score
                .partial_cmp(&a.risk_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        if self.options.top_k > 0 {
            predictions.truncate(self.options.top_k);
        }

        let files_analysed = all_paths.len();
        let average_risk = if files_analysed > 0 {
            total_risk / files_analysed as f32
        } else {
            0.0
        };

        // Generate ecosystem alerts.
        let alerts = self.generate_alerts(history, &predictions, average_risk);

        ProphecyResult {
            predictions,
            alerts,
            average_risk,
            files_analysed,
        }
    }

    /// Calculate change velocity for a path (how fast it is changing recently).
    fn calculate_velocity(&self, path: &Path, history: &ChangeHistory) -> f32 {
        let changes = history.changes_for_path(path);
        if changes.is_empty() {
            return 0.0;
        }

        let now = self.effective_now();
        let cutoff = now.saturating_sub(self.options.recent_window_secs);
        let recent_count = changes.iter().filter(|c| c.timestamp >= cutoff).count();
        let total_count = changes.len();

        // Velocity = recent_proportion * frequency factor.
        let recent_ratio = recent_count as f32 / total_count.max(1) as f32;
        let freq_factor = (recent_count as f32 / 5.0).min(1.0);
        (recent_ratio * 0.5 + freq_factor * 0.5).min(1.0)
    }

    /// Calculate bugfix trend for a path.
    fn calculate_bugfix_trend(&self, path: &Path, history: &ChangeHistory) -> f32 {
        let changes = history.changes_for_path(path);
        if changes.is_empty() {
            return 0.0;
        }

        let bugfix_count = changes.iter().filter(|c| c.is_bugfix).count();
        let total = changes.len();
        let ratio = bugfix_count as f32 / total as f32;

        // Check if bugfixes are increasing in the recent window.
        let now = self.effective_now();
        let cutoff = now.saturating_sub(self.options.recent_window_secs);
        let recent_bugfixes = changes
            .iter()
            .filter(|c| c.is_bugfix && c.timestamp >= cutoff)
            .count();
        let recent_total = changes.iter().filter(|c| c.timestamp >= cutoff).count();
        let recent_ratio = if recent_total > 0 {
            recent_bugfixes as f32 / recent_total as f32
        } else {
            0.0
        };

        // Combine historical and recent trends.
        (ratio * 0.4 + recent_ratio * 0.6).min(1.0)
    }

    /// Calculate complexity growth signal from churn patterns.
    fn calculate_complexity_growth(&self, path: &Path, history: &ChangeHistory) -> f32 {
        let changes = history.changes_for_path(path);
        if changes.is_empty() {
            return 0.0;
        }

        // Use net line additions as a proxy for complexity growth.
        let total_added: u64 = changes.iter().map(|c| c.lines_added as u64).sum();
        let total_deleted: u64 = changes.iter().map(|c| c.lines_deleted as u64).sum();

        let net_growth = if total_added > total_deleted {
            (total_added - total_deleted) as f32
        } else {
            0.0
        };

        // Normalise: score rises as net growth increases.
        let growth_signal = net_growth / (net_growth + 100.0);
        growth_signal.min(1.0)
    }

    /// Calculate coupling risk from detected couplings.
    fn calculate_coupling_risk(&self, path: &Path, couplings: &[super::coupling::Coupling]) -> f32 {
        let path_str = path.to_path_buf();
        let relevant: Vec<f32> = couplings
            .iter()
            .filter(|c| c.path_a == path_str || c.path_b == path_str)
            .map(|c| c.strength)
            .collect();

        if relevant.is_empty() {
            return 0.0;
        }

        // Coupling risk = average strength * sqrt(count) normalised.
        let avg_strength: f32 = relevant.iter().sum::<f32>() / relevant.len() as f32;
        let count_factor = (relevant.len() as f32).sqrt() / 3.0;
        (avg_strength * 0.6 + count_factor.min(1.0) * 0.4).min(1.0)
    }

    /// Get the effective "now" timestamp.
    fn effective_now(&self) -> u64 {
        if self.options.now_timestamp > 0 {
            self.options.now_timestamp
        } else {
            crate::types::now_micros() / 1_000_000
        }
    }

    /// Generate ecosystem-level alerts.
    fn generate_alerts(
        &self,
        history: &ChangeHistory,
        predictions: &[Prediction],
        average_risk: f32,
    ) -> Vec<EcosystemAlert> {
        let mut alerts = Vec::new();

        // Alert: systemic instability if average risk is high.
        if average_risk > 0.6 {
            let affected: Vec<String> =
                predictions.iter().take(5).map(|p| p.path.clone()).collect();
            alerts.push(EcosystemAlert {
                alert_type: AlertType::SystemicInstability,
                severity: average_risk.min(1.0),
                message: format!(
                    "Systemic instability detected: average risk {:.2} across {} files.",
                    average_risk,
                    history.all_paths().len()
                ),
                affected_paths: affected,
            });
        }

        // Alert: hotspots (files with very high risk).
        for pred in predictions.iter().filter(|p| p.risk_score > 0.7) {
            alerts.push(EcosystemAlert {
                alert_type: AlertType::Hotspot,
                severity: pred.risk_score,
                message: format!(
                    "Hotspot detected: {} (risk {:.2}).",
                    pred.path, pred.risk_score
                ),
                affected_paths: vec![pred.path.clone()],
            });
        }

        alerts
    }
}

impl Default for ProphecyEngine {
    fn default() -> Self {
        Self::new()
    }
}
