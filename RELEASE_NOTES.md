# Surp v1.0.1 Pre-release Notes

Surp `v1.0.1` is staged as a pre-release update for validation. Do not create a
GitHub release or tag from this entry yet.

## Release Title

Surp v1.0.1 (pre-release)

## Highlights

- Adds `surp.model`, an RFC-001-native Python class schema and validation layer
  shipped inside the existing `surp` package. Users can define `SurpModel` and
  `SurpDocument` classes with explicit RFC-001 type markers, validate eagerly,
  encode/decode through `surp.rfc001`, query with CQL, and generate typed
  constructor stubs.
- Adds JSON-like Rust introspection helpers for v1 `Value` and zero-copy
  `SurpValue<'_>` without changing the v1 wire format.
- Adds RFC-001 AST introspection helpers so products, sums, sequences, tensors,
  documents, and decoded CBF metadata can be inspected at the Rust source of
  truth.
- Adds native-backed Python model/view objects for IDE-friendly access:
  `SurpValue`, `RfcDocument`, `RfcDecodedCbf`, `RfcValue`, and related RFC
  model classes.
- Keeps existing `dumps`, `loads`, `dump`, `load`, `parse_ctn`, `decode_cbf`,
  and query dictionary/list behavior backward compatible.
- Expands `.pyi` stubs, including a private `_surp_native.pyi`, so mypy and
  Pyright can resolve the public Python API, native facade, and `surp.model`
  validation layer.
- Updates docs, examples, and tests for the new introspection APIs.
- Packages the Python README and license files directly with wheels and sdists.
- Fixes RFC-001 empty-map formatting so canonical CTN stays parseable and
  normalization remains idempotent.

## Validation

Validated locally with:

```sh
cargo fmt --all -- --check
cargo test --workspace
cargo clippy --workspace -- -D warnings
cd surp-python
../.venv/bin/maturin develop --release
../.venv/bin/python -m pytest tests/ -v
../.venv/bin/python -m mypy python/surp
../.venv/bin/pyright python/surp
```

## Notes

- This is a pre-release entry only.
- No release tag has been created.
- No GitHub release has been created.

# Surp v1.0.0 Release Notes

Surp `v1.0.0` is the first stable release of the Rust-backed Surp serialization
toolkit under the `surp` name.

## Release Title

Surp v1.0.0

## Highlights

- Stable v1 block-framed binary format with checksummed blocks and trailer
  validation.
- Rust API for owned `Value` encoding and zero-copy `SurpValue` decoding.
- Native Python package published as `surp`, backed by the Rust implementation.
- Human-readable Surp text notation for debugging and fixtures.
- CLI tooling for JSON conversion, text encode/decode, inspection, validation,
  and simple benchmarks.
- Additive RFC-001 implementation with CTN parsing/formatting, CBF
  encode/decode, CRC64 validation, symbol tables, and baseline CQL queries.
- Native Python `surp.rfc001` helpers for CTN, CBF, and CQL.
- Runnable Rust, Python, and CLI examples under `examples/`.
- Comprehensive documentation under `docs/`.

## Rust Crates

- `surp-core`
- `surp-derive`
- `surp-cli`
- `surp-io`
- `surp-compression`
- `surp-ffi`
- `surp-simd`
- `surp-python`

All Surp workspace crates are versioned as `1.0.0`.

## Python

Install the native Python package:

```sh
pip install surp
```

Example:

```py
import surp

data = surp.dumps({"name": "Alice", "active": True}, sort_keys=True)
assert surp.loads(data) == {"name": "Alice", "active": True}
```

RFC-001 helpers:

```py
from surp import rfc001

cbf = rfc001.compile_ctn('User\n  name = "Alice"')
assert rfc001.query_cbf(cbf, ".name", as_ctn=True) == ['"Alice"']
```

## CLI

```sh
surp from-json examples/data/user.json -o /tmp/user.surp
surp validate /tmp/user.surp
surp to-json /tmp/user.surp

surp rfc-compile examples/data/user.ctn -o /tmp/user.crb
surp rfc-query /tmp/user.crb ".tags[-1]"
```

## Notes

- Release tag: `v1.0.0`
- GitHub release title: `Surp v1.0.0`
- This release intentionally resets public Surp versioning to `v1.0.0`.
- Existing older tags are removed as part of the fresh version line.
