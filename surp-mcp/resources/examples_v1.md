# Surp v1 MCP Examples

Encode JSON:

```json
{"value":{"name":"Alice","tags":["admin","ops"]},"dedup":true}
```

Decode returned base64:

```json
{"data_base64":"...","limits_profile":"strict"}
```

Compile text notation:

```json
{"text":"{ name: \"Alice\"; tags: [\"admin\", \"ops\"]; }"}
```

Query v1 data:

```json
{"data_base64":"...","query":".tags[-1]","result_format":"text"}
```
