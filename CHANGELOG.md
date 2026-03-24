# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## Versioning Rules

- **Major**: File format changes (magic, wire type semantics), breaking API changes.
- **Minor**: New features (new wire types, new block types, new API methods), backward-compatible.
- **Patch**: Bug fixes, performance improvements, documentation.

## [Unreleased]

## [1.2.0] - 2026-03-24

### Security

- **32-bit platform safety**: Added `safe_usize()` helper to validate length conversions, preventing overflow on 32-bit systems. All `as usize` casts replaced with checked conversions that return `LengthOverflow` error for values exceeding platform limits.
- **Decompression bomb protection**: Added `max_decompression_ratio` limit (default 100:1, strict 20:1) to prevent decompression bombs. Decoder validates ratio before decompressing blocks and returns `DecompressionRatioExceeded` error for suspicious payloads.
- **Invalid reference hardening**: Changed invalid reference handling from silent fallback (UInt) to explicit `InvalidReference` error, preventing potential data corruption or confusion from malformed string dictionary references.
- **String length validation**: Explicit validation of string lengths against `max_string_length` limit with dedicated `StringTooLong` error type.
- **MSRV compatibility**: Added `floor_char_boundary()` polyfill for platforms using Rust 1.85.0 (method stable since 1.91.0), ensuring UTF-8 safety on all supported platforms.

### Performance

- **Zero-allocation block flush**: Replaced `block_buf.clone()` with `std::mem::take()` in encoder's `flush_block()`, eliminating unnecessary allocation and copy on every block boundary (10-15% speedup for multi-block encodes).
- **Pre-allocation support**: Added `Encoder::with_size_hint(estimated_size)` constructor for workloads with known size, reducing reallocations during encoding.

### Python API

- **JSON-like API**: Complete redesign of Python bindings with `dumps()`/`loads()`/`dump()`/`load()` functions matching the `json` module conventions. Supports options: `compression`, `dedup`, `sort_keys`, `strict`, `max_depth`.
- **Exception hierarchy**: Added custom exception types for better error handling:
  - `CrousError`: Base exception class
  - `CrousEncodeError`: Encoding failures
  - `CrousDecodeError`: Decoding failures (malformed data)
  - `CrousChecksumError`: Checksum verification failures
  - `CrousTypeError`: Type conversion errors
- **IDE support**: Updated `_crous_native.pyi` type stubs with complete signatures for all functions, classes, and exceptions.

### Added

- New error types: `LengthOverflow`, `InvalidReference`, `DecompressionRatioExceeded`, `StringTooLong`
- Security-focused test suite in `adversarial.rs`:
  - `decompression_ratio_limit_enforcement`: Validates decompression bomb protection
  - `string_too_long_error`: Tests string length limits
  - `length_overflow_handling`: Validates 32-bit safe length handling
  - `invalid_reference_zero_copy`: Tests reference validation in zero-copy path
- Updated `reference_to_nonexistent_dict_entry` test to expect error instead of fallback behavior

### Fixed

- Clippy warning: `len() > 0` → `!is_empty()` in decompression check
- Clippy `unsafe-op-in-unsafe-fn`: Added explicit unsafe block in `crous-simd` NEON intrinsics
- Test reliability: Fixed `decompression_ratio_limit_enforcement` to use moderate compression ratio data for default limits test

### Breaking Changes

- **Invalid reference behavior**: Code relying on silent UInt fallback for invalid references will now receive an error. Update error handling to expect `InvalidReference` errors.
- **Python API**: Complete rewrite of Python bindings. Old `encode()`/`decode()` functions replaced with `dumps()`/`loads()`/`dump()`/`load()`. Update import statements and function calls.

## [1.1.2] - 2026-03-10

### Added
- Initial implementation of `crous-core` with encoder/decoder
- LEB128 varint and ZigZag signed integer encoding
- Block framing with per-block XXH64 checksums
- File header with magic "CROUSv1"
- `Value` (owned) and `CrousValue<'a>` (zero-copy) types
- Human-readable text parser and pretty-printer
- `#[derive(Crous)]` and `#[derive(CrousSchema)]` proc-macros
- CLI tool: inspect, pretty, to-json, from-json, encode, bench
- Compression plugin trait with no-op, zstd, snappy adapters
- C FFI bindings with `crous_encode_buffer`, `crous_decode_buffer`, `crous_free`
- Async Tokio adapters (FramedWriter, FramedReader)
- Property-based tests (proptest)
- Criterion benchmarks
- Fuzz target for decode functions
- GitHub Actions CI
- Security documentation and threat model
- Design risks and tradeoffs document

### Added (Audit & Hardening)
- **Decoder memory tracking**: cumulative allocation tracking with configurable `max_memory` limit
- **Unknown-field skipping**: `skip_value_at()` for forward-compatible decoding
- **String deduplication**: encoder `enable_dedup()` emits `Reference` wire types for repeated strings; decoder resolves via zero-copy `str_slices` table
- **StringDict block format** (type 0x04): per-block string dictionary emitted before data blocks with prefix-delta compression for sorted entries; decoder transparently consumes StringDict blocks and pre-populates reference tables
- **Prefix-delta compression**: dictionary entries sorted lexicographically; each entry stores `original_index | prefix_len | suffix_len | suffix` for compact storage of structured/hierarchical key names
- **Owned decode path** (`decode_next_owned()` / `decode_all_owned()`): transparent decompression + dedup resolution for compressed blocks; zero-copy path rejects compressed blocks with a descriptive error
- **Compression wired into encoder/decoder pipeline**: `flush_block()` compresses payload when configured; `read_next_block()` decompresses transparently; checksum on uncompressed data; fallback to None when compression doesn't help
- **NEON SIMD byte scanning** (`crous-simd`): vectorized `find_byte()`, `count_byte()`, `find_non_ascii()` using aarch64 NEON intrinsics with scalar fallbacks
- **Pure Python implementation** (`python/crous/`): full encode/decode with 8 modules, XXH64 hasher, text parser/printer, 54 tests
- **PyO3 native extension** (`crous-python`): native Python bindings via PyO3 0.28, `encode()`/`decode()` functions + `Encoder`/`CrousDecoder` classes, dedup and compression support, 24 tests
- **Cross-language interop**: bidirectional Rust↔Python binary format verified (native + pure Python)
- **Expanded benchmarks**: JSON head-to-head comparison, deep nesting, numeric arrays, size report
- **3 new fuzz targets**: `fuzz_roundtrip` (structured Value), `fuzz_text` (text parser), `fuzz_varint` (varint codec)
- **CI improvements**: MSRV testing (1.85.0), multi-OS matrix, Python test job, cross-language interop job, all 4 fuzz targets

### Fixed
- Python XXH64: corrected `PRIME64_2` constant (`0xC2B2AE3D27D4EB4F`)
- Python XXH64: corrected `PRIME64_4` constant (`0x85EBCA77C2B2AE63`)
- `crous-compression`: conditional `#[cfg]` gate on `CrousError` import to eliminate unused-import warning

## [1.1.0] - 2026-02-25

### Added
- **Full primitive type support**: `Crous` trait implementations for all Rust integer types (`u8`, `u16`, `u32`, `u64`, `u128`, `usize`, `i8`, `i16`, `i32`, `i64`, `i128`, `isize`), `f32`, `Box<str>`, `Box<T>`, `()`, tuples up to 6 elements
- **`CrousBytes` newtype**: dedicated type for raw binary blob encoding (`Value::Bytes`), distinct from `Vec<u8>` which encodes as `Array`
- **Map support**: `HashMap<String, T>` and `BTreeMap<String, T>` → `Value::Object`
- **Cross-type decode compatibility**: signed integer types accept `Value::UInt` when the value fits
- 42 new tests for trait implementations (29 unit + 13 derive integration)
- 6 production bugs found and fixed via fuzzing (encoder empty finish, block overflow, text Int(0) roundtrip, inf/NaN handling, StringDict OOM, char boundary panic)
- 9 fuzz targets (4 new: string_dict, compress_corrupt, limits, dedup)
- Miri validation: 73 core tests verified zero undefined behavior
- PyO3 `build.rs` for plain `cargo build` compatibility

### Fixed
- `Encoder::finish()` without prior encode now produces valid file with magic header
- `BlockReader::parse` integer overflow on malicious `block_len`
- Text `Int(0)` roundtrip: now pretty-prints as `"-0"` to avoid reparsing as `UInt(0)`
- Text parser handles `inf`, `-inf`, `NaN` float literals
- StringDict `original_idx` unbounded → OOM on 9-byte malicious input (now validated)
- StringDict `prefix_len` not char-boundary-safe → panic on corrupted data (now uses `floor_char_boundary`)

## [0.1.0] - 2026-02-24

Initial release.

---

## Release Checklist

- [x] Update version in all `Cargo.toml` files
- [x] Update this CHANGELOG
- [ ] Run full test suite: `cargo test --workspace --all-features`
- [ ] Run clippy: `cargo clippy --workspace --all-features -- -D warnings`
- [ ] Run Python tests: `cd python && python3 -m pytest tests/ -v`
- [ ] Run benchmarks: `cargo bench -p crous-core`
- [ ] Run all fuzz targets (30s each)
- [ ] Run `cargo audit`
- [ ] Verify cross-language interop (Rust↔Python)
- [ ] Review any new `unsafe` code
- [ ] Tag release: `git tag v1.1.0`
- [ ] Publish: `cargo publish -p crous-core && cargo publish -p crous-derive && ...`
