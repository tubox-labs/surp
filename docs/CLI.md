# Surp CLI

The `surp` binary is implemented by `surp-cli`. It handles v1 block-framed
Surp files and the additive RFC-001 CBF path.

## Build

```bash
cargo run -p surp-cli -- --help
cargo run -p surp-cli -- from-json examples/data/user.json -o /tmp/user.surp
```

Optional compression support is feature-gated:

```bash
cargo run -p surp-cli --features lz4 -- from-json examples/data/user.json --compression lz4 -o /tmp/user.surp
cargo run -p surp-cli --features zstd -- from-json examples/data/user.json --compression zstd -o /tmp/user.surp
cargo run -p surp-cli --features snappy -- from-json examples/data/user.json --compression snappy -o /tmp/user.surp
```

If a non-`none` compression is requested without the matching feature, the CLI
returns an error telling you which feature to enable.

## Global Options

| Option | Description |
| --- | --- |
| `--color auto|always|never` | Color mode. `auto` respects `NO_COLOR` and terminal detection. |
| `-q`, `--quiet` | Suppress non-essential informational logs. |

## v1 Commands

### `from-json`

Convert JSON to v1 Surp binary.

```bash
cargo run -p surp-cli -- from-json examples/data/user.json -o /tmp/user.surp
cargo run -p surp-cli -- from-json examples/data/user.json --dedup -o /tmp/user-dedup.surp
```

Options:

- `-o, --output <path>`: output path. Defaults to the input path with `.surp`.
- `--dedup`: enable per-block string dictionary deduplication.
- `--compression none|lz4|snappy|zstd`: block compression.

Input path `-` reads JSON from stdin. When stdin is used, `--output` is
required.

### `to-json`

Convert v1 Surp binary to JSON.

```bash
cargo run -p surp-cli -- to-json /tmp/user.surp
cargo run -p surp-cli -- to-json /tmp/user.surp --style compact -o /tmp/user.json
```

Options:

- `--style pretty|compact`: JSON output style.
- `-o, --output <path>`: output path. Defaults to stdout.

If a Surp file contains multiple top-level values, `to-json` renders a JSON
array. If it contains one top-level value, it renders that value directly.

### `encode`

Parse v1 Surp text notation and write v1 Surp binary.

```bash
cargo run -p surp-cli -- encode examples/data/user.surp.txt -o /tmp/user-from-text.surp
```

Options:

- `-o, --output <path>`: output path. Defaults to the input path with `.surp`.
- `--dedup`: enable per-block string dictionary deduplication.
- `--compression none|lz4|snappy|zstd`: block compression.

### `decode` and `pretty`

Decode v1 Surp binary to Surp text notation. `decode` delegates to the same
implementation as `pretty`.

```bash
cargo run -p surp-cli -- pretty /tmp/user.surp
cargo run -p surp-cli -- decode /tmp/user.surp --indent 4 -o /tmp/user.surp.txt
```

Options:

- `-i, --indent <n>`: indentation width. Default is `2`.
- `-o, --output <path>`: output path. Defaults to stdout.

### `inspect`

Inspect block layout, compression type, payload length, and checksum state.

```bash
cargo run -p surp-cli -- inspect /tmp/user.surp
```

For compressed data blocks, `inspect` reports payload checksum as `n/a`
because checksum validation requires decompression. Run `validate` without
`--checksums-only` for full decode validation.

### `validate`

Validate trailer checksums, block checksums, and value decoding.

```bash
cargo run -p surp-cli -- validate /tmp/user.surp
cargo run -p surp-cli -- validate /tmp/user.surp --strict
cargo run -p surp-cli -- validate /tmp/user.surp --checksums-only
```

Options:

- `--checksums-only`: validate framing/checksums/trailer and skip value decode.
- `--strict`: decode using `Limits::strict()`.

`--checksums-only` rejects compressed blocks because validating compressed
payload checksums requires decoding the block.

### `bench`

Run a simple encode/decode throughput loop from a JSON input.

```bash
cargo run -p surp-cli --release -- bench examples/data/user.json -n 1000 --warmup 100
```

Options:

- `-n, --iterations <n>`: measured iterations. Default is `1000`.
- `--warmup <n>`: warmup iterations. Default is `100`.
- `--dedup`: enable string dictionary deduplication.
- `--compression none|lz4|snappy|zstd`: block compression.

The benchmark prints JSON size, Surp size, total/average encode and decode
time, and MB/s throughput.

## RFC-001 Commands

### `rfc-compile`

Compile RFC-001 CTN to RFC-001 CBF.

```bash
cargo run -p surp-cli -- rfc-compile examples/data/user.ctn -o /tmp/user.crb
cargo run -p surp-cli -- rfc-compile examples/data/user.ctn --alignment 4 -o /tmp/user-aligned.crb
```

Options:

- `-o, --output <path>`: output path. Defaults to the input path with `.crb`.
- `--no-symtab`: disable symbol-table generation.
- `--alignment <u8>`: write an alignment hint into the CBF header.

`--no-symtab` rejects CTN documents that contain symbol values such as
`'Admin`, because current symbol segment encoding requires a symbol table.

### `rfc-inspect`

Inspect RFC-001 CBF header and symbol metadata.

```bash
cargo run -p surp-cli -- rfc-inspect /tmp/user.crb
cargo run -p surp-cli -- rfc-inspect /tmp/user.crb --ctn
```

Options:

- `--ctn`: decode and print the CTN representation.
- `-o, --output <path>`: CTN output path when `--ctn` is used.

### `rfc-query`

Run baseline CQL over RFC-001 CBF.

```bash
cargo run -p surp-cli -- rfc-query /tmp/user.crb ".name"
cargo run -p surp-cli -- rfc-query /tmp/user.crb ".tags[-1]"
cargo run -p surp-cli -- rfc-query /tmp/user.crb ".settings[\"theme\"]"
```

Output behavior:

- no results: `null`
- one result: that CTN value
- multiple results: CTN sequence

Implemented CQL selectors are `.field`, `[]`, `[index]`, negative indexes,
`['symbol]`, and `["string"]`.

## End-To-End Smoke

```bash
cargo run -p surp-cli -- from-json examples/data/user.json -o /tmp/user.surp
cargo run -p surp-cli -- validate /tmp/user.surp
cargo run -p surp-cli -- to-json /tmp/user.surp --style compact

cargo run -p surp-cli -- encode examples/data/user.surp.txt -o /tmp/user-text.surp
cargo run -p surp-cli -- pretty /tmp/user-text.surp

cargo run -p surp-cli -- rfc-compile examples/data/user.ctn -o /tmp/user.crb
cargo run -p surp-cli -- rfc-inspect /tmp/user.crb --ctn
cargo run -p surp-cli -- rfc-query /tmp/user.crb ".tags[]"
```
