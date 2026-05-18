from __future__ import annotations

from typing import Any


class SurpStream:
    r"""SurpStream(annotations=None) -> SurpStream

    Value for a field declared with ``StreamOf[...]``.

    RFC-001 streams currently carry item type metadata and annotations. The
    Python model representation stores those annotations as a plain dictionary.

    Args:
        annotations (dict[str, Any], optional): Stream annotation values keyed
          by annotation name. Default: ``None``
    """

    annotations: dict[str, Any]

    __slots__ = ("annotations",)

    def __init__(self, annotations: dict[str, Any] | None = None) -> None:
        r"""__init__(annotations=None) -> None

        Create stream metadata from optional annotation values.
        """
        self.annotations = dict(annotations or {})

    def __repr__(self) -> str:
        r"""__repr__() -> str

        Return a constructor-style representation.
        """
        return f"SurpStream(annotations={self.annotations!r})"

    def __eq__(self, other: Any) -> bool:
        r"""__eq__(other) -> bool

        Compare stream values by annotation mapping.
        """
        return isinstance(other, SurpStream) and self.annotations == other.annotations
