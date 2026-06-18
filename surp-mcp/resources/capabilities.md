# Surp MCP Capabilities

This server exposes Surp directly through MCP stdio tools.

- Stable v1 `.surp`: JSON encode/decode, Surp text parse/pretty, text/binary conversion, block inspection, validation, and basic structural queries.
- RFC-001: CTN parse/normalize, CTN-to-CBF compile, CBF decode/inspect, CBF-to-CTN rendering, and baseline CQL queries.
- Binary payloads use base64 for inline MCP transport and may read/write files under configured roots.
- Tools call `surp-core` directly; they do not shell out to `surp-cli`.

RFC-001 gaps that are not exposed as implemented tools include full CSL, witness cryptography, full CQL pipelines, stream chunk framing, CPC RPC, DB pages, migration DSL, CBF index generation, and schema hash generation.
