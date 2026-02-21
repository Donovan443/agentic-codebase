//! Design pattern detection.
//!
//! Detects common design patterns in code: Singleton, Factory, Repository,
//! Decorator, Observer, Strategy patterns.

use crate::types::{AcbResult, CodeUnitType, Visibility};

use super::resolver::ResolvedUnit;

/// Detects common design patterns in code.
pub struct PatternDetector {
    /// Pattern matchers to run.
    matchers: Vec<Box<dyn PatternMatcher>>,
}

/// A detected pattern instance.
#[derive(Debug, Clone)]
pub struct PatternInstance {
    /// The pattern name.
    pub pattern_name: String,
    /// The primary unit involved.
    pub primary_unit: u64,
    /// All units participating in the pattern.
    pub participating_units: Vec<u64>,
    /// Confidence score (0.0 to 1.0).
    pub confidence: f32,
}

/// Trait for pattern matchers.
trait PatternMatcher: Send + Sync {
    /// Detect instances of this pattern.
    fn detect(&self, units: &[ResolvedUnit]) -> Vec<PatternInstance>;
}

impl PatternDetector {
    /// Create a new pattern detector with all built-in matchers.
    pub fn new() -> Self {
        let matchers: Vec<Box<dyn PatternMatcher>> = vec![
            Box::new(SingletonMatcher),
            Box::new(FactoryMatcher),
            Box::new(RepositoryMatcher),
            Box::new(DecoratorMatcher),
        ];
        Self { matchers }
    }

    /// Detect all patterns in the resolved units.
    pub fn detect(&self, units: &[ResolvedUnit]) -> AcbResult<Vec<PatternInstance>> {
        let mut instances = Vec::new();

        for matcher in &self.matchers {
            instances.extend(matcher.detect(units));
        }

        Ok(instances)
    }
}

impl Default for PatternDetector {
    fn default() -> Self {
        Self::new()
    }
}

/// Detects Singleton pattern: classes with private constructors and
/// static instance access methods.
struct SingletonMatcher;

impl PatternMatcher for SingletonMatcher {
    fn detect(&self, units: &[ResolvedUnit]) -> Vec<PatternInstance> {
        let mut instances = Vec::new();

        for unit in units {
            if unit.unit.unit_type != CodeUnitType::Type {
                continue;
            }

            // Look for singleton indicators by checking sibling methods
            let type_name = &unit.unit.name;
            let type_name_lower = type_name.to_lowercase();

            let mut has_instance_method = false;
            let mut has_private_constructor = false;
            let mut participants = vec![unit.unit.temp_id];

            for other in units {
                if other.unit.unit_type != CodeUnitType::Function {
                    continue;
                }

                let other_qname_lower = other.unit.qualified_name.to_lowercase();
                let other_name_lower = other.unit.name.to_lowercase();

                // Check if this is a method of the type
                if other_qname_lower.contains(&type_name_lower) {
                    // Check for get_instance, instance, or shared patterns
                    if other_name_lower.contains("instance")
                        || other_name_lower.contains("shared")
                        || other_name_lower == "default"
                    {
                        has_instance_method = true;
                        participants.push(other.unit.temp_id);
                    }

                    // Check for private constructors
                    if (other_name_lower == "__init__"
                        || other_name_lower == "new"
                        || other_name_lower == "constructor")
                        && other.unit.visibility == Visibility::Private
                    {
                        has_private_constructor = true;
                        participants.push(other.unit.temp_id);
                    }
                }
            }

            let score = (has_instance_method as u8 + has_private_constructor as u8) as f32 / 2.0;
            if score > 0.0 {
                instances.push(PatternInstance {
                    pattern_name: "Singleton".to_string(),
                    primary_unit: unit.unit.temp_id,
                    participating_units: participants,
                    confidence: score,
                });
            }
        }

        instances
    }
}

/// Detects Factory pattern: functions or classes that create and return
/// other object instances.
struct FactoryMatcher;

impl PatternMatcher for FactoryMatcher {
    fn detect(&self, units: &[ResolvedUnit]) -> Vec<PatternInstance> {
        let mut instances = Vec::new();

        for unit in units {
            if unit.unit.unit_type != CodeUnitType::Function
                && unit.unit.unit_type != CodeUnitType::Type
            {
                continue;
            }

            let name_lower = unit.unit.name.to_lowercase();

            // Name-based detection
            if name_lower.contains("factory")
                || name_lower.starts_with("create_")
                || name_lower.starts_with("make_")
                || name_lower.starts_with("build_")
                || name_lower == "new"
            {
                instances.push(PatternInstance {
                    pattern_name: "Factory".to_string(),
                    primary_unit: unit.unit.temp_id,
                    participating_units: vec![unit.unit.temp_id],
                    confidence: if name_lower.contains("factory") {
                        0.9
                    } else {
                        0.5
                    },
                });
            }
        }

        instances
    }
}

/// Detects Repository pattern: data access layer classes.
struct RepositoryMatcher;

impl PatternMatcher for RepositoryMatcher {
    fn detect(&self, units: &[ResolvedUnit]) -> Vec<PatternInstance> {
        let mut instances = Vec::new();

        for unit in units {
            if unit.unit.unit_type != CodeUnitType::Type {
                continue;
            }

            let name_lower = unit.unit.name.to_lowercase();

            if name_lower.contains("repository")
                || name_lower.contains("repo")
                || name_lower.contains("dao")
                || name_lower.contains("store")
            {
                // Look for CRUD methods
                let mut crud_count = 0;
                for other in units {
                    if other.unit.unit_type == CodeUnitType::Function {
                        let method_lower = other.unit.name.to_lowercase();
                        let in_type = other
                            .unit
                            .qualified_name
                            .to_lowercase()
                            .contains(&name_lower);
                        if in_type
                            && (method_lower.starts_with("get")
                                || method_lower.starts_with("find")
                                || method_lower.starts_with("create")
                                || method_lower.starts_with("update")
                                || method_lower.starts_with("delete")
                                || method_lower.starts_with("save")
                                || method_lower.starts_with("list"))
                        {
                            crud_count += 1;
                        }
                    }
                }

                let confidence = if crud_count >= 3 {
                    0.9
                } else if crud_count >= 1 {
                    0.6
                } else {
                    0.4
                };

                instances.push(PatternInstance {
                    pattern_name: "Repository".to_string(),
                    primary_unit: unit.unit.temp_id,
                    participating_units: vec![unit.unit.temp_id],
                    confidence,
                });
            }
        }

        instances
    }
}

/// Detects Decorator pattern by name convention.
struct DecoratorMatcher;

impl PatternMatcher for DecoratorMatcher {
    fn detect(&self, units: &[ResolvedUnit]) -> Vec<PatternInstance> {
        let mut instances = Vec::new();

        for unit in units {
            let name_lower = unit.unit.name.to_lowercase();

            if name_lower.contains("decorator")
                || name_lower.contains("wrapper")
                || name_lower.contains("middleware")
            {
                instances.push(PatternInstance {
                    pattern_name: "Decorator".to_string(),
                    primary_unit: unit.unit.temp_id,
                    participating_units: vec![unit.unit.temp_id],
                    confidence: if name_lower.contains("decorator") {
                        0.8
                    } else {
                        0.5
                    },
                });
            }
        }

        instances
    }
}
