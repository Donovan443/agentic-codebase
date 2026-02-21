//! FFI boundary tracing.
//!
//! Detects calls across language boundaries: Python↔Rust (PyO3, ctypes),
//! Rust↔C (FFI), Node↔native (N-API), and HTTP/RPC boundaries.

use crate::parse::ReferenceKind;
use crate::types::{AcbResult, Language};

use super::resolver::ResolvedUnit;

/// Traces function calls across language boundaries.
pub struct FfiTracer {
    /// Known FFI pattern detectors.
    detectors: Vec<Box<dyn FfiDetector>>,
}

/// An FFI edge connecting two units across languages.
#[derive(Debug, Clone)]
pub struct FfiEdge {
    /// Source unit temp_id.
    pub source_id: u64,
    /// Target unit temp_id (None if external).
    pub target_id: Option<u64>,
    /// Type of FFI pattern.
    pub ffi_type: FfiPatternType,
    /// Description of the binding.
    pub binding_info: String,
}

/// Types of FFI patterns.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FfiPatternType {
    /// Python calling Rust via PyO3.
    PyO3,
    /// Python calling C via ctypes.
    Ctypes,
    /// Python calling C via cffi.
    Cffi,
    /// Rust calling C via extern "C".
    RustCFfi,
    /// Node.js calling native via N-API.
    NodeNapi,
    /// WebAssembly boundary.
    Wasm,
    /// HTTP/RPC call.
    HttpRpc,
}

/// Trait for detecting specific FFI patterns.
trait FfiDetector: Send + Sync {
    /// Detect FFI calls in a unit.
    fn detect(&self, unit: &ResolvedUnit, all_units: &[ResolvedUnit]) -> Vec<FfiEdge>;
}

impl FfiTracer {
    /// Create a new FFI tracer with all pattern detectors.
    pub fn new() -> Self {
        let detectors: Vec<Box<dyn FfiDetector>> = vec![
            Box::new(PyO3Detector),
            Box::new(CtypesDetector),
            Box::new(HttpRpcDetector),
        ];
        Self { detectors }
    }

    /// Trace all FFI boundaries in the resolved units.
    pub fn trace(&self, units: &[ResolvedUnit]) -> AcbResult<Vec<FfiEdge>> {
        let mut edges = Vec::new();

        for unit in units {
            for detector in &self.detectors {
                let calls = detector.detect(unit, units);
                edges.extend(calls);
            }
        }

        Ok(edges)
    }
}

impl Default for FfiTracer {
    fn default() -> Self {
        Self::new()
    }
}

/// Detects PyO3 bindings: Python importing Rust modules annotated with #[pymodule].
struct PyO3Detector;

impl FfiDetector for PyO3Detector {
    fn detect(&self, unit: &ResolvedUnit, all_units: &[ResolvedUnit]) -> Vec<FfiEdge> {
        let mut calls = Vec::new();

        if unit.unit.language != Language::Python {
            return calls;
        }

        for ref_info in &unit.resolved_refs {
            if ref_info.raw.kind == ReferenceKind::Import {
                if let Some(rust_id) = find_pyo3_module(&ref_info.raw.name, all_units) {
                    calls.push(FfiEdge {
                        source_id: unit.unit.temp_id,
                        target_id: Some(rust_id),
                        ffi_type: FfiPatternType::PyO3,
                        binding_info: format!("import {}", ref_info.raw.name),
                    });
                }
            }
        }

        calls
    }
}

/// Detects ctypes/cffi usage in Python.
struct CtypesDetector;

impl FfiDetector for CtypesDetector {
    fn detect(&self, unit: &ResolvedUnit, _all_units: &[ResolvedUnit]) -> Vec<FfiEdge> {
        let mut calls = Vec::new();

        if unit.unit.language != Language::Python {
            return calls;
        }

        for ref_info in &unit.resolved_refs {
            if ref_info.raw.kind == ReferenceKind::Import {
                if ref_info.raw.name.contains("ctypes") {
                    calls.push(FfiEdge {
                        source_id: unit.unit.temp_id,
                        target_id: None,
                        ffi_type: FfiPatternType::Ctypes,
                        binding_info: format!("ctypes usage: {}", ref_info.raw.name),
                    });
                } else if ref_info.raw.name.contains("cffi") {
                    calls.push(FfiEdge {
                        source_id: unit.unit.temp_id,
                        target_id: None,
                        ffi_type: FfiPatternType::Cffi,
                        binding_info: format!("cffi usage: {}", ref_info.raw.name),
                    });
                }
            }
        }

        calls
    }
}

/// Detects HTTP/RPC boundary calls.
struct HttpRpcDetector;

impl FfiDetector for HttpRpcDetector {
    fn detect(&self, unit: &ResolvedUnit, _all_units: &[ResolvedUnit]) -> Vec<FfiEdge> {
        let mut calls = Vec::new();

        for ref_info in &unit.resolved_refs {
            if ref_info.raw.kind == ReferenceKind::Call {
                let name_lower = ref_info.raw.name.to_lowercase();
                if name_lower.contains("fetch")
                    || name_lower.contains("request")
                    || name_lower.contains("http")
                    || name_lower.contains("axios")
                {
                    calls.push(FfiEdge {
                        source_id: unit.unit.temp_id,
                        target_id: None,
                        ffi_type: FfiPatternType::HttpRpc,
                        binding_info: format!("HTTP call: {}", ref_info.raw.name),
                    });
                }
            }
        }

        calls
    }
}

/// Find a Rust unit with pymodule metadata matching the given name.
fn find_pyo3_module(name: &str, units: &[ResolvedUnit]) -> Option<u64> {
    for unit in units {
        if unit.unit.language == Language::Rust {
            if let Some(meta) = unit.unit.metadata.get("pymodule") {
                if meta == name {
                    return Some(unit.unit.temp_id);
                }
            }
        }
    }
    None
}
