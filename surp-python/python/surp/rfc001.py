"""RFC-001 CTN, CBF, and CQL helpers backed by surp-core."""

from __future__ import annotations

from typing import Any

try:
    from . import _surp_native as _native
except ImportError as exc:  # pragma: no cover - import-time installation guard
    raise ImportError(
        "The native Surp extension is not installed. Build or install the "
        "package with maturin before importing surp.rfc001."
    ) from exc

CBF_MAGIC = _native.RFC001_CBF_MAGIC
CBF_HEADER_SIZE = _native.RFC001_CBF_HEADER_SIZE


def parse_ctn(text: str) -> dict[str, Any]:
    """Parse an RFC-001 CTN document into a typed Python dictionary."""
    return _native.rfc_parse_ctn(text)


def normalize_ctn(text: str) -> str:
    """Parse and reformat RFC-001 CTN into canonical repository formatting."""
    return _native.rfc_format_ctn(text)


def compile_ctn(text: str, *, with_symtab: bool = True, alignment: int = 0) -> bytes:
    """Compile RFC-001 CTN text to CBF bytes."""
    return _native.rfc_compile_ctn(text, with_symtab=with_symtab, alignment=alignment)


def decode_cbf(data: bytes) -> dict[str, Any]:
    """Decode RFC-001 CBF bytes into header, symbol, document, and CTN data."""
    return _native.rfc_decode_cbf(data)


def cbf_to_ctn(data: bytes) -> str:
    """Decode RFC-001 CBF bytes and return CTN text."""
    return _native.rfc_cbf_to_ctn(data)


def query_cbf(data: bytes, query: str, *, as_ctn: bool = False) -> list[Any]:
    """Run a baseline RFC-001 CQL path query over CBF bytes."""
    return _native.rfc_query_cbf(data, query, as_ctn=as_ctn)


def query_ctn(text: str, query: str, *, as_ctn: bool = False) -> list[Any]:
    """Run a baseline RFC-001 CQL path query over CTN text."""
    return _native.rfc_query_ctn(text, query, as_ctn=as_ctn)


def parse_ctn_model(text: str) -> Any:
    """Parse CTN into a native-backed RfcDocument model."""
    return _native.rfc_parse_ctn_model(text)


def decode_cbf_model(data: bytes) -> Any:
    """Decode CBF into a native-backed RfcDecodedCbf model."""
    return _native.rfc_decode_cbf_model(data)


def query_cbf_model(data: bytes, query: str) -> list[Any]:
    """Run CQL over CBF and return native-backed RfcValue models."""
    return _native.rfc_query_cbf_model(data, query)


def query_ctn_model(text: str, query: str) -> list[Any]:
    """Run CQL over CTN and return native-backed RfcValue models."""
    return _native.rfc_query_ctn_model(text, query)


RfcAnnotation = _native.RfcAnnotation
RfcField = _native.RfcField
RfcBinding = _native.RfcBinding
RfcHeader = _native.RfcHeader
RfcDocument = _native.RfcDocument
RfcDecodedCbf = _native.RfcDecodedCbf
RfcValue = _native.RfcValue


__all__ = [
    "CBF_MAGIC",
    "CBF_HEADER_SIZE",
    "parse_ctn",
    "normalize_ctn",
    "compile_ctn",
    "decode_cbf",
    "cbf_to_ctn",
    "query_cbf",
    "query_ctn",
    "parse_ctn_model",
    "decode_cbf_model",
    "query_cbf_model",
    "query_ctn_model",
    "RfcAnnotation",
    "RfcField",
    "RfcBinding",
    "RfcHeader",
    "RfcDocument",
    "RfcDecodedCbf",
    "RfcValue",
]
