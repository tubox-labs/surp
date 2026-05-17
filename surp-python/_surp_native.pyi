"""
Type stubs for the ``surp-native`` extension module.

This file provides IDE support (auto-complete, type-checking, inline docs)
for the Rust-backed ``_surp_native`` module built with PyO3.
"""

from __future__ import annotations

from os import PathLike
from typing import Any, BinaryIO, Literal, Union

__version__: str
"""Package version string (e.g. ``'1.1.3'``)."""

# ---------------------------------------------------------------------------
# Exception Hierarchy
# ---------------------------------------------------------------------------

class SurpError(Exception):
    """Base exception for all Surp errors."""
    ...

class SurpEncodeError(SurpError):
    """Error during encoding."""
    ...

class SurpDecodeError(SurpError):
    """Error during decoding."""
    ...

class SurpChecksumError(SurpDecodeError):
    """Checksum verification failed during decoding."""
    ...

class SurpTypeError(SurpEncodeError):
    """Type cannot be serialized to Surp format."""
    ...

# ---------------------------------------------------------------------------
# JSON-like API
# ---------------------------------------------------------------------------

def dumps(
    obj: Any,
    *,
    compression: Literal["lz4", "zstd", "snappy", "none"] | None = None,
    dedup: bool = False,
    sort_keys: bool = False,
) -> bytes:
    """Serialize a Python object to Surp binary format.

    This is the JSON-like API with options. Similar to ``json.dumps()``.

    Args:
        obj: The Python object to encode (dict, list, str, int, float,
            bytes, bool, None).
        compression: Compression algorithm: ``None``, ``"lz4"``, ``"zstd"``,
            or ``"snappy"``. Default is no compression.
        dedup: Enable string deduplication. Default ``False``.
        sort_keys: Sort object keys alphabetically for canonical output.
            Default ``False``.

    Returns:
        The encoded Surp binary data as ``bytes``.

    Raises:
        SurpEncodeError: If encoding fails.
        SurpTypeError: If an unsupported type is encountered.

    Example::

        >>> import surp
        >>> data = surp.dumps({"hello": "world"}, compression="lz4", dedup=True)
    """
    ...

def loads(
    data: bytes,
    *,
    strict: bool = True,
    max_depth: int = 128,
) -> Any:
    """Deserialize Surp binary data to a Python object.

    This is the JSON-like API with options. Similar to ``json.loads()``.

    Args:
        data: The Surp binary data (bytes).
        strict: If ``True`` (default), verify checksums and enforce limits.
            If ``False``, attempt best-effort decoding with relaxed limits.
        max_depth: Maximum nesting depth (default: 128). Set to prevent
            stack overflow on deeply nested data.

    Returns:
        The decoded Python object.

    Raises:
        SurpDecodeError: If decoding fails.
        SurpChecksumError: If checksum verification fails (when strict=True).

    Example::

        >>> import surp
        >>> obj = surp.loads(data)
    """
    ...

def dump(
    obj: Any,
    fp: BinaryIO,
    *,
    compression: Literal["lz4", "zstd", "snappy", "none"] | None = None,
    dedup: bool = False,
    sort_keys: bool = False,
) -> None:
    """Serialize a Python object and write to a file-like object.

    Similar to ``json.dump()``.

    Args:
        obj: The Python object to encode.
        fp: A file-like object with a ``write()`` method (must accept bytes).
        compression: Compression algorithm.
        dedup: Enable string deduplication.
        sort_keys: Sort object keys alphabetically.

    Raises:
        SurpEncodeError: If encoding fails.
        SurpTypeError: If an unsupported type is encountered.
        TypeError: If fp doesn't have a write method.

    Example::

        >>> with open("data.surp", "wb") as f:
        ...     surp.dump({"hello": "world"}, f)
    """
    ...

def load(
    fp: BinaryIO,
    *,
    strict: bool = True,
    max_depth: int = 128,
) -> Any:
    """Read and deserialize Surp binary data from a file-like object.

    Similar to ``json.load()``.

    Args:
        fp: A file-like object with a ``read()`` method.
        strict: If ``True`` (default), verify checksums.
        max_depth: Maximum nesting depth (default: 128).

    Returns:
        The decoded Python object.

    Raises:
        SurpDecodeError: If decoding fails.
        SurpChecksumError: If checksum verification fails.

    Example::

        >>> with open("data.surp", "rb") as f:
        ...     obj = surp.load(f)
    """
    ...

# ---------------------------------------------------------------------------
# Legacy API (backward compatible)
# ---------------------------------------------------------------------------

def encode(obj: Any) -> bytes:
    """Encode a Python object into Surp binary format.

    This is the simple API. For more options, use ``dumps()``.

    Supported types: ``dict``, ``list``, ``str``, ``int``, ``float``,
    ``bytes``, ``bool``, and ``None``.

    Args:
        obj: The Python object to encode.

    Returns:
        The encoded binary data as ``bytes``.

    Raises:
        SurpTypeError: If *obj* contains a type that cannot be converted.
        SurpEncodeError: If the encoder encounters an internal error.

    Example::

        >>> import _surp_native as cn
        >>> data = cn.encode({"name": "Alice", "age": 30})
        >>> isinstance(data, bytes)
        True
    """
    ...

def decode(data: bytes) -> Any:
    """Decode Surp binary ``bytes`` into Python objects.

    This is the simple API. For more options, use ``loads()``.

    Returns a single value if the data contains exactly one top-level
    value, or a ``list`` of values otherwise.

    Args:
        data: Raw Surp binary data.

    Returns:
        The decoded Python object (``dict``, ``list``, ``str``, ``int``,
        ``float``, ``bytes``, ``bool``, or ``None``).

    Raises:
        SurpDecodeError: If decoding fails (corrupt data, unsupported wire
            type, checksum mismatch, etc.).

    Example::

        >>> import _surp_native as cn
        >>> cn.decode(cn.encode({"key": "value"}))
        {'key': 'value'}
    """
    ...

def encode_to_file(obj: Any, path: Union[str, PathLike[str]]) -> None:
    """Encode a Python object and write the binary output to a file.

    This is a convenience wrapper equivalent to::

        with open(path, "wb") as f:
            f.write(encode(obj))

    Args:
        obj: The Python object to encode.
        path: Destination file path.

    Raises:
        SurpTypeError: If *obj* contains an unsupported type.
        SurpEncodeError: If encoding fails.
        OSError: If writing to *path* fails.

    Example::

        >>> import _surp_native as cn
        >>> cn.encode_to_file({"name": "Alice"}, "data.surp")
    """
    ...

def decode_from_file(path: Union[str, PathLike[str]]) -> Any:
    """Read a Surp binary file and decode it into Python objects.

    This is a convenience wrapper equivalent to::

        with open(path, "rb") as f:
            return decode(f.read())

    Args:
        path: Source file path to read.

    Returns:
        The decoded Python object.

    Raises:
        SurpDecodeError: If decoding fails.
        OSError: If reading *path* fails.

    Example::

        >>> import _surp_native as cn
        >>> cn.encode_to_file([1, 2, 3], "data.surp")
        >>> cn.decode_from_file("data.surp")
        [1, 2, 3]
    """
    ...

def parse_text(text: str) -> Any:
    """Parse Surp human-readable text notation into a Python object.

    Args:
        text: A string in Surp text format.

    Returns:
        The parsed Python value.

    Raises:
        ValueError: If the text is syntactically invalid.

    Example::

        >>> import _surp_native as cn
        >>> cn.parse_text('{name: "Alice"; age: 30;}')
        {'name': 'Alice', 'age': 30}
    """
    ...

def pretty_print(obj: Any, indent: int = 2) -> str:
    """Format a Python object in Surp human-readable text notation.

    Args:
        obj: The Python object to format.
        indent: Number of spaces per indentation level (default ``2``).

    Returns:
        A string in Surp text format.

    Raises:
        SurpTypeError: If *obj* cannot be converted to a Surp value.

    Example::

        >>> import _surp_native as cn
        >>> print(cn.pretty_print({"name": "Alice", "active": True}))
        {
          name: "Alice";
          active: true;
        }
    """
    ...

# ---------------------------------------------------------------------------
# Encoder class
# ---------------------------------------------------------------------------

class Encoder:
    """Incremental Surp encoder.

    Build up encoded data by calling :meth:`encode` one or more times,
    then retrieve the final binary with :meth:`finish` (or write it
    directly with :meth:`finish_to_file`).

    Args:
        sort_keys: If ``True``, sort object keys alphabetically for
            canonical output. Default ``False``.

    Example::

        >>> enc = Encoder()
        >>> enc.enable_dedup()
        >>> enc.set_compression("lz4")
        >>> enc.encode({"key": "value"})
        >>> data = enc.finish()
    """

    def __init__(self, *, sort_keys: bool = False) -> None:
        """Create a new encoder.

        Args:
            sort_keys: Sort object keys alphabetically.
        """
        ...

    def enable_dedup(self) -> None:
        """Enable string deduplication for subsequent blocks.

        When enabled, repeated strings are stored once and referenced
        by index, reducing output size for data with many duplicate
        string values.

        Raises:
            SurpEncodeError: If the encoder has already been finished.
        """
        ...

    def set_compression(self, comp: str) -> None:
        """Set the compression algorithm for subsequent blocks.

        Args:
            comp: One of ``"none"``, ``"lz4"``, ``"zstd"``, or
                ``"snappy"`` (case-insensitive).

        Raises:
            ValueError: If *comp* is not a recognised algorithm name.
            SurpEncodeError: If the encoder has already been finished.
        """
        ...

    def encode(self, obj: Any) -> None:
        """Encode a Python value into the current block.

        Args:
            obj: The Python object to encode.

        Raises:
            SurpTypeError: If *obj* contains an unsupported type.
            SurpEncodeError: If encoding fails or the encoder is finished.
        """
        ...

    def finish(self) -> bytes:
        """Finalise the encoder and return the Surp binary output.

        The encoder **cannot** be used after this call.

        Returns:
            The complete encoded binary as ``bytes``.

        Raises:
            SurpEncodeError: If the encoder has already been finished.
        """
        ...

    def finish_to_file(self, path: Union[str, PathLike[str]]) -> None:
        """Finalise the encoder and write the output directly to a file.

        The encoder **cannot** be used after this call.

        Args:
            path: Destination file path.

        Raises:
            SurpEncodeError: If the encoder has already been finished.
            OSError: If writing to *path* fails.
        """
        ...

# ---------------------------------------------------------------------------
# Decoder class
# ---------------------------------------------------------------------------

class SurpDecoder:
    """Incremental Surp decoder.

    Wraps binary data and decodes all top-level values from it.

    Args:
        data: Raw Surp binary ``bytes`` to decode.
        max_depth: Maximum nesting depth (default: 128).

    Example::

        >>> dec = SurpDecoder(data)
        >>> values = dec.decode_all()
        >>> print(values[0])
    """

    def __init__(self, data: bytes, *, max_depth: int = 128) -> None:
        """Create a decoder over the given binary data.

        Args:
            data: Raw Surp binary ``bytes`` to decode.
            max_depth: Maximum nesting depth (default: 128).
        """
        ...

    def decode_all(self) -> list[Any]:
        """Decode all values from the binary data.

        Can only be called **once** per decoder instance.

        Returns:
            A list of decoded Python objects.

        Raises:
            SurpDecodeError: If the decoder has already been consumed, or
                if decoding fails.
        """
        ...
