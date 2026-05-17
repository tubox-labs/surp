from __future__ import annotations

from enum import Enum
from typing import Any


class _TypeSpec:
    kind: str

    def describe(self) -> str:
        return self.kind


class _ForwardRef(_TypeSpec):
    kind = "forward"

    def __init__(self, name: str) -> None:
        self.name = name

    def describe(self) -> str:
        return self.name

    def __repr__(self) -> str:
        return f"ForwardRef({self.name!r})"


class _ScalarSentinel(_TypeSpec):
    kind = "scalar"

    def __init__(
        self,
        rfc_name: str,
        *,
        py_name: str | None = None,
        min_value: int | None = None,
        max_value: int | None = None,
    ) -> None:
        self.rfc_name = rfc_name
        self.py_name = py_name or rfc_name
        self.min_value = min_value
        self.max_value = max_value

    def describe(self) -> str:
        return self.rfc_name

    def __repr__(self) -> str:
        return self.rfc_name


class _TaggedSpec(_TypeSpec):
    kind = "tagged"

    def __init__(self, tag: str, inner: Any) -> None:
        self.tag = tag
        self.inner = normalize_annotation(inner)

    def describe(self) -> str:
        return f"{self.tag}<{describe_type(self.inner)}>"

    def __repr__(self) -> str:
        return f"Tagged[{self.tag!r}, {self.inner!r}]"


class _SeqSpec(_TypeSpec):
    kind = "sequence"

    def __init__(self, elem: Any) -> None:
        self.elem = normalize_annotation(elem)

    def describe(self) -> str:
        return f"seq<{describe_type(self.elem)}>"

    def __repr__(self) -> str:
        return f"SeqOf[{self.elem!r}]"


class _MapSpec(_TypeSpec):
    kind = "map"

    def __init__(self, key: Any, value: Any) -> None:
        self.key = normalize_annotation(key)
        self.value = normalize_annotation(value)

    def describe(self) -> str:
        return f"map<{describe_type(self.key)}, {describe_type(self.value)}>"

    def __repr__(self) -> str:
        return f"MapOf[{self.key!r}, {self.value!r}]"


class _RefSpec(_TypeSpec):
    kind = "reference"

    def __init__(self, inner: Any) -> None:
        self.inner = normalize_annotation(inner)

    def describe(self) -> str:
        return f"ref<{describe_type(self.inner)}>"

    def __repr__(self) -> str:
        return f"RefOf[{self.inner!r}]"


class _NullableSpec(_TypeSpec):
    kind = "nullable"

    def __init__(self, inner: Any) -> None:
        self.inner = normalize_annotation(inner)

    def describe(self) -> str:
        return f"nullable<{describe_type(self.inner)}>"

    def __repr__(self) -> str:
        return f"Nullable[{self.inner!r}]"


class _OneOfSpec(_TypeSpec):
    kind = "oneof"

    def __init__(self, options: tuple[Any, ...]) -> None:
        self.options = tuple(normalize_annotation(option) for option in options)

    def describe(self) -> str:
        return "oneof<" + ", ".join(describe_type(option) for option in self.options) + ">"

    def __repr__(self) -> str:
        return f"OneOf[{', '.join(repr(option) for option in self.options)}]"


class _VariantSpec(_TypeSpec):
    kind = "variant"

    def __init__(self, name: str, payload: tuple[Any, ...]) -> None:
        self.name = name
        self.payload = tuple(payload)

    @property
    def payload_kind(self) -> str:
        if not self.payload:
            return "unit"
        if len(self.payload) == 1 and not _is_named_payload_item(self.payload[0]):
            return "tuple"
        return "struct"

    def describe(self) -> str:
        return f"variant<{self.name}>"

    def __repr__(self) -> str:
        return f"Variant[{self.name!r}]"


class _SumSpec(_TypeSpec):
    kind = "sum"

    def __init__(self, variants: tuple[Any, ...]) -> None:
        self.variants = tuple(variants)

    def describe(self) -> str:
        return "sum<" + ", ".join(v.name for v in self.variants) + ">"

    def __repr__(self) -> str:
        return f"SumOf[{', '.join(repr(variant) for variant in self.variants)}]"


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


class _TensorSpec(_TypeSpec):
    kind = "tensor"

    def __init__(self, dtype: TensorDType, shape: tuple[int | None, ...]) -> None:
        self.dtype = dtype
        self.shape = shape

    def describe(self) -> str:
        dims = ", ".join("_" if dim is None else str(dim) for dim in self.shape)
        return f"tensor<{self.dtype.value}>[{dims}]"

    def __repr__(self) -> str:
        return f"Tensor[{self.dtype!r}, {self.shape!r}]"


class _StreamSpec(_TypeSpec):
    kind = "stream"

    def __init__(self, item: Any) -> None:
        self.item = normalize_annotation(item)

    def describe(self) -> str:
        return f"stream<{describe_type(self.item)}>"

    def __repr__(self) -> str:
        return f"StreamOf[{self.item!r}]"


class _Alias:
    def __init__(self, name: str) -> None:
        self.name = name

    def __getitem__(self, args: Any) -> Any:
        if not isinstance(args, tuple):
            args = (args,)
        if self.name == "Tagged":
            if len(args) != 2 or not isinstance(args[0], str):
                raise TypeError("Tagged[...] expects a tag string and an inner scalar type")
            return _TaggedSpec(args[0], args[1])
        if self.name == "SeqOf":
            if len(args) != 1:
                raise TypeError("SeqOf[...] expects one element type")
            return _SeqSpec(args[0])
        if self.name == "MapOf":
            if len(args) != 2:
                raise TypeError("MapOf[...] expects key and value types")
            return _MapSpec(args[0], args[1])
        if self.name == "RefOf":
            if len(args) != 1:
                raise TypeError("RefOf[...] expects one referenced type")
            return _RefSpec(args[0])
        if self.name == "Nullable":
            if len(args) != 1:
                raise TypeError("Nullable[...] expects one inner type")
            return _NullableSpec(args[0])
        if self.name == "OneOf":
            if len(args) < 2:
                raise TypeError("OneOf[...] expects at least two options")
            return _OneOfSpec(args)
        if self.name == "Variant":
            if not args or not isinstance(args[0], str):
                raise TypeError("Variant[...] expects a variant name string")
            return _VariantSpec(args[0], tuple(args[1:]))
        if self.name == "SumOf":
            if not args:
                raise TypeError("SumOf[...] expects at least one Variant")
            return _SumSpec(args)
        if self.name == "Tensor":
            if len(args) != 2 or not isinstance(args[0], TensorDType):
                raise TypeError("Tensor[...] expects TensorDType and shape tuple")
            shape_arg = args[1]
            if not isinstance(shape_arg, tuple):
                raise TypeError("Tensor shape must be a tuple")
            shape: list[int | None] = []
            for dim in shape_arg:
                if dim is not None and (not isinstance(dim, int) or dim < 0):
                    raise TypeError("Tensor shape dimensions must be non-negative int or None")
                shape.append(dim)
            return _TensorSpec(args[0], tuple(shape))
        if self.name == "StreamOf":
            if len(args) != 1:
                raise TypeError("StreamOf[...] expects one item type")
            return _StreamSpec(args[0])
        raise TypeError(f"unknown alias {self.name}")

    def __repr__(self) -> str:
        return self.name


def _is_named_payload_item(item: Any) -> bool:
    return isinstance(item, tuple) and len(item) == 2 and isinstance(item[0], str)


def normalize_annotation(annotation: Any) -> Any:
    if isinstance(annotation, str):
        return _ForwardRef(annotation)
    return annotation


def describe_type(annotation: Any) -> str:
    if hasattr(annotation, "describe"):
        return annotation.describe()
    if isinstance(annotation, type):
        return getattr(annotation, "__rfc_type__", annotation.__name__)
    return repr(annotation)


Str = _ScalarSentinel("str", py_name="str")
Bool = _ScalarSentinel("bool", py_name="bool")
Null = _ScalarSentinel("null", py_name="None")
Unit = _ScalarSentinel("unit", py_name="()")
Int8 = _ScalarSentinel("i8", min_value=-(2**7), max_value=2**7 - 1)
Int16 = _ScalarSentinel("i16", min_value=-(2**15), max_value=2**15 - 1)
Int32 = _ScalarSentinel("i32", min_value=-(2**31), max_value=2**31 - 1)
Int64 = _ScalarSentinel("i64", min_value=-(2**63), max_value=2**63 - 1)
UInt8 = _ScalarSentinel("u8", min_value=0, max_value=2**8 - 1)
UInt16 = _ScalarSentinel("u16", min_value=0, max_value=2**16 - 1)
UInt32 = _ScalarSentinel("u32", min_value=0, max_value=2**32 - 1)
UInt64 = _ScalarSentinel("u64", min_value=0, max_value=2**64 - 1)
VarInt32 = _ScalarSentinel("vi32", min_value=-(2**31), max_value=2**31 - 1)
VarInt64 = _ScalarSentinel("vi64", min_value=-(2**63), max_value=2**63 - 1)
VarUInt32 = _ScalarSentinel("vu32", min_value=0, max_value=2**32 - 1)
VarUInt64 = _ScalarSentinel("vu64", min_value=0, max_value=2**64 - 1)
F16 = _ScalarSentinel("f16")
BF16 = _ScalarSentinel("bf16")
F32 = _ScalarSentinel("f32")
F64 = _ScalarSentinel("f64")
Dec32 = _ScalarSentinel("dec32")
Dec64 = _ScalarSentinel("dec64")
Dec128 = _ScalarSentinel("dec128")
Bytes = _ScalarSentinel("bytes", py_name="bytes")
Symbol = _ScalarSentinel("sym", py_name="symbol")

Tagged = _Alias("Tagged")
SeqOf = _Alias("SeqOf")
MapOf = _Alias("MapOf")
RefOf = _Alias("RefOf")
Nullable = _Alias("Nullable")
OneOf = _Alias("OneOf")
Variant = _Alias("Variant")
SumOf = _Alias("SumOf")
Tensor = _Alias("Tensor")
StreamOf = _Alias("StreamOf")


__all__ = [
    "Str",
    "Bool",
    "Null",
    "Unit",
    "Int8",
    "Int16",
    "Int32",
    "Int64",
    "UInt8",
    "UInt16",
    "UInt32",
    "UInt64",
    "VarInt32",
    "VarInt64",
    "VarUInt32",
    "VarUInt64",
    "F16",
    "BF16",
    "F32",
    "F64",
    "Dec32",
    "Dec64",
    "Dec128",
    "Bytes",
    "Symbol",
    "Tagged",
    "SeqOf",
    "MapOf",
    "RefOf",
    "Nullable",
    "OneOf",
    "Variant",
    "SumOf",
    "Tensor",
    "StreamOf",
    "TensorDType",
]
