from __future__ import annotations

from . import _registry as registry
from ._base import SurpDocument, SurpModel, SurpSymbolEnum, annotation
from ._field import Field, FieldInfo
from ._stream import SurpStream
from ._variant import SurpVariant
from .exceptions import (
    SurpDecodeModelError,
    SurpEncodeModelError,
    SurpFieldError,
    SurpModelDefinitionError,
    SurpModelError,
    SurpQueryError,
    SurpValidationError,
)
from .stubgen import generate_model_stubs, write_model_stubs

SurpEnum = SurpSymbolEnum

__all__ = [
    "SurpModel",
    "SurpDocument",
    "SurpSymbolEnum",
    "SurpEnum",
    "SurpStream",
    "SurpVariant",
    "Field",
    "FieldInfo",
    "annotation",
    "registry",
    "generate_model_stubs",
    "write_model_stubs",
    "SurpModelError",
    "SurpModelDefinitionError",
    "SurpValidationError",
    "SurpFieldError",
    "SurpEncodeModelError",
    "SurpDecodeModelError",
    "SurpQueryError",
]
