# Surp Rust API

Version: 1.0.0
Edition: 2024
MSRV: 1.85.0
License: MIT OR Apache-2.0

This document describes the Rust API as implemented in this repository. It
covers the stable v1 binary/text APIs and the additive RFC-001 CTN/CBF/CQL
modules.

## Workspace Crates

| Crate | Purpose | Default features |
| --- | --- | --- |
| `surp-core` | v1 `Value`, encoder, decoder, text parser/printer, RFC-001 modules | none |
| `surp-derive` | `#[derive(Surp)]` and `#[derive(SurpSchema)]` for named structs | none |
| `surp-io` | Tokio framed IO, file/shared-buffer helpers, optional mmap reader | none |
| `surp-cli` | `surp` command line tool for v1 files and RFC-001 CBF files | none |
| `surp-compression` | pluggable compression trait and optional zstd/lz4/snappy adapters | none |
| `surp-ffi` | C ABI helpers for JSON-to-Surp and Surp-to-JSON buffers | none |
| `surp-simd` | byte scanning and batched varint helpers with scalar fallback | none |
| `surp-python` | PyO3 extension backing the Python package named `surp` | none |
| `bench` | local regression benchmark harness | none |

## Install

```toml
[dependencies]
surp-core = "1.0.0"
```

For local examples in this repository, use path dependencies:

```toml
[dependencies]
surp-core = { path = "../../surp-core" }
surp-derive = { path = "../../surp-derive" }
```

## v1 Binary API

```rust
use surp_core::{Decoder, Encoder, Value};

let value = Value::Object(vec![
    ("name".into(), Value::Str("Alice".into())),
    ("age".into(), Value::UInt(30)),
    ("active".into(), Value::Bool(true)),
]);

let mut encoder = Encoder::new();
encoder.encode_value(&value)?;
let bytes = encoder.finish()?;

let mut decoder = Decoder::new(&bytes);
let decoded = decoder.decode_next()?.to_owned_value();
assert_eq!(decoded, value);
# Ok::<(), surp_core::SurpError>(())
```

### `Value`

`Value` is the owned v1 data model:

```rust
pub enum Value {
    Null,
    Bool(bool),
    UInt(u64),
    Int(i64),
    Float(f64),
    Str(String),
    Bytes(Vec<u8>),
    Array(Vec<Value>),
    Object(Vec<(String, Value)>),
}
```

Objects preserve entry order because they are represented as `Vec<(String,
Value)>`. `Value` has helper methods such as `type_name()`, `is_null()`,
`as_str()`, `as_uint()`, `as_int()`, `as_float()`, `as_bool()`, `as_array()`,
and `as_object()`.

### `SurpValue<'a>`

`SurpValue<'a>` is the zero-copy decode representation:

```rust
pub enum SurpValue<'a> {
    Null,
    Bool(bool),
    UInt(u64),
    Int(i64),
    Float(f64),
    Str(&'a str),
    Bytes(&'a [u8]),
    Array(Vec<SurpValue<'a>>),
    Object(Vec<(&'a str, SurpValue<'a>)>),
}
```

Call `to_owned_value()` when a decoded value needs to outlive the input bytes.
Zero-copy decoding works on uncompressed data blocks. Use the owned decode path
for compressed blocks.

## Encoder

| Method | Behavior |
| --- | --- |
| `Encoder::new()` | Default encoder with default limits and no compression/dedup |
| `Encoder::with_limits(limits)` | Encoder with custom `Limits` |
| `Encoder::with_size_hint(bytes)` | Pre-allocate output and block buffers |
| `enable_dedup()` | Enable per-block string dictionary deduplication |
| `set_compression(CompressionType)` | Select `None`, `Lz4`, `Zstd`, or `Snappy` |
| `encode_value(&Value)` | Append one value to the current block payload |
| `flush_block()` | Emit the current block without finalizing the file |
| `finish()` | Flush remaining data and append the trailer block |
| `current_size()` | Return written bytes plus unflushed block bytes |

`finish()` writes a data block and a trailer block. If string deduplication is
enabled and any strings were observed, it emits a `StringDict` block before the
data block. The current implementation deduplicates repeated strings only; it
does not perform structural subtree deduplication.

Compression is feature gated in `surp-core`. If a requested compression feature
is not compiled in, the encoder falls back to uncompressed output because
`compress_payload()` returns `None`.

## Decoder

| Method | Behavior |
| --- | --- |
| `Decoder::new(data)` | Decoder with default limits |
| `Decoder::with_limits(data, limits)` | Decoder with explicit limits |
| `decode_next()` | Decode one uncompressed value as `SurpValue<'a>` |
| `decode_next_owned()` | Decode one value as owned `Value`, including compressed blocks |
| `decode_all()` | Decode all remaining uncompressed values |
| `decode_all_owned()` | Decode all remaining values as owned `Value`s |
| `skip_value_at(block_end)` | Skip a value from the current block position |
| `position()` | Current input offset |
| `memory_used()` | Cumulative tracked allocations |

The decoder verifies block checksums, consumes `StringDict` blocks
transparently, enforces resource limits, and returns an invalid-reference error
for out-of-bounds dictionary references.

## Resource Limits

```rust
use surp_core::Limits;

let strict = Limits::strict();
let custom = Limits {
    max_nesting_depth: 64,
    ..Limits::default()
};
```

| Field | Default | Strict |
| --- | ---: | ---: |
| `max_nesting_depth` | 128 | 32 |
| `max_block_size` | 64 MiB | 1 MiB |
| `max_items` | 1,000,000 | 10,000 |
| `max_memory` | 256 MiB | 4 MiB |
| `max_string_length` | 16 MiB | 64 KiB |
| `max_decompression_ratio` | 100:1 | 20:1 |

`Limits::unlimited()` is available for trusted inputs only.

## Text Format

```rust
use surp_core::text::{parse, pretty_print};

let value = parse(r#"{ name: "Alice"; age: 30; avatar: b64#AQID; }"#)?;
let text = pretty_print(&value, 2);
# Ok::<(), surp_core::SurpError>(())
```

Implemented syntax includes objects, arrays, strings, base64 bytes, signed and
unsigned integers, floats, `inf`, `-inf`, `NaN`, `null`, booleans, optional
`::type` annotations, `//` line comments, and nested `/* ... */` block
comments.

Important parser details:

- Object fields use `key: value;`.
- Arrays use commas, and the parser also accepts semicolons between array
  elements.
- Binary values are written as `b64#<standard-base64>`; object field
  semicolons are field terminators, not part of the bytes literal.
- Pretty printing emits `Int(0)` as `-0` so reparsing preserves signed zero as
  `Value::Int(0)`.

## Derive Macros

```rust
use surp_core::{Surp, SurpBytes};

#[derive(Debug, PartialEq, surp_derive::Surp, surp_derive::SurpSchema)]
struct Profile {
    #[surp(id = 1)]
    name: String,
    #[surp(id = 2)]
    age: u8,
    #[surp(id = 3)]
    avatar: SurpBytes,
}

let profile = Profile {
    name: "Alice".into(),
    age: 30,
    avatar: SurpBytes::new(vec![1, 2, 3]),
};

let bytes = profile.to_surp_bytes()?;
let decoded = Profile::from_surp_bytes(&bytes)?;
assert_eq!(decoded, profile);
# Ok::<(), surp_core::SurpError>(())
```

`#[derive(Surp)]` currently supports named structs. Fields should use stable
`#[surp(id = N)]` attributes. If an ID is omitted, the macro falls back to a
field-name hash, but explicit IDs are the intended schema-evolution path.

Generated `Surp` behavior:

- `to_surp_value(&self) -> Value`
- `from_surp_value(&Value) -> Result<Self>`
- `schema_fingerprint() -> u64`
- `type_name() -> &'static str`
- Unknown fields are skipped during decode.

Generated `SurpSchema` behavior:

- `schema_info() -> &'static [(&'static str, u64)]`
- `schema_type_name() -> &'static str`

The `Surp` trait has implementations for booleans, unsigned and signed integer
families, floats, `String`, `Box<str>`, `SurpBytes`, `Vec<T>`, `Option<T>`,
`Box<T>`, `HashMap<String, T>`, `BTreeMap<String, T>`, tuples up to six
elements, and `()`.

Use `SurpBytes` when raw bytes should encode as `Value::Bytes`. A plain
`Vec<u8>` encodes through the generic `Vec<T>` implementation as an array of
unsigned integers.

## Compression

`surp-core` can encode/decode compressed blocks when built with matching
features:

| `surp-core` feature | Wire type | Dependency |
| --- | --- | --- |
| `lz4` | `CompressionType::Lz4` | `lz4_flex` |
| `zstd` | `CompressionType::Zstd` | `zstd` |
| `snappy` | `CompressionType::Snappy` | `snap` |

The separate `surp-compression` crate provides:

- `Compressor` trait
- `NoCompression`
- `ZstdCompressor` behind `zstd`
- `Lz4Compressor` behind `lz4`
- `SnappyCompressor` behind `snappy`
- `CompressorRegistry`
- `AdaptiveSelector`

## IO Helpers

`surp-io` exposes:

- `write_values_to_bytes(&[Value])`
- `read_file_bytes(&[u8])`
- `write_to_shared(&[Value]) -> bytes::Bytes`
- `read_from_shared(bytes::Bytes)`
- `FramedWriter<W>` and `FramedReader<R>` for Tokio async streams
- `MmapReader` behind the `mmap` feature

## SIMD Helpers

`surp-simd` exposes scalar-safe helpers and an optional aarch64 NEON path:

- `find_byte(data, needle)`
- `count_byte(data, needle)`
- `find_non_ascii(data)`
- `batch_decode_varints(data, count)`
- `batch_decode_varints_simd(data, count)` behind `simd-varint`

Unsupported architectures use scalar fallback implementations. The source does
not currently implement an x86-specific SIMD path.

## FFI

`surp-ffi` exports a small C ABI:

```c
int surp_encode_buffer(
    const uint8_t *in_ptr, size_t in_len,
    uint8_t **out_ptr, size_t *out_len
);

int surp_decode_buffer(
    const uint8_t *in_ptr, size_t in_len,
    uint8_t **json_out, size_t *json_len
);

void surp_free(uint8_t *ptr, size_t len);
```

The FFI layer accepts JSON bytes for encode, returns JSON bytes for decode, and
requires callers to release library-allocated output with `surp_free`.

## RFC-001 Rust API

RFC-001 lives under `surp_core::rfc001` and is additive to the v1 API.

```rust
use surp_core::rfc001;

let ctn = r#"
User
  name = "Alice"
  tags = ["admin", "ops"]
"#;

let doc = rfc001::parse_document(ctn)?;
let cbf = rfc001::encode_document(&doc, rfc001::EncodeOptions::default())?;
let decoded = rfc001::decode_document(&cbf)?;
let root = decoded.document.effective_root()?;
let tags = rfc001::query(&root, ".tags[]")?;
assert_eq!(tags.len(), 2);
# Ok::<(), surp_core::SurpError>(())
```

Public RFC-001 exports include:

- AST: `Document`, `Annotation`, `Binding`, `Value`, `Scalar`, `Product`,
  `Field`, `Sum`, `SumPayload`, `Sequence`, `Reference`, `Tensor`,
  `TensorData`, `Stream`, `Opaque`
- CTN: `parse_document`, `parse_value`, `format_document`, `format_value`
- CBF: `CBF_MAGIC`, `CBF_HEADER_SIZE`, `CbfHeader`, `EncodeOptions`,
  `DecodedDocument`, `encode_document`, `encode_value`, `decode_document`,
  `decode_value`
- CQL: `query`, `query_one`

Implemented CQL is a baseline path engine: `.field`, `[]`, `[index]`,
negative indexes, `['symbol]`, and `["string"]`.

See `docs/RFC-001-IMPLEMENTATION.md` for coverage and known gaps.

## Feature Flags

| Crate | Feature | Effect |
| --- | --- | --- |
| `surp-core` | `xxh3` | Select XXH3-64 in the checksum API |
| `surp-core` | `compat-crc32` | CRC32 checksum API support |
| `surp-core` | `fast-alloc` | Enable `BumpDecoder` |
| `surp-core` | `lz4` | LZ4 block compression |
| `surp-core` | `zstd` | Zstandard block compression |
| `surp-core` | `snappy` | Snappy block compression |
| `surp-cli` | `lz4`, `zstd`, `snappy` | Forward compression support to `surp-core` |
| `surp-io` | `mmap` | Enable `MmapReader` |
| `surp-compression` | `lz4`, `zstd`, `snappy` | Enable matching compressor adapters |
| `surp-simd` | `simd-varint` | Enable aarch64 NEON varint boundary prescan |

## Verification Commands

```bash
cargo fmt --all
cargo test --workspace --all-features
cargo clippy --workspace --all-features -- -D warnings
cargo run --manifest-path examples/rust/Cargo.toml --bin v1_roundtrip
cargo run --manifest-path examples/rust/Cargo.toml --bin derive_struct
cargo run --manifest-path examples/rust/Cargo.toml --bin text_format
cargo run --manifest-path examples/rust/Cargo.toml --bin rfc001_ctn_cbf_cql
```
