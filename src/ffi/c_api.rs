//! C-compatible FFI bindings.
//!
//! Thin wrappers around the public API. Opaque pointers. No Rust types exposed.
//! All functions use `panic::catch_unwind` for safety and null-pointer checks.

use std::ffi::CStr;
use std::os::raw::c_char;
use std::path::Path;

use crate::format::AcbReader;
use crate::graph::CodeGraph;

// ========================================================================
// Error codes
// ========================================================================

/// Success.
pub const ACB_OK: i32 = 0;
/// I/O error.
pub const ACB_ERR_IO: i32 = -1;
/// Invalid argument.
pub const ACB_ERR_INVALID: i32 = -2;
/// Not found.
pub const ACB_ERR_NOT_FOUND: i32 = -3;
/// Buffer overflow.
pub const ACB_ERR_OVERFLOW: i32 = -4;
/// Null pointer.
pub const ACB_ERR_NULL_PTR: i32 = -5;

// ========================================================================
// Graph lifecycle
// ========================================================================

/// Load a code graph from an `.acb` file. Returns handle or NULL on failure.
///
/// # Safety
///
/// `path` must be a valid, non-null, null-terminated C string.
#[no_mangle]
pub unsafe extern "C" fn acb_graph_open(path: *const c_char) -> *mut std::ffi::c_void {
    std::panic::catch_unwind(|| {
        if path.is_null() {
            return std::ptr::null_mut();
        }
        let path_str = unsafe { CStr::from_ptr(path) };
        let path_str = match path_str.to_str() {
            Ok(s) => s,
            Err(_) => return std::ptr::null_mut(),
        };
        match AcbReader::read_from_file(Path::new(path_str)) {
            Ok(graph) => Box::into_raw(Box::new(graph)) as *mut std::ffi::c_void,
            Err(_) => std::ptr::null_mut(),
        }
    })
    .unwrap_or(std::ptr::null_mut())
}

/// Free a graph handle.
///
/// # Safety
///
/// `graph` must be a valid handle returned by `acb_graph_open`, or null.
/// Must not be called more than once for the same handle.
#[no_mangle]
pub unsafe extern "C" fn acb_graph_free(graph: *mut std::ffi::c_void) {
    if !graph.is_null() {
        let _ = std::panic::catch_unwind(|| unsafe {
            drop(Box::from_raw(graph as *mut CodeGraph));
        });
    }
}

// ========================================================================
// Graph metadata
// ========================================================================

/// Get the number of code units in the graph.
///
/// # Safety
///
/// `graph` must be a valid, non-null handle from `acb_graph_open`.
#[no_mangle]
pub unsafe extern "C" fn acb_graph_unit_count(graph: *mut std::ffi::c_void) -> u64 {
    std::panic::catch_unwind(|| {
        if graph.is_null() {
            return 0;
        }
        let graph = unsafe { &*(graph as *const CodeGraph) };
        graph.unit_count() as u64
    })
    .unwrap_or(0)
}

/// Get the number of edges in the graph.
///
/// # Safety
///
/// `graph` must be a valid, non-null handle from `acb_graph_open`.
#[no_mangle]
pub unsafe extern "C" fn acb_graph_edge_count(graph: *mut std::ffi::c_void) -> u64 {
    std::panic::catch_unwind(|| {
        if graph.is_null() {
            return 0;
        }
        let graph = unsafe { &*(graph as *const CodeGraph) };
        graph.edge_count() as u64
    })
    .unwrap_or(0)
}

/// Get the embedding dimension.
///
/// # Safety
///
/// `graph` must be a valid, non-null handle from `acb_graph_open`.
#[no_mangle]
pub unsafe extern "C" fn acb_graph_dimension(graph: *mut std::ffi::c_void) -> u32 {
    std::panic::catch_unwind(|| {
        if graph.is_null() {
            return 0;
        }
        let graph = unsafe { &*(graph as *const CodeGraph) };
        graph.dimension() as u32
    })
    .unwrap_or(0)
}

// ========================================================================
// Unit access
// ========================================================================

/// Get a unit's name. Writes to buffer. Returns name length or error code.
///
/// # Safety
///
/// `graph` must be a valid handle. `buffer` must point to at least
/// `buffer_size` bytes of writable memory.
#[no_mangle]
pub unsafe extern "C" fn acb_graph_get_unit_name(
    graph: *mut std::ffi::c_void,
    unit_id: u64,
    buffer: *mut c_char,
    buffer_size: u32,
) -> i32 {
    std::panic::catch_unwind(|| {
        if graph.is_null() || buffer.is_null() {
            return ACB_ERR_NULL_PTR;
        }
        let graph = unsafe { &*(graph as *const CodeGraph) };
        match graph.get_unit(unit_id) {
            Some(unit) => {
                let name_bytes = unit.name.as_bytes();
                if name_bytes.len() + 1 > buffer_size as usize {
                    return ACB_ERR_OVERFLOW;
                }
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        name_bytes.as_ptr(),
                        buffer as *mut u8,
                        name_bytes.len(),
                    );
                    *buffer.add(name_bytes.len()) = 0; // null terminator
                }
                name_bytes.len() as i32
            }
            None => ACB_ERR_NOT_FOUND,
        }
    })
    .unwrap_or(ACB_ERR_INVALID)
}

/// Get a unit's type as a u8. Returns -1 if not found.
///
/// # Safety
///
/// `graph` must be a valid, non-null handle from `acb_graph_open`.
#[no_mangle]
pub unsafe extern "C" fn acb_graph_get_unit_type(
    graph: *mut std::ffi::c_void,
    unit_id: u64,
) -> i32 {
    std::panic::catch_unwind(|| {
        if graph.is_null() {
            return -1;
        }
        let graph = unsafe { &*(graph as *const CodeGraph) };
        graph
            .get_unit(unit_id)
            .map(|u| u.unit_type as i32)
            .unwrap_or(-1)
    })
    .unwrap_or(-1)
}

/// Get a unit's file path. Writes to buffer. Returns path length or error code.
///
/// # Safety
///
/// `graph` must be a valid handle. `buffer` must point to at least
/// `buffer_size` bytes of writable memory.
#[no_mangle]
pub unsafe extern "C" fn acb_graph_get_unit_file(
    graph: *mut std::ffi::c_void,
    unit_id: u64,
    buffer: *mut c_char,
    buffer_size: u32,
) -> i32 {
    std::panic::catch_unwind(|| {
        if graph.is_null() || buffer.is_null() {
            return ACB_ERR_NULL_PTR;
        }
        let graph = unsafe { &*(graph as *const CodeGraph) };
        match graph.get_unit(unit_id) {
            Some(unit) => {
                let path_str = unit.file_path.display().to_string();
                let path_bytes = path_str.as_bytes();
                if path_bytes.len() + 1 > buffer_size as usize {
                    return ACB_ERR_OVERFLOW;
                }
                unsafe {
                    std::ptr::copy_nonoverlapping(
                        path_bytes.as_ptr(),
                        buffer as *mut u8,
                        path_bytes.len(),
                    );
                    *buffer.add(path_bytes.len()) = 0;
                }
                path_bytes.len() as i32
            }
            None => ACB_ERR_NOT_FOUND,
        }
    })
    .unwrap_or(ACB_ERR_INVALID)
}

/// Get a unit's complexity score. Returns -1.0 if not found.
///
/// # Safety
///
/// `graph` must be a valid, non-null handle from `acb_graph_open`.
#[no_mangle]
pub unsafe extern "C" fn acb_graph_get_unit_complexity(
    graph: *mut std::ffi::c_void,
    unit_id: u64,
) -> f32 {
    std::panic::catch_unwind(|| {
        if graph.is_null() {
            return -1.0;
        }
        let graph = unsafe { &*(graph as *const CodeGraph) };
        graph
            .get_unit(unit_id)
            .map(|u| u.complexity as f32)
            .unwrap_or(-1.0)
    })
    .unwrap_or(-1.0)
}

// ========================================================================
// Edge access
// ========================================================================

/// Get outgoing edges from a unit. Returns edge count or error code.
///
/// # Safety
///
/// `graph` must be a valid handle. `target_ids`, `edge_types`, and `weights`
/// must each point to at least `max_edges` elements of writable memory.
#[no_mangle]
pub unsafe extern "C" fn acb_graph_get_edges(
    graph: *mut std::ffi::c_void,
    unit_id: u64,
    target_ids: *mut u64,
    edge_types: *mut u8,
    weights: *mut f32,
    max_edges: u32,
) -> i32 {
    std::panic::catch_unwind(|| {
        if graph.is_null() || target_ids.is_null() || edge_types.is_null() || weights.is_null() {
            return ACB_ERR_NULL_PTR;
        }
        let graph = unsafe { &*(graph as *const CodeGraph) };
        let edges = graph.edges_from(unit_id);
        let count = edges.len().min(max_edges as usize);
        for (i, edge) in edges.iter().take(count).enumerate() {
            unsafe {
                *target_ids.add(i) = edge.target_id;
                *edge_types.add(i) = edge.edge_type as u8;
                *weights.add(i) = edge.weight;
            }
        }
        count as i32
    })
    .unwrap_or(ACB_ERR_INVALID)
}

/// Get a unit's language. Returns language as u8, or -1 if not found.
///
/// # Safety
///
/// `graph` must be a valid, non-null handle from `acb_graph_open`.
#[no_mangle]
pub unsafe extern "C" fn acb_graph_get_unit_language(
    graph: *mut std::ffi::c_void,
    unit_id: u64,
) -> i32 {
    std::panic::catch_unwind(|| {
        if graph.is_null() {
            return -1;
        }
        let graph = unsafe { &*(graph as *const CodeGraph) };
        graph
            .get_unit(unit_id)
            .map(|u| u.language as i32)
            .unwrap_or(-1)
    })
    .unwrap_or(-1)
}

/// Get a unit's stability score. Returns -1.0 if not found.
///
/// # Safety
///
/// `graph` must be a valid, non-null handle from `acb_graph_open`.
#[no_mangle]
pub unsafe extern "C" fn acb_graph_get_unit_stability(
    graph: *mut std::ffi::c_void,
    unit_id: u64,
) -> f32 {
    std::panic::catch_unwind(|| {
        if graph.is_null() {
            return -1.0;
        }
        let graph = unsafe { &*(graph as *const CodeGraph) };
        graph
            .get_unit(unit_id)
            .map(|u| u.stability_score)
            .unwrap_or(-1.0)
    })
    .unwrap_or(-1.0)
}
