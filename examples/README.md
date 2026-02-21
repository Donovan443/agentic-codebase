# Examples

Rust examples demonstrating AgenticCodebase workflows. Each example is self-contained and runs against a temporary directory with sample source files.

## Running Examples

```bash
# Run a specific example
cargo run --example 01_basic_compile

# Run with release mode (faster)
cargo run --release --example 01_basic_compile
```

## Examples

| # | Example | Description |
|:---|:---|:---|
| 01 | `01_basic_compile.rs` | Compile a directory and inspect the resulting graph |
| 02 | `02_query_symbols.rs` | Find code units by name with different match modes |
| 03 | `03_impact_analysis.rs` | Analyze the impact of changing a specific unit |
| 04 | `04_dependency_graph.rs` | Trace dependency chains (forward and reverse) |
| 05 | `05_code_prophecy.rs` | Predict which units are most likely to break |
| 06 | `06_binary_format.rs` | Write and read the .acb binary format |
| 07 | `07_full_pipeline.rs` | End-to-end: parse, analyze, write, read, query |
