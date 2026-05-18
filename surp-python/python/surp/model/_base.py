from __future__ import annotations

import copy
from enum import Enum
from typing import Any

from ._decode import from_cbf as _from_cbf
from ._decode import from_ctn as _from_ctn
from ._decode import from_dict as _from_dict
from ._decode import from_rfc_value as _from_rfc_value
from ._decode import from_surp as _from_surp
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
    r"""SurpSymbolEnum(value) -> SurpSymbolEnum

    Enum base class whose values encode as RFC-001 symbols.

    Examples:
        >>> class Role(SurpSymbolEnum):
        ...     ADMIN = "admin"
        >>> Role.ADMIN.value
        'admin'
    """

    __surp_symbol_enum__ = True


class SurpModel(metaclass=SurpModelMeta):
    r"""SurpModel(**kwargs) -> SurpModel

    Base class for strongly described RFC-001 model values.

    A model class declares fields with Surp type markers and ``Field(...)``.
    Instances validate values on construction by default and can encode to CTN,
    CBF, or the stable v1 Surp binary format.

    Examples::

        >>> class User(SurpModel):
        ...     name: Str = Field(required=True)
        >>> user = User(name="Alice")
        >>> user.to_dict()
        {'name': 'Alice'}
    """

    __surp_base__ = True
    __surp_is_document__ = False
    __strict__ = True
    __rfc_type__: str
    __surp_fields__: dict[str, Any] = {}

    def __init__(self, *args: Any, _validate: bool = True, **kwargs: Any) -> None:
        r"""__init__(*args, _validate=True, **kwargs) -> None

        Initialize a model from keyword field values.

        Missing optional values are populated from ``default`` or
        ``default_factory``. Unknown fields raise ``TypeError`` so accidental
        misspellings do not silently disappear.

        Args:
            *args (Any): Positional arguments. Positional values are rejected.
            _validate (bool, optional): Whether to validate after assignment.
              Default: ``True``
            **kwargs (Any): Field values keyed by field name.
        """
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
        r"""validate() -> None

        Validate all stored field values and raise on the first batch of errors.
        """
        validate_model(self)

    def collect_errors(self) -> list[Any]:
        r"""collect_errors() -> list[Any]

        Return all validation errors without raising.
        """
        return collect_model_errors(self)

    def to_ctn(self, indent: int = 2) -> str:
        r"""to_ctn(indent=2) -> str

        Encode this model as canonical RFC-001 CTN text.

        Args:
            indent (int, optional): Formatting indentation retained for API
              compatibility. Default: ``2``
        """
        return model_to_ctn(self, indent=indent)

    def to_cbf(self, *, alignment: int = 4, with_symtab: bool = True) -> bytes:
        r"""to_cbf(*, alignment=4, with_symtab=True) -> bytes

        Encode this model as RFC-001 CBF bytes.

        Args:
            alignment (int, optional): CBF alignment hint passed to the native
              compiler. Default: ``4``
            with_symtab (bool, optional): Whether to include a CBF symbol table.
              Default: ``True``
        """
        return model_to_cbf(self, alignment=alignment, with_symtab=with_symtab)

    def to_surp(self, *, dedup: bool = True, sort_keys: bool = True) -> bytes:
        r"""to_surp(*, dedup=True, sort_keys=True) -> bytes

        Encode this model's plain dictionary as stable v1 Surp bytes.

        Args:
            dedup (bool, optional): Enable the v1 string dictionary. Default:
              ``True``
            sort_keys (bool, optional): Sort object keys before encoding.
              Default: ``True``
        """
        return model_to_surp(self, dedup=dedup, sort_keys=sort_keys)

    @classmethod
    def from_ctn(cls, text: str, *, validate: bool = True) -> Any:
        r"""from_ctn(text, *, validate=True) -> Any

        Decode a model instance from RFC-001 CTN text.

        Args:
            text (str): CTN document or product text.
            validate (bool, optional): Whether to validate decoded fields.
              Default: ``True``
        """
        return _from_ctn(cls, text, validate=validate)

    @classmethod
    def from_cbf(cls, data: bytes, *, validate: bool = True) -> Any:
        r"""from_cbf(data, *, validate=True) -> Any

        Decode a model instance from RFC-001 CBF bytes.
        """
        return _from_cbf(cls, data, validate=validate)

    @classmethod
    def from_rfc_value(cls, value: Any, *, validate: bool = True) -> Any:
        r"""from_rfc_value(value, *, validate=True) -> Any

        Decode a model instance from an RFC value dictionary or native view.
        """
        return _from_rfc_value(cls, value, validate=validate)

    @classmethod
    def from_surp(cls, data: bytes, *, validate: bool = True) -> Any:
        r"""from_surp(data, *, validate=True) -> Any

        Decode a model instance from stable v1 Surp bytes.

        This is the inverse of :meth:`to_surp` for object payloads.
        """
        return _from_surp(cls, data, validate=validate)

    @classmethod
    def from_dict(cls, data: dict[str, Any], *, validate: bool = True) -> Any:
        r"""from_dict(data, *, validate=True) -> Any

        Build a model instance from plain Python dictionary data.
        """
        return _from_dict(cls, data, validate=validate)

    def query(self, expr: str) -> list[Any]:
        r"""query(expr) -> list[Any]

        Run a baseline RFC-001 CQL path query against this instance.
        """
        return _query(self, expr)

    def query_one(self, expr: str) -> Any:
        r"""query_one(expr) -> Any

        Run a CQL query and require exactly one result.
        """
        return _query_one(self, expr)

    @classmethod
    def schema_ctn(cls) -> str:
        r"""schema_ctn() -> str

        Return a CTN document describing this model's fields.
        """
        return _schema_ctn(cls)

    @classmethod
    def schema_json(cls) -> dict[str, Any]:
        r"""schema_json() -> dict[str, Any]

        Return a JSON-schema-like dictionary for this model.
        """
        return _schema_json(cls)

    def to_dict(self) -> dict[str, Any]:
        r"""to_dict() -> dict[str, Any]

        Return plain Python data suitable for ``surp.dumps`` or JSON tools.
        """
        return {name: _plain(value) for name, value in self.__surp_values__.items()}

    def __eq__(self, other: Any) -> bool:
        r"""__eq__(other) -> bool

        Compare model instances by exact class and field values.
        """
        return type(self) is type(other) and self.__surp_values__ == other.__surp_values__

    def __repr__(self) -> str:
        r"""__repr__() -> str

        Return a constructor-style representation of stored fields.
        """
        args = ", ".join(f"{name}={value!r}" for name, value in self.__surp_values__.items())
        return f"{self.__class__.__name__}({args})"


class SurpDocument(SurpModel):
    r"""SurpDocument(**kwargs) -> SurpDocument

    Model base class for full RFC-001 documents with bindings and root.

    Document models encode each field as a CTN ``let`` binding. ``__root__`` can
    name the binding that should be emitted as the document root.
    """

    __surp_base__ = True
    __surp_is_document__ = True
    __surp_annotations__: list[tuple[str, Any | None]] = []


def annotation(name: str, value: Any | None = None) -> Any:
    r"""annotation(name, value=None) -> Any

    Attach a document-level RFC-001 annotation to a ``SurpDocument`` class.

    Args:
        name (str): Annotation name without the leading ``@``.
        value (Any, optional): Optional scalar annotation value. Default:
          ``None``
    """
    def decorate(cls: Any) -> Any:
        r"""decorate(cls) -> Any

        Store the annotation on the decorated class.
        """
        existing = list(getattr(cls, "__surp_annotations__", []))
        cls.__surp_annotations__ = [(name, value), *existing]
        return cls

    return decorate


def _plain(value: Any) -> Any:
    r"""_plain(value) -> Any

    Recursively convert model-specific objects into plain Python values.
    """
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
