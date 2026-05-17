# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Versioning Rules

- **Major**: File format changes, breaking API changes, and incompatible wire semantics.
- **Minor**: Backward-compatible features and public API additions.
- **Patch**: Bug fixes, performance improvements, tests, and documentation.

## [Unreleased]

No unreleased changes.

## [1.0.0] - 2026-05-17

### Added

- First stable Surp release under the `surp` project and package name.
- Rust workspace crates:
  - `surp-core` for v1 binary encoding/decoding, block framing, `Value`,
    zero-copy `SurpValue`, resource limits, checksums, text parsing, and
    RFC-001 modules.
  - `surp-derive` for `#[derive(Surp)]` and `#[derive(SurpSchema)]`.
  - `surp-cli` for v1 file inspection/conversion/validation and RFC-001
    CTN/CBF/CQL commands.
  - `surp-io` for file/shared-buffer helpers, Tokio framed IO, and optional
    mmap support.
  - `surp-compression` for compressor adapters and adaptive selection.
  - `surp-ffi` for C-compatible JSON-to-Surp and Surp-to-JSON buffers.
  - `surp-simd` for byte scanning and batched varint helpers.
  - `surp-python`, published as the native Python package `surp`.
- v1 binary format support for null, booleans, unsigned and signed integers,
  floats, strings, bytes, arrays, ordered objects, block checksums, trailer
  checksums, optional block compression, and per-block string dictionary
  deduplication.
- Human-readable v1 text notation with object fields, arrays, base64 bytes,
  comments, optional type annotations, `inf`, `-inf`, and `NaN`.
- Native Python API:
  - `dumps`, `loads`, `dump`, `load`
  - `encode`, `decode`, `encode_to_file`, `decode_from_file`
  - `parse_text`, `pretty_print`
  - `Encoder`, `SurpDecoder`
  - `SurpError`, `SurpEncodeError`, `SurpDecodeError`,
    `SurpChecksumError`, `SurpTypeError`, and `SurpRfcError`
- RFC-001 implementation under `surp_core::rfc001` with CTN parsing and
  formatting, CBF encoding/decoding, CRC64 validation, symbol tables, product,
  sum, sequence, map, reference, tensor, stream, and opaque/tagged value
  support.
- Python RFC-001 API under `surp.rfc001` for CTN parsing/normalization,
  CTN-to-CBF compilation, CBF decoding, CBF-to-CTN formatting, and baseline
  CQL queries.
- CLI RFC-001 commands: `rfc-compile`, `rfc-inspect`, and `rfc-query`.
- Comprehensive Rust, Python, CLI, and RFC-001 documentation under `docs/`.
- Runnable examples and fixtures under `examples/` for Rust, Python, CLI, v1
  text/binary workflows, derives, and RFC-001 CTN/CBF/CQL.
- Regression tests for v1 roundtrips, adversarial inputs, resource limits,
  string dictionary references, compression, text parsing, derive macros,
  RFC-001 CTN/CBF/CQL behavior, Python native APIs, and cross-language
  behavior.

### Changed

- Versioning is reset to `1.0.0` for the Surp project, Rust workspace crates,
  Python package metadata, native extension metadata, benchmark crate, and
  example package.
- Release metadata now targets a fresh `v1.0.0` GitHub release.

### Removed

- Removed the legacy pure-Python package tree. Python users should install the
  Rust-backed native `surp` package.
- Removed legacy project/version history from the active changelog so the
  public Surp line starts at `v1.0.0`.

## Release Checklist

- [x] Update version metadata to `1.0.0`
- [x] Update changelog and release notes
- [x] Run `cargo fmt --all`
- [x] Run `cargo test --workspace --all-features`
- [x] Run `cargo clippy --workspace --all-features -- -D warnings`
- [x] Run Python native tests
- [x] Verify Rust, Python, and CLI examples
- [ ] Push commits and tag `v1.0.0`
- [ ] Create GitHub release with `gh`
