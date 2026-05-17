# Surp v1.0.1 Release Notes

Surp `v1.0.1` is a patch release for the Rust-backed Surp serialization
toolkit. It promotes the staged Python model and introspection APIs, aligns Rust
workspace metadata with the Python package version, and ships a full README plus
committed benchmark charts.

## Release Title

Surp v1.0.1

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
- Rewrites the root README as a complete installation, API, RFC-001, Python
  model, benchmark, and local development guide.
- Adds Protocol Buffers to the Rust benchmark harness and generates release
  SVG charts for serialized size, encode throughput, and decode throughput.
- Commits full benchmark artifacts under `docs/assets/bench/v1.0.1`.
- Updates Rust workspace package and dependency metadata to `1.0.1`.

## Benchmark Artifacts

The `v1.0.1` release benchmark was run locally with:

```sh
cargo run -p surp-bench --release -- --mode full --output docs/assets/bench/v1.0.1 --version v1.0.1
```

Committed artifacts:

- `docs/assets/bench/v1.0.1/raw.json`
- `docs/assets/bench/v1.0.1/summary.csv`
- `docs/assets/bench/v1.0.1/regression_report.md`
- `docs/assets/bench/v1.0.1/size_comparison.md`
- `docs/assets/bench/v1.0.1/system_info.json`
- `docs/assets/bench/v1.0.1/charts/serialized-size.svg`
- `docs/assets/bench/v1.0.1/charts/encode-throughput.svg`
- `docs/assets/bench/v1.0.1/charts/decode-throughput.svg`

Size summary:

| Dataset | Surp | Surp+Dedup | JSON | MsgPack | Protobuf | Surp/JSON |
| --- | ---: | ---: | ---: | ---: | ---: | ---: |
| small_objects | 8.6 MB | 12.0 MB | 10.5 MB | 7.8 MB | 11.4 MB | 0.82x |
| string_heavy | 1.0 MB | 668.3 KB | 1.1 MB | 925.8 KB | 1.2 MB | 0.96x |
| nested_deep | 1.0 MB | 1.5 MB | 1.2 MB | 835.1 KB | 1.4 MB | 0.87x |
| binary_blobs | 6.4 MB | 6.4 MB | 8.5 MB | 8.5 MB | 6.4 MB | 0.75x |
| mixed_api_events | 1.9 MB | 2.8 MB | 2.0 MB | 1.7 MB | 2.2 MB | 0.92x |
| numeric_heavy | 3.7 MB | 3.7 MB | 6.0 MB | 3.5 MB | 5.0 MB | 0.63x |

## Validation

Validated locally with:

```sh
cargo fmt --all -- --check
cargo test --workspace --all-features
cargo clippy --workspace --all-features -- -D warnings
cargo check -p surp-bench
cargo run -p surp-bench --release -- --mode full --output docs/assets/bench/v1.0.1 --version v1.0.1
cd surp-python
../.venv/bin/maturin develop --release
../.venv/bin/python -m pytest tests/ -v
../.venv/bin/python -m mypy python/surp
../.venv/bin/pyright python/surp
```

Additional validation expected before publishing distribution artifacts:

```sh
cargo +nightly fuzz run fuzz_decode -- -max_total_time=30 -max_len=4096
cargo +nightly fuzz run fuzz_roundtrip -- -max_total_time=30 -max_len=4096
```

## Notes

- Release tag: `v1.0.1`
- GitHub release title: `Surp v1.0.1`
- The release notes used by `gh release create` live at
  `.github/releases/v1.0.1.md`.

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
