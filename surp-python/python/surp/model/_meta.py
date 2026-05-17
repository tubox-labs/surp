from __future__ import annotations

import copy
import inspect
import sys
from collections import OrderedDict
from enum import Enum
from typing import Any

from . import _registry as registry
from ._field import Field, FieldInfo, MISSING
from .exceptions import SurpModelDefinitionError
from .types import (
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
    _VariantSpec,
    normalize_annotation,
)


_BUILTIN_REJECTS = {str, int, float, bool, bytes, list, dict, tuple, set}


class SurpModelMeta(type):
    def __new__(
        mcls,
        name: str,
        bases: tuple[type, ...],
        namespace: dict[str, Any],
        **kwargs: Any,
    ) -> type:
        cls: Any = super().__new__(mcls, name, bases, namespace, **kwargs)
        if namespace.get("__surp_base__", False):
            cls.__surp_fields__ = {}
            return cls

        annotations = _resolved_annotations(namespace, cls)
        field_names = {key for key, value in namespace.items() if isinstance(value, Field)}

        inherited: OrderedDict[str, FieldInfo] = OrderedDict()
        for base in bases:
            inherited.update(getattr(base, "__surp_fields__", {}))

        infos: OrderedDict[str, FieldInfo] = OrderedDict(inherited)

        for field_name in field_names:
            if field_name not in annotations:
                raise SurpModelDefinitionError(
                    f"{name}.{field_name} uses Field(...) but has no annotation"
                )

        for field_name, annotation in annotations.items():
            if field_name.startswith("__"):
                continue
            raw_value = namespace.get(field_name, MISSING)
            if not isinstance(raw_value, Field):
                raise SurpModelDefinitionError(
                    f"{name}.{field_name} must use Field(...) on the right-hand side"
                )
            if raw_value.default is not MISSING and isinstance(
                raw_value.default, (list, dict, set)
            ):
                raise SurpModelDefinitionError(
                    f"{name}.{field_name} uses a mutable default; use default_factory"
                )
            normalized = normalize_annotation(annotation)
            _validate_annotation(normalized, owner=cls, field_name=field_name)
            infos[field_name] = raw_value.to_info(field_name, normalized)

        cls.__rfc_type__ = namespace.get("__rfc_type__", name)
        cls.__strict__ = bool(namespace.get("__strict__", True))
        cls.__surp_fields__ = dict(infos)
        cls.__signature__ = _signature_for(cls.__surp_fields__)

        if not getattr(cls, "__surp_is_document__", False):
            registry.register(cls)
        _resolve_known_forward_refs()
        return cls


def _resolved_annotations(namespace: dict[str, Any], cls: type) -> dict[str, Any]:
    raw = namespace.get("__annotations__", {})
    module = sys.modules.get(cls.__module__)
    globalns = dict(vars(module)) if module is not None else {}
    localns = dict(namespace)
    out: dict[str, Any] = {}
    for name, annotation in raw.items():
        if isinstance(annotation, str):
            try:
                out[name] = eval(annotation, globalns, localns)
            except Exception:
                out[name] = _ForwardRef(annotation.strip("\"'"))
        else:
            out[name] = annotation
    return out


def _signature_for(fields: dict[str, FieldInfo]) -> inspect.Signature:
    params = []
    for name, field in fields.items():
        default: Any = inspect.Parameter.empty
        if field.default is not MISSING:
            default = field.default
        elif field.default_factory is not None or not field.required:
            default = None
        params.append(
            inspect.Parameter(
                name,
                inspect.Parameter.KEYWORD_ONLY,
                default=default,
            )
        )
    return inspect.Signature(params, return_annotation=None)


def _validate_annotation(annotation: Any, *, owner: type, field_name: str) -> None:
    if annotation in _BUILTIN_REJECTS:
        raise SurpModelDefinitionError(
            f"{owner.__name__}.{field_name} uses Python built-in type {annotation!r}; "
            "use surp.model.types markers"
        )
    if isinstance(annotation, _ForwardRef):
        return
    if isinstance(annotation, _ScalarSentinel):
        return
    if isinstance(annotation, _TaggedSpec):
        if not isinstance(annotation.inner, (_ScalarSentinel, _ForwardRef)):
            raise SurpModelDefinitionError("Tagged[...] inner type must be scalar")
        return
    if isinstance(annotation, _SeqSpec):
        _validate_annotation(annotation.elem, owner=owner, field_name=field_name)
        return
    if isinstance(annotation, _MapSpec):
        if not _is_scalar_like(annotation.key):
            raise SurpModelDefinitionError("MapOf[...] keys must be scalar types")
        _validate_annotation(annotation.value, owner=owner, field_name=field_name)
        return
    if isinstance(annotation, _RefSpec):
        _validate_annotation(annotation.inner, owner=owner, field_name=field_name)
        return
    if isinstance(annotation, _NullableSpec):
        _validate_annotation(annotation.inner, owner=owner, field_name=field_name)
        return
    if isinstance(annotation, _OneOfSpec):
        for option in annotation.options:
            _validate_annotation(option, owner=owner, field_name=field_name)
        return
    if isinstance(annotation, _SumSpec):
        for variant in annotation.variants:
            if not isinstance(variant, _VariantSpec):
                raise SurpModelDefinitionError("SumOf[...] only accepts Variant[...] entries")
            for payload in variant.payload:
                if isinstance(payload, tuple) and len(payload) == 2 and isinstance(payload[0], str):
                    _validate_annotation(payload[1], owner=owner, field_name=field_name)
                else:
                    _validate_annotation(payload, owner=owner, field_name=field_name)
        return
    if isinstance(annotation, _TensorSpec):
        return
    if isinstance(annotation, _StreamSpec):
        _validate_annotation(annotation.item, owner=owner, field_name=field_name)
        return
    if _is_surp_symbol_enum(annotation):
        return
    if isinstance(annotation, type) and hasattr(annotation, "__surp_fields__"):
        if getattr(annotation, "__surp_is_document__", False):
            raise SurpModelDefinitionError("SurpDocument cannot be used as a nested field")
        return
    raise SurpModelDefinitionError(
        f"{owner.__name__}.{field_name} has unsupported RFC-001 annotation {annotation!r}"
    )


def _is_scalar_like(annotation: Any) -> bool:
    return isinstance(annotation, (_ScalarSentinel, _TaggedSpec, _ForwardRef)) or _is_surp_symbol_enum(
        annotation
    )


def _is_surp_symbol_enum(annotation: Any) -> bool:
    return isinstance(annotation, type) and issubclass(annotation, Enum) and hasattr(
        annotation, "__surp_symbol_enum__"
    )


def _resolve_known_forward_refs() -> None:
    for model_cls in set(registry.all_models().values()):
        cls: Any = model_cls
        changed = False
        fields = copy.copy(getattr(cls, "__surp_fields__", {}))
        for name, info in fields.items():
            resolved = _resolve_type(info.rfc_type)
            if resolved is not info.rfc_type:
                fields[name] = FieldInfo(
                    name=info.name,
                    rfc_type=resolved,
                    required=info.required,
                    default=info.default,
                    default_factory=info.default_factory,
                    doc=info.doc,
                    binding=info.binding,
                )
                changed = True
        if changed:
            cls.__surp_fields__ = fields


def _resolve_type(annotation: Any) -> Any:
    if isinstance(annotation, _ForwardRef):
        return registry.get(annotation.name) or annotation
    if isinstance(annotation, _SeqSpec):
        elem = _resolve_type(annotation.elem)
        if elem is not annotation.elem:
            return _SeqSpec(elem)
    if isinstance(annotation, _MapSpec):
        key = _resolve_type(annotation.key)
        value = _resolve_type(annotation.value)
        if key is not annotation.key or value is not annotation.value:
            return _MapSpec(key, value)
    if isinstance(annotation, _RefSpec):
        inner = _resolve_type(annotation.inner)
        if inner is not annotation.inner:
            return _RefSpec(inner)
    if isinstance(annotation, _NullableSpec):
        inner = _resolve_type(annotation.inner)
        if inner is not annotation.inner:
            return _NullableSpec(inner)
    if isinstance(annotation, _OneOfSpec):
        options = tuple(_resolve_type(option) for option in annotation.options)
        if options != annotation.options:
            return _OneOfSpec(options)
    if isinstance(annotation, _TaggedSpec):
        inner = _resolve_type(annotation.inner)
        if inner is not annotation.inner:
            return _TaggedSpec(annotation.tag, inner)
    if isinstance(annotation, _StreamSpec):
        item = _resolve_type(annotation.item)
        if item is not annotation.item:
            return _StreamSpec(item)
    return annotation
