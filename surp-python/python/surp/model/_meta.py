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
    r"""SurpModelMeta(name, bases, namespace, **kwargs) -> type

    Metaclass that turns Surp model annotations into runtime field metadata.

    The metaclass is intentionally strict: every declared field must use a
    Surp type marker annotation and a ``Field(...)`` descriptor. During class
    creation it resolves Python 3.14 lazy annotations, validates RFC-001 type
    descriptors, registers concrete models, and prepares an inspectable
    keyword-only constructor signature.

    Examples::

        >>> class User(SurpModel):
        ...     name: Str = Field(required=True)
        >>> User.__surp_fields__["name"].rfc_type.describe()
        'str'
    """

    def __new__(
        mcls,
        name: str,
        bases: tuple[type, ...],
        namespace: dict[str, Any],
        **kwargs: Any,
    ) -> type:
        r"""__new__(mcls, name, bases, namespace, **kwargs) -> type

        Create a Surp model class and collect validated field definitions.
        """
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
    r"""_resolved_annotations(namespace, cls) -> dict[str, Any]

    Return class-local annotations with Python 3.14 lazy annotation support.

    Python 3.14 stores ordinary class annotations lazily instead of always
    materializing ``__annotations__`` in the class namespace. ``annotationlib``
    is therefore used when ``namespace`` has no eager annotations. String
    annotations are still evaluated against the model module and class
    namespace so existing ``from __future__ import annotations`` files retain
    their behavior.
    """
    module = sys.modules.get(cls.__module__)
    globalns = dict(vars(module)) if module is not None else {}
    localns = dict(namespace)
    raw = namespace.get("__annotations__", {}) or _annotationlib_annotations(
        cls,
        globalns,
        localns,
    )
    out: dict[str, Any] = {}
    for name, annotation in raw.items():
        if isinstance(annotation, str):
            try:
                out[name] = eval(annotation, globalns, localns)
            except Exception:
                out[name] = _ForwardRef(annotation.strip("\"'"))
        elif isinstance(getattr(annotation, "__forward_arg__", None), str):
            forward_arg = annotation.__forward_arg__.strip("\"'")
            try:
                out[name] = eval(forward_arg, globalns, localns)
            except Exception:
                out[name] = _ForwardRef(forward_arg)
        else:
            out[name] = annotation
    return out


def _annotationlib_annotations(
    cls: type,
    globalns: dict[str, Any],
    localns: dict[str, Any],
) -> dict[str, Any]:
    r"""_annotationlib_annotations(cls, globalns, localns) -> dict[str, Any]

    Read Python 3.14 lazy class annotations when available.

    Returns an empty dictionary on older Python versions or when annotation
    extraction fails, leaving the caller to report the normal model definition
    errors.
    """
    try:
        import annotationlib
    except Exception:
        return {}
    try:
        return dict(
            annotationlib.get_annotations(
                cls,
                globals=globalns,
                locals=localns,
                format=annotationlib.Format.FORWARDREF,
            )
        )
    except Exception:
        return {}


def _signature_for(fields: dict[str, FieldInfo]) -> inspect.Signature:
    r"""_signature_for(fields) -> inspect.Signature

    Build the keyword-only ``inspect.Signature`` exposed by model classes.
    """
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
    r"""_validate_annotation(annotation, *, owner, field_name) -> None

    Validate that an annotation is a supported Surp RFC-001 descriptor.
    """
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
    r"""_is_scalar_like(annotation) -> bool

    Return true for annotations allowed as RFC-001 association keys.
    """
    return isinstance(annotation, (_ScalarSentinel, _TaggedSpec, _ForwardRef)) or _is_surp_symbol_enum(
        annotation
    )


def _is_surp_symbol_enum(annotation: Any) -> bool:
    r"""_is_surp_symbol_enum(annotation) -> bool

    Return true for enums declared as Surp symbol enums.
    """
    return isinstance(annotation, type) and issubclass(annotation, Enum) and hasattr(
        annotation, "__surp_symbol_enum__"
    )


def _resolve_known_forward_refs() -> None:
    r"""_resolve_known_forward_refs() -> None

    Resolve forward references across all models already in the registry.
    """
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
    r"""_resolve_type(annotation) -> Any

    Resolve nested ``_ForwardRef`` instances inside composite descriptors.
    """
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
