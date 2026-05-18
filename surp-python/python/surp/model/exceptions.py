from __future__ import annotations


class SurpModelError(Exception):
    r"""SurpModelError(*args) -> SurpModelError

    Base class for all ``surp.model`` errors.
    """


class SurpModelDefinitionError(SurpModelError):
    r"""SurpModelDefinitionError(*args) -> SurpModelDefinitionError

    Raised when a model class is not a valid RFC-001 schema.
    """


class SurpFieldError(SurpModelError):
    r"""SurpFieldError(field_path, expected, got, message) -> SurpFieldError

    Validation error for one field path.

    Attributes:
        field_path (str): Dot/bracket path to the failing field.
        expected (str): Human-readable expected Surp type expression.
        got (str): Compact description of the received value.
        message (str): Specific validation failure.
    """

    def __init__(self, field_path: str, expected: str, got: str, message: str) -> None:
        r"""__init__(field_path, expected, got, message) -> None

        Create a field validation error.
        """
        self.field_path = field_path
        self.expected = expected
        self.got = got
        self.message = message
        super().__init__(f"{field_path}: expected {expected}, got {got}: {message}")

    def __repr__(self) -> str:
        r"""__repr__() -> str

        Return a developer-oriented representation of the field error.
        """
        return (
            "SurpFieldError("
            f"field_path={self.field_path!r}, expected={self.expected!r}, "
            f"got={self.got!r}, message={self.message!r})"
        )


class SurpValidationError(SurpModelError):
    r"""SurpValidationError(errors) -> SurpValidationError

    Raised when one or more model fields fail validation.
    """

    def __init__(self, errors: list[SurpFieldError]) -> None:
        r"""__init__(errors) -> None

        Create an aggregate validation error from field errors.
        """
        self.errors = errors
        if errors:
            message = "; ".join(error.message for error in errors)
        else:
            message = "validation failed"
        super().__init__(message)


class SurpEncodeModelError(SurpModelError):
    r"""SurpEncodeModelError(*args) -> SurpEncodeModelError

    Raised when a model instance cannot be serialized.
    """


class SurpDecodeModelError(SurpModelError):
    r"""SurpDecodeModelError(*args) -> SurpDecodeModelError

    Raised when CTN, CBF, or v1 Surp bytes cannot decode into the requested model.
    """


class SurpQueryError(SurpModelError):
    r"""SurpQueryError(*args) -> SurpQueryError

    Raised when an RFC-001 CQL query fails.
    """
