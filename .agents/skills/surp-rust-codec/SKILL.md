---
name: surp-rust-codec
description: Maintain Surp's stable v1 Rust serialization implementation. Use for tasks touching surp-core v1 Value/SurpValue, Encoder, Decoder, block framing, wire types, varints, checksums, text parsing/pretty printing, resource limits, derive macros, compression hooks, IO adapters, SIMD helpers, or C FFI behavior. Also use when reviewing v1 binary compatibility, zero-copy decode semantics, dedup/StringDict behavior, or Rust API examples.
---

# Surp Rust Codec

## Source Authority

Use repository source as the authority. Start with the file that owns the behavior before editing:

- `surp-core/src/lib.rs`: public v1 exports.
- `surp-core/src/value.rs`: owned `Value` and borrowed `SurpValue<'a>`.
- `surp-core/src/encoder.rs`: v1 encoding, block flush, compression fallback, string dedup.
- `surp-core/src/decoder.rs`: block reading, checksums, zero-copy and owned decode, limits.
- `surp-core/src/wire.rs`: stable wire, block, and compression enum IDs.
- `surp-core/src/block.rs`: block header parsing and writing.
- `surp-core/src/text.rs`: v1 text notation parser and formatter.
- `surp-core/src/traits.rs`: `Surp`, `SurpBytes`, blanket impls.
- `surp-derive/src/lib.rs`: `#[derive(Surp)]` and `#[derive(SurpSchema)]`.
- `surp-io/src/lib.rs`, `surp-compression/src/lib.rs`, `surp-simd/src/lib.rs`, `surp-ffi/src/lib.rs`: adapter crates.

Docs in `docs/` are useful only after checking the implementation.

## Architecture Boundaries

Preserve these package boundaries:

- `surp-core` owns the v1 wire format, block framing, checksums, text notation, resource limits, and RFC-001 namespace.
- `surp-derive` only generates trait/schema impls for named structs.
- `surp-io` provides Tokio framed blocks, `bytes::Bytes` helpers, and optional mmap reader.
- `surp-compression` provides pluggable compressor traits and optional zstd/lz4/snappy adapters; core still has feature-gated block compression.
- `surp-simd` contains optional byte scan and batch varint helpers with scalar fallback.
- `surp-ffi` exposes JSON-to-Surp and Surp-to-JSON C ABI buffers with `surp_free`.

Do not move v1 behavior into adapters, and do not make adapters depend on CLI or Python.

## v1 Wire Constraints

Treat these as compatibility-sensitive:

- Block layout is `block_type | block_len(varint) | compression | checksum(8 LE) | payload`.
- Trailer block stores an XXH64 checksum over all preceding bytes.
- Wire tag low nibble selects `WireType`; high nibble is flags.
- Object keys are encoded as raw UTF-8 key length plus bytes, followed by the value.
- Arrays and objects carry item counts and optional end markers.
- `Value::Object(Vec<(String, Value)>)` preserves order; only Python/CLI explicit sort paths reorder.
- `Encoder::enable_dedup()` emits `StringDict` blocks and `Reference` values for repeated strings in a block.
- Zero-copy `Decoder::decode_next()` works only on uncompressed data blocks. Use owned decode for compressed blocks.
- Compression features are optional. Unsupported compression in core falls back to uncompressed; CLI rejects unsupported requested compression.

## Implementation Workflow

1. Locate the owning crate and tests before editing.
2. Keep public enum discriminants and block constants stable unless the task explicitly changes the file format.
3. For decoder changes, preserve checked arithmetic, `Limits`, checksum validation, UTF-8 validation, and clean `SurpError` returns.
4. For encoder changes, update owned and borrowed decode paths when wire output changes.
5. For text-format changes, update parser, pretty printer, ABNF/docs if needed, and roundtrip tests.
6. For derive changes, verify generated behavior still skips unknown fields and requires existing `Surp` impls for field types.

## Validation

Run focused checks first, then broader ones when behavior is shared:

```sh
cargo test -p surp-core
cargo test -p surp-derive
cargo test -p surp-io
cargo test -p surp-compression
cargo test -p surp-simd
cargo test -p surp-ffi
cargo test --workspace --all-features
cargo clippy --workspace --all-features -- -D warnings
cargo fmt --all -- --check
```

For text and adversarial behavior, include:

```sh
cargo test -p surp-core --test roundtrip
cargo test -p surp-core --test adversarial
cargo test -p surp-core --test proptest_extended
```
