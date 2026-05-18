from __future__ import annotations

from typing import Any

from .types import (
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


def schema_ctn(cls: Any) -> str:
    r"""schema_ctn(cls) -> str

    Return a compact CTN schema document for a Surp model class.
    """
    pairs = ", ".join(
        f'"{name}" => "{describe_type(field.rfc_type)}"'
        for name, field in cls.__surp_fields__.items()
    )
    required = ", ".join(
        f'"{name}"'
        for name, field in cls.__surp_fields__.items()
        if field.required
    )
    return (
        "Schema\n"
        f'  type = "{getattr(cls, "__rfc_type__", cls.__name__)}"\n'
        f"  fields = map<str, str> [{pairs}]\n"
        f"  required = [{required}]"
    )


def schema_json(cls: Any) -> dict[str, Any]:
    r"""schema_json(cls) -> dict[str, Any]

    Return a JSON-schema-like dictionary for a Surp model class.
    """
    properties: dict[str, Any] = {}
    required: list[str] = []
    for name, field in cls.__surp_fields__.items():
        properties[name] = _json_schema_for(field.rfc_type)
        if field.doc:
            properties[name]["description"] = field.doc
        if field.required:
            required.append(name)
    return {
        "title": getattr(cls, "__rfc_type__", cls.__name__),
        "type": "object",
        "additionalProperties": not getattr(cls, "__strict__", True),
        "properties": properties,
        "required": required,
    }


def _json_schema_for(annotation: Any) -> dict[str, Any]:
    r"""_json_schema_for(annotation) -> dict[str, Any]

    Convert one Surp annotation descriptor into JSON-schema-like metadata.
    """
    if isinstance(annotation, _ScalarSentinel):
        if annotation.rfc_name == "str" or annotation.rfc_name == "sym":
            return {"type": "string", "x-surp-type": annotation.rfc_name}
        if annotation.rfc_name == "bool":
            return {"type": "boolean", "x-surp-type": annotation.rfc_name}
        if annotation.rfc_name == "bytes":
            return {"type": "string", "contentEncoding": "base64", "x-surp-type": "bytes"}
        if annotation.rfc_name in {"null", "unit"}:
            return {"type": "null", "x-surp-type": annotation.rfc_name}
        if annotation.rfc_name.startswith("f") or annotation.rfc_name == "bf16":
            return {"type": "number", "x-surp-type": annotation.rfc_name}
        return {"type": "integer", "x-surp-type": annotation.rfc_name}
    if isinstance(annotation, _TaggedSpec):
        return {"type": "string", "x-surp-tag": annotation.tag}
    if isinstance(annotation, _SeqSpec):
        return {"type": "array", "items": _json_schema_for(annotation.elem)}
    if isinstance(annotation, _MapSpec):
        return {"type": "object", "additionalProperties": _json_schema_for(annotation.value)}
    if isinstance(annotation, _RefSpec):
        schema = _json_schema_for(annotation.inner)
        schema["x-surp-type"] = annotation.describe()
        return schema
    if isinstance(annotation, _NullableSpec):
        schema = _json_schema_for(annotation.inner)
        schema["nullable"] = True
        return schema
    if isinstance(annotation, _OneOfSpec):
        return {"oneOf": [_json_schema_for(option) for option in annotation.options]}
    if isinstance(annotation, _SumSpec):
        return {"type": "object", "x-surp-type": annotation.describe()}
    if isinstance(annotation, _TensorSpec):
        return {"type": "array", "items": {"type": "number"}, "x-surp-type": annotation.describe()}
    if isinstance(annotation, _StreamSpec):
        return {"type": "object", "additionalProperties": True, "x-surp-type": annotation.describe()}
    if isinstance(annotation, type) and hasattr(annotation, "__surp_fields__"):
        return {"$ref": f"#/components/schemas/{getattr(annotation, '__rfc_type__', annotation.__name__)}"}
    return {"x-surp-type": describe_type(annotation)}
