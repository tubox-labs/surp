---
name: surp-python-package
description: Maintain the Rust-backed Surp Python distribution. Use for tasks touching surp-python PyO3 bindings, pyproject/maturin packaging, Python facade modules, type stubs, py.typed, exceptions, surp.rfc001 wrappers, Python tests, wheel/sdist publishing, or cross-language Rust/Python compatibility.
---

# Surp Python Package

## Source Authority

The Python package is native and Rust-backed. Check these files before editing:

- `surp-python/pyproject.toml`: Python package metadata and maturin config.
- `surp-python/Cargo.toml`: PyO3 extension crate named `_surp_native`.
- `surp-python/build.rs`: non-maturin Python link handling.
- `surp-python/src/lib.rs`: native API, conversion, exceptions, classes, RFC wrappers.
- `surp-python/python/surp/__init__.py`: public facade.
- `surp-python/python/surp/exceptions.py`: exception re-exports.
- `surp-python/python/surp/rfc001.py`: RFC-001 Python helpers.
- `surp-python/python/surp/*.pyi` and `py.typed`: public typing surface.
- `surp-python/tests/test_native.py`: package behavior tests.
- `.github/workflows/python-publish.yml`: wheel and sdist build matrix.

## Public API

Keep the facade stable unless the task asks for a breaking change:

- `dumps`, `loads`, `dump`, `load`.
- `encode`, `decode`, `encode_to_file`, `decode_from_file`.
- `parse_text`, `pretty_print`.
- `Encoder` and `SurpDecoder`.
- Exceptions: `SurpError`, `SurpEncodeError`, `SurpDecodeError`, `SurpChecksumError`, `SurpTypeError`, `SurpRfcError`.
- `surp.rfc001`: `parse_ctn`, `normalize_ctn`, `compile_ctn`, `decode_cbf`, `cbf_to_ctn`, `query_cbf`, `query_ctn`, `CBF_MAGIC`, `CBF_HEADER_SIZE`.

## Conversion Constraints

- Supported Python inputs are `None`, `bool`, signed-extractable `int`, `float`, `str`, `bytes`, `list`, `tuple`, and `dict` with string keys.
- Check bool before int because Python bool is an int subclass.
- Non-string dict keys raise `SurpTypeError`.
- `tuple` decodes back as `list`.
- `sort_keys=True` sorts dict keys before encoding.
- `dedup=True` enables the v1 string dictionary.
- Compression names are `None`, `"none"`, `"lz4"`, `"zstd"`, `"snappy"`.
- `Encoder.finish()` and `finish_to_file()` are one-shot.
- `SurpDecoder.decode_all()` is one-shot and returns a list of top-level values.

## Packaging Rules

- The distribution name is `surp`; the native module is `surp._surp_native`.
- Keep `python-source = "python"` and `module-name = "surp._surp_native"` in sync with imports.
- Include `.py`, `.pyi`, and `py.typed` in both wheel and sdist.
- Keep type stubs aligned with `src/lib.rs` and facade modules whenever public signatures change.
- Keep README/package metadata suitable for PyPI; the publish workflow currently builds wheels for Linux, macOS, Windows, and CPython 3.9-3.14.
- Do not import `_surp_native` as public API outside facade modules.

## Validation

Use the native package test path:

```sh
cd surp-python
maturin develop --release
python -m pytest tests/ -v
```

For cross-language changes:

```sh
cargo build -p surp-cli --release
cargo run -p surp-cli --release -- from-json ../examples/data/user.json -o /tmp/user.surp
python -c "import surp; print(surp.decode(open('/tmp/user.surp','rb').read()))"
```
