from __future__ import annotations

from typing import Any


class SurpVariant:
    variant: str
    payload: dict[str, Any] | Any | None

    __slots__ = ("variant", "payload")

    def __init__(self, variant: str, payload: dict[str, Any] | Any | None = None) -> None:
        object.__setattr__(self, "variant", variant)
        object.__setattr__(self, "payload", payload)

    def __setattr__(self, name: str, value: Any) -> None:
        raise AttributeError("SurpVariant is immutable")

    def __repr__(self) -> str:
        return f"SurpVariant(variant={self.variant!r}, payload={self.payload!r})"

    def __eq__(self, other: Any) -> bool:
        return (
            isinstance(other, SurpVariant)
            and self.variant == other.variant
            and self.payload == other.payload
        )
