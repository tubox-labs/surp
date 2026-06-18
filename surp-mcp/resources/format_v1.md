# Surp v1 Format

Surp v1 is the stable block-framed `.surp` binary format backed by `surp-core`.

- Values: null, bool, unsigned/signed integers, f64, UTF-8 strings, raw bytes, arrays, and ordered string-keyed objects.
- Blocks carry type, length, compression marker, and XXH64 payload checksum.
- A trailer block stores the file checksum over preceding bytes.
- Optional per-block string deduplication emits string dictionary blocks.
- Compression support is compile-time feature gated for lz4, snappy, and zstd.
- Surp text notation maps to v1 values and is distinct from JSON.

Use `surp_v1_inspect` before `surp_v1_decode` when diagnosing corrupt files. Use strict limits for untrusted inputs.
