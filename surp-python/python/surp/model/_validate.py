from __future__ import annotations

from enum import Enum
from typing import Any

from . import _registry as registry
from ._field import MISSING, FieldInfo
from ._stream import SurpStream
from ._variant import SurpVariant
from .exceptions import SurpFieldError, SurpValidationError
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
    describe_type,
)


def collect_model_errors(instance: Any) -> list[SurpFieldError]:
    errors: list[SurpFieldError] = []
    for name, field in instance.__surp_fields__.items():
        if name not in instance.__surp_values__:
            if field.required:
                errors.append(
                    SurpFieldError(name, describe_type(field.rfc_type), "missing", "field is required")
                )
            continue
        value = instance.__surp_values__[name]
        errors.extend(validate_value(value, field.rfc_type, name, strict=instance.__strict__))
    return errors


def validate_model(instance: Any) -> None:
    errors = collect_model_errors(instance)
    if errors:
        raise SurpValidationError(errors)


def validate_value(value: Any, annotation: Any, path: str, *, strict: bool = True) -> list[SurpFieldError]:
    annotation = _resolve(annotation)
    if isinstance(annotation, _ForwardRef):
        return [_err(path, annotation.describe(), value, f"unresolved forward reference {annotation.name!r}")]
    if isinstance(annotation, _ScalarSentinel):
        return _validate_scalar(value, annotation, path)
    if isinstance(annotation, _TaggedSpec):
        return _validate_tagged(value, annotation, path)
    if isinstance(annotation, _SeqSpec):
        if not isinstance(value, (list, tuple)):
            return [_err(path, annotation.describe(), value, "expected a sequence")]
        errors: list[SurpFieldError] = []
        for idx, item in enumerate(value):
            errors.extend(validate_value(item, annotation.elem, f"{path}[{idx}]", strict=strict))
        return errors
    if isinstance(annotation, _MapSpec):
        if not isinstance(value, dict):
            return [_err(path, annotation.describe(), value, "expected a mapping")]
        errors = []
        for key, item in value.items():
            errors.extend(validate_value(key, annotation.key, f"{path}.<key>", strict=strict))
            errors.extend(validate_value(item, annotation.value, f"{path}.{key}", strict=strict))
        return errors
    if isinstance(annotation, _RefSpec):
        return validate_value(value, annotation.inner, path, strict=strict)
    if isinstance(annotation, _NullableSpec):
        if value is None:
            return []
        return validate_value(value, annotation.inner, path, strict=strict)
    if isinstance(annotation, _OneOfSpec):
        for option in annotation.options:
            if not validate_value(value, option, path, strict=strict):
                return []
        return [_err(path, annotation.describe(), value, "value did not match any OneOf option")]
    if isinstance(annotation, _SumSpec):
        return _validate_sum(value, annotation, path, strict=strict)
    if isinstance(annotation, _TensorSpec):
        return _validate_tensor(value, annotation, path)
    if isinstance(annotation, _StreamSpec):
        if not isinstance(value, SurpStream):
            return [_err(path, annotation.describe(), value, "expected SurpStream")]
        for key, item in value.annotations.items():
            if not isinstance(key, str):
                return [_err(f"{path}.@annotation", "annotation name", key, "expected str")]
            if not _is_annotation_scalar(item):
                return [_err(f"{path}.@{key}", "annotation scalar", item, "expected scalar annotation value")]
        return []
    if _is_symbol_enum(annotation):
        return _validate_symbol_enum(value, annotation, path, strict=strict)
    if isinstance(annotation, type) and hasattr(annotation, "__surp_fields__"):
        if not isinstance(value, annotation):
            return [_err(path, getattr(annotation, "__rfc_type__", annotation.__name__), value, "expected nested SurpModel instance")]
        return [with_prefix(path, error) for error in collect_model_errors(value)]
    return [_err(path, describe_type(annotation), value, "unsupported annotation")]


def with_prefix(prefix: str, error: SurpFieldError) -> SurpFieldError:
    return SurpFieldError(
        f"{prefix}.{error.field_path}",
        error.expected,
        error.got,
        error.message,
    )


def _validate_scalar(value: Any, annotation: _ScalarSentinel, path: str) -> list[SurpFieldError]:
    if annotation is Str:
        if isinstance(value, str):
            return []
        return [_err(path, annotation.describe(), value, "expected str")]
    if annotation is Bool:
        if isinstance(value, bool):
            return []
        return [_err(path, annotation.describe(), value, "expected bool")]
    if annotation is Null:
        if value is None:
            return []
        return [_err(path, annotation.describe(), value, "expected None")]
    if annotation is Unit:
        if value is None or value == ():
            return []
        return [_err(path, annotation.describe(), value, "expected None or ()")]
    if annotation is Bytes:
        if isinstance(value, (bytes, bytearray)):
            return []
        return [_err(path, annotation.describe(), value, "expected bytes")]
    if annotation is Symbol:
        if isinstance(value, str):
            return []
        return [_err(path, annotation.describe(), value, "expected RFC-001 symbol string")]
    if annotation in {F16, BF16, F32, F64}:
        if isinstance(value, (int, float)) and not isinstance(value, bool):
            return []
        return [_err(path, annotation.describe(), value, "expected number")]
    if annotation.rfc_name.startswith("dec"):
        if isinstance(value, (str, int, float)) and not isinstance(value, bool):
            return []
        return [_err(path, annotation.describe(), value, "expected decimal-compatible value")]
    if isinstance(value, int) and not isinstance(value, bool):
        if annotation.min_value is not None and value < annotation.min_value:
            return [_err(path, annotation.describe(), value, "integer below RFC-001 width range")]
        if annotation.max_value is not None and value > annotation.max_value:
            return [_err(path, annotation.describe(), value, "integer above RFC-001 width range")]
        return []
    return [_err(path, annotation.describe(), value, "expected int")]


def _validate_tagged(value: Any, annotation: _TaggedSpec, path: str) -> list[SurpFieldError]:
    return validate_value(value, annotation.inner, path, strict=True)


def _validate_symbol_enum(value: Any, enum_cls: type[Enum], path: str, *, strict: bool) -> list[SurpFieldError]:
    if isinstance(value, enum_cls):
        return []
    if isinstance(value, str):
        if any(member.value == value for member in enum_cls):
            return []
        if not strict:
            return []
    return [_err(path, enum_cls.__name__, value, "expected a declared RFC-001 symbol")]


def _validate_sum(value: Any, annotation: _SumSpec, path: str, *, strict: bool) -> list[SurpFieldError]:
    if isinstance(value, dict) and "variant" in value:
        value = SurpVariant(value["variant"], value.get("payload"))
    if not isinstance(value, SurpVariant):
        return [_err(path, annotation.describe(), value, "expected SurpVariant")]
    variant = next((item for item in annotation.variants if item.name == value.variant), None)
    if variant is None:
        return [_err(path, annotation.describe(), value, f"unknown variant {value.variant!r}")]
    if variant.payload_kind == "unit":
        if value.payload is None:
            return []
        return [_err(path, variant.name, value.payload, "unit variant cannot carry payload")]
    if variant.payload_kind == "tuple":
        expected = variant.payload[0]
        return validate_value(value.payload, expected, f"{path}.{value.variant}", strict=strict)
    if not isinstance(value.payload, dict):
        return [_err(path, variant.name, value.payload, "named variant payload must be dict")]
    errors: list[SurpFieldError] = []
    names = {item[0]: item[1] for item in variant.payload if isinstance(item, tuple)}
    for name, field_type in names.items():
        if name not in value.payload:
            errors.append(_err(f"{path}.{name}", describe_type(field_type), "missing", "payload field is required"))
        else:
            errors.extend(validate_value(value.payload[name], field_type, f"{path}.{name}", strict=strict))
    if strict:
        for name in value.payload:
            if name not in names:
                errors.append(_err(f"{path}.{name}", "no extra payload fields", value.payload[name], "unknown payload field"))
    return errors


def _validate_tensor(value: Any, annotation: _TensorSpec, path: str) -> list[SurpFieldError]:
    if not isinstance(value, (list, tuple)):
        return [_err(path, annotation.describe(), value, "tensor value must be a flat list")]
    expected_count = 1
    dynamic = False
    for dim in annotation.shape:
        if dim is None:
            dynamic = True
            break
        expected_count *= dim
    if not dynamic and len(value) != expected_count:
        return [_err(path, annotation.describe(), value, f"tensor length must be {expected_count}")]
    for idx, item in enumerate(value):
        if not isinstance(item, (int, float)) or isinstance(item, bool):
            return [_err(f"{path}[{idx}]", annotation.dtype.value, item, "tensor element must be numeric")]
    return []


def _resolve(annotation: Any) -> Any:
    if isinstance(annotation, _ForwardRef):
        return registry.get(annotation.name) or annotation
    return annotation


def _is_annotation_scalar(value: Any) -> bool:
    return (
        value is None
        or isinstance(value, (str, bool, float, bytes, bytearray))
        or (isinstance(value, int) and not isinstance(value, bool))
    )


def _is_symbol_enum(annotation: Any) -> bool:
    return isinstance(annotation, type) and issubclass(annotation, Enum) and hasattr(
        annotation, "__surp_symbol_enum__"
    )


def _err(path: str, expected: str, got_value: Any, message: str) -> SurpFieldError:
    return SurpFieldError(path, expected, _got(got_value), message)


def _got(value: Any) -> str:
    if value == "missing":
        return "missing"
    if value is None:
        return "None"
    return type(value).__name__
