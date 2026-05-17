# RFC-001 Implementation

This repository contains an executable RFC-001 implementation under
`surp_core::rfc001`. It is additive: existing v1 binary/text APIs remain in
place and RFC-001 lives in a parallel module namespace.

## Source Map

| File | Role |
| --- | --- |
| `surp-core/src/rfc001/ast.rs` | RFC data model: documents, annotations, scalars, products, sums, sequences, maps, references, tensors, streams, opaque values |
| `surp-core/src/rfc001/ctn.rs` | CTN parser and formatter |
| `surp-core/src/rfc001/cbf.rs` | CBF encoder/decoder, header, symbol table, segment encoding, CRC64 trailer |
| `surp-core/src/rfc001/cql.rs` | Baseline CQL path query engine |
| `surp-cli/src/main.rs` | `rfc-compile`, `rfc-inspect`, and `rfc-query` commands |
| `surp-python/src/lib.rs` | Native Python `surp.rfc001` bindings |

## Rust API

```rust
use surp_core::rfc001;

let doc = rfc001::parse_document("User\n  name = \"Alice\"")?;
let cbf = rfc001::encode_document(&doc, rfc001::EncodeOptions::default())?;
let decoded = rfc001::decode_document(&cbf)?;
let root = decoded.document.effective_root()?;
let result = rfc001::query(&root, ".name")?;
assert_eq!(rfc001::format_value(&result[0]), "\"Alice\"");
# Ok::<(), surp_core::SurpError>(())
```

Public exports:

- `Document`, `Binding`, `Annotation`
- `Value`, `Scalar`, `Product`, `Field`, `Sum`, `SumPayload`
- `Sequence`, `Reference`, `Tensor`, `TensorData`, `Stream`, `Opaque`
- `parse_document`, `parse_value`
- `format_document`, `format_value`
- `CBF_MAGIC`, `CBF_HEADER_SIZE`
- `CbfHeader`, `EncodeOptions`, `DecodedDocument`
- `encode_document`, `encode_value`
- `decode_document`, `decode_value`
- `query`, `query_one`

The RFC AST exposes native introspection helpers. `Document` supports
`binding()`, `binding_value()`, `binding_names()`, `annotation()`, and
`annotation_names()`. `Value` supports `len()`, `is_empty()`, `get(name)`,
`get_index(index)`, `contains_key()`, `keys()`, and `values()` over products,
associations, sequences, struct-style sum payloads, tuple-style sum payloads,
and `Reference::ById` wrappers. `Product`, `Sum`, `SumPayload`, `Sequence`,
`Tensor`, `TensorData`, and `Stream` expose matching metadata helpers for their
owned fields.

## CTN Coverage

Implemented CTN document features:

- Document annotations with `@name` and optional scalar values
- `use ...` statements preserved in the parsed document and formatter
- `let name = value` bindings
- Explicit root expressions
- Root fallback to the last `let` binding when no explicit root exists
- Line comments with `--`
- Block comments with `--[[ ... ]]`
- Shebang-like/comment directive lines starting with `--!`
- Space indentation; tabs are rejected for indentation

Implemented value forms:

- Products: `TypeName` with indented `field = value`
- Anonymous products with `struct`
- Sum variants: `Type :: Variant`, tuple payloads, and named payloads
- Inline sequences: `[a, b, c]`
- Parenthesized tuple-like sequences: `(a, b, c)`
- Block sequences: `seq<T>` with indented elements
- Inline maps: `map<K, V> [key => value, ...]`
- Block maps: `map<K, V>` with indented `key => value`
- Binding references: `&name`
- Reference-by-id expression wrapper: `ref <value>`
- Tensors: `tensor<T>[shape]`, `vec<T>[shape]`, and `mat<T>[shape]`
- Streams: `stream<T>` with annotation-only body
- Strings, triple-quoted strings, booleans, null, unit
- Symbols with `'Name`
- Tagged quoted literals such as `uid"..."`, `ts"..."`, and `url"..."`
- Bytes as `b64"..."`, compact hex `b"deadbeef"`, or spaced hex `<de ad be ef>`
- Numeric suffixes including `u8`, `u16`, `u32`, `u64`, `u128`, `i8`,
  `i16`, `i32`, `i64`, `i128`, `vi32`, `vi64`, `vu32`, `vu64`, `f16`,
  `bf16`, `f32`, `f64`, `f128`, `dec32`, `dec64`, and `dec128`
- Decimal and unsupported-width numeric suffixes are preserved as tagged
  scalar values when they cannot map to the current native scalar set.

The formatter emits repository-canonical CTN for the supported AST. It is not
a whitespace-preserving formatter.

## CBF Coverage

Implemented CBF file structure:

- 32-byte header
- Magic bytes: `SURP`
- `cbf_version == 1`
- `ctn_version == 1`
- Flags for self-describing files, symbol-table presence, and index presence
- Alignment byte recorded in the header
- 8-byte schema hash prefix field, currently zero-filled by the encoder
- Root offset
- Symbol-table offset
- Index offset field, currently zero when no index is emitted
- CRC64-ECMA trailer over all bytes before the trailer

Implemented segment behavior:

- Compact 4-byte segment header with 4-bit type, 4-bit config, and 24-bit payload length
- 12-byte extended segment header for large payloads
- Special segments for null, unit, booleans, empty sequence, empty map,
  empty string, and empty bytes
- Fixed and varint primitive integer encodings
- f32 and f64 primitive encodings
- UTF-8 string and raw bytes segments
- Symbol table for symbols and field/type names when enabled
- Product and sum segments
- Sequence and map segments with offset tables
- Reference segments
- Tensor segments for dense f64/i64/u64 and binary blobs
- Stream segments with annotations
- Opaque/tagged scalar segments

`encode_document()` resolves CTN binding references before writing CBF. Cyclic
binding references are rejected.

`decode_document()` validates the CRC64 trailer, decodes symbols if the symbol
table flag is set, decodes the root segment, and returns a `DecodedDocument`
containing the header, symbols, and a document whose `root` is populated.

## CQL Coverage

The implemented CQL engine is a baseline structural path evaluator.

Supported selectors:

- `.field`
- `[]` to flatten sequences or map values
- `[0]` and other zero-based sequence indexes
- `[-1]` and other negative sequence indexes
- `['symbol]` map key selector
- `["string"]` map key selector

Traversal applies to products, maps/associations, struct-style sum payloads,
sequences, and `Reference::ById` wrappers. Missing fields/selectors return an
empty result list rather than an error.

Unsupported CQL syntax returns an error. Pipeline operators such as `where`,
`select`, `group_by`, projections, joins, and aggregates are not implemented.

## CLI Integration

```bash
cargo run -p surp-cli -- rfc-compile examples/data/user.ctn -o /tmp/user.crb
cargo run -p surp-cli -- rfc-inspect /tmp/user.crb --ctn
cargo run -p surp-cli -- rfc-query /tmp/user.crb ".tags[-1]"
```

Command behavior:

- `rfc-compile` parses CTN and writes CBF.
- `--no-symtab` disables symbol-table generation and rejects symbol values.
- `--alignment` stores the alignment hint byte in the CBF header.
- `rfc-inspect` prints header metadata and symbol count.
- `rfc-inspect --ctn` also writes decoded CTN to stdout or `--output`.
- `rfc-query` prints `null` for no results, a single CTN value for one result,
  or a CTN sequence for multiple results.

## Python Integration

```python
from surp import rfc001

cbf = rfc001.compile_ctn('User\n  name = "Alice"')
decoded = rfc001.decode_cbf(cbf)
assert decoded["header"]["magic"] == "SURP"
assert rfc001.query_cbf(cbf, ".name", as_ctn=True) == ['"Alice"']
```

Python functions:

- `parse_ctn(text)`
- `normalize_ctn(text)`
- `compile_ctn(text, *, with_symtab=True, alignment=0)`
- `decode_cbf(data)`
- `cbf_to_ctn(data)`
- `query_cbf(data, query, *, as_ctn=False)`
- `query_ctn(text, query, *, as_ctn=False)`
- `parse_ctn_model(text)`
- `decode_cbf_model(data)`
- `query_cbf_model(data, query)`
- `query_ctn_model(text, query)`

Python typed dictionaries preserve RFC-specific kinds and scalar types. See
`docs/PYTHON_API.md` for the exact shape.

The `_model` helpers are additive and return native-backed Python classes:
`RfcAnnotation`, `RfcField`, `RfcBinding`, `RfcHeader`, `RfcDocument`,
`RfcDecodedCbf`, and `RfcValue`. They expose the same Rust metadata through
attributes and helpers such as `RfcDocument.binding_names()`, `RfcValue.keys()`,
`RfcValue["field"]`, `RfcValue[0]`, `RfcValue.scalar_type`, and
`RfcValue.scalar_value`.

## Known Gaps

The following RFC draft areas are not fully implemented in this repository:

- Full CSL parser/compiler
- Witness cryptography pipeline
- Compact mode schema-driven binary elision
- Full CQL pipeline operations
- Stream chunk framing protocol (`CRFM`)
- Rich tensor quantization and sparse tensor formats
- CPC RPC framework
- Database page format
- Formal migration DSL execution
- CBF index generation and random-access index lookup
- Non-zero schema hash prefix generation

## Compatibility Notes

- RFC-001 CBF is not the same wire format as v1 block-framed Surp files.
- v1 APIs remain stable under `Encoder`, `Decoder`, `Value`, and `text`.
- RFC-001 errors use the shared `surp_core::SurpError` type.
- RFC-001 Python errors are exposed as `surp.SurpRfcError`.
