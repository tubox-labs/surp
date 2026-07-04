from __future__ import annotations

from typing import Any

from ..exceptions import SurpError
from .exceptions import SurpQueryError


def query(instance: Any, expr: str) -> list[Any]:
    r"""query(instance, expr) -> list[Any]

    Run an RFC-001 CQL expression against a model instance.

    The instance is encoded to CBF first so the query uses the same native CQL
    engine as ``surp.rfc001.query_cbf``.

    Args:
        instance (Any): Model instance to query.
        expr (str): Baseline CQL path expression.
    """
    from surp import rfc001

    # Only wrap genuine query failures (malformed CQL, native encode/decode
    # errors). A bare `except Exception` here would also swallow programming
    # errors - e.g. an AttributeError from a broken model definition's
    # `to_cbf()` - and mislabel them as query failures.
    try:
        cbf = instance.to_cbf()
    except SurpError as exc:
        raise SurpQueryError(str(exc)) from exc

    try:
        values = rfc001.query_cbf(cbf, expr)
    except SurpError as exc:
        raise SurpQueryError(str(exc)) from exc

    return [_plain(value) for value in values]


def query_one(instance: Any, expr: str) -> Any:
    r"""query_one(instance, expr) -> Any

    Run a CQL expression and require exactly one result.
    """
    results = query(instance, expr)
    if not results:
        raise SurpQueryError(f"query returned no results: {expr}")
    if len(results) > 1:
        raise SurpQueryError(f"query returned multiple results: {expr}")
    return results[0]


def _plain(value: dict[str, Any]) -> Any:
    r"""_plain(value) -> Any

    Convert RFC value dictionaries returned by CQL into plain Python data.
    """
    kind = value.get("kind")
    if kind == "scalar":
        return value.get("value")
    if kind == "sequence":
        return [_plain(item) for item in value.get("items", [])]
    if kind == "association":
        return {_plain(key): _plain(item) for key, item in value.get("pairs", [])}
    if kind == "product":
        return {field["name"]: _plain(field["value"]) for field in value.get("fields", [])}
    if kind == "sum":
        return value
    if kind == "tensor":
        data = value.get("data", {})
        return data.get("values", data.get("bytes"))
    if kind == "reference":
        if value.get("reference_kind") == "by_id":
            return _plain(value["value"])
        return value
    if kind == "stream":
        return {
            item["name"]: _plain(item["value"]) if item.get("value") is not None else None
            for item in value.get("annotations", [])
        }
    return value
