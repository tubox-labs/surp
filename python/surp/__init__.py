"""
surp — Pure-Python encoder/decoder for the Surp binary serialization format.

A compact, canonical binary serializer and human-readable alternative to JSON.

Quick start:

    >>> import surp
    >>> data = surp.encode({"name": "Alice", "age": 30})
    >>> surp.decode(data)
    {'name': 'Alice', 'age': 30}
"""

from surp.value import Value, ValueType
from surp.encoder import Encoder, encode
from surp.decoder import Decoder, decode
from surp.text import parse as parse_text, pretty_print
from surp.wire import WireType, BlockType, CompressionType
from surp.varint import encode_varint, decode_varint, zigzag_encode, zigzag_decode
from surp.error import SurpError

__version__ = "1.1.3"
__all__ = [
    "Value",
    "ValueType",
    "Encoder",
    "Decoder",
    "encode",
    "decode",
    "parse_text",
    "pretty_print",
    "WireType",
    "BlockType",
    "CompressionType",
    "encode_varint",
    "decode_varint",
    "zigzag_encode",
    "zigzag_decode",
    "SurpError",
]
