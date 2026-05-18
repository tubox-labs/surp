from __future__ import annotations

from enum import Enum
from typing import Any


class _TypeSpec:
    r"""_TypeSpec() -> _TypeSpec

    Base class for runtime RFC-001 type descriptors.

    Type descriptors are metadata objects used by ``SurpModelMeta`` during
    model class creation. They intentionally are not Python value types; for
    example ``Int64`` describes an RFC-001 integer field while the Python value
    remains a normal ``int``.
    """

    kind: str

    def describe(self) -> str:
        r"""describe() -> str

        Return the compact RFC-001 type expression for diagnostics.
        """
        return self.kind


class _ForwardRef(_TypeSpec):
    r"""_ForwardRef(name) -> _ForwardRef

    Deferred reference to a model class that may be declared later.

    Examples::

        >>> ref = _ForwardRef("Post")
        >>> ref.describe()
        'Post'
    """

    kind = "forward"

    def __init__(self, name: str) -> None:
        r"""__init__(name) -> None

        Create a reference to a model or symbol enum by name.
        """
        self.name = name

    def describe(self) -> str:
        r"""describe() -> str

        Return the referenced type name.
        """
        return self.name

    def __repr__(self) -> str:
        r"""__repr__() -> str

        Return a developer-oriented representation for error messages.
        """
        return f"ForwardRef({self.name!r})"


class _ScalarSentinel(_TypeSpec):
    r"""_ScalarSentinel(rfc_name, *, py_name=None, min_value=None, max_value=None) -> _ScalarSentinel

    Runtime marker for scalar RFC-001 field types.

    The marker carries the RFC scalar name and optional integer bounds. It is
    used in model annotations such as ``name: Str`` and ``age: Int64``.
    """

    kind = "scalar"

    def __init__(
        self,
        rfc_name: str,
        *,
        py_name: str | None = None,
        min_value: int | None = None,
        max_value: int | None = None,
    ) -> None:
        r"""__init__(rfc_name, *, py_name=None, min_value=None, max_value=None) -> None

        Create a scalar marker with optional Python display name and bounds.
        """
        self.rfc_name = rfc_name
        self.py_name = py_name or rfc_name
        self.min_value = min_value
        self.max_value = max_value

    def describe(self) -> str:
        r"""describe() -> str

        Return the RFC scalar name, such as ``str`` or ``i64``.
        """
        return self.rfc_name

    def __repr__(self) -> str:
        r"""__repr__() -> str

        Return the RFC scalar name for concise schema display.
        """
        return self.rfc_name


class _TaggedSpec(_TypeSpec):
    r"""_TaggedSpec(tag, inner) -> _TaggedSpec

    Descriptor for tagged scalar values such as ``Tagged["uid", Str]``.
    """

    kind = "tagged"

    def __init__(self, tag: str, inner: Any) -> None:
        r"""__init__(tag, inner) -> None

        Create a tagged scalar descriptor.
        """
        self.tag = tag
        self.inner = normalize_annotation(inner)

    def describe(self) -> str:
        r"""describe() -> str

        Return the RFC-001 tagged type expression.
        """
        return f"{self.tag}<{describe_type(self.inner)}>"

    def __repr__(self) -> str:
        r"""__repr__() -> str

        Return a Python-like representation of the tagged descriptor.
        """
        return f"Tagged[{self.tag!r}, {self.inner!r}]"


class _SeqSpec(_TypeSpec):
    r"""_SeqSpec(elem) -> _SeqSpec

    Descriptor for homogeneous RFC-001 sequences.
    """

    kind = "sequence"

    def __init__(self, elem: Any) -> None:
        r"""__init__(elem) -> None

        Create a sequence descriptor for ``elem`` values.
        """
        self.elem = normalize_annotation(elem)

    def describe(self) -> str:
        r"""describe() -> str

        Return the RFC-001 sequence type expression.
        """
        return f"seq<{describe_type(self.elem)}>"

    def __repr__(self) -> str:
        r"""__repr__() -> str

        Return a Python-like representation of the sequence descriptor.
        """
        return f"SeqOf[{self.elem!r}]"


class _MapSpec(_TypeSpec):
    r"""_MapSpec(key, value) -> _MapSpec

    Descriptor for RFC-001 associations with scalar-compatible keys.
    """

    kind = "map"

    def __init__(self, key: Any, value: Any) -> None:
        r"""__init__(key, value) -> None

        Create an association descriptor for ``key`` and ``value`` types.
        """
        self.key = normalize_annotation(key)
        self.value = normalize_annotation(value)

    def describe(self) -> str:
        r"""describe() -> str

        Return the RFC-001 association type expression.
        """
        return f"map<{describe_type(self.key)}, {describe_type(self.value)}>"

    def __repr__(self) -> str:
        r"""__repr__() -> str

        Return a Python-like representation of the map descriptor.
        """
        return f"MapOf[{self.key!r}, {self.value!r}]"


class _RefSpec(_TypeSpec):
    r"""_RefSpec(inner) -> _RefSpec

    Descriptor for by-id RFC-001 references.
    """

    kind = "reference"

    def __init__(self, inner: Any) -> None:
        r"""__init__(inner) -> None

        Create a reference descriptor for the referenced value type.
        """
        self.inner = normalize_annotation(inner)

    def describe(self) -> str:
        r"""describe() -> str

        Return the RFC-001 reference type expression.
        """
        return f"ref<{describe_type(self.inner)}>"

    def __repr__(self) -> str:
        r"""__repr__() -> str

        Return a Python-like representation of the reference descriptor.
        """
        return f"RefOf[{self.inner!r}]"


class _NullableSpec(_TypeSpec):
    r"""_NullableSpec(inner) -> _NullableSpec

    Descriptor for optional values that may encode as RFC-001 ``null``.
    """

    kind = "nullable"

    def __init__(self, inner: Any) -> None:
        r"""__init__(inner) -> None

        Create a nullable descriptor for the non-null value type.
        """
        self.inner = normalize_annotation(inner)

    def describe(self) -> str:
        r"""describe() -> str

        Return the RFC-001 nullable type expression.
        """
        return f"nullable<{describe_type(self.inner)}>"

    def __repr__(self) -> str:
        r"""__repr__() -> str

        Return a Python-like representation of the nullable descriptor.
        """
        return f"Nullable[{self.inner!r}]"


class _OneOfSpec(_TypeSpec):
    r"""_OneOfSpec(options) -> _OneOfSpec

    Descriptor for union-like fields that accept several RFC-001 types.
    """

    kind = "oneof"

    def __init__(self, options: tuple[Any, ...]) -> None:
        r"""__init__(options) -> None

        Create a descriptor from two or more candidate type markers.
        """
        self.options = tuple(normalize_annotation(option) for option in options)

    def describe(self) -> str:
        r"""describe() -> str

        Return the RFC-001 one-of type expression.
        """
        return "oneof<" + ", ".join(describe_type(option) for option in self.options) + ">"

    def __repr__(self) -> str:
        r"""__repr__() -> str

        Return a Python-like representation of the one-of descriptor.
        """
        return f"OneOf[{', '.join(repr(option) for option in self.options)}]"


class _VariantSpec(_TypeSpec):
    r"""_VariantSpec(name, payload) -> _VariantSpec

    Descriptor for a single variant inside ``SumOf[...]``.
    """

    kind = "variant"

    def __init__(self, name: str, payload: tuple[Any, ...]) -> None:
        r"""__init__(name, payload) -> None

        Create a sum variant with optional tuple or named payload fields.
        """
        self.name = name
        self.payload = tuple(payload)

    @property
    def payload_kind(self) -> str:
        r"""payload_kind() -> str

        Return ``unit``, ``tuple``, or ``struct`` for this variant payload.
        """
        if not self.payload:
            return "unit"
        if len(self.payload) == 1 and not _is_named_payload_item(self.payload[0]):
            return "tuple"
        return "struct"

    def describe(self) -> str:
        r"""describe() -> str

        Return the compact variant descriptor used in schema output.
        """
        return f"variant<{self.name}>"

    def __repr__(self) -> str:
        r"""__repr__() -> str

        Return a Python-like representation of the variant descriptor.
        """
        return f"Variant[{self.name!r}]"


class _SumSpec(_TypeSpec):
    r"""_SumSpec(variants) -> _SumSpec

    Descriptor for tagged sum fields made from ``Variant[...]`` entries.
    """

    kind = "sum"

    def __init__(self, variants: tuple[Any, ...]) -> None:
        r"""__init__(variants) -> None

        Create a sum descriptor from one or more variant descriptors.
        """
        self.variants = tuple(variants)

    def describe(self) -> str:
        r"""describe() -> str

        Return the RFC-001 sum type expression.
        """
        return "sum<" + ", ".join(v.name for v in self.variants) + ">"

    def __repr__(self) -> str:
        r"""__repr__() -> str

        Return a Python-like representation of the sum descriptor.
        """
        return f"SumOf[{', '.join(repr(variant) for variant in self.variants)}]"


class TensorDType(Enum):
    r"""TensorDType(value) -> TensorDType

    Supported RFC-001 tensor element types.

    Examples::

        >>> Tensor[TensorDType.F32, (3,)].describe()
        'tensor<f32>[3]'
    """

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
    r"""_TensorSpec(dtype, shape) -> _TensorSpec

    Descriptor for dense tensor fields.
    """

    kind = "tensor"

    def __init__(self, dtype: TensorDType, shape: tuple[int | None, ...]) -> None:
        r"""__init__(dtype, shape) -> None

        Create a tensor descriptor with element dtype and shape.
        """
        self.dtype = dtype
        self.shape = shape

    def describe(self) -> str:
        r"""describe() -> str

        Return the RFC-001 tensor type expression.
        """
        dims = ", ".join("_" if dim is None else str(dim) for dim in self.shape)
        return f"tensor<{self.dtype.value}>[{dims}]"

    def __repr__(self) -> str:
        r"""__repr__() -> str

        Return a Python-like representation of the tensor descriptor.
        """
        return f"Tensor[{self.dtype!r}, {self.shape!r}]"


class _StreamSpec(_TypeSpec):
    r"""_StreamSpec(item) -> _StreamSpec

    Descriptor for RFC-001 stream metadata fields.
    """

    kind = "stream"

    def __init__(self, item: Any) -> None:
        r"""__init__(item) -> None

        Create a stream descriptor for items of ``item`` type.
        """
        self.item = normalize_annotation(item)

    def describe(self) -> str:
        r"""describe() -> str

        Return the RFC-001 stream type expression.
        """
        return f"stream<{describe_type(self.item)}>"

    def __repr__(self) -> str:
        r"""__repr__() -> str

        Return a Python-like representation of the stream descriptor.
        """
        return f"StreamOf[{self.item!r}]"


class _Alias:
    r"""_Alias(name) -> _Alias

    Subscriptable runtime factory used by public composite markers.

    ``SeqOf[Str]`` and related forms are executed at class creation time and
    return immutable descriptor objects consumed by the metaclass.
    """

    def __init__(self, name: str) -> None:
        r"""__init__(name) -> None

        Create a named marker factory.
        """
        self.name = name

    def __getitem__(self, args: Any) -> Any:
        r"""__getitem__(args) -> Any

        Build the concrete descriptor represented by ``Alias[...]`` syntax.
        """
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
        r"""__repr__() -> str

        Return the public marker factory name.
        """
        return self.name


def _is_named_payload_item(item: Any) -> bool:
    r"""_is_named_payload_item(item) -> bool

    Return true for ``("field_name", TypeMarker)`` variant payload entries.
    """
    return isinstance(item, tuple) and len(item) == 2 and isinstance(item[0], str)


def normalize_annotation(annotation: Any) -> Any:
    r"""normalize_annotation(annotation) -> Any

    Normalize raw Python annotations into Surp runtime descriptors.

    This accepts string forward references and Python 3.14 ``annotationlib``
    forward references, then converts them to Surp's registry-resolved
    ``_ForwardRef`` objects.

    Examples::

        >>> normalize_annotation("Post").describe()
        'Post'
    """
    if isinstance(annotation, str):
        return _ForwardRef(annotation)
    forward_arg = getattr(annotation, "__forward_arg__", None)
    if isinstance(forward_arg, str):
        return _ForwardRef(forward_arg.strip("\"'"))
    return annotation


def describe_type(annotation: Any) -> str:
    r"""describe_type(annotation) -> str

    Return a stable human-readable type expression for diagnostics.
    """
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
