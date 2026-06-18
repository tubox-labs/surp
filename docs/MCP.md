# Surp MCP Server

Surp ships two MCP stdio servers with the same tool names:

- Rust-native binary: `surp-mcp`
- Python-native module: `python -m surp.mcp` or the Python package script `surp-mcp-python`

Both servers expose Surp directly through library APIs. They do not shell out to `surp-cli`.

## Transports

The servers implement local stdio MCP JSON-RPC. Logs are structured JSON on stderr only; stdout is reserved for MCP protocol messages.

## Security

File tools are constrained by environment variables:

- `SURP_MCP_ROOTS`: allowed filesystem roots, separated by the platform path separator. Defaults to the process current directory.
- `SURP_MCP_READ_ONLY=1`: disables writes.
- `SURP_MCP_MAX_INPUT_BYTES`: maximum file or base64 input size. Default: 16 MiB.
- `SURP_MCP_MAX_INLINE_BYTES`: maximum inline response payload. Default: 4 MiB.
- `SURP_MCP_MAX_TEXT_BYTES`: maximum text input size. Default: 4 MiB.

Decode tools default to strict limits for untrusted MCP inputs.

## Tools

General:

- `surp_capabilities`

Stable v1 Surp:

- `surp_v1_encode_json`
- `surp_v1_decode`
- `surp_v1_text_parse`
- `surp_v1_text_pretty`
- `surp_v1_text_to_binary`
- `surp_v1_binary_to_text`
- `surp_v1_inspect`
- `surp_v1_validate`
- `surp_v1_query`

RFC-001:

- `surp_rfc001_parse_ctn`
- `surp_rfc001_normalize_ctn`
- `surp_rfc001_compile_ctn`
- `surp_rfc001_decode_cbf`
- `surp_rfc001_cbf_to_ctn`
- `surp_rfc001_inspect_cbf`
- `surp_rfc001_query_ctn`
- `surp_rfc001_query_cbf`

## Resources

- `surp://capabilities`
- `surp://formats/v1`
- `surp://formats/rfc001`
- `surp://security`
- `surp://examples/v1`
- `surp://examples/rfc001`

## Examples

Encode JSON to v1 Surp:

```json
{
  "name": "surp_v1_encode_json",
  "arguments": {
    "value": {"name": "Alice", "tags": ["admin", "ops"]},
    "dedup": true
  }
}
```

Validate returned bytes:

```json
{
  "name": "surp_v1_validate",
  "arguments": {
    "data_base64": "..."
  }
}
```

Compile RFC-001 CTN:

```json
{
  "name": "surp_rfc001_compile_ctn",
  "arguments": {
    "text": "User\n  name = \"Alice\"\n  tags = [\"admin\", \"ops\"]",
    "with_symtab": true
  }
}
```

Query RFC-001 CTN:

```json
{
  "name": "surp_rfc001_query_ctn",
  "arguments": {
    "text": "User\n  name = \"Alice\"\n  tags = [\"admin\", \"ops\"]",
    "query": ".tags[-1]",
    "as_ctn": true
  }
}
```

## Format Boundaries

Stable v1 `.surp` and RFC-001 CBF `.crb` are different binary formats. Use `surp_v1_*` tools for v1 data and `surp_rfc001_*` tools for RFC-001 CTN/CBF/CQL.

The MCP server exposes implemented behavior only. RFC-001 features not implemented in `surp-core`, such as full CSL, witness cryptography, full CQL pipelines, stream chunk framing, CPC RPC, DB pages, migrations, CBF index generation, and schema hash generation, are documented as gaps rather than advertised as tools.
