//! Time-based analysis for code evolution.
//!
//! Change history tracking, stability scores, coupling detection,
//! and predictive analysis. Can work with or without git integration.

pub mod coupling;
pub mod history;
pub mod prophecy;
pub mod stability;

pub use coupling::{Coupling, CouplingDetector, CouplingOptions, CouplingType};
pub use history::{ChangeHistory, ChangeType, FileChange, HistoryOptions};
pub use prophecy::{
    AlertType, EcosystemAlert, Prediction, PredictionType, ProphecyEngine, ProphecyOptions,
    ProphecyResult,
};
pub use stability::{
    StabilityAnalyzer, StabilityFactor, StabilityOptions, StabilityRecommendation, StabilityResult,
};
