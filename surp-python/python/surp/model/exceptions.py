from __future__ import annotations


class SurpModelError(Exception):
    """Base class for all surp.model errors."""


class SurpModelDefinitionError(SurpModelError):
    """Raised when a model class is not a valid RFC-001 schema."""


class SurpFieldError(SurpModelError):
    def __init__(self, field_path: str, expected: str, got: str, message: str) -> None:
        self.field_path = field_path
        self.expected = expected
        self.got = got
        self.message = message
        super().__init__(f"{field_path}: expected {expected}, got {got}: {message}")

    def __repr__(self) -> str:
        return (
            "SurpFieldError("
            f"field_path={self.field_path!r}, expected={self.expected!r}, "
            f"got={self.got!r}, message={self.message!r})"
        )


class SurpValidationError(SurpModelError):
    def __init__(self, errors: list[SurpFieldError]) -> None:
        self.errors = errors
        if errors:
            message = "; ".join(error.message for error in errors)
        else:
            message = "validation failed"
        super().__init__(message)


class SurpEncodeModelError(SurpModelError):
    """Raised when a model instance cannot be serialized."""


class SurpDecodeModelError(SurpModelError):
    """Raised when CTN/CBF cannot be decoded into the requested model."""


class SurpQueryError(SurpModelError):
    """Raised when an RFC-001 CQL query fails."""
