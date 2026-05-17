# Examples

The `examples/` directory contains runnable examples and fixture data built
against the APIs currently implemented in this repository.

## Layout

```text
examples/
  data/
    user.json          # JSON fixture for CLI from-json/to-json
    user.surp.txt      # v1 Surp text fixture for CLI encode/decode
    user.ctn           # RFC-001 CTN fixture for CLI/Python/Rust examples
  python_v1.py         # Python v1 binary/text/file APIs
  python_rfc001.py     # Python RFC-001 CTN/CBF/CQL APIs
  rust/
    Cargo.toml
    src/bin/
      v1_roundtrip.rs
      text_format.rs
      derive_struct.rs
      rfc001_ctn_cbf_cql.rs
```

## Python

Build the native extension before running the Python examples:

```bash
cd surp-python
maturin develop --release
cd ..
```

Run v1 binary/text APIs:

```bash
python examples/python_v1.py
```

Run RFC-001 CTN/CBF/CQL APIs:

```bash
python examples/python_rfc001.py
```

## Rust

The Rust examples are a workspace package with path dependencies on the
library crates.

```bash
cargo run --manifest-path examples/rust/Cargo.toml --bin v1_roundtrip
cargo run --manifest-path examples/rust/Cargo.toml --bin text_format
cargo run --manifest-path examples/rust/Cargo.toml --bin derive_struct
cargo run --manifest-path examples/rust/Cargo.toml --bin rfc001_ctn_cbf_cql
```

## CLI

v1 JSON to Surp and back:

```bash
cargo run -p surp-cli -- from-json examples/data/user.json -o /tmp/user.surp
cargo run -p surp-cli -- validate /tmp/user.surp
cargo run -p surp-cli -- to-json /tmp/user.surp --style pretty
```

v1 text to Surp and back:

```bash
cargo run -p surp-cli -- encode examples/data/user.surp.txt -o /tmp/user-text.surp
cargo run -p surp-cli -- pretty /tmp/user-text.surp
```

RFC-001 CTN to CBF and CQL:

```bash
cargo run -p surp-cli -- rfc-compile examples/data/user.ctn -o /tmp/user.crb
cargo run -p surp-cli -- rfc-inspect /tmp/user.crb --ctn
cargo run -p surp-cli -- rfc-query /tmp/user.crb ".name"
cargo run -p surp-cli -- rfc-query /tmp/user.crb ".tags[-1]"
```

## What Each Example Demonstrates

| Example | Demonstrates |
| --- | --- |
| `python_v1.py` | `dumps`, `loads`, file-like APIs, text parse/pretty print, incremental encoder/decoder |
| `python_rfc001.py` | `parse_ctn`, `normalize_ctn`, `compile_ctn`, `decode_cbf`, `query_cbf`, `query_ctn` |
| `v1_roundtrip.rs` | `Value`, `Encoder`, `Decoder`, dedup, owned decode |
| `text_format.rs` | `surp_core::text::parse` and `pretty_print` |
| `derive_struct.rs` | `#[derive(Surp)]`, `#[derive(SurpSchema)]`, `SurpBytes`, binary roundtrip |
| `rfc001_ctn_cbf_cql.rs` | CTN parsing, CBF encode/decode, baseline CQL |

## Verification

Use these commands after changing examples or public APIs:

```bash
cargo run --manifest-path examples/rust/Cargo.toml --bin v1_roundtrip
cargo run --manifest-path examples/rust/Cargo.toml --bin text_format
cargo run --manifest-path examples/rust/Cargo.toml --bin derive_struct
cargo run --manifest-path examples/rust/Cargo.toml --bin rfc001_ctn_cbf_cql
python examples/python_v1.py
python examples/python_rfc001.py
```
