# Surp MCP Security

The server is designed for local stdio MCP sessions.

- Logs are structured JSON on stderr only; stdout is reserved for MCP JSON-RPC.
- File reads and writes are restricted to allowed roots.
- `SURP_MCP_ROOTS` sets allowed roots using the platform path separator. The default root is the current working directory.
- `SURP_MCP_READ_ONLY` defaults to enabled (`true`): the server is read-only unless an operator explicitly opts in to writes by setting `SURP_MCP_READ_ONLY=false` (or `0`/`no`, matching the same falsy values accepted elsewhere). This is because the server executes filesystem operations on behalf of potentially untrusted LLM/agent input arriving over stdio, so unrestricted write access is not assumed silently.
- `SURP_MCP_MAX_INPUT_BYTES` defaults to 16 MiB.
- `SURP_MCP_MAX_INLINE_BYTES` defaults to 4 MiB.
- `SURP_MCP_MAX_TEXT_BYTES` defaults to 4 MiB.
- Decode tools default to `limits_profile="strict"`.

Tool failures are returned as MCP tool errors (`isError: true`) with actionable messages. Protocol errors are reserved for malformed JSON-RPC.
