---
name: surp-rfc001
description: Work on Surp RFC-001 CTN, CBF, and baseline CQL behavior. Use for tasks involving surp_core::rfc001, CTN parsing or formatting, CBF encode/decode/header/symbol-table/CRC64 segments, CQL path queries, RFC-001 CLI commands rfc-compile/rfc-inspect/rfc-query, Python surp.rfc001 bindings, or RFC-001 examples and tests.
---

# Surp RFC-001

## Source Authority

RFC-001 is implemented in parallel to the stable v1 codec. Read source before docs:

- `surp-core/src/rfc001/ast.rs`: RFC data model.
- `surp-core/src/rfc001/ctn.rs`: indentation-oriented CTN parser and formatter.
- `surp-core/src/rfc001/cbf.rs`: CBF header, segment encoder/decoder, symbol table, CRC64 trailer.
- `surp-core/src/rfc001/cql.rs`: baseline structural path query engine.
- `surp-core/src/rfc001/mod.rs`: public exports.
- `surp-cli/src/main.rs`: `rfc-compile`, `rfc-inspect`, `rfc-query`.
- `surp-python/src/lib.rs` and `surp-python/python/surp/rfc001.py`: Python bindings.
- `surp-python/tests/test_native.py`: current RFC-001 regression coverage.

## Implemented Capabilities

Document only behavior that exists in source:

- CTN documents support annotations, `use`, `let`, explicit root values, and root fallback to the last binding.
- CTN comments are `--` line comments, `--[[...]]` block comments, and `--!` directive/comment lines.
- CTN indentation uses spaces; tabs are rejected.
- CTN values include products, anonymous `struct`, sums, sequences, maps, binding references, `ref` wrappers, tensors, streams with annotation-only bodies, symbols, tagged quoted literals, bytes, and numeric suffixes.
- CBF files use magic `SURP`, 32-byte header, version 1 fields, optional symbol table, root offset, and CRC64-ECMA trailer.
- CBF segments cover primitives, strings, bytes, symbols, product, sum, sequence, map, reference, tensor, stream, and opaque/tagged values.
- Encoding resolves CTN binding references and rejects cycles.
- `with_symtab=False` rejects symbol values.
- CQL supports `.field`, `[]`, `[index]`, negative indexes, `['symbol]`, and `["string"]`.

Known gaps are real product boundaries: full CSL, witness cryptography, full CQL pipelines, stream chunk framing, rich tensor formats, CPC RPC, DB pages, migration DSL, CBF index generation, and non-zero schema hash prefix are not implemented.

## Workflow

1. Decide whether the task belongs to RFC-001 or v1. RFC-001 CBF is not the v1 block-framed Surp file format.
2. For syntax changes, update parser and formatter together; the formatter is canonical, not whitespace-preserving.
3. For CBF changes, keep header/trailer validation and segment bounds checks paired between encode and decode.
4. For symbol behavior, check both `with_symtab=true` and `with_symtab=false`.
5. For query changes, verify missing selectors still return an empty list unless the syntax itself is invalid.
6. Keep Python and CLI RFC behavior aligned with Rust exports.

## Inputs And Outputs

- CTN input is UTF-8 text.
- CBF output is bytes with `CBF_MAGIC == b"SURP"` and `CBF_HEADER_SIZE == 32`.
- CLI `rfc-query` prints `null`, one CTN value, or a CTN sequence.
- Python `surp.rfc001` returns typed dictionaries for RFC values unless `as_ctn=True`, which returns CTN strings.

## Validation

Use focused checks:

```sh
cargo test -p surp-core rfc001
cargo run -p surp-cli -- rfc-compile examples/data/user.ctn -o /tmp/user.crb
cargo run -p surp-cli -- rfc-inspect /tmp/user.crb --ctn
cargo run -p surp-cli -- rfc-query /tmp/user.crb ".tags[-1]"
```

For Python bindings:

```sh
cd surp-python
maturin develop --release
python -m pytest tests/test_native.py -v
```
