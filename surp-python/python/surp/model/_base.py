from __future__ import annotations

import copy
from enum import Enum
from typing import Any

from ._decode import from_cbf as _from_cbf
from ._decode import from_ctn as _from_ctn
from ._decode import from_dict as _from_dict
from ._decode import from_rfc_value as _from_rfc_value
from ._encode import model_to_cbf, model_to_ctn, model_to_surp
from ._field import MISSING
from ._meta import SurpModelMeta
from ._query import query as _query
from ._query import query_one as _query_one
from ._schema import schema_ctn as _schema_ctn
from ._schema import schema_json as _schema_json
from ._validate import collect_model_errors, validate_model
from .exceptions import SurpValidationError


class SurpSymbolEnum(Enum):
    __surp_symbol_enum__ = True


class SurpModel(metaclass=SurpModelMeta):
    __surp_base__ = True
    __surp_is_document__ = False
    __strict__ = True
    __rfc_type__: str
    __surp_fields__: dict[str, Any] = {}

    def __init__(self, *args: Any, _validate: bool = True, **kwargs: Any) -> None:
        if args:
            raise TypeError(f"{self.__class__.__name__} only accepts keyword arguments")
        fields = self.__surp_fields__
        unknown = sorted(set(kwargs) - set(fields))
        if unknown:
            raise TypeError(f"unknown field(s): {', '.join(unknown)}")
        self.__surp_values__: dict[str, Any] = {}
        for name, field in fields.items():
            if name in kwargs:
                self.__surp_values__[name] = kwargs[name]
            elif field.default_factory is not None:
                self.__surp_values__[name] = field.default_factory()
            elif field.default is not MISSING:
                self.__surp_values__[name] = copy.deepcopy(field.default)
        if _validate:
            validate_model(self)

    def validate(self) -> None:
        validate_model(self)

    def collect_errors(self) -> list[Any]:
        return collect_model_errors(self)

    def to_ctn(self, indent: int = 2) -> str:
        return model_to_ctn(self, indent=indent)

    def to_cbf(self, *, alignment: int = 4, with_symtab: bool = True) -> bytes:
        return model_to_cbf(self, alignment=alignment, with_symtab=with_symtab)

    def to_surp(self, *, dedup: bool = True, sort_keys: bool = True) -> bytes:
        return model_to_surp(self, dedup=dedup, sort_keys=sort_keys)

    @classmethod
    def from_ctn(cls, text: str, *, validate: bool = True) -> Any:
        return _from_ctn(cls, text, validate=validate)

    @classmethod
    def from_cbf(cls, data: bytes, *, validate: bool = True) -> Any:
        return _from_cbf(cls, data, validate=validate)

    @classmethod
    def from_rfc_value(cls, value: Any, *, validate: bool = True) -> Any:
        return _from_rfc_value(cls, value, validate=validate)

    @classmethod
    def from_dict(cls, data: dict[str, Any], *, validate: bool = True) -> Any:
        return _from_dict(cls, data, validate=validate)

    def query(self, expr: str) -> list[Any]:
        return _query(self, expr)

    def query_one(self, expr: str) -> Any:
        return _query_one(self, expr)

    @classmethod
    def schema_ctn(cls) -> str:
        return _schema_ctn(cls)

    @classmethod
    def schema_json(cls) -> dict[str, Any]:
        return _schema_json(cls)

    def to_dict(self) -> dict[str, Any]:
        return {name: _plain(value) for name, value in self.__surp_values__.items()}

    def __eq__(self, other: Any) -> bool:
        return type(self) is type(other) and self.__surp_values__ == other.__surp_values__

    def __repr__(self) -> str:
        args = ", ".join(f"{name}={value!r}" for name, value in self.__surp_values__.items())
        return f"{self.__class__.__name__}({args})"


class SurpDocument(SurpModel):
    __surp_base__ = True
    __surp_is_document__ = True
    __surp_annotations__: list[tuple[str, Any | None]] = []


def annotation(name: str, value: Any | None = None) -> Any:
    def decorate(cls: Any) -> Any:
        existing = list(getattr(cls, "__surp_annotations__", []))
        cls.__surp_annotations__ = [(name, value), *existing]
        return cls

    return decorate


def _plain(value: Any) -> Any:
    if isinstance(value, SurpModel):
        return value.to_dict()
    if isinstance(value, Enum):
        return value.value
    if isinstance(value, list):
        return [_plain(item) for item in value]
    if isinstance(value, tuple):
        return [_plain(item) for item in value]
    if isinstance(value, dict):
        return {_plain(key): _plain(item) for key, item in value.items()}
    return value
