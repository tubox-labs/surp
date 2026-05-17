from __future__ import annotations

from typing import Any, Callable


MISSING = object()


class FieldInfo:
    name: str
    rfc_type: Any
    required: bool
    default: Any
    default_factory: Callable[[], Any] | None
    doc: str | None
    binding: str | None

    __slots__ = (
        "name",
        "rfc_type",
        "required",
        "default",
        "default_factory",
        "doc",
        "binding",
    )

    def __init__(
        self,
        name: str,
        rfc_type: Any,
        required: bool,
        default: Any = MISSING,
        default_factory: Callable[[], Any] | None = None,
        doc: str | None = None,
        binding: str | None = None,
    ) -> None:
        object.__setattr__(self, "name", name)
        object.__setattr__(self, "rfc_type", rfc_type)
        object.__setattr__(self, "required", required)
        object.__setattr__(self, "default", default)
        object.__setattr__(self, "default_factory", default_factory)
        object.__setattr__(self, "doc", doc)
        object.__setattr__(self, "binding", binding)

    def __setattr__(self, name: str, value: Any) -> None:
        raise AttributeError("FieldInfo is immutable")

    def __repr__(self) -> str:
        return (
            "FieldInfo("
            f"name={self.name!r}, rfc_type={self.rfc_type!r}, "
            f"required={self.required!r}, default={self.default!r}, "
            f"default_factory={self.default_factory!r}, doc={self.doc!r}, "
            f"binding={self.binding!r})"
        )

    def __eq__(self, other: Any) -> bool:
        return (
            isinstance(other, FieldInfo)
            and self.name == other.name
            and self.rfc_type == other.rfc_type
            and self.required == other.required
            and self.default == other.default
            and self.default_factory == other.default_factory
            and self.doc == other.doc
            and self.binding == other.binding
        )


class Field:
    def __init__(
        self,
        *,
        required: bool = True,
        default: Any = MISSING,
        default_factory: Callable[[], Any] | None = None,
        doc: str | None = None,
        binding: str | None = None,
    ) -> None:
        if default is not MISSING and default_factory is not None:
            raise TypeError("Field cannot specify both default and default_factory")
        self.required = required
        self.default = default
        self.default_factory = default_factory
        self.doc = doc
        self.binding = binding
        self.name: str | None = None

    def __set_name__(self, owner: type, name: str) -> None:
        self.name = name

    def __get__(self, instance: Any, owner: type | None = None) -> Any:
        if instance is None:
            return self
        return instance.__surp_values__.get(self.name)

    def __set__(self, instance: Any, value: Any) -> None:
        instance.__surp_values__[self.name] = value

    def to_info(self, name: str, rfc_type: Any) -> FieldInfo:
        return FieldInfo(
            name=name,
            rfc_type=rfc_type,
            required=self.required,
            default=self.default,
            default_factory=self.default_factory,
            doc=self.doc,
            binding=self.binding,
        )


def FieldFactory(
    *,
    required: bool = True,
    default: Any = MISSING,
    default_factory: Callable[[], Any] | None = None,
    doc: str | None = None,
    binding: str | None = None,
) -> Field:
    return Field(
        required=required,
        default=default,
        default_factory=default_factory,
        doc=doc,
        binding=binding,
    )
