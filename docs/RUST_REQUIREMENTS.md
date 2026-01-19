# Rust Ecosystem Requirements

Cargo workspace structure with Rust 1.75+
Dependencies: tokio, serde, apache-avro, json-schema, criterion, proptest, clippy
Linting: cargo clippy --all-targets --all-features -- -D warnings -W clippy::all -W clippy::pedantic
Serialization: AVRO for evolution, JSON Schema Draft 2020-12 for validation
Testing: Given/When/Then unit tests + proptest + criterion benchmarks
Distribution: Binary (target/release/ralph) + optional library crate
CI/CD: fmt, lint, typecheck, test every commit; full test sweep + benchmarks every 5th
