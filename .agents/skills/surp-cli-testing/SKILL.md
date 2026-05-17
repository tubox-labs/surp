---
name: surp-cli-testing
description: Run and maintain Surp CLI, interop, fuzz, benchmark, and release verification workflows. Use for tasks involving surp-cli commands, JSON/text conversion, validate/inspect/bench behavior, cross-language Rust/Python tests, cargo-fuzz targets, benchmark harnesses, GitHub CI workflows, release checklist, or deciding which Surp tests to run after a change.
---

# Surp CLI Testing

## Source Authority

Start from the executable code and workflows:

- `surp-cli/src/main.rs`: CLI commands and options.
- `docs/CLI.md`: command guide after confirming source.
- `.github/workflows/ci.yml`: build, test, clippy, format, Python, interop, bench, fuzz, audit.
- `.github/workflows/bench.yml`: Rust/Python benchmark workflow.
- `.github/workflows/python-publish.yml`: wheel/sdist publishing.
- `bench/src/*` and `bench/python/bench_surp.py`: benchmark harnesses.
- `fuzz/fuzz_targets/*.rs`: fuzz targets.
- `examples/`: CLI, Rust, and Python smoke fixtures.

## CLI Commands

The `surp` binary supports:

- `inspect <file>`: block layout, compression, payload length, checksum status.
- `pretty <file>` and `decode <file>`: v1 Surp binary to text notation.
- `to-json <file>`: v1 Surp binary to JSON, pretty or compact.
- `from-json <file>`: JSON to v1 Surp binary, with `--dedup` and `--compression`.
- `encode <file>`: v1 Surp text notation to binary.
- `validate <file>`: trailer, checksums, and decode integrity; supports `--strict` and `--checksums-only`.
- `bench <json>`: simple encode/decode throughput loop.
- `rfc-compile`, `rfc-inspect`, `rfc-query`: RFC-001 CTN/CBF/CQL workflows.

Use `-` for stdin only where the source implements it. Commands that derive an output path require explicit `--output` when reading from stdin.

## Compression And Validation Constraints

- CLI rejects requested `lz4`, `zstd`, or `snappy` compression unless the corresponding feature is enabled.
- `validate --checksums-only` rejects compressed data blocks because full payload checksum validation requires decode.
- `inspect` reports compressed data block payload checksum as `n/a`.
- `to-json` renders one top-level value directly and multiple values as a JSON array.
- `rfc-query` returns `null`, a single CTN value, or a CTN sequence.

## Test Selection

Choose the smallest useful set first:

- Core wire or parser change: `cargo test -p surp-core`.
- CLI command change: `cargo test -p surp-cli` plus command smoke tests.
- Python interop change: build Python package and run native tests.
- RFC-001 change: run CLI RFC commands and Python RFC tests.
- Performance-sensitive change: run `cargo run -p surp-bench --release -- --mode ci`.
- Parser/decoder hardening: run adversarial tests and relevant fuzz smoke.

## Smoke Commands

```sh
cargo run -p surp-cli -- from-json examples/data/user.json -o /tmp/user.surp
cargo run -p surp-cli -- validate /tmp/user.surp
cargo run -p surp-cli -- to-json /tmp/user.surp --style compact
cargo run -p surp-cli -- encode examples/data/user.surp.txt -o /tmp/user-text.surp
cargo run -p surp-cli -- pretty /tmp/user-text.surp
cargo run -p surp-cli -- rfc-compile examples/data/user.ctn -o /tmp/user.crb
cargo run -p surp-cli -- rfc-inspect /tmp/user.crb --ctn
cargo run -p surp-cli -- rfc-query /tmp/user.crb ".tags[-1]"
```

## Full Verification

```sh
cargo fmt --all -- --check
cargo test --workspace --all-features
cargo clippy --workspace --all-features -- -D warnings
cd surp-python && maturin develop --release && python -m pytest tests/ -v
```

Fuzzing requires nightly and cargo-fuzz:

```sh
cd fuzz
cargo +nightly fuzz run fuzz_decode -- -max_total_time=30 -max_len=4096
cargo +nightly fuzz run fuzz_roundtrip -- -max_total_time=30 -max_len=4096
cargo +nightly fuzz run fuzz_text -- -max_total_time=30 -max_len=4096
cargo +nightly fuzz run fuzz_varint -- -max_total_time=30 -max_len=4096
cargo +nightly fuzz run fuzz_block -- -max_total_time=30 -max_len=4096
```
