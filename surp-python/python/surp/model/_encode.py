from __future__ import annotations

import base64
from enum import Enum
from typing import Any

from ._field import MISSING
from ._stream import SurpStream
from ._variant import SurpVariant
from .exceptions import SurpEncodeModelError
from .types import (
    BF16,
    Bytes,
    Dec128,
    Dec32,
    Dec64,
    F16,
    F32,
    F64,
    Null,
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
from ._validate import validate_model
from ._validate import validate_value


def model_to_ctn(instance: Any, *, indent: int = 2) -> str:
    validate_model(instance)
    text = _document_to_ctn(instance) if getattr(instance, "__surp_is_document__", False) else _value_to_ctn(instance, _model_type(instance), 0)
    try:
        from surp import rfc001

        normalized = rfc001.normalize_ctn(text)
        if rfc001.normalize_ctn(normalized) != normalized:
            raise SurpEncodeModelError("surp.rfc001.normalize_ctn was not idempotent")
        return normalized
    except SurpEncodeModelError:
        raise
    except Exception as exc:  # pragma: no cover - depends on native package availability
        raise SurpEncodeModelError(str(exc)) from exc


def model_to_cbf(instance: Any, *, alignment: int = 0, with_symtab: bool = True) -> bytes:
    try:
        from surp import rfc001

        return rfc001.compile_ctn(instance.to_ctn(), alignment=alignment, with_symtab=with_symtab)
    except Exception as exc:  # pragma: no cover - depends on native package availability
        raise SurpEncodeModelError(str(exc)) from exc


def model_to_surp(instance: Any, *, dedup: bool = True, sort_keys: bool = True) -> bytes:
    try:
        import surp

        return surp.dumps(instance.to_dict(), dedup=dedup, sort_keys=sort_keys)
    except Exception as exc:  # pragma: no cover - depends on native package availability
        raise SurpEncodeModelError(str(exc)) from exc


def _document_to_ctn(instance: Any) -> str:
    lines: list[str] = []
    for name, value in getattr(instance, "__surp_annotations__", []):
        if value is None:
            lines.append(f"@{name}")
        else:
            lines.append(f"@{name} {_scalar_literal(value, _annotation_scalar_type(value))}")
    root_binding = getattr(instance, "__root__", None)
    first_binding: str | None = None
    for field_name, field in instance.__surp_fields__.items():
        if field_name not in instance.__surp_values__:
            continue
        if not field.required and _is_default_value(instance.__surp_values__[field_name], field):
            continue
        binding = field.binding or field_name
        if first_binding is None:
            first_binding = binding
        value = instance.__surp_values__[field_name]
        rendered = _value_to_ctn(value, field.rfc_type, 0)
        first, *rest = rendered.splitlines()
        lines.append(f"let {binding} = {first}")
        lines.extend(rest)
    root = root_binding or first_binding
    if root:
        lines.append("")
        lines.append(f"&{root}")
    return "\n".join(lines)


def _value_to_ctn(value: Any, annotation: Any, level: int) -> str:
    annotation = _resolve(annotation)
    if isinstance(annotation, _NullableSpec):
        if value is None:
            return "null"
        return _value_to_ctn(value, annotation.inner, level)
    if isinstance(annotation, _OneOfSpec):
        for option in annotation.options:
            if not validate_value(value, option, "$", strict=True):
                return _value_to_ctn(value, option, level)
        return _scalar_literal(value, _ScalarSentinel("str"))
    if isinstance(annotation, _TaggedSpec):
        return f"{annotation.tag}{_quote(str(value))}"
    if isinstance(annotation, _ScalarSentinel):
        return _scalar_literal(value, annotation)
    if isinstance(annotation, _SeqSpec):
        items = list(value)
        if not items:
            return "[]"
        if _all_inline(items, annotation.elem):
            return "[" + ", ".join(_value_to_ctn(item, annotation.elem, level) for item in items) + "]"
        lines = [f"seq<{_type_expr(annotation.elem)}>"]
        for item in items:
            lines.extend(_indent_block(_value_to_ctn(item, annotation.elem, level + 2)))
        return "\n".join(lines)
    if isinstance(annotation, _MapSpec):
        if not value:
            return f"map<{_type_expr(annotation.key)}, {_type_expr(annotation.value)}> []"
        lines = [f"map<{_type_expr(annotation.key)}, {_type_expr(annotation.value)}>"]
        for key, item in value.items():
            key_text = _value_to_ctn(key, annotation.key, level + 2)
            item_text = _value_to_ctn(item, annotation.value, level + 2)
            first, *rest = item_text.splitlines()
            lines.append(f"  {key_text} => {first}")
            lines.extend("  " + line for line in rest)
        return "\n".join(lines)
    if isinstance(annotation, _RefSpec):
        return f"ref {_value_to_ctn(value, annotation.inner, level)}"
    if isinstance(annotation, _SumSpec):
        return _sum_to_ctn(value, annotation, level)
    if isinstance(annotation, _TensorSpec):
        return _tensor_to_ctn(value, annotation)
    if isinstance(annotation, _StreamSpec):
        return _stream_to_ctn(value, annotation)
    if _is_symbol_enum(annotation):
        raw = value.value if isinstance(value, annotation) else str(value)
        return "'" + raw
    if isinstance(annotation, type) and hasattr(annotation, "__surp_fields__"):
        return _model_to_ctn(value, level)
    if isinstance(annotation, _ForwardRef):
        raise SurpEncodeModelError(f"unresolved forward reference {annotation.name!r}")
    raise SurpEncodeModelError(f"unsupported RFC-001 annotation {annotation!r}")


def _model_to_ctn(instance: Any, level: int) -> str:
    lines = [getattr(instance, "__rfc_type__", instance.__class__.__name__)]
    for name, field in instance.__surp_fields__.items():
        if name not in instance.__surp_values__:
            continue
        if not field.required and _is_default_value(instance.__surp_values__[name], field):
            continue
        value_text = _value_to_ctn(instance.__surp_values__[name], field.rfc_type, level + 2)
        first, *rest = value_text.splitlines()
        lines.append(f"  {name} = {first}")
        lines.extend("  " + line for line in rest)
    return "\n".join(lines)


def _sum_to_ctn(value: Any, annotation: _SumSpec, level: int) -> str:
    if isinstance(value, dict):
        value = SurpVariant(value["variant"], value.get("payload"))
    variant = next(item for item in annotation.variants if item.name == value.variant)
    type_name = "Sum"
    if variant.payload_kind == "unit":
        return f"{type_name} :: {variant.name}"
    if variant.payload_kind == "tuple":
        tuple_payload = _value_to_ctn(value.payload, variant.payload[0], level)
        return f"{type_name} :: {variant.name}({tuple_payload})"
    lines = [f"{type_name} :: {variant.name}"]
    payload_map = value.payload or {}
    for name, field_type in variant.payload:
        rendered = _value_to_ctn(payload_map[name], field_type, level + 2)
        first, *rest = rendered.splitlines()
        lines.append(f"  {name} = {first}")
        lines.extend("  " + line for line in rest)
    return "\n".join(lines)


def _tensor_to_ctn(value: Any, annotation: _TensorSpec) -> str:
    dims = ", ".join("_" if dim is None else str(dim) for dim in annotation.shape)
    suffix = "[" + dims + "]" if annotation.shape else ""
    values = ", ".join(_tensor_number(item, annotation) for item in value)
    return f"tensor<{annotation.dtype.value}>{suffix}\n  [{values}]"


def _stream_to_ctn(value: SurpStream, annotation: _StreamSpec) -> str:
    lines = [f"stream<{_type_expr(annotation.item)}>"]
    for name, item in value.annotations.items():
        if item is None:
            lines.append(f"  @{name}")
        else:
            lines.append(f"  @{name} {_scalar_literal(item, _annotation_scalar_type(item))}")
    return "\n".join(lines)


def _scalar_literal(value: Any, annotation: _ScalarSentinel) -> str:
    if annotation is Null:
        return "null"
    if annotation is Unit:
        return "unit"
    if annotation.rfc_name == "bool":
        return "true" if value else "false"
    if annotation.rfc_name == "str":
        return _quote(str(value))
    if annotation.rfc_name == "bytes":
        raw = bytes(value)
        return 'b64"' + base64.b64encode(raw).decode("ascii") + '"'
    if annotation.rfc_name == "sym":
        return "'" + str(value)
    if annotation.rfc_name in {"i8", "i16", "i32", "i64", "u8", "u16", "u32", "u64", "vi32", "vi64", "vu32", "vu64"}:
        return f"{int(value)}{annotation.rfc_name}"
    if annotation.rfc_name in {"f16", "bf16", "f32", "f64"}:
        return f"{float(value)!r}{annotation.rfc_name}"
    if annotation in {Dec32, Dec64, Dec128}:
        return f"{value}{annotation.rfc_name}"
    return str(value)


def _tensor_number(value: Any, annotation: _TensorSpec) -> str:
    dtype = annotation.dtype.value
    if dtype.startswith("f") or dtype == "bf16":
        return f"{float(value)!r}{dtype}"
    return f"{int(value)}{dtype}"


def _type_expr(annotation: Any) -> str:
    annotation = _resolve(annotation)
    if isinstance(annotation, _ScalarSentinel):
        if annotation.rfc_name.startswith("v"):
            return annotation.rfc_name
        if annotation.rfc_name == "sym":
            return "sym"
        return annotation.rfc_name
    if isinstance(annotation, _TaggedSpec):
        return annotation.tag
    if isinstance(annotation, _SeqSpec):
        return f"seq<{_type_expr(annotation.elem)}>"
    if isinstance(annotation, _MapSpec):
        return f"map<{_type_expr(annotation.key)}, {_type_expr(annotation.value)}>"
    if isinstance(annotation, _RefSpec):
        return f"ref<{_type_expr(annotation.inner)}>"
    if isinstance(annotation, type) and hasattr(annotation, "__surp_fields__"):
        return getattr(annotation, "__rfc_type__", annotation.__name__)
    if isinstance(annotation, _StreamSpec):
        return f"stream<{_type_expr(annotation.item)}>"
    if _is_symbol_enum(annotation):
        return "sym"
    return "any"


def _annotation_scalar_type(value: Any) -> _ScalarSentinel:
    if isinstance(value, bool):
        return _ScalarSentinel("bool")
    if isinstance(value, int):
        return _ScalarSentinel("vi64")
    if isinstance(value, float):
        return F64
    return _ScalarSentinel("str")


def _model_type(instance: Any) -> type:
    return instance.__class__


def _resolve(annotation: Any) -> Any:
    if isinstance(annotation, _ForwardRef):
        from . import _registry as registry

        return registry.get(annotation.name) or annotation
    return annotation


def _all_inline(values: list[Any], annotation: Any) -> bool:
    if len(values) > 8:
        return False
    annotation = _resolve(annotation)
    return isinstance(annotation, (_ScalarSentinel, _TaggedSpec, _NullableSpec)) or _is_symbol_enum(annotation)


def _indent_block(text: str) -> list[str]:
    return ["  " + line for line in text.splitlines()]


def _quote(value: str) -> str:
    return '"' + value.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n").replace("\r", "\\r").replace("\t", "\\t") + '"'


def _is_symbol_enum(annotation: Any) -> bool:
    return isinstance(annotation, type) and issubclass(annotation, Enum) and hasattr(
        annotation, "__surp_symbol_enum__"
    )


def _is_default_value(value: Any, field: Any) -> bool:
    if field.default is not MISSING:
        return value == field.default
    if field.default_factory is not None:
        try:
            return value == field.default_factory()
        except Exception:
            return False
    return False
