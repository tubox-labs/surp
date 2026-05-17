from __future__ import annotations

from typing import Any

from .exceptions import SurpQueryError


def query(instance: Any, expr: str) -> list[Any]:
    try:
        from surp import rfc001

        values = rfc001.query_cbf(instance.to_cbf(), expr)
        return [_plain(value) for value in values]
    except Exception as exc:  # pragma: no cover - depends on native package availability
        raise SurpQueryError(str(exc)) from exc


def query_one(instance: Any, expr: str) -> Any:
    results = query(instance, expr)
    if not results:
        raise SurpQueryError(f"query returned no results: {expr}")
    if len(results) > 1:
        raise SurpQueryError(f"query returned multiple results: {expr}")
    return results[0]


def _plain(value: dict[str, Any]) -> Any:
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
