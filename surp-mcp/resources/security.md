# Surp MCP Security

The server is designed for local stdio MCP sessions.

- Logs are structured JSON on stderr only; stdout is reserved for MCP JSON-RPC.
- File reads and writes are restricted to allowed roots.
- `SURP_MCP_ROOTS` sets allowed roots using the platform path separator. The default root is the current working directory.
- `SURP_MCP_READ_ONLY=1` disables file writes.
- `SURP_MCP_MAX_INPUT_BYTES` defaults to 16 MiB.
- `SURP_MCP_MAX_INLINE_BYTES` defaults to 4 MiB.
- `SURP_MCP_MAX_TEXT_BYTES` defaults to 4 MiB.
- Decode tools default to `limits_profile="strict"`.

Tool failures are returned as MCP tool errors (`isError: true`) with actionable messages. Protocol errors are reserved for malformed JSON-RPC.
