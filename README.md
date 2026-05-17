# Surp

**A compact, canonical binary serializer and human-readable alternative to JSON, written in Rust.**

[![CI](https://github.com/surp-format/surp/actions/workflows/ci.yml/badge.svg)](https://github.com/surp-format/surp/actions)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

## Overview

Surp is a production-grade binary serialization format that provides:

- **Compact binary encoding** — 2×–5× smaller than equivalent JSON
- **Human-readable text notation** — unique syntax with deterministic binary mapping
- **Zero-copy decoding** — borrow directly from input buffers
- **Schema evolution** — stable field IDs, unknown-field skipping
- **Streaming support** — block-framed format with per-block checksums
- **Pluggable compression** — zstd/snappy per-block, with plugin trait
- **Cross-language FFI** — clean C bindings with documented ownership

## Quick Start

```rust
use surp_core::{Encoder, Decoder, Value};

// Encode
let value = Value::Object(vec![
    ("name".into(), Value::Str("Alice".into())),
    ("age".into(), Value::UInt(30)),
    ("active".into(), Value::Bool(true)),
]);

let mut encoder = Encoder::new();
encoder.encode_value(&value).unwrap();
let bytes = encoder.finish().unwrap();

// Decode (zero-copy)
let mut decoder = Decoder::new(&bytes);
let decoded = decoder.decode_next().unwrap();
assert_eq!(decoded.to_owned_value(), value);
```

## Human-Readable Syntax

```surp
{
    name: "Alice";
    age: 30;
    tags: ["admin", "user"];
    config: {
        theme: "dark";
        notifications: true;
    };
    avatar: b64#iVBORw0KGgo=;
}
```

## Workspace Crates

| Crate | Description |
|-------|-------------|
| `surp-core` | Encoder/decoder, block framing, Value types |
| `surp-derive` | `#[derive(Surp)]` proc-macro with stable field IDs |
| `surp-io` | Async Tokio adapters, framed streams |
| `surp-cli` | CLI: inspect, pretty-print, convert |
| `surp-compression` | Pluggable zstd/snappy compression |
| `surp-ffi` | C FFI bindings |
| `surp-simd` | NEON/SSE2 SIMD acceleration (byte scanning) |

## Python Implementation

A pure-Python implementation is included in `python/surp/`, providing
full encode/decode compatibility with the Rust implementation:

```python
import surp

# Encode
data = surp.encode({"name": "Alice", "age": 30, "active": True})

# Decode
obj = surp.decode(data)
print(obj)  # {'name': 'Alice', 'age': 30, 'active': True}

# Human-readable text format
text = surp.pretty_print(surp.Value.from_python(obj))
print(text)
```

Cross-language interop verified: Rust-encoded files decode correctly in
Python and vice versa.

```bash
# Run Python tests
cd python && python -m pytest tests/ -v
```

## CLI Usage

```bash
# Install
cargo install --path surp-cli

# Convert JSON to Surp
surp from-json data.json -o data.surp

# Pretty-print a Surp file
surp pretty data.surp

# Inspect block layout
surp inspect data.surp

# Convert back to JSON
surp to-json data.surp

# Quick benchmark
surp bench data.json -n 10000
```

## RFC-001 Preview (Next-Generation Path)

The repository now includes an additive RFC-001 implementation in
`surp_core::rfc001` (CTN + CBF + baseline CQL) without breaking v1 APIs.

```bash
# Compile RFC-001 CTN -> CBF
surp rfc-compile input.surp -o output.crb

# Inspect RFC-001 CBF (and optionally print CTN)
surp rfc-inspect output.crb --ctn

# Run baseline RFC-001 CQL path query
surp rfc-query output.crb ".user.email"
```

Implementation details and current feature coverage:
`docs/RFC-001-IMPLEMENTATION.md`

## Derive Macro

```rust
use surp_derive::{Surp, SurpSchema};

#[derive(Debug, PartialEq, Surp, SurpSchema)]
struct Person {
    #[surp(id = 1)] name: String,
    #[surp(id = 2)] age: u8,
    #[surp(id = 3)] tags: Vec<String>,
}

let alice = Person {
    name: "Alice".into(),
    age: 30,
    tags: vec!["admin".into()],
};

// Encode
let value = alice.to_surp_value();
let mut encoder = Encoder::new();
encoder.encode_value(&value).unwrap();
let bytes = encoder.finish().unwrap();
```

## Binary Format Summary

```
File:    [Block]* [Trailer Block]
Block:   type(1B) | length(varint) | compression(1B) | checksum(8B) | payload
```

Wire types: Null, Bool, VarUInt (LEB128), VarInt (ZigZag+LEB128), Fixed64, LenDelimited, StartObject, EndObject, StartArray, EndArray, Reference.

## Build & Test

```bash
cargo build --workspace
cargo test --workspace --all-features
cargo clippy --workspace --all-features
cargo bench -p surp-core
```

## Fuzzing

```bash
cargo +nightly fuzz run fuzz_decode     # arbitrary byte decoder
cargo +nightly fuzz run fuzz_roundtrip  # structured Value roundtrip
cargo +nightly fuzz run fuzz_text       # text parser
cargo +nightly fuzz run fuzz_varint     # varint codec
```

## Performance

Surp vs JSON (serde_json) on Apple Silicon (M-series):

| Payload | Surp size | JSON size | Ratio | Decode speed |
|---------|-----------|-----------|-------|--------------|
| Small object | 118 B | 90 B | 1.31× | **4× faster** |
| 100 users nested | 9.8 KB | 10.3 KB | 0.95× | ~2× faster |
| 10K integers | 29.9 KB | 52.8 KB | **0.57×** | ~3× faster |
| 64 KB binary | 65.6 KB | 87.4 KB | **0.75×** | ~10× faster |

Surp decode throughput: **1.2 GiB/s** for small objects.

## License

Licensed under either of:
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
