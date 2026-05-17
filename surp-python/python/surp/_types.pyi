from __future__ import annotations

from typing import Any, Literal, TypedDict

Compression = Literal["lz4", "zstd", "snappy", "none"]

RfcScalarType = Literal[
    "null",
    "unit",
    "bool",
    "i64",
    "u64",
    "vi64",
    "vu64",
    "f32",
    "f64",
    "str",
    "bytes",
    "sym",
    "tagged",
]
RfcKind = Literal[
    "scalar",
    "product",
    "sum",
    "sequence",
    "association",
    "reference",
    "tensor",
    "stream",
    "opaque",
]

class RfcScalarValue(TypedDict, total=False):
    kind: Literal["scalar"]
    type: RfcScalarType
    value: Any
    tag: str

class RfcField(TypedDict):
    name: str
    value: "RfcValueDict"

class RfcSumPayload(TypedDict, total=False):
    kind: Literal["unit", "tuple", "struct"]
    items: list["RfcValueDict"]
    fields: list[RfcField]

class RfcTensorData(TypedDict, total=False):
    kind: Literal["dense_f64", "dense_i64", "dense_u64", "binary_blob"]
    values: list[float] | list[int]
    bytes: bytes

class RfcValueDict(TypedDict, total=False):
    kind: RfcKind
    type: RfcScalarType
    value: Any
    tag: str
    type_name: str | None
    fields: list[RfcField]
    variant: str
    payload: RfcSumPayload
    elem_type: str | None
    items: list["RfcValueDict"]
    pairs: list[tuple["RfcValueDict", "RfcValueDict"]]
    reference_kind: Literal["binding", "by_id"]
    name: str
    element_type: str
    shape: list[int | None]
    annotations: list["RfcAnnotation"]
    data: RfcTensorData
    item_type: str
    type_tag: str
    bytes: bytes

class RfcAnnotation(TypedDict):
    name: str
    value: RfcScalarValue | None

class RfcBinding(TypedDict):
    name: str
    value: RfcValueDict

class RfcDocument(TypedDict):
    annotations: list[RfcAnnotation]
    uses: list[str]
    bindings: list[RfcBinding]
    root: RfcValueDict | None

class RfcHeader(TypedDict):
    magic: str
    cbf_version: int
    ctn_version: int
    flags: int
    alignment: int
    schema_hash_prefix: bytes
    root_offset: int
    symtab_offset: int
    index_offset: int
    self_describing: bool
    has_symtab: bool
    has_index: bool

class RfcDecodedCbf(TypedDict):
    header: RfcHeader
    symbols: list[str]
    document: RfcDocument
    ctn: str
