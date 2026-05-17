# SURP RFC-001 Implementation (Repository Status)

This document describes the executable RFC-001 implementation introduced in this repository.

## Scope

RFC-001 has been added as a **parallel v2 architecture** without breaking existing v1 APIs.

Implemented in `surp-core/src/rfc001/`:

- `ast.rs`: Native RFC data model (`Value`, `Scalar`, `Product`, `Sum`, `Tensor`, `Stream`, `Reference`, `Document`).
- `ctn.rs`: RFC-style CTN parser/formatter (indentation-first syntax, `let`, `@annotations`, product/sum/sequence/map/tensor/stream forms).
- `cbf.rs`: Segment-tree CBF encoder/decoder with 32-byte header, symbol table support, typed segments, and checksum trailer.
- `cql.rs`: Baseline CQL path engine (`.field`, `[index]`, `[]`, map-key selectors).

CLI integration in `surp-cli`:

- `surp rfc-compile <input.ctn> -o <output.crb>`
- `surp rfc-inspect <input.crb> [--ctn]`
- `surp rfc-query <input.crb> '<expr>'`

## RFC-001 Features Currently Implemented

### CTN

- Document-level annotations (`@name [value]`)
- `let` bindings and root expressions
- Product literals via indentation:
  - `TypeName` + indented `field = value`
  - `struct` anonymous products
- Sum/enum variants:
  - `Type :: Variant`
  - `Type :: Variant(...)`
  - `Type :: Variant` + indented named payload
- Sequences:
  - Inline: `[a, b, c]`
  - Block: `seq<T>` + indented elements
- Maps:
  - Inline: `map<K, V> [k => v]`
  - Block: `map<K, V>` + indented pairs
- References:
  - `&binding`
  - `ref <value>`
- Tensor and stream headers with block payload/annotations
- Typed literals and tagged literals:
  - integer/float suffixes (`u8`, `i64`, `vi64`, `f32`, `f64`, ...)
  - tagged quoted forms (`ts"..."`, `uid"..."`, `url"..."`, etc.)

### CBF

- 32-byte header with RFC-aligned fields:
  - magic `SURP`
  - versions, flags, alignment, offsets
- Segment headers (4-byte compact + 12-byte extended for large payloads)
- Implemented segment categories:
  - `PRIMITIVE`, `STRING`, `BYTES`, `SYMBOL`, `STRUCT`, `ENUM`, `SEQUENCE`, `MAP`, `TENSOR`, `REFERENCE`, `STREAM`, `OPAQUE`, `SPECIAL`
- Special encodings for null/unit/booleans/empty values
- Sequence/map offset-table encoding for direct element lookup
- Symbol table block (interned symbols and field/type names)
- End-of-file checksum trailer (CRC64-ECMA)
- Reference resolution from CTN `let` bindings at encode-time

### CQL (Baseline)

- Dot traversal: `.a.b.c`
- Sequence flatten: `[]`
- Sequence indexing: `[0]`, `[-1]`
- Map key selectors: `['theme]`, `["key"]`

## Compatibility and Safety Notes

- RFC-001 is additive: v1 APIs (`Encoder`, `Decoder`, `text`) are unchanged.
- RFC-001 modules return the same crate-wide `SurpError` type for uniform handling.
- Decoder enforces nesting-depth limits during recursive segment decoding.
- Checksum validation is mandatory in RFC-001 decode path.

## Known Gaps vs RFC Draft

The following are not yet fully implemented:

- Full CSL parser/compiler and witness cryptography pipeline
- Compact mode schema-driven binary elision
- Full CQL pipeline ops (`where`, `select`, `group_by`, aggregates)
- Stream framing (`CRFM` chunk protocol)
- Rich tensor quantization/sparse formats
- CPC RPC framework and database page format
- Formal migration DSL execution

## Design Decision: Parallel V2 Path

RFC-001 is intentionally built as `surp_core::rfc001::*` so teams can:

1. Keep v1 production workflows stable.
2. Incrementally adopt RFC-001 capabilities.
3. Benchmark and harden v2 before defaulting to it.
