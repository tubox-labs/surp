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

## [1.0.1] - 2026-05-17 (pre-release)

This is a pre-release changelog entry for validation and packaging review. No
release tag or GitHub release has been created for this version.

### Added

- Added Rust v1 introspection helpers on `Value` and `SurpValue<'_>` for
  JSON-like object and array access: `get`, `get_index`, `contains_key`,
  `keys`, `values`, `items`/`entries`, container predicates, and length checks.
- Added RFC-001 AST introspection helpers for documents, products, sums,
  sequences, tensors, tensor data, streams, and RFC values.
- Added native-backed Python view/model APIs for discoverable attribute access:
  `SurpValue`, `to_value`, `loads_value`, and `parse_text_value`.
- Added native-backed Python RFC-001 model APIs:
  `RfcAnnotation`, `RfcField`, `RfcBinding`, `RfcHeader`, `RfcDocument`,
  `RfcDecodedCbf`, `RfcValue`, `parse_ctn_model`, `decode_cbf_model`,
  `query_cbf_model`, and `query_ctn_model`.
- Added a private `_surp_native.pyi` stub so Pyright and other type checkers can
  resolve the compiled extension module through the public facade.

### Changed

- Expanded Python `.pyi` stubs and shared typed dictionaries to describe the new
  v1 and RFC-001 introspection surfaces.
- Updated Python and Rust docs plus examples to show model/view access while
  keeping the existing dictionary and built-in Python value APIs documented.
- Updated Python packaging metadata to ship package-local README and license
  files directly with wheels and sdists.

### Fixed

- Fixed Pyright analysis of `surp`, `surp.exceptions`, and `surp.rfc001` by
  providing a typed private stub for the native extension import.
- Fixed RFC-001 query overload stubs so `as_ctn=True` and `as_ctn=False` are
  distinguishable to type checkers.

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
