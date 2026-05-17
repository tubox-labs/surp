from __future__ import annotations

from typing import Any, Literal, TypedDict

Compression = Literal["lz4", "zstd", "snappy", "none"]

class RfcAnnotation(TypedDict):
    name: str
    value: dict[str, Any] | None

class RfcBinding(TypedDict):
    name: str
    value: dict[str, Any]

class RfcDocument(TypedDict):
    annotations: list[RfcAnnotation]
    uses: list[str]
    bindings: list[RfcBinding]
    root: dict[str, Any] | None

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
