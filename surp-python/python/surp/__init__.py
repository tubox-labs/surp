"""Native Surp Python API."""

from __future__ import annotations

try:
    from . import _surp_native as _native
except ImportError as exc:  # pragma: no cover - import-time installation guard
    raise ImportError(
        "The native Surp extension is not installed. Build or install the "
        "package with maturin before importing surp."
    ) from exc

from . import model, rfc001
from .exceptions import (
    SurpChecksumError,
    SurpDecodeError,
    SurpEncodeError,
    SurpError,
    SurpRfcError,
    SurpTypeError,
)

__version__ = _native.__version__

dumps = _native.dumps
loads = _native.loads
dump = _native.dump
load = _native.load
encode = _native.encode
decode = _native.decode
encode_to_file = _native.encode_to_file
decode_from_file = _native.decode_from_file
parse_text = _native.parse_text
pretty_print = _native.pretty_print
to_value = _native.to_value
loads_value = _native.loads_value
parse_text_value = _native.parse_text_value
Encoder = _native.Encoder
SurpDecoder = _native.SurpDecoder
SurpValue = _native.SurpValue

__all__ = [
    "__version__",
    "dumps",
    "loads",
    "dump",
    "load",
    "encode",
    "decode",
    "encode_to_file",
    "decode_from_file",
    "parse_text",
    "pretty_print",
    "to_value",
    "loads_value",
    "parse_text_value",
    "Encoder",
    "SurpDecoder",
    "SurpValue",
    "SurpError",
    "SurpEncodeError",
    "SurpDecodeError",
    "SurpChecksumError",
    "SurpTypeError",
    "SurpRfcError",
    "model",
    "rfc001",
]
