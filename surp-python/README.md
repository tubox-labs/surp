# Surp Python

Rust-backed Python bindings for Surp, a compact binary serialization format
with a human-readable text notation and an additive RFC-001 CTN/CBF/CQL path.

The Python distribution is named `surp` and is backed by the PyO3 extension
module `surp._surp_native`.

## Install

```sh
pip install surp
```

From a checkout:

```sh
cd surp-python
maturin develop --release
python -m pytest tests/ -v
```

## v1 API

```python
import surp

payload = {
    "name": "Alice",
    "age": 30,
    "active": True,
    "avatar": b"\x01\x02\x03",
}

data = surp.dumps(payload, dedup=True, sort_keys=True)
assert surp.loads(data) == payload
```

Supported inputs are `None`, `bool`, `int`, `float`, `str`, `bytes`, `list`,
`tuple`, and dictionaries with string keys. Tuples decode as lists.

Public helpers:

- `dumps`, `loads`, `dump`, `load`
- `encode`, `decode`, `encode_to_file`, `decode_from_file`
- `parse_text`, `pretty_print`
- `to_value`, `loads_value`, `parse_text_value`, `SurpValue`
- `Encoder`, `SurpDecoder`

Use `loads_value()` or `parse_text_value()` when you want native-backed
attribute access instead of plain Python containers:

```python
view = surp.loads_value(data)
assert view["name"].value == "Alice"
assert view["tags"][0].value == "admin"
```

## RFC-001 Helpers

```python
from surp import rfc001

cbf = rfc001.compile_ctn('User\n  name = "Alice"', alignment=4)
decoded = rfc001.decode_cbf(cbf)

assert decoded["header"]["magic"] == "SURP"
assert rfc001.query_cbf(cbf, ".name", as_ctn=True) == ['"Alice"']
```

`surp.rfc001` exposes CTN parsing/normalization, CTN-to-CBF compilation, CBF
decoding, CBF-to-CTN formatting, and baseline CQL path queries.

For IDE-friendly RFC access, use the model helpers:

```python
doc = rfc001.parse_ctn_model('User\n  name = "Alice"')
user = doc.effective_root()
assert user["name"].scalar_value == "Alice"
```

## Typing

The wheel includes `.pyi` stubs and `py.typed` for type checkers.

## License

Licensed under either MIT or Apache-2.0, at your option.
