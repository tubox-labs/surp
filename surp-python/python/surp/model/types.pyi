from __future__ import annotations

from enum import Enum
from typing import Any

class _TypeSpec:
    kind: str
    def describe(self) -> str: ...

class _ForwardRef(_TypeSpec):
    name: str
    def __init__(self, name: str) -> None: ...

class _ScalarSentinel(_TypeSpec):
    rfc_name: str
    py_name: str
    min_value: int | None
    max_value: int | None
    def __init__(
        self,
        rfc_name: str,
        *,
        py_name: str | None = None,
        min_value: int | None = None,
        max_value: int | None = None,
    ) -> None: ...

class _TaggedSpec(_TypeSpec):
    tag: str
    inner: Any
    def __init__(self, tag: str, inner: Any) -> None: ...

class _SeqSpec(_TypeSpec):
    elem: Any
    def __init__(self, elem: Any) -> None: ...

class _MapSpec(_TypeSpec):
    key: Any
    value: Any
    def __init__(self, key: Any, value: Any) -> None: ...

class _RefSpec(_TypeSpec):
    inner: Any
    def __init__(self, inner: Any) -> None: ...

class _NullableSpec(_TypeSpec):
    inner: Any
    def __init__(self, inner: Any) -> None: ...

class _OneOfSpec(_TypeSpec):
    options: tuple[Any, ...]
    def __init__(self, options: tuple[Any, ...]) -> None: ...

class _VariantSpec(_TypeSpec):
    name: str
    payload: tuple[Any, ...]
    payload_kind: str
    def __init__(self, name: str, payload: tuple[Any, ...]) -> None: ...

class _SumSpec(_TypeSpec):
    variants: tuple[Any, ...]
    def __init__(self, variants: tuple[Any, ...]) -> None: ...

class _TensorSpec(_TypeSpec):
    dtype: TensorDType
    shape: tuple[int | None, ...]
    def __init__(self, dtype: TensorDType, shape: tuple[int | None, ...]) -> None: ...

class _StreamSpec(_TypeSpec):
    item: Any
    def __init__(self, item: Any) -> None: ...

class TensorDType(Enum):
    F16 = "f16"
    BF16 = "bf16"
    F32 = "f32"
    F64 = "f64"
    I8 = "i8"
    I16 = "i16"
    I32 = "i32"
    I64 = "i64"
    U8 = "u8"
    U16 = "u16"
    U32 = "u32"
    U64 = "u64"

class _Alias:
    def __getitem__(self, args: Any) -> Any: ...

def normalize_annotation(annotation: Any) -> Any: ...
def describe_type(annotation: Any) -> str: ...

Str: _ScalarSentinel
Bool: _ScalarSentinel
Null: _ScalarSentinel
Unit: _ScalarSentinel
Int8: _ScalarSentinel
Int16: _ScalarSentinel
Int32: _ScalarSentinel
Int64: _ScalarSentinel
UInt8: _ScalarSentinel
UInt16: _ScalarSentinel
UInt32: _ScalarSentinel
UInt64: _ScalarSentinel
VarInt32: _ScalarSentinel
VarInt64: _ScalarSentinel
VarUInt32: _ScalarSentinel
VarUInt64: _ScalarSentinel
F16: _ScalarSentinel
BF16: _ScalarSentinel
F32: _ScalarSentinel
F64: _ScalarSentinel
Dec32: _ScalarSentinel
Dec64: _ScalarSentinel
Dec128: _ScalarSentinel
Bytes: _ScalarSentinel
Symbol: _ScalarSentinel
Tagged: _Alias
SeqOf: _Alias
MapOf: _Alias
RefOf: _Alias
Nullable: _Alias
OneOf: _Alias
Variant: _Alias
SumOf: _Alias
Tensor: _Alias
StreamOf: _Alias
