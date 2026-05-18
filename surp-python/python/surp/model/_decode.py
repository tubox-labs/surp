from __future__ import annotations

from enum import Enum
from typing import Any, cast

from ._field import MISSING
from ._stream import SurpStream
from ._variant import SurpVariant
from .exceptions import SurpDecodeModelError, SurpValidationError
from .types import (
    Bytes,
    Bool,
    F16,
    F32,
    F64,
    BF16,
    Null,
    Str,
    Symbol,
    Unit,
    _ForwardRef,
    _MapSpec,
    _NullableSpec,
    _OneOfSpec,
    _RefSpec,
    _ScalarSentinel,
    _SeqSpec,
    _StreamSpec,
    _SumSpec,
    _TaggedSpec,
    _TensorSpec,
)


def from_ctn(cls: type, text: str, *, validate: bool = True) -> Any:
    r"""from_ctn(cls, text, *, validate=True) -> Any

    Decode RFC-001 CTN text into an instance of ``cls``.

    ``SurpDocument`` subclasses read document bindings, while regular models
    compile CTN through CBF and decode the effective root product.

    Args:
        cls (type): Target ``SurpModel`` subclass.
        text (str): RFC-001 CTN input.
        validate (bool, optional): Whether to validate decoded values. Default:
          ``True``
    """
    try:
        from surp import rfc001

        if getattr(cls, "__surp_is_document__", False):
            doc = rfc001.parse_ctn(text)
            return _document_from_rfc(cls, doc, validate=validate)
        cbf = rfc001.compile_ctn(text)
        decoded = rfc001.decode_cbf(cbf)
        root = decoded["document"]["root"]
        return _model_from_rfc(cls, root, validate=validate)
    except SurpValidationError:
        raise
    except Exception as exc:  # pragma: no cover - depends on native package availability
        raise SurpDecodeModelError(str(exc)) from exc


def from_cbf(cls: type, data: bytes, *, validate: bool = True) -> Any:
    r"""from_cbf(cls, data, *, validate=True) -> Any

    Decode RFC-001 CBF bytes into an instance of ``cls``.
    """
    try:
        from surp import rfc001

        decoded = rfc001.decode_cbf(data)
        root = decoded["document"]["root"]
        if getattr(cls, "__surp_is_document__", False):
            return _document_from_root(cls, root, validate=validate)
        return _model_from_rfc(cls, root, validate=validate)
    except SurpValidationError:
        raise
    except Exception as exc:  # pragma: no cover - depends on native package availability
        raise SurpDecodeModelError(str(exc)) from exc


def from_rfc_value(cls: type, value: Any, *, validate: bool = True) -> Any:
    r"""from_rfc_value(cls, value, *, validate=True) -> Any

    Decode a native ``RfcValue`` view or RFC value dictionary into ``cls``.
    """
    if hasattr(value, "to_dict"):
        value = value.to_dict()
    return _model_from_rfc(cls, value, validate=validate)


def from_surp(cls: Any, data: bytes, *, validate: bool = True) -> Any:
    r"""from_surp(cls, data, *, validate=True) -> Any

    Decode stable v1 Surp bytes into a model through its plain dictionary form.

    This is the inverse of ``SurpModel.to_surp`` and is useful when a model is
    transported over the stable v1 Surp codec rather than RFC-001 CTN/CBF.
    """
    try:
        import surp

        decoded = surp.loads(data)
    except Exception as exc:  # pragma: no cover - depends on native package availability
        raise SurpDecodeModelError(str(exc)) from exc
    if not isinstance(decoded, dict):
        raise SurpDecodeModelError(
            f"expected object payload for {cls.__name__}, got {type(decoded).__name__}"
        )
    return from_dict(cls, decoded, validate=validate)


def from_dict(cls: Any, data: dict[str, Any], *, validate: bool = True) -> Any:
    r"""from_dict(cls, data, *, validate=True) -> Any

    Create a model instance from plain Python data.

    Nested dictionaries are coerced into nested ``SurpModel`` instances when
    the field annotation names a model class.
    """
    if getattr(cls, "__strict__", True):
        extra = sorted(set(data) - set(cls.__surp_fields__))
        if extra:
            raise SurpDecodeModelError(f"unknown field(s): {', '.join(extra)}")
    values: dict[str, Any] = {}
    for name, field in cls.__surp_fields__.items():
        if name in data:
            values[name] = _coerce_plain(data[name], field.rfc_type, validate=validate)
    instance = cls(_validate=validate, **values)
    return instance


def _document_from_rfc(cls: Any, doc: Any, *, validate: bool) -> Any:
    r"""_document_from_rfc(cls, doc, *, validate) -> Any

    Build a document model from a parsed CTN document dictionary.
    """
    binding_values = {binding["name"]: binding["value"] for binding in doc.get("bindings", [])}
    values: dict[str, Any] = {}
    for name, field in cls.__surp_fields__.items():
        binding = field.binding or name
        if binding in binding_values:
            values[name] = _decode_value(binding_values[binding], field.rfc_type, validate=validate, strict=cls.__strict__)
    return cls(_validate=validate, **values)


def _document_from_root(cls: Any, root: Any, *, validate: bool) -> Any:
    r"""_document_from_root(cls, root, *, validate) -> Any

    Build a document model from a decoded CBF root value.
    """
    fields = list(cls.__surp_fields__.items())
    if not fields:
        return cls(_validate=validate)
    name, field = fields[0]
    return cls(_validate=validate, **{name: _decode_value(root, field.rfc_type, validate=validate, strict=cls.__strict__)})


def _model_from_rfc(cls: Any, value: Any, *, validate: bool) -> Any:
    r"""_model_from_rfc(cls, value, *, validate) -> Any

    Build a model instance from an RFC-001 product dictionary.
    """
    if value is None:
        raise SurpDecodeModelError(f"expected RFC-001 product for {cls.__name__}")
    if value.get("kind") != "product":
        raise SurpDecodeModelError(f"expected RFC-001 product for {cls.__name__}")
    type_name = value.get("type_name")
    expected = getattr(cls, "__rfc_type__", cls.__name__)
    if type_name is not None and type_name != expected:
        raise SurpDecodeModelError(f"expected RFC type {expected!r}, got {type_name!r}")
    by_name = {field["name"]: field["value"] for field in value.get("fields", [])}
    if cls.__strict__:
        extra = sorted(set(by_name) - set(cls.__surp_fields__))
        if extra:
            raise SurpDecodeModelError(f"unknown RFC-001 field(s): {', '.join(extra)}")
    values: dict[str, Any] = {}
    for name, field in cls.__surp_fields__.items():
        if name in by_name:
            values[name] = _decode_value(by_name[name], field.rfc_type, validate=validate, strict=cls.__strict__)
    return cls(_validate=validate, **values)


def _decode_value(value: dict[str, Any], annotation: Any, *, validate: bool, strict: bool) -> Any:
    r"""_decode_value(value, annotation, *, validate, strict) -> Any

    Decode one RFC-001 value dictionary according to a Surp annotation.
    """
    annotation = _resolve(annotation)
    if isinstance(annotation, _NullableSpec):
        if _is_null(value):
            return None
        return _decode_value(value, annotation.inner, validate=validate, strict=strict)
    if isinstance(annotation, _OneOfSpec):
        last_error: Exception | None = None
        for option in annotation.options:
            try:
                return _decode_value(value, option, validate=validate, strict=strict)
            except Exception as exc:
                last_error = exc
        raise SurpDecodeModelError(str(last_error) if last_error else "OneOf decode failed")
    if isinstance(annotation, _TaggedSpec):
        if value.get("kind") != "scalar" or value.get("type") != "tagged":
            raise SurpDecodeModelError("expected tagged scalar")
        if value.get("tag") != annotation.tag:
            raise SurpDecodeModelError(f"expected tag {annotation.tag!r}")
        return value.get("value")
    if isinstance(annotation, _ScalarSentinel):
        return _decode_scalar(value, annotation)
    if isinstance(annotation, _SeqSpec):
        if value.get("kind") != "sequence":
            raise SurpDecodeModelError("expected sequence")
        return [
            _decode_value(item, annotation.elem, validate=validate, strict=strict)
            for item in value.get("items", [])
        ]
    if isinstance(annotation, _MapSpec):
        if value.get("kind") != "association":
            raise SurpDecodeModelError("expected association")
        return {
            _decode_value(key, annotation.key, validate=validate, strict=strict): _decode_value(item, annotation.value, validate=validate, strict=strict)
            for key, item in value.get("pairs", [])
        }
    if isinstance(annotation, _RefSpec):
        if value.get("kind") != "reference":
            raise SurpDecodeModelError("expected reference")
        if value.get("reference_kind") != "by_id":
            raise SurpDecodeModelError("only by-id references can be decoded as model fields")
        return _decode_value(value["value"], annotation.inner, validate=validate, strict=strict)
    if isinstance(annotation, _SumSpec):
        if value.get("kind") != "sum":
            raise SurpDecodeModelError("expected sum")
        payload = value.get("payload", {})
        if payload.get("kind") == "unit":
            decoded_payload = None
        elif payload.get("kind") == "tuple":
            items = payload.get("items", [])
            decoded_payload = _decode_value(items[0], _variant_tuple_type(annotation, value["variant"]), validate=validate, strict=strict) if items else None
        else:
            variant = next((item for item in annotation.variants if item.name == value["variant"]), None)
            field_types = {name: typ for name, typ in (variant.payload if variant else [])}
            decoded_payload = {
                field["name"]: _decode_value(field["value"], field_types[field["name"]], validate=validate, strict=strict)
                for field in payload.get("fields", [])
                if field["name"] in field_types
            }
        return SurpVariant(value["variant"], decoded_payload)
    if isinstance(annotation, _TensorSpec):
        if value.get("kind") != "tensor":
            raise SurpDecodeModelError("expected tensor")
        data = value.get("data", {})
        if "values" in data:
            return list(data["values"])
        return list(data.get("bytes", b""))
    if isinstance(annotation, _StreamSpec):
        if value.get("kind") != "stream":
            raise SurpDecodeModelError("expected stream")
        annotations = {
            item["name"]: _decode_annotation_scalar(item.get("value"))
            for item in value.get("annotations", [])
        }
        return SurpStream(annotations)
    if _is_symbol_enum(annotation):
        raw = _decode_scalar(value, cast(_ScalarSentinel, Symbol))
        for member in annotation:
            if member.value == raw:
                return member
        if not strict:
            return raw
        raise SurpDecodeModelError(f"unknown symbol {raw!r} for {annotation.__name__}")
    if isinstance(annotation, type) and hasattr(annotation, "__surp_fields__"):
        return _model_from_rfc(annotation, value, validate=validate)
    raise SurpDecodeModelError(f"unsupported annotation {annotation!r}")


def _decode_scalar(value: dict[str, Any], annotation: _ScalarSentinel) -> Any:
    r"""_decode_scalar(value, annotation) -> Any

    Decode one RFC-001 scalar dictionary to its Python value.
    """
    if value.get("kind") != "scalar":
        raise SurpDecodeModelError(f"expected scalar {annotation.rfc_name}")
    scalar_type = value.get("type")
    raw = value.get("value")
    if annotation is Str:
        if scalar_type != "str":
            raise SurpDecodeModelError("expected str scalar")
        return raw
    if annotation is Bool:
        if scalar_type != "bool":
            raise SurpDecodeModelError("expected bool scalar")
        return raw
    if annotation is Null:
        if scalar_type != "null":
            raise SurpDecodeModelError("expected null scalar")
        return None
    if annotation is Unit:
        if scalar_type != "unit":
            raise SurpDecodeModelError("expected unit scalar")
        return None
    if annotation is Bytes:
        if scalar_type != "bytes":
            raise SurpDecodeModelError("expected bytes scalar")
        return raw
    if annotation is Symbol:
        if scalar_type != "sym":
            raise SurpDecodeModelError("expected symbol scalar")
        return raw
    if annotation.rfc_name.startswith("dec"):
        if scalar_type != "tagged" or value.get("tag") != annotation.rfc_name:
            raise SurpDecodeModelError(f"expected {annotation.rfc_name} tagged decimal")
        return raw
    if annotation in {F16, BF16, F32, F64}:
        if scalar_type not in {"f32", "f64", "i64", "u64", "vi64", "vu64"}:
            raise SurpDecodeModelError("expected numeric scalar")
        return raw
    if scalar_type not in {"i64", "u64", "vi64", "vu64"}:
        raise SurpDecodeModelError("expected integer scalar")
    return raw


def _coerce_plain(value: Any, annotation: Any, *, validate: bool) -> Any:
    r"""_coerce_plain(value, annotation, *, validate) -> Any

    Coerce plain dictionary/list data into richer model helper objects.
    """
    annotation = _resolve(annotation)
    if isinstance(annotation, _NullableSpec):
        if value is None:
            return None
        return _coerce_plain(value, annotation.inner, validate=validate)
    if isinstance(annotation, _SeqSpec):
        return [_coerce_plain(item, annotation.elem, validate=validate) for item in value]
    if isinstance(annotation, _MapSpec):
        return {
            _coerce_plain(key, annotation.key, validate=validate): _coerce_plain(item, annotation.value, validate=validate)
            for key, item in value.items()
        }
    if isinstance(annotation, _RefSpec):
        return _coerce_plain(value, annotation.inner, validate=validate)
    if isinstance(annotation, _SumSpec):
        if isinstance(value, SurpVariant):
            return value
        if isinstance(value, dict) and "variant" in value:
            return SurpVariant(value["variant"], value.get("payload"))
    if _is_symbol_enum(annotation) and isinstance(value, str):
        enum_cls = cast(type[Enum], annotation)
        for member in enum_cls:
            if member.value == value:
                return member
    if isinstance(annotation, type) and hasattr(annotation, "__surp_fields__") and isinstance(value, dict):
        return from_dict(annotation, value, validate=validate)
    if isinstance(annotation, _StreamSpec):
        if isinstance(value, SurpStream):
            return value
        if isinstance(value, dict):
            return SurpStream(value)
    return value


def _variant_tuple_type(annotation: _SumSpec, name: str) -> Any:
    r"""_variant_tuple_type(annotation, name) -> Any

    Return the tuple payload type for a named sum variant.
    """
    variant = next(item for item in annotation.variants if item.name == name)
    return variant.payload[0]


def _is_null(value: dict[str, Any]) -> bool:
    r"""_is_null(value) -> bool

    Return true when an RFC-001 value dictionary represents ``null``.
    """
    return value.get("kind") == "scalar" and value.get("type") == "null"


def _decode_annotation_scalar(value: dict[str, Any] | None) -> Any:
    r"""_decode_annotation_scalar(value) -> Any

    Decode a stream or document annotation scalar to plain Python.
    """
    if value is None:
        return None
    scalar_type = value.get("type")
    if scalar_type == "null" or scalar_type == "unit":
        return None
    if scalar_type == "sym":
        return value.get("value")
    if scalar_type == "bytes":
        return value.get("value")
    return value.get("value")


def _resolve(annotation: Any) -> Any:
    r"""_resolve(annotation) -> Any

    Resolve a forward reference through the model registry when possible.
    """
    if isinstance(annotation, _ForwardRef):
        from . import _registry as registry

        return registry.get(annotation.name) or annotation
    return annotation


def _is_symbol_enum(annotation: Any) -> bool:
    r"""_is_symbol_enum(annotation) -> bool

    Return true for ``SurpSymbolEnum`` subclasses.
    """
    return isinstance(annotation, type) and issubclass(annotation, Enum) and hasattr(
        annotation, "__surp_symbol_enum__"
    )
