from __future__ import annotations

from enum import Enum
from pathlib import Path
from typing import Any

from ._field import MISSING
from .types import (
    BF16,
    F16,
    F32,
    F64,
    Bytes,
    Bool,
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
    TensorDType,
)


def generate_model_stubs(*classes: type) -> str:
    r"""generate_model_stubs(*classes) -> str

    Generate static ``.pyi`` constructor stubs for model classes.

    The generated stubs expose plain Python value types, not runtime Surp
    markers, so editors can understand model initialization.

    Args:
        *classes (type): Model classes to include in the generated stub text.

    Returns:
        str: Complete ``.pyi`` source text.
    """
    lines = [
        "from __future__ import annotations",
        "",
        "from typing import Any",
        "",
        "from surp.model import SurpDocument, SurpModel, SurpStream, SurpVariant",
        "",
    ]
    for cls in classes:
        base = "SurpDocument" if getattr(cls, "__surp_is_document__", False) else "SurpModel"
        lines.append(f"class {cls.__name__}({base}):")
        params = _init_params(cls)
        if params:
            joined = ", ".join(params)
            lines.append(f"    def __init__(self, *, {joined}) -> None: ...")
        else:
            lines.append("    def __init__(self) -> None: ...")
        lines.append("")
    return "\n".join(lines).rstrip() + "\n"


def write_model_stubs(path: str | Path, *classes: type) -> None:
    r"""write_model_stubs(path, *classes) -> None

    Write generated model stubs to ``path`` using UTF-8.
    """
    Path(path).write_text(generate_model_stubs(*classes), encoding="utf-8")


def _init_params(cls: Any) -> list[str]:
    r"""_init_params(cls) -> list[str]

    Return typed keyword-only ``__init__`` parameters for a model class.
    """
    params: list[str] = []
    for name, field in cls.__surp_fields__.items():
        annotation = _python_type_expr(field.rfc_type)
        suffix = "" if field.required and field.default is MISSING and field.default_factory is None else " = ..."
        params.append(f"{name}: {annotation}{suffix}")
    return params


def _python_type_expr(annotation: Any) -> str:
    r"""_python_type_expr(annotation) -> str

    Convert a Surp annotation descriptor to a Python type expression.
    """
    if isinstance(annotation, _ForwardRef):
        return _quote_type(annotation.name)
    if isinstance(annotation, _TaggedSpec):
        return _python_type_expr(annotation.inner)
    if isinstance(annotation, _ScalarSentinel):
        return _scalar_type(annotation)
    if isinstance(annotation, _SeqSpec):
        return f"list[{_python_type_expr(annotation.elem)}]"
    if isinstance(annotation, _MapSpec):
        return f"dict[{_python_type_expr(annotation.key)}, {_python_type_expr(annotation.value)}]"
    if isinstance(annotation, _RefSpec):
        return _python_type_expr(annotation.inner)
    if isinstance(annotation, _NullableSpec):
        return f"{_python_type_expr(annotation.inner)} | None"
    if isinstance(annotation, _OneOfSpec):
        return " | ".join(_python_type_expr(option) for option in annotation.options)
    if isinstance(annotation, _SumSpec):
        return "SurpVariant"
    if isinstance(annotation, _TensorSpec):
        if annotation.dtype in {
            TensorDType.F16,
            TensorDType.BF16,
            TensorDType.F32,
            TensorDType.F64,
        }:
            return "list[float]"
        return "list[int]"
    if isinstance(annotation, _StreamSpec):
        return "SurpStream"
    if _is_symbol_enum(annotation) or (
        isinstance(annotation, type) and hasattr(annotation, "__surp_fields__")
    ):
        return _quote_type(annotation.__name__)
    return "Any"


def _scalar_type(annotation: _ScalarSentinel) -> str:
    r"""_scalar_type(annotation) -> str

    Map a scalar marker to the Python value type used by callers.
    """
    if annotation is Str or annotation is Symbol:
        return "str"
    if annotation is Bool:
        return "bool"
    if annotation is Bytes:
        return "bytes"
    if annotation is Null or annotation is Unit:
        return "None"
    if annotation in {F16, BF16, F32, F64}:
        return "float"
    if annotation.rfc_name.startswith("dec"):
        return "str | int | float"
    return "int"


def _quote_type(name: str) -> str:
    r"""_quote_type(name) -> str

    Quote a forward type name for generated stub output.
    """
    return f'"{name}"'


def _is_symbol_enum(annotation: Any) -> bool:
    r"""_is_symbol_enum(annotation) -> bool

    Return true for ``SurpSymbolEnum`` subclasses.
    """
    return isinstance(annotation, type) and issubclass(annotation, Enum) and hasattr(
        annotation, "__surp_symbol_enum__"
    )
