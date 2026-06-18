# Surp RFC-001 MCP Examples

Compile CTN:

```json
{"text":"User\n  name = \"Alice\"\n  tags = [\"admin\", \"ops\"]","with_symtab":true}
```

Decode CBF:

```json
{"data_base64":"...","include_ctn":true}
```

Query CTN:

```json
{"text":"User\n  name = \"Alice\"\n  tags = [\"admin\", \"ops\"]","query":".tags[-1]","as_ctn":true}
```
