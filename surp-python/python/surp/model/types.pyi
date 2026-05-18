from __future__ import annotations

from enum import Enum
from typing import Any, Generic, TypeAlias, TypeVar

_T = TypeVar("_T")
_K = TypeVar("_K")
_V = TypeVar("_V")

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
    def __class_getitem__(cls, args: Any) -> Any: ...

def normalize_annotation(annotation: Any) -> Any: ...
def describe_type(annotation: Any) -> str: ...

Str: TypeAlias = str
Bool: TypeAlias = bool
Null: TypeAlias = None
Unit: TypeAlias = None
Int8: TypeAlias = int
Int16: TypeAlias = int
Int32: TypeAlias = int
Int64: TypeAlias = int
UInt8: TypeAlias = int
UInt16: TypeAlias = int
UInt32: TypeAlias = int
UInt64: TypeAlias = int
VarInt32: TypeAlias = int
VarInt64: TypeAlias = int
VarUInt32: TypeAlias = int
VarUInt64: TypeAlias = int
F16: TypeAlias = float
BF16: TypeAlias = float
F32: TypeAlias = float
F64: TypeAlias = float
Dec32: TypeAlias = str | int | float
Dec64: TypeAlias = str | int | float
Dec128: TypeAlias = str | int | float
Bytes: TypeAlias = bytes
Symbol: TypeAlias = str

class Tagged:
    def __class_getitem__(cls, args: Any) -> Any: ...

class SeqOf(list[_T], Generic[_T]):
    def __class_getitem__(cls, args: Any) -> Any: ...

class MapOf(dict[_K, _V], Generic[_K, _V]):
    def __class_getitem__(cls, args: Any) -> Any: ...

class RefOf(Generic[_T]):
    def __class_getitem__(cls, args: Any) -> Any: ...

class Nullable(Generic[_T]):
    def __class_getitem__(cls, args: Any) -> Any: ...

class OneOf:
    def __class_getitem__(cls, args: Any) -> Any: ...

class Variant:
    def __class_getitem__(cls, args: Any) -> Any: ...

class SumOf:
    def __class_getitem__(cls, args: Any) -> Any: ...

class Tensor:
    def __class_getitem__(cls, args: Any) -> Any: ...

class StreamOf(Generic[_T]):
    def __class_getitem__(cls, args: Any) -> Any: ...
