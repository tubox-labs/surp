"""Model Context Protocol stdio server for the native Surp Python package."""

from __future__ import annotations

import base64
import json
import os
import pathlib
import sys
from dataclasses import dataclass
from typing import Any, Callable

from . import Encoder, SurpDecoder, __version__, dumps, parse_text, pretty_print, rfc001

SERVER_NAME = "surp_mcp"
PROTOCOL_VERSION = "2024-11-05"
MASK64 = 0xFFFFFFFFFFFFFFFF


class McpToolError(Exception):
    """Tool-level error returned as an MCP tool result."""


@dataclass
class SecurityConfig:
    roots: list[pathlib.Path]
    read_only: bool
    max_input_bytes: int
    max_inline_bytes: int
    max_text_bytes: int

    @classmethod
    def from_env(cls) -> "SecurityConfig":
        roots_env = os.environ.get("SURP_MCP_ROOTS")
        if roots_env:
            roots = [pathlib.Path(part).resolve() for part in roots_env.split(os.pathsep) if part]
        else:
            roots = [pathlib.Path.cwd().resolve()]
        return cls(
            roots=roots,
            read_only=os.environ.get("SURP_MCP_READ_ONLY", "").lower() in {"1", "true", "yes"},
            max_input_bytes=int(os.environ.get("SURP_MCP_MAX_INPUT_BYTES", 16 * 1024 * 1024)),
            max_inline_bytes=int(os.environ.get("SURP_MCP_MAX_INLINE_BYTES", 4 * 1024 * 1024)),
            max_text_bytes=int(os.environ.get("SURP_MCP_MAX_TEXT_BYTES", 4 * 1024 * 1024)),
        )

    def check_size(self, size: int, limit: int, label: str) -> None:
        if size > limit:
            raise McpToolError(f"{label} is {size} bytes, exceeding the configured limit {limit}")

    def _ensure_root(self, path: pathlib.Path) -> pathlib.Path:
        resolved = path.resolve()
        if not any(resolved == root or root in resolved.parents for root in self.roots):
            raise McpToolError(f"path '{resolved}' is outside the allowed MCP roots")
        return resolved

    def read_file(self, path: str) -> bytes:
        resolved = self._ensure_root(pathlib.Path(path))
        data = resolved.read_bytes()
        self.check_size(len(data), self.max_input_bytes, "input payload")
        return data

    def write_file(self, path: str, data: bytes, allow_overwrite: bool) -> str:
        if self.read_only:
            raise McpToolError("filesystem writes are disabled by SURP_MCP_READ_ONLY")
        target = pathlib.Path(path)
        parent = self._ensure_root(target.parent if str(target.parent) else pathlib.Path("."))
        output = parent / target.name
        if output.exists() and not allow_overwrite:
            raise McpToolError(
                f"output path '{output}' already exists; set allow_overwrite=true to replace it"
            )
        output.write_bytes(data)
        return str(output)


def main() -> None:
    security = SecurityConfig.from_env()
    _log("info", "server_start", {})
    for raw in sys.stdin:
        raw = raw.strip()
        if not raw:
            continue
        response = handle_message(raw, security)
        if response is not None:
            sys.stdout.write(json.dumps(response, separators=(",", ":")) + "\n")
            sys.stdout.flush()
    _log("info", "server_stop", {})


def handle_message(raw: str, security: SecurityConfig) -> dict[str, Any] | None:
    try:
        message = json.loads(raw)
    except json.JSONDecodeError as exc:
        return _error(None, -32700, f"parse error: {exc}")

    msg_id = message.get("id")
    method = message.get("method")
    if not method:
        return _error(msg_id, -32600, "missing method")
    if msg_id is None and method.startswith("notifications/"):
        return None

    try:
        if method == "initialize":
            result = {
                "protocolVersion": message.get("params", {}).get(
                    "protocolVersion", PROTOCOL_VERSION
                ),
                "capabilities": {
                    "tools": {"listChanged": False},
                    "resources": {"subscribe": False, "listChanged": False},
                    "prompts": {"listChanged": False},
                    "logging": {},
                },
                "serverInfo": {"name": SERVER_NAME, "version": __version__},
            }
        elif method == "ping":
            result = {}
        elif method == "tools/list":
            result = {"tools": TOOLS}
        elif method == "tools/call":
            result = _handle_tool_call(message.get("params") or {}, security)
        elif method == "resources/list":
            result = {"resources": RESOURCES}
        elif method == "resources/read":
            uri = (message.get("params") or {}).get("uri")
            if uri not in RESOURCE_TEXT:
                return _error(msg_id, -32602, f"unknown resource URI '{uri}'")
            result = {
                "contents": [{"uri": uri, "mimeType": "text/markdown", "text": RESOURCE_TEXT[uri]}]
            }
        elif method == "prompts/list":
            result = {"prompts": PROMPTS}
        elif method == "prompts/get":
            result = _prompt(message.get("params") or {})
        else:
            return _error(msg_id, -32601, f"method not found: {method}")
    except Exception as exc:
        return _error(msg_id, -32603, f"internal MCP error: {exc}")
    return {"jsonrpc": "2.0", "id": msg_id, "result": result}


def _handle_tool_call(params: dict[str, Any], security: SecurityConfig) -> dict[str, Any]:
    name = params.get("name")
    args = params.get("arguments") or {}
    if not isinstance(name, str):
        raise McpToolError("tools/call params.name must be a string")
    if not isinstance(args, dict):
        raise McpToolError("tools/call params.arguments must be an object")
    _log("info", "tool_call", {"tool": name})
    try:
        result = call_tool(name, args, security)
        return {
            "content": [{"type": "text", "text": result["summary"]}],
            "structuredContent": result,
            "isError": False,
        }
    except Exception as exc:
        _log("warn", "tool_error", {"tool": name, "error": str(exc)})
        return {
            "content": [{"type": "text", "text": f"Surp tool error: {exc}"}],
            "structuredContent": {"error": str(exc)},
            "isError": True,
        }


def call_tool(name: str, args: dict[str, Any], security: SecurityConfig) -> dict[str, Any]:
    handlers: dict[str, Callable[[dict[str, Any], SecurityConfig], dict[str, Any]]] = {
        "surp_capabilities": _capabilities,
        "surp_v1_encode_json": _v1_encode_json,
        "surp_v1_decode": _v1_decode,
        "surp_v1_text_parse": _v1_text_parse,
        "surp_v1_text_pretty": _v1_text_pretty,
        "surp_v1_text_to_binary": _v1_text_to_binary,
        "surp_v1_binary_to_text": _v1_binary_to_text,
        "surp_v1_inspect": _v1_inspect,
        "surp_v1_validate": _v1_validate,
        "surp_v1_query": _v1_query,
        "surp_rfc001_parse_ctn": _rfc001_parse_ctn,
        "surp_rfc001_normalize_ctn": _rfc001_normalize_ctn,
        "surp_rfc001_compile_ctn": _rfc001_compile_ctn,
        "surp_rfc001_decode_cbf": _rfc001_decode_cbf,
        "surp_rfc001_cbf_to_ctn": _rfc001_cbf_to_ctn,
        "surp_rfc001_inspect_cbf": _rfc001_inspect_cbf,
        "surp_rfc001_query_ctn": _rfc001_query_ctn,
        "surp_rfc001_query_cbf": _rfc001_query_cbf,
    }
    try:
        handler = handlers[name]
    except KeyError as exc:
        raise McpToolError(f"unknown Surp MCP tool '{name}'") from exc
    return handler(args, security)


def _capabilities(args: dict[str, Any], security: SecurityConfig) -> dict[str, Any]:
    return {
        "summary": "Surp MCP exposes v1 Surp and RFC-001 CTN/CBF/CQL operations.",
        "server": {"name": SERVER_NAME, "version": __version__, "protocol_version": PROTOCOL_VERSION},
        "formats": ["v1", "rfc001"],
        "compiled_features": {
            "none": True,
            "lz4": "accepted_by_api; requires native surp-core feature to compress",
            "snappy": "accepted_by_api; requires native surp-core feature to compress",
            "zstd": "accepted_by_api; requires native surp-core feature to compress",
        },
        "security": {
            "roots": [str(root) for root in security.roots],
            "max_input_bytes": security.max_input_bytes,
            "max_inline_bytes": security.max_inline_bytes,
            "max_text_bytes": security.max_text_bytes,
            "default_limits_profile": "strict",
        },
        "tool_count": len(TOOLS),
        "resource_count": len(RESOURCES),
    }


def _v1_encode_json(args: dict[str, Any], security: SecurityConfig) -> dict[str, Any]:
    values = _json_values(args, security)
    if len(values) == 1:
        data = dumps(
            values[0],
            compression=args.get("compression"),
            dedup=bool(args.get("dedup", False)),
            sort_keys=bool(args.get("sort_keys", False)),
        )
    else:
        enc = Encoder(sort_keys=bool(args.get("sort_keys", False)))
        if args.get("dedup", False):
            enc.enable_dedup()
        if args.get("compression"):
            enc.set_compression(str(args["compression"]))
        for value in values:
            enc.encode(value)
        data = enc.finish()
    result = _binary_output(args, security, data)
    result.update({"summary": f"Encoded {len(values)} v1 Surp value(s), {len(data)} bytes.", "inspect": _inspect_v1_bytes(data), "value_count": len(values)})
    return result


def _v1_decode(args: dict[str, Any], security: SecurityConfig) -> dict[str, Any]:
    data, source = _read_binary(args, security)
    values = _decode_all_v1(data, args)
    rendered = values[0] if len(values) == 1 else values
    json_text = json.dumps(
        _json_safe(rendered),
        indent=2 if args.get("json_style", "pretty") == "pretty" else None,
    )
    result = _text_output(args, security, "json_text", json_text)
    result.update({"summary": f"Decoded {len(values)} v1 Surp value(s).", "source": source, "value_count": len(values), "values": _json_safe(values)})
    return result


def _v1_text_parse(args: dict[str, Any], security: SecurityConfig) -> dict[str, Any]:
    text, source = _read_text(args, security)
    value = parse_text(text)
    normalized = pretty_print(value, int(args.get("indent", 2)))
    return {"summary": "Parsed v1 Surp text notation.", "source": source, "value": _json_safe(value), "normalized_text": normalized}


def _v1_text_pretty(args: dict[str, Any], security: SecurityConfig) -> dict[str, Any]:
    value = _single_json_value(args, security)
    text = pretty_print(value, int(args.get("indent", 2)))
    result = _text_output(args, security, "text", text)
    result.update({"summary": "Rendered v1 Surp text notation."})
    return result


def _v1_text_to_binary(args: dict[str, Any], security: SecurityConfig) -> dict[str, Any]:
    text, source = _read_text(args, security)
    value = parse_text(text)
    data = dumps(value, compression=args.get("compression"), dedup=bool(args.get("dedup", False)))
    result = _binary_output(args, security, data)
    result.update({"summary": f"Compiled v1 Surp text to {len(data)} bytes.", "source": source, "inspect": _inspect_v1_bytes(data)})
    return result


def _v1_binary_to_text(args: dict[str, Any], security: SecurityConfig) -> dict[str, Any]:
    data, source = _read_binary(args, security)
    values = _decode_all_v1(data, args)
    text = "\n\n".join(pretty_print(value, int(args.get("indent", 2))) for value in values)
    result = _text_output(args, security, "text", text)
    result.update({"summary": f"Rendered {len(values)} v1 Surp value(s) as text.", "source": source, "value_count": len(values)})
    return result


def _v1_inspect(args: dict[str, Any], security: SecurityConfig) -> dict[str, Any]:
    data, source = _read_binary(args, security)
    report = _inspect_v1_bytes(data)
    report.update({"summary": "Inspected v1 Surp block layout.", "source": source})
    return report


def _v1_validate(args: dict[str, Any], security: SecurityConfig) -> dict[str, Any]:
    data, source = _read_binary(args, security)
    report = _inspect_v1_bytes(data)
    errors: list[str] = []
    for block in report["blocks"]:
        if block["payload_checksum_valid"] is False:
            errors.append(f"block #{block['index']} payload checksum is invalid")
    trailer = report.get("trailer")
    if not trailer:
        errors.append("missing trailer block")
    else:
        if not trailer.get("payload_checksum_valid"):
            errors.append("trailer payload checksum is invalid")
        if not trailer.get("file_checksum_valid"):
            errors.append("trailer file checksum is invalid")
    decoded_count = None
    if not errors and not bool(args.get("checksums_only", False)):
        try:
            decoded_count = len(_decode_all_v1(data, args))
        except Exception as exc:
            errors.append(f"decode validation failed: {exc}")
    valid = not errors
    return {"summary": "v1 Surp validation passed." if valid else "v1 Surp validation failed.", "source": source, "valid": valid, "errors": errors, "decoded_value_count": decoded_count, "inspect": report}


def _v1_query(args: dict[str, Any], security: SecurityConfig) -> dict[str, Any]:
    query = _required_str(args, "query")
    values = _v1_query_values(args, security)
    matches = _query_python_values(values, query)
    fmt = args.get("result_format", "json")
    if fmt == "text":
        rendered = [pretty_print(value, int(args.get("indent", 2))) for value in matches]
    else:
        rendered = _json_safe(matches)
    return {"summary": f"v1 query returned {len(matches)} match(es).", "query": query, "match_count": len(matches), "matches": rendered}


def _rfc001_parse_ctn(args: dict[str, Any], security: SecurityConfig) -> dict[str, Any]:
    text, source = _read_text(args, security)
    doc = rfc001.parse_ctn(text)
    result = {"summary": "Parsed RFC-001 CTN document.", "source": source, "document": _json_safe(doc)}
    if args.get("include_normalized_ctn", True):
        result["normalized_ctn"] = rfc001.normalize_ctn(text)
    return result


def _rfc001_normalize_ctn(args: dict[str, Any], security: SecurityConfig) -> dict[str, Any]:
    text, source = _read_text(args, security)
    normalized = rfc001.normalize_ctn(text)
    result = _text_output(args, security, "text", normalized)
    result.update({"summary": "Normalized RFC-001 CTN.", "source": source})
    return result


def _rfc001_compile_ctn(args: dict[str, Any], security: SecurityConfig) -> dict[str, Any]:
    text, source = _read_text(args, security)
    data = rfc001.compile_ctn(text, with_symtab=bool(args.get("with_symtab", True)), alignment=int(args.get("alignment", 0)))
    result = _binary_output(args, security, data)
    result.update({"summary": f"Compiled RFC-001 CTN to {len(data)} CBF bytes.", "source": source})
    return result


def _rfc001_decode_cbf(args: dict[str, Any], security: SecurityConfig) -> dict[str, Any]:
    data, source = _read_binary(args, security)
    decoded = _json_safe(rfc001.decode_cbf(data))
    if not args.get("include_ctn", True):
        decoded.pop("ctn", None)
    decoded.update({"summary": "Decoded RFC-001 CBF document.", "source": source})
    return decoded


def _rfc001_cbf_to_ctn(args: dict[str, Any], security: SecurityConfig) -> dict[str, Any]:
    data, source = _read_binary(args, security)
    text = rfc001.cbf_to_ctn(data)
    result = _text_output(args, security, "text", text)
    result.update({"summary": "Rendered RFC-001 CBF as CTN.", "source": source})
    return result


def _rfc001_inspect_cbf(args: dict[str, Any], security: SecurityConfig) -> dict[str, Any]:
    data, source = _read_binary(args, security)
    try:
        decoded = _json_safe(rfc001.decode_cbf(data))
        return {"summary": "Inspected RFC-001 CBF.", "source": source, "valid": True, "byte_length": len(data), "header": decoded["header"], "symbol_count": len(decoded["symbols"]), "symbols": decoded["symbols"]}
    except Exception as exc:
        return {"summary": "Inspected RFC-001 CBF.", "source": source, "valid": False, "byte_length": len(data), "error": str(exc)}


def _rfc001_query_ctn(args: dict[str, Any], security: SecurityConfig) -> dict[str, Any]:
    text, source = _read_text(args, security)
    query = _required_str(args, "query")
    matches = rfc001.query_ctn(text, query, as_ctn=bool(args.get("as_ctn", False)))
    return {"summary": f"RFC-001 query returned {len(matches)} match(es).", "source": source, "query": query, "match_count": len(matches), "matches": _json_safe(matches)}


def _rfc001_query_cbf(args: dict[str, Any], security: SecurityConfig) -> dict[str, Any]:
    data, source = _read_binary(args, security)
    query = _required_str(args, "query")
    matches = rfc001.query_cbf(data, query, as_ctn=bool(args.get("as_ctn", False)))
    return {"summary": f"RFC-001 query returned {len(matches)} match(es).", "source": source, "query": query, "match_count": len(matches), "matches": _json_safe(matches)}


def _read_binary(args: dict[str, Any], security: SecurityConfig) -> tuple[bytes, str]:
    if "data_base64" in args and "input_path" in args:
        raise McpToolError("provide exactly one of data_base64 or input_path")
    if "data_base64" in args:
        data = base64.b64decode(args["data_base64"], validate=True)
        security.check_size(len(data), security.max_input_bytes, "input payload")
        return data, "inline_base64"
    if "input_path" in args:
        return security.read_file(str(args["input_path"])), str(args["input_path"])
    raise McpToolError("provide one of data_base64 or input_path")


def _read_text(args: dict[str, Any], security: SecurityConfig) -> tuple[str, str]:
    if "text" in args and "input_path" in args:
        raise McpToolError("provide exactly one of text or input_path")
    if "text" in args:
        text = str(args["text"])
        security.check_size(len(text.encode()), security.max_text_bytes, "text payload")
        return text, "inline_text"
    if "input_path" in args:
        data = security.read_file(str(args["input_path"]))
        security.check_size(len(data), security.max_text_bytes, "text payload")
        return data.decode("utf-8"), str(args["input_path"])
    raise McpToolError("provide one of text or input_path")


def _json_values(args: dict[str, Any], security: SecurityConfig) -> list[Any]:
    present = [key for key in ("value", "values", "json_text", "input_path") if key in args]
    if len(present) != 1:
        raise McpToolError("provide exactly one of value, values, json_text, or input_path")
    if "value" in args:
        return [args["value"]]
    if "values" in args:
        if not isinstance(args["values"], list):
            raise McpToolError("values must be an array")
        return list(args["values"])
    if "json_text" in args:
        return [json.loads(str(args["json_text"]))]
    return [json.loads(security.read_file(str(args["input_path"])).decode("utf-8"))]


def _single_json_value(args: dict[str, Any], security: SecurityConfig) -> Any:
    values = _json_values(args, security)
    if len(values) != 1:
        raise McpToolError("this tool expects exactly one JSON value")
    return values[0]


def _binary_output(args: dict[str, Any], security: SecurityConfig, data: bytes) -> dict[str, Any]:
    result: dict[str, Any] = {"byte_length": len(data)}
    output_path = args.get("output_path")
    include_base64 = args.get("include_base64", output_path is None)
    if output_path:
        result["output_path"] = security.write_file(str(output_path), data, bool(args.get("allow_overwrite", False)))
    if include_base64:
        security.check_size(len(data), security.max_inline_bytes, "inline output")
        result["data_base64"] = base64.b64encode(data).decode("ascii")
    return result


def _text_output(args: dict[str, Any], security: SecurityConfig, key: str, text: str) -> dict[str, Any]:
    encoded = text.encode()
    result: dict[str, Any] = {"text_length": len(encoded)}
    output_path = args.get("output_path")
    include_text = args.get("include_text", args.get("include_json_text", output_path is None))
    if output_path:
        result["output_path"] = security.write_file(str(output_path), encoded, bool(args.get("allow_overwrite", False)))
    if include_text:
        security.check_size(len(encoded), security.max_inline_bytes, "inline text output")
        result[key] = text
    return result


def _max_depth(args: dict[str, Any]) -> int:
    limits = args.get("limits") if isinstance(args.get("limits"), dict) else {}
    if "max_nesting_depth" in limits:
        return int(limits["max_nesting_depth"])
    profile = args.get("limits_profile", "strict")
    return 32 if profile == "strict" else 128


def _decode_all_v1(data: bytes, args: dict[str, Any]) -> list[Any]:
    decoder = SurpDecoder(data, max_depth=_max_depth(args))
    return list(decoder.decode_all())


def _required_str(args: dict[str, Any], key: str) -> str:
    value = args.get(key)
    if not isinstance(value, str) or not value:
        raise McpToolError(f"missing required string '{key}'")
    return value


def _json_safe(value: Any) -> Any:
    if isinstance(value, bytes):
        return {"bytes_base64": base64.b64encode(value).decode("ascii"), "byte_length": len(value)}
    if isinstance(value, bytearray):
        return _json_safe(bytes(value))
    if isinstance(value, list):
        return [_json_safe(item) for item in value]
    if isinstance(value, tuple):
        return [_json_safe(item) for item in value]
    if isinstance(value, dict):
        return {str(key): _json_safe(item) for key, item in value.items()}
    return value


def _v1_query_values(args: dict[str, Any], security: SecurityConfig) -> list[Any]:
    if "data_base64" in args:
        data, _ = _read_binary(args, security)
        return _decode_all_v1(data, args)
    if "text" in args:
        text, _ = _read_text(args, security)
        return [parse_text(text)]
    if "value" in args or "json_text" in args:
        return _json_values(args, security)
    if "input_path" in args:
        fmt = args.get("input_format")
        data = security.read_file(str(args["input_path"]))
        if fmt == "surp_binary":
            return _decode_all_v1(data, args)
        if fmt == "surp_text":
            return [parse_text(data.decode("utf-8"))]
        if fmt == "json":
            return [json.loads(data.decode("utf-8"))]
        raise McpToolError("input_format is required for input_path queries")
    raise McpToolError("missing query input")


def _query_python_values(values: list[Any], expr: str) -> list[Any]:
    steps = _parse_query(expr)
    nodes = list(values)
    for kind, payload in steps:
        next_nodes: list[Any] = []
        for node in nodes:
            if kind == "field" and isinstance(node, dict) and payload in node:
                next_nodes.append(node[payload])
            elif kind == "flatten":
                if isinstance(node, dict):
                    next_nodes.extend(node.values())
                elif isinstance(node, list):
                    next_nodes.extend(node)
            elif kind == "index" and isinstance(node, list) and node:
                idx = int(payload)
                if idx < 0:
                    idx = len(node) + idx
                if 0 <= idx < len(node):
                    next_nodes.append(node[idx])
        nodes = next_nodes
    return nodes


def _parse_query(expr: str) -> list[tuple[str, Any]]:
    expr = expr.strip()
    if not expr or expr[0] not in ".[":
        raise McpToolError("query expression must start with '.' or '['")
    steps: list[tuple[str, Any]] = []
    i = 0
    while i < len(expr):
        if expr[i] == ".":
            i += 1
            start = i
            while i < len(expr) and (expr[i].isalnum() or expr[i] in "_-"):
                i += 1
            if i == start:
                raise McpToolError("expected field name after '.'")
            steps.append(("field", expr[start:i]))
        elif expr[i] == "[":
            end = expr.find("]", i)
            if end < 0:
                raise McpToolError("unterminated bracket selector")
            inner = expr[i + 1 : end].strip()
            steps.append(("flatten", None) if not inner else ("index", int(inner)))
            i = end + 1
        else:
            raise McpToolError(f"unexpected query token near '{expr[i:]}'")
    return steps


def _inspect_v1_bytes(data: bytes) -> dict[str, Any]:
    blocks = []
    trailer = None
    offset = 0
    index = 0
    while offset < len(data):
        block_offset = offset
        block_type = data[offset]
        offset += 1
        block_len, offset = _decode_varint(data, offset)
        compression = data[offset]
        offset += 1
        checksum = int.from_bytes(data[offset : offset + 8], "little")
        offset += 8
        payload = data[offset : offset + block_len]
        if len(payload) != block_len:
            raise McpToolError(f"unexpected EOF while parsing block #{index}")
        offset += block_len
        checksum_valid: bool | None = None
        if compression == 0 or block_type == 0xFF:
            checksum_valid = _xxh64(payload) == checksum
        if block_type == 0xFF:
            expected = int.from_bytes(payload, "little") if len(payload) == 8 else None
            trailer = {"payload_checksum_valid": checksum_valid, "file_checksum_valid": expected == _xxh64(data[:block_offset])}
        blocks.append({"index": index, "offset": block_offset, "block_type": _block_type_name(block_type), "compression": _compression_name(compression), "payload_len": block_len, "payload_checksum_valid": checksum_valid})
        index += 1
    return {"file_size": len(data), "blocks": blocks, "trailer": trailer, "valid_framing_checksums": bool(trailer and trailer["payload_checksum_valid"] and trailer["file_checksum_valid"])}


def _decode_varint(data: bytes, offset: int) -> tuple[int, int]:
    shift = 0
    value = 0
    while True:
        if offset >= len(data):
            raise McpToolError("unexpected EOF while reading varint")
        byte = data[offset]
        offset += 1
        value |= (byte & 0x7F) << shift
        if byte & 0x80 == 0:
            return value, offset
        shift += 7
        if shift >= 64:
            raise McpToolError("varint overflow")


def _block_type_name(value: int) -> str:
    return {1: "data", 2: "index", 3: "schema", 4: "string_dict", 255: "trailer"}.get(value, f"unknown:{value}")


def _compression_name(value: int) -> str:
    return {0: "none", 1: "zstd", 2: "snappy", 3: "lz4"}.get(value, f"unknown:{value}")


def _xxh64(data: bytes, seed: int = 0) -> int:
    p1 = 11400714785074694791
    p2 = 14029467366897019727
    p3 = 1609587929392839161
    p4 = 9650029242287828579
    p5 = 2870177450012600261

    def rotl(x: int, r: int) -> int:
        return ((x << r) | (x >> (64 - r))) & MASK64

    def round_(acc: int, lane: int) -> int:
        acc = (acc + lane * p2) & MASK64
        acc = rotl(acc, 31)
        return (acc * p1) & MASK64

    def merge(acc: int, lane: int) -> int:
        acc ^= round_(0, lane)
        return (acc * p1 + p4) & MASK64

    length = len(data)
    offset = 0
    if length >= 32:
        v1 = (seed + p1 + p2) & MASK64
        v2 = (seed + p2) & MASK64
        v3 = seed & MASK64
        v4 = (seed - p1) & MASK64
        limit = length - 32
        while offset <= limit:
            v1 = round_(v1, int.from_bytes(data[offset : offset + 8], "little"))
            offset += 8
            v2 = round_(v2, int.from_bytes(data[offset : offset + 8], "little"))
            offset += 8
            v3 = round_(v3, int.from_bytes(data[offset : offset + 8], "little"))
            offset += 8
            v4 = round_(v4, int.from_bytes(data[offset : offset + 8], "little"))
            offset += 8
        h64 = (rotl(v1, 1) + rotl(v2, 7) + rotl(v3, 12) + rotl(v4, 18)) & MASK64
        h64 = merge(merge(merge(merge(h64, v1), v2), v3), v4)
    else:
        h64 = (seed + p5) & MASK64
    h64 = (h64 + length) & MASK64
    while offset + 8 <= length:
        k1 = round_(0, int.from_bytes(data[offset : offset + 8], "little"))
        h64 ^= k1
        h64 = (rotl(h64, 27) * p1 + p4) & MASK64
        offset += 8
    if offset + 4 <= length:
        h64 ^= (int.from_bytes(data[offset : offset + 4], "little") * p1) & MASK64
        h64 = (rotl(h64, 23) * p2 + p3) & MASK64
        offset += 4
    while offset < length:
        h64 ^= (data[offset] * p5) & MASK64
        h64 = (rotl(h64, 11) * p1) & MASK64
        offset += 1
    h64 ^= h64 >> 33
    h64 = (h64 * p2) & MASK64
    h64 ^= h64 >> 29
    h64 = (h64 * p3) & MASK64
    h64 ^= h64 >> 32
    return h64 & MASK64


def _error(msg_id: Any, code: int, message: str) -> dict[str, Any]:
    return {"jsonrpc": "2.0", "id": msg_id, "error": {"code": code, "message": message}}


def _log(level: str, event: str, fields: dict[str, Any]) -> None:
    payload = {"level": level, "target": "surp_mcp", "event": event, **fields}
    print(json.dumps(payload, separators=(",", ":")), file=sys.stderr)


def _tool(name: str, description: str, schema: dict[str, Any] | None = None) -> dict[str, Any]:
    return {
        "name": name,
        "description": description,
        "inputSchema": schema or {"type": "object", "properties": {}, "additionalProperties": False},
        "outputSchema": {"type": "object"},
        "annotations": {
            "readOnlyHint": True,
            "destructiveHint": False,
            "idempotentHint": True,
            "openWorldHint": False,
        },
    }


def _schema(properties: dict[str, Any], one_of: list[list[str]] | None = None, required: list[str] | None = None) -> dict[str, Any]:
    schema: dict[str, Any] = {
        "type": "object",
        "properties": properties,
        "additionalProperties": False,
    }
    if one_of:
        schema["oneOf"] = [{"required": item} for item in one_of]
    if required:
        schema["required"] = required
    return schema


def _binary_schema(extra: dict[str, Any] | None = None, required: list[str] | None = None) -> dict[str, Any]:
    properties = {
        "data_base64": {"type": "string"},
        "input_path": {"type": "string"},
        **(extra or {}),
    }
    return _schema(properties, one_of=[["data_base64"], ["input_path"]], required=required)


def _text_schema(extra: dict[str, Any] | None = None, required: list[str] | None = None) -> dict[str, Any]:
    properties = {
        "text": {"type": "string"},
        "input_path": {"type": "string"},
        **(extra or {}),
    }
    return _schema(properties, one_of=[["text"], ["input_path"]], required=required)


COMPRESSION_SCHEMA = {"type": "string", "enum": ["none", "lz4", "snappy", "zstd"], "default": "none"}
LIMITS_SCHEMA = {
    "type": "object",
    "properties": {"max_nesting_depth": {"type": "integer", "minimum": 1}},
    "additionalProperties": True,
}

TOOLS = [
    _tool("surp_capabilities", "Return Surp MCP capabilities, supported formats, safety limits, and feature metadata."),
    _tool(
        "surp_v1_encode_json",
        "Encode JSON input into the stable v1 Surp block-framed binary format.",
        _schema(
            {
                "value": {},
                "values": {"type": "array"},
                "json_text": {"type": "string"},
                "input_path": {"type": "string"},
                "dedup": {"type": "boolean", "default": False},
                "sort_keys": {"type": "boolean", "default": False},
                "compression": COMPRESSION_SCHEMA,
                "output_path": {"type": "string"},
                "allow_overwrite": {"type": "boolean", "default": False},
                "include_base64": {"type": "boolean"},
            },
            one_of=[["value"], ["values"], ["json_text"], ["input_path"]],
        ),
    ),
    _tool(
        "surp_v1_decode",
        "Decode stable v1 Surp binary data into JSON-compatible values.",
        _binary_schema({"limits_profile": {"type": "string"}, "limits": LIMITS_SCHEMA, "json_style": {"type": "string"}, "output_path": {"type": "string"}, "include_json_text": {"type": "boolean"}}),
    ),
    _tool("surp_v1_text_parse", "Parse Surp v1 human-readable text notation.", _text_schema({"indent": {"type": "integer", "default": 2}})),
    _tool(
        "surp_v1_text_pretty",
        "Pretty-print a JSON value as Surp v1 text notation.",
        _schema(
            {"value": {}, "json_text": {"type": "string"}, "input_path": {"type": "string"}, "indent": {"type": "integer", "default": 2}, "output_path": {"type": "string"}, "include_text": {"type": "boolean"}},
            one_of=[["value"], ["json_text"], ["input_path"]],
        ),
    ),
    _tool("surp_v1_text_to_binary", "Compile Surp v1 text notation into binary data.", _text_schema({"dedup": {"type": "boolean"}, "compression": COMPRESSION_SCHEMA, "output_path": {"type": "string"}, "include_base64": {"type": "boolean"}})),
    _tool("surp_v1_binary_to_text", "Decode v1 Surp binary data and render text notation.", _binary_schema({"limits_profile": {"type": "string"}, "limits": LIMITS_SCHEMA, "indent": {"type": "integer", "default": 2}, "output_path": {"type": "string"}, "include_text": {"type": "boolean"}})),
    _tool("surp_v1_inspect", "Inspect v1 Surp block layout and checksum metadata.", _binary_schema()),
    _tool("surp_v1_validate", "Validate v1 Surp framing, checksums, trailer, and decode integrity.", _binary_schema({"checksums_only": {"type": "boolean"}, "limits_profile": {"type": "string"}, "limits": LIMITS_SCHEMA})),
    _tool(
        "surp_v1_query",
        "Run a basic structural path query over v1 Surp data.",
        _schema(
            {"query": {"type": "string"}, "data_base64": {"type": "string"}, "text": {"type": "string"}, "value": {}, "json_text": {"type": "string"}, "input_path": {"type": "string"}, "input_format": {"type": "string"}, "result_format": {"type": "string"}, "limits": LIMITS_SCHEMA},
            one_of=[["data_base64"], ["text"], ["value"], ["json_text"], ["input_path"]],
            required=["query"],
        ),
    ),
    _tool("surp_rfc001_parse_ctn", "Parse an RFC-001 CTN document.", _text_schema({"include_normalized_ctn": {"type": "boolean"}})),
    _tool("surp_rfc001_normalize_ctn", "Parse and canonicalize RFC-001 CTN text.", _text_schema({"output_path": {"type": "string"}, "include_text": {"type": "boolean"}})),
    _tool("surp_rfc001_compile_ctn", "Compile RFC-001 CTN text into CBF bytes.", _text_schema({"with_symtab": {"type": "boolean"}, "alignment": {"type": "integer"}, "output_path": {"type": "string"}, "include_base64": {"type": "boolean"}})),
    _tool("surp_rfc001_decode_cbf", "Decode RFC-001 CBF bytes.", _binary_schema({"include_ctn": {"type": "boolean"}})),
    _tool("surp_rfc001_cbf_to_ctn", "Decode RFC-001 CBF bytes and return CTN text.", _binary_schema({"output_path": {"type": "string"}, "include_text": {"type": "boolean"}})),
    _tool("surp_rfc001_inspect_cbf", "Inspect RFC-001 CBF header fields and symbol metadata.", _binary_schema()),
    _tool("surp_rfc001_query_ctn", "Run baseline RFC-001 CQL over CTN text.", _text_schema({"query": {"type": "string"}, "as_ctn": {"type": "boolean"}}, required=["query"])),
    _tool("surp_rfc001_query_cbf", "Run baseline RFC-001 CQL over CBF bytes.", _binary_schema({"query": {"type": "string"}, "as_ctn": {"type": "boolean"}}, required=["query"])),
]

RESOURCES = [
    {"uri": "surp://capabilities", "name": "Surp MCP Capabilities", "description": "Server capabilities and safety defaults.", "mimeType": "text/markdown"},
    {"uri": "surp://formats/v1", "name": "Surp v1 Format", "description": "Stable Surp v1 behavior.", "mimeType": "text/markdown"},
    {"uri": "surp://formats/rfc001", "name": "Surp RFC-001 Format", "description": "CTN, CBF, and CQL behavior.", "mimeType": "text/markdown"},
    {"uri": "surp://security", "name": "Surp MCP Security", "description": "Path and payload safeguards.", "mimeType": "text/markdown"},
    {"uri": "surp://examples/v1", "name": "Surp v1 Examples", "description": "v1 encode, decode, validate, and query examples.", "mimeType": "text/markdown"},
    {"uri": "surp://examples/rfc001", "name": "Surp RFC-001 Examples", "description": "CTN, CBF, and CQL examples.", "mimeType": "text/markdown"},
]

RESOURCE_TEXT = {
    "surp://capabilities": "Surp MCP exposes v1 Surp and RFC-001 CTN/CBF/CQL through stdio tools.",
    "surp://formats/v1": "Surp v1 is the stable block-framed .surp format with text notation.",
    "surp://formats/rfc001": "RFC-001 covers CTN text, CBF binary, and the implemented baseline CQL subset.",
    "surp://security": "Reads and writes are restricted to SURP_MCP_ROOTS, with payload size limits and stderr-only logs.",
    "surp://examples/v1": "Example: call surp_v1_encode_json with {'value': {'name': 'Alice'}}, then surp_v1_validate with returned data_base64.",
    "surp://examples/rfc001": "Example: call surp_rfc001_query_ctn with CTN text, query='.tags[-1]', and as_ctn=true.",
}

PROMPTS = [
    {"name": "surp_convert_json_to_binary", "description": "Encode JSON to v1 Surp and validate it.", "arguments": []},
    {"name": "surp_inspect_binary", "description": "Inspect v1 or RFC-001 binary data.", "arguments": []},
    {"name": "surp_query_rfc001", "description": "Query RFC-001 CTN/CBF with CQL.", "arguments": []},
]


def _prompt(params: dict[str, Any]) -> dict[str, Any]:
    name = params.get("name")
    if name not in {prompt["name"] for prompt in PROMPTS}:
        raise McpToolError(f"unknown prompt '{name}'")
    return {"description": str(name), "messages": [{"role": "user", "content": {"type": "text", "text": f"Use the Surp MCP tool sequence for {name}."}}]}


if __name__ == "__main__":
    main()
