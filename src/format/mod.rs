//! Binary file I/O for the `.acb` format.
//!
//! This module handles reading and writing `.acb` files, including
//! LZ4-compressed string pools and memory-mapped file access.

pub mod compression;
pub mod mmap;
pub mod reader;
pub mod writer;

pub use reader::AcbReader;
pub use writer::AcbWriter;
