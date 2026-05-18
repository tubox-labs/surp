from __future__ import annotations

from typing import Any, Callable


MISSING = object()


class FieldInfo:
    r"""FieldInfo(name, rfc_type, required, default=MISSING, default_factory=None, doc=None, binding=None) -> FieldInfo

    Immutable metadata captured for a single ``SurpModel`` field.

    ``FieldInfo`` is produced by ``Field.to_info`` during class creation and
    stored in ``Model.__surp_fields__``. Runtime code uses it to validate,
    encode, decode, and describe a field without mutating the original
    descriptor.
    """

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
        r"""__init__(name, rfc_type, required, default=MISSING, default_factory=None, doc=None, binding=None) -> None

        Create immutable field metadata from a descriptor and annotation.

        Args:
            name (str): Field name on the owning model class.
            rfc_type (Any): Normalized Surp annotation descriptor.
            required (bool): Whether callers must provide the field.
            default (Any, optional): Static default value. Default: ``MISSING``
            default_factory (Callable[[], Any], optional): Factory for dynamic
              defaults. Default: ``None``
            doc (str, optional): Field documentation. Default: ``None``
            binding (str, optional): CTN document binding name. Default:
              ``None``
        """
        object.__setattr__(self, "name", name)
        object.__setattr__(self, "rfc_type", rfc_type)
        object.__setattr__(self, "required", required)
        object.__setattr__(self, "default", default)
        object.__setattr__(self, "default_factory", default_factory)
        object.__setattr__(self, "doc", doc)
        object.__setattr__(self, "binding", binding)

    def __setattr__(self, name: str, value: Any) -> None:
        r"""__setattr__(name, value) -> None

        Reject mutation after class creation.
        """
        raise AttributeError("FieldInfo is immutable")

    def __repr__(self) -> str:
        r"""__repr__() -> str

        Return a developer-oriented representation of the field metadata.
        """
        return (
            "FieldInfo("
            f"name={self.name!r}, rfc_type={self.rfc_type!r}, "
            f"required={self.required!r}, default={self.default!r}, "
            f"default_factory={self.default_factory!r}, doc={self.doc!r}, "
            f"binding={self.binding!r})"
        )

    def __eq__(self, other: Any) -> bool:
        r"""__eq__(other) -> bool

        Compare two field metadata objects by their stored attributes.
        """
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
    r"""Field(*, required=True, default=MISSING, default_factory=None, doc=None, binding=None) -> Field

    Descriptor used to declare Surp model fields.

    Args:
        required (bool, optional): Whether the field is required. Default:
          ``True``
        default (Any, optional): Static default value. Default: ``MISSING``
        default_factory (Callable[[], Any], optional): Callable used to create
          a fresh default per instance. Default: ``None``
        doc (str, optional): Field-level documentation for schemas. Default:
          ``None``
        binding (str, optional): CTN document binding override. Default:
          ``None``

    Examples::

        >>> class User(SurpModel):
        ...     name: Str = Field(required=True, doc="Display name")
    """

    def __init__(
        self,
        *,
        required: bool = True,
        default: Any = MISSING,
        default_factory: Callable[[], Any] | None = None,
        doc: str | None = None,
        binding: str | None = None,
    ) -> None:
        r"""__init__(*, required=True, default=MISSING, default_factory=None, doc=None, binding=None) -> None

        Create a field descriptor with validation and encoding options.
        """
        if default is not MISSING and default_factory is not None:
            raise TypeError("Field cannot specify both default and default_factory")
        self.required = required
        self.default = default
        self.default_factory = default_factory
        self.doc = doc
        self.binding = binding
        self.name: str | None = None

    def __set_name__(self, owner: type, name: str) -> None:
        r"""__set_name__(owner, name) -> None

        Remember the attribute name assigned by the owning class.
        """
        self.name = name

    def __get__(self, instance: Any, owner: type | None = None) -> Any:
        r"""__get__(instance, owner=None) -> Any

        Return the descriptor on the class or the stored value on instances.
        """
        if instance is None:
            return self
        return instance.__surp_values__.get(self.name)

    def __set__(self, instance: Any, value: Any) -> None:
        r"""__set__(instance, value) -> None

        Store a field value on a model instance.
        """
        instance.__surp_values__[self.name] = value

    def to_info(self, name: str, rfc_type: Any) -> FieldInfo:
        r"""to_info(name, rfc_type) -> FieldInfo

        Freeze this descriptor into ``FieldInfo`` for a normalized type.
        """
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
    r"""FieldFactory(*, required=True, default=MISSING, default_factory=None, doc=None, binding=None) -> Field

    Create a ``Field`` descriptor.

    This factory is kept for code that wants a callable helper while preserving
    the public ``Field(...)`` class constructor.
    """
    return Field(
        required=required,
        default=default,
        default_factory=default_factory,
        doc=doc,
        binding=binding,
    )
