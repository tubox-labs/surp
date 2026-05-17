from __future__ import annotations

from typing import Any


class SurpStream:
    annotations: dict[str, Any]

    __slots__ = ("annotations",)

    def __init__(self, annotations: dict[str, Any] | None = None) -> None:
        self.annotations = dict(annotations or {})

    def __repr__(self) -> str:
        return f"SurpStream(annotations={self.annotations!r})"

    def __eq__(self, other: Any) -> bool:
        return isinstance(other, SurpStream) and self.annotations == other.annotations
