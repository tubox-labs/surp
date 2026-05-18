from __future__ import annotations

from typing import Any


class SurpVariant:
    r"""SurpVariant(variant, payload=None) -> SurpVariant

    Immutable value for a field declared with ``SumOf[...]``.

    ``variant`` names the selected RFC-001 sum branch. ``payload`` is ``None``
    for unit variants, a scalar/object for tuple variants, or a dictionary for
    named struct variants.

    Args:
        variant (str): Selected variant name.
        payload (dict[str, Any] or Any, optional): Variant payload. Default:
          ``None``
    """

    variant: str
    payload: dict[str, Any] | Any | None

    __slots__ = ("variant", "payload")

    def __init__(self, variant: str, payload: dict[str, Any] | Any | None = None) -> None:
        r"""__init__(variant, payload=None) -> None

        Create a selected sum variant with its optional payload.
        """
        object.__setattr__(self, "variant", variant)
        object.__setattr__(self, "payload", payload)

    def __setattr__(self, name: str, value: Any) -> None:
        r"""__setattr__(name, value) -> None

        Reject mutation after construction.
        """
        raise AttributeError("SurpVariant is immutable")

    def __repr__(self) -> str:
        r"""__repr__() -> str

        Return a constructor-style representation.
        """
        return f"SurpVariant(variant={self.variant!r}, payload={self.payload!r})"

    def __eq__(self, other: Any) -> bool:
        r"""__eq__(other) -> bool

        Compare variants by selected branch and payload.
        """
        return (
            isinstance(other, SurpVariant)
            and self.variant == other.variant
            and self.payload == other.payload
        )
