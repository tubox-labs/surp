from __future__ import annotations

import json

from surp import mcp


def test_mcp_initialize(tmp_path):
    security = mcp.SecurityConfig(
        roots=[tmp_path.resolve()],
        read_only=False,
        max_input_bytes=1024 * 1024,
        max_inline_bytes=1024 * 1024,
        max_text_bytes=1024 * 1024,
    )
    response = mcp.handle_message(
        json.dumps(
            {
                "jsonrpc": "2.0",
                "id": 1,
                "method": "initialize",
                "params": {"protocolVersion": "test"},
            }
        ),
        security,
    )

    assert response["result"]["protocolVersion"] == "test"
    assert response["result"]["capabilities"]["tools"]["listChanged"] is False


def test_mcp_v1_encode_decode_tool_roundtrip(tmp_path):
    security = mcp.SecurityConfig(
        roots=[tmp_path.resolve()],
        read_only=False,
        max_input_bytes=1024 * 1024,
        max_inline_bytes=1024 * 1024,
        max_text_bytes=1024 * 1024,
    )

    encoded = mcp.call_tool(
        "surp_v1_encode_json",
        {"value": {"name": "Alice", "tags": ["admin", "ops"]}},
        security,
    )
    decoded = mcp.call_tool(
        "surp_v1_decode",
        {"data_base64": encoded["data_base64"]},
        security,
    )

    assert decoded["values"][0]["name"] == "Alice"
    assert decoded["values"][0]["tags"] == ["admin", "ops"]


def test_mcp_rfc001_query_ctn(tmp_path):
    security = mcp.SecurityConfig(
        roots=[tmp_path.resolve()],
        read_only=False,
        max_input_bytes=1024 * 1024,
        max_inline_bytes=1024 * 1024,
        max_text_bytes=1024 * 1024,
    )

    result = mcp.call_tool(
        "surp_rfc001_query_ctn",
        {
            "text": 'User\n  name = "Alice"\n  tags = ["admin", "ops"]',
            "query": ".tags[-1]",
            "as_ctn": True,
        },
        security,
    )

    assert result["matches"] == ['"ops"']
