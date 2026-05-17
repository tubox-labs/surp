"""Surp native exception classes."""

from __future__ import annotations

try:
    from . import _surp_native as _native
except ImportError as exc:  # pragma: no cover - import-time installation guard
    raise ImportError(
        "The native Surp extension is not installed. Build or install the "
        "package with maturin before importing surp."
    ) from exc

SurpError = _native.SurpError
SurpEncodeError = _native.SurpEncodeError
SurpDecodeError = _native.SurpDecodeError
SurpChecksumError = _native.SurpChecksumError
SurpTypeError = _native.SurpTypeError
SurpRfcError = _native.SurpRfcError

__all__ = [
    "SurpError",
    "SurpEncodeError",
    "SurpDecodeError",
    "SurpChecksumError",
    "SurpTypeError",
    "SurpRfcError",
]
