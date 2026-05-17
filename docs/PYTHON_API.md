# Surp Python API

The repository now has one Python package: `surp`. It is a native PyO3 package
backed by the Rust `surp-core` implementation. The old pure-Python package
under `python/` has been removed.

## Install And Build

From a checkout:

```bash
cd surp-python
maturin develop --release
python -m pytest tests/ -v
```

Published wheels install as:

```bash
pip install surp
```

The package layout is:

```text
surp-python/
  pyproject.toml          # publishes the Python distribution named "surp"
  src/lib.rs              # PyO3 extension module: surp._surp_native
  python/surp/
    __init__.py           # public facade
    exceptions.py         # exception re-exports
    model/                # RFC-001 class schema and validation layer
    rfc001.py             # CTN / CBF / CQL helpers
    __init__.pyi          # public API stubs
    model/__init__.pyi    # RFC-001 model schema stubs
    rfc001.pyi            # RFC helper stubs
    _types.pyi            # shared TypedDict/Literal types
    py.typed              # PEP 561 marker
  tests/test_native.py    # native regression tests
```

`surp._surp_native` is private implementation detail. Application code should
use `import surp`, `from surp import rfc001`, and `from surp.model import ...`.

## v1 Binary API

```python
import surp

payload = {
    "name": "Alice",
    "age": 30,
    "active": True,
    "avatar": b"\x01\x02\x03",
}

data = surp.dumps(payload, dedup=True, sort_keys=True)
decoded = surp.loads(data)
assert decoded == payload
```

Supported Python inputs:

- `None`
- `bool`
- `int` that fits in signed 64-bit extraction in the native binding
- `float`
- `str`
- `bytes`
- `list`
- `tuple` as an array
- `dict` with string keys

`sort_keys=True` sorts dictionary keys before encoding. `dedup=True` enables
the v1 per-block string dictionary. `compression` accepts `None`, `"none"`,
`"lz4"`, `"zstd"`, or `"snappy"`; non-`none` algorithms require a native build
with the matching Rust feature.

### Functions

| Function | Description |
| --- | --- |
| `dumps(obj, *, compression=None, dedup=False, sort_keys=False) -> bytes` | Encode one Python object to v1 Surp bytes. |
| `loads(data, *, strict=True, max_depth=128) -> Any` | Decode v1 Surp bytes. One top-level value returns as the value; multiple values return as a list. |
| `dump(obj, fp, *, compression=None, dedup=False, sort_keys=False) -> None` | Encode and write bytes to a binary file-like object. |
| `load(fp, *, strict=True, max_depth=128) -> Any` | Read all bytes from a binary file-like object and decode them. |
| `encode(obj) -> bytes` | Compatibility wrapper for `dumps(obj)`. |
| `decode(data) -> Any` | Compatibility wrapper for `loads(data)`. |
| `encode_to_file(obj, path) -> None` | Path-based encode helper using default options. |
| `decode_from_file(path) -> Any` | Path-based decode helper. |
| `parse_text(text) -> Any` | Parse v1 Surp text notation into Python values. |
| `pretty_print(obj, indent=2) -> str` | Render Python values as v1 Surp text notation. |
| `to_value(obj, *, sort_keys=False) -> SurpValue` | Convert a Python value into a native-backed typed view. |
| `loads_value(data, *, strict=True, max_depth=128) -> SurpValue | list[SurpValue]` | Decode v1 bytes into native-backed typed views without changing `loads()`. |
| `parse_text_value(text) -> SurpValue` | Parse v1 text notation into a native-backed typed view. |

### File-Like API

```python
from io import BytesIO
import surp

buf = BytesIO()
surp.dump({"status": "ok"}, buf)
buf.seek(0)
assert surp.load(buf) == {"status": "ok"}
```

### Text API

```python
import surp

obj = surp.parse_text("""
{
  name: "Alice";
  avatar: b64#AQID;
  tags: ["admin", "ops"];
}
""")

text = surp.pretty_print(obj, indent=2)
assert "Alice" in text
```

## Incremental API

```python
import surp

enc = surp.Encoder(sort_keys=True)
enc.enable_dedup()
enc.set_compression("none")
enc.encode({"kind": "user", "name": "Alice"})
enc.encode({"kind": "user", "name": "Bob"})
data = enc.finish()

dec = surp.SurpDecoder(data)
values = dec.decode_all()
assert len(values) == 2
```

`Encoder.finish()` and `Encoder.finish_to_file(path)` are one-shot. After an
encoder is finished, subsequent `encode()` or `finish()` calls raise
`SurpEncodeError`. `SurpDecoder.decode_all()` is also one-shot and raises
`SurpDecodeError` if called again.

## v1 Introspection API

`loads()`, `load()`, `decode()`, and `parse_text()` still return ordinary Python
values for compatibility. For object-style introspection and IDE-visible
attributes, use the additive native-backed `SurpValue` APIs:

```python
view = surp.loads_value(surp.dumps({"name": "Alice", "tags": ["admin", "ops"]}))

assert view.kind == "object"
assert view.is_object is True
assert view.keys() == ["name", "tags"]
assert view["name"].kind == "str"
assert view["name"].value == "Alice"
assert view["tags"][1].value == "ops"
assert view.as_python() == {"name": "Alice", "tags": ["admin", "ops"]}
```

`SurpValue` exposes:

- `kind`, `value`, `is_null`, `is_scalar`, `is_array`, and `is_object`
- `__getitem__`, `get()`, `keys()`, `values()`, `items()`, `len()`, and
  membership checks for object keys
- `as_python()` to return the compatibility Python representation

## Exceptions

| Exception | Base | Raised for |
| --- | --- | --- |
| `SurpError` | `Exception` | Base class for package-specific errors |
| `SurpEncodeError` | `SurpError` | v1 encoding failures |
| `SurpDecodeError` | `SurpError` | v1 decoding failures |
| `SurpChecksumError` | `SurpDecodeError` | v1 checksum mismatch detected during decode |
| `SurpTypeError` | `SurpEncodeError` | unsupported Python value or non-string dict key |
| `SurpRfcError` | `SurpError` | RFC-001 CTN, CBF, or CQL failures |

## RFC-001 API

RFC-001 helpers live under `surp.rfc001` and call the Rust
`surp_core::rfc001` implementation.

```python
from surp import rfc001

ctn = """
User
  name = "Alice"
  tags = ["admin", "ops"]
  settings = map<str, str> ["theme" => "dark"]
"""

cbf = rfc001.compile_ctn(ctn, alignment=4)
decoded = rfc001.decode_cbf(cbf)

assert decoded["header"]["magic"] == "SURP"
assert decoded["header"]["cbf_version"] == 1
assert rfc001.query_cbf(cbf, ".tags[-1]", as_ctn=True) == ['"ops"']
```

### RFC-001 Functions

| Function | Description |
| --- | --- |
| `parse_ctn(text) -> RfcDocument` | Parse CTN into typed dictionaries. |
| `normalize_ctn(text) -> str` | Parse and render CTN using the Rust formatter. |
| `compile_ctn(text, *, with_symtab=True, alignment=0) -> bytes` | Compile CTN to CBF bytes. |
| `decode_cbf(data) -> RfcDecodedCbf` | Decode CBF into header, symbols, document, and formatted CTN. |
| `cbf_to_ctn(data) -> str` | Decode CBF and return only CTN text. |
| `query_cbf(data, query, *, as_ctn=False) -> list[Any]` | Run baseline CQL over CBF. |
| `query_ctn(text, query, *, as_ctn=False) -> list[Any]` | Compile/decode CTN and run baseline CQL. |
| `parse_ctn_model(text) -> RfcDocument` | Parse CTN into a native-backed document model. |
| `decode_cbf_model(data) -> RfcDecodedCbf` | Decode CBF into native-backed header/document/value models. |
| `query_cbf_model(data, query) -> list[RfcValue]` | Run CQL over CBF and return native-backed values. |
| `query_ctn_model(text, query) -> list[RfcValue]` | Run CQL over CTN and return native-backed values. |

Constants:

- `rfc001.CBF_MAGIC == b"SURP"`
- `rfc001.CBF_HEADER_SIZE == 32`

`compile_ctn(..., with_symtab=False)` rejects documents that contain symbol
values because the current CBF symbol encoding requires a symbol table. Plain
string/number/bool/product documents can compile without a symbol table.

### Typed RFC Values

RFC values are explicit dictionaries so Python callers do not lose RFC-specific
types.

Scalar:

```python
{"kind": "scalar", "type": "str", "value": "Alice"}
{"kind": "scalar", "type": "sym", "value": "Admin"}
{"kind": "scalar", "type": "tagged", "tag": "uid", "value": "..."}
{"kind": "scalar", "type": "bytes", "value": b"..."}
```

Product:

```python
{
    "kind": "product",
    "type_name": "User",
    "fields": [
        {"name": "name", "value": {"kind": "scalar", "type": "str", "value": "Alice"}}
    ],
}
```

Sequence:

```python
{
    "kind": "sequence",
    "elem_type": None,
    "items": [{"kind": "scalar", "type": "str", "value": "admin"}],
}
```

Association/map:

```python
{
    "kind": "association",
    "pairs": [
        [
            {"kind": "scalar", "type": "str", "value": "theme"},
            {"kind": "scalar", "type": "str", "value": "dark"},
        ]
    ],
}
```

Tensor:

```python
{
    "kind": "tensor",
    "element_type": "f32",
    "shape": [2, 2],
    "annotations": [],
    "data": {"kind": "dense_f64", "values": [1.0, 2.0, 3.0, 4.0]},
}
```

CBF decoding validates the RFC-001 CRC64 trailer before returning data.

### Native RFC Models

The dictionary-returning RFC helpers remain unchanged. The model helpers expose
the same Rust RFC-001 data model through typed Python attributes:

```python
doc = rfc001.parse_ctn_model(ctn)
assert doc.annotation_names() == ["surp", "encoding"]
assert doc.binding_names() == ["alice"]

alice = doc.binding("alice").value
assert alice.kind == "product"
assert alice.type_name == "User"
assert alice.keys() == ["id", "name", "role", "tags", "settings", "matrix"]
assert alice["name"].scalar_type == "str"
assert alice["name"].scalar_value == "Alice"
assert alice["tags"][1].scalar_value == "ops"

decoded = rfc001.decode_cbf_model(cbf)
assert decoded.header.magic == "SURP"
assert decoded.document.effective_root()["name"].scalar_value == "Alice"
```

Model classes exported from `surp.rfc001` are `RfcAnnotation`, `RfcField`,
`RfcBinding`, `RfcHeader`, `RfcDocument`, `RfcDecodedCbf`, and `RfcValue`.
Each model has `to_dict()` where a compatibility dictionary is meaningful;
`RfcDocument` and `RfcValue` also expose `to_ctn()`.

### RFC-001 Class Schemas

`surp.model` is a pure-Python validation layer built on top of `surp.rfc001`.
It is RFC-001-specific: class annotations use Surp type markers rather than
Python built-ins, and encode/decode paths use CTN/CBF through `surp.rfc001`.

```python
from surp.model import Field, SurpModel, SurpSymbolEnum
from surp.model.types import Bool, Int64, MapOf, SeqOf, Str


class Role(SurpSymbolEnum):
    ADMIN = "Admin"
    VIEWER = "Viewer"


class User(SurpModel):
    name: Str = Field(required=True)
    age: Int64 = Field(required=False, default=0)
    active: Bool = Field(required=True)
    tags: SeqOf[Str] = Field(required=False, default_factory=list)
    settings: MapOf[Str, Str] = Field(required=False, default_factory=dict)
    role: Role = Field(required=True, default=Role.VIEWER)


user = User(name="Alice", active=True, tags=["admin"], role=Role.ADMIN)
ctn = user.to_ctn()
cbf = user.to_cbf()
assert User.from_cbf(cbf) == user
assert user.query_one(".name") == "Alice"
```

Important public exports include `SurpModel`, `SurpDocument`,
`SurpSymbolEnum`, `SurpVariant`, `SurpStream`, `Field`, `FieldInfo`,
`annotation`, `registry`, `generate_model_stubs`, and `write_model_stubs`.
Type markers live under `surp.model.types`.

## Examples

Runnable examples are under `examples/`:

```bash
python examples/python_v1.py
python examples/python_rfc001.py
cargo run --manifest-path examples/rust/Cargo.toml --bin v1_roundtrip
cargo run --manifest-path examples/rust/Cargo.toml --bin rfc001_ctn_cbf_cql
```
