use serde_json::{Value as Json, json};

pub const SERVER_NAME: &str = "surp-mcp";
pub const SERVER_VERSION: &str = env!("CARGO_PKG_VERSION");
pub const PROTOCOL_VERSION: &str = "2024-11-05";

pub fn tools() -> Vec<Json> {
    vec![
        tool(
            "surp_capabilities",
            "Return Surp MCP capabilities, supported formats, safety limits, and compiled compression features.",
            object_schema(vec![]),
        ),
        tool(
            "surp_v1_encode_json",
            "Encode JSON input into the stable v1 Surp block-framed binary format.",
            json!({
                "type": "object",
                "properties": {
                    "value": {"description": "JSON value to encode as one top-level Surp value."},
                    "values": {"type": "array", "description": "Multiple JSON values to encode as multiple top-level Surp values."},
                    "json_text": {"type": "string", "description": "JSON text to parse and encode."},
                    "input_path": {"type": "string", "description": "Allowed path to a UTF-8 JSON file."},
                    "dedup": {"type": "boolean", "default": false},
                    "sort_keys": {"type": "boolean", "default": false},
                    "compression": compression_schema(),
                    "output_path": {"type": "string", "description": "Optional allowed path for binary output."},
                    "allow_overwrite": {"type": "boolean", "default": false},
                    "include_base64": {"type": "boolean", "description": "Include base64 bytes in the response. Defaults to true when output_path is omitted."}
                },
                "oneOf": [
                    {"required": ["value"]},
                    {"required": ["values"]},
                    {"required": ["json_text"]},
                    {"required": ["input_path"]}
                ],
                "additionalProperties": false
            }),
        ),
        tool(
            "surp_v1_decode",
            "Decode stable v1 Surp binary data into JSON-compatible and typed Surp values.",
            binary_input_schema(vec![
                (
                    "limits_profile",
                    json!({"type": "string", "enum": ["strict", "default", "unlimited"], "default": "strict"}),
                ),
                ("limits", limits_schema()),
                (
                    "json_style",
                    json!({"type": "string", "enum": ["pretty", "compact"], "default": "pretty"}),
                ),
                (
                    "output_path",
                    json!({"type": "string", "description": "Optional allowed path for rendered JSON output."}),
                ),
                (
                    "allow_overwrite",
                    json!({"type": "boolean", "default": false}),
                ),
                (
                    "include_json_text",
                    json!({"type": "boolean", "description": "Include rendered JSON text. Defaults to true when output_path is omitted."}),
                ),
            ]),
        ),
        tool(
            "surp_v1_text_parse",
            "Parse Surp v1 human-readable text notation into JSON-compatible and typed values.",
            text_input_schema(vec![(
                "indent",
                json!({"type": "integer", "minimum": 0, "maximum": 16, "default": 2}),
            )]),
        ),
        tool(
            "surp_v1_text_pretty",
            "Pretty-print a JSON value as Surp v1 human-readable text notation.",
            json!({
                "type": "object",
                "properties": {
                    "value": {"description": "JSON value to format."},
                    "json_text": {"type": "string", "description": "JSON text to parse before formatting."},
                    "input_path": {"type": "string", "description": "Allowed path to a UTF-8 JSON file."},
                    "sort_keys": {"type": "boolean", "default": false},
                    "indent": {"type": "integer", "minimum": 0, "maximum": 16, "default": 2},
                    "output_path": {"type": "string"},
                    "allow_overwrite": {"type": "boolean", "default": false},
                    "include_text": {"type": "boolean"}
                },
                "oneOf": [
                    {"required": ["value"]},
                    {"required": ["json_text"]},
                    {"required": ["input_path"]}
                ],
                "additionalProperties": false
            }),
        ),
        tool(
            "surp_v1_text_to_binary",
            "Compile Surp v1 text notation into stable v1 Surp binary data.",
            text_input_schema(vec![
                ("dedup", json!({"type": "boolean", "default": false})),
                ("compression", compression_schema()),
                ("output_path", json!({"type": "string"})),
                (
                    "allow_overwrite",
                    json!({"type": "boolean", "default": false}),
                ),
                ("include_base64", json!({"type": "boolean"})),
            ]),
        ),
        tool(
            "surp_v1_binary_to_text",
            "Decode stable v1 Surp binary data and render Surp text notation.",
            binary_input_schema(vec![
                (
                    "limits_profile",
                    json!({"type": "string", "enum": ["strict", "default", "unlimited"], "default": "strict"}),
                ),
                ("limits", limits_schema()),
                (
                    "indent",
                    json!({"type": "integer", "minimum": 0, "maximum": 16, "default": 2}),
                ),
                ("output_path", json!({"type": "string"})),
                (
                    "allow_overwrite",
                    json!({"type": "boolean", "default": false}),
                ),
                ("include_text", json!({"type": "boolean"})),
            ]),
        ),
        tool(
            "surp_v1_inspect",
            "Inspect stable v1 Surp block layout, compression markers, payload checksums, and trailer integrity.",
            binary_input_schema(vec![]),
        ),
        tool(
            "surp_v1_validate",
            "Validate stable v1 Surp framing, checksums, trailer, and optionally full decode integrity.",
            binary_input_schema(vec![
                (
                    "checksums_only",
                    json!({"type": "boolean", "default": false}),
                ),
                (
                    "limits_profile",
                    json!({"type": "string", "enum": ["strict", "default", "unlimited"], "default": "strict"}),
                ),
                ("limits", limits_schema()),
            ]),
        ),
        tool(
            "surp_v1_query",
            "Run a basic structural path query over v1 Surp data using .field, [index], negative indexes, and [] flatten selectors.",
            json!({
                "type": "object",
                "properties": {
                    "query": {"type": "string", "description": "Path expression such as .users[0].name or .tags[]."},
                    "data_base64": {"type": "string"},
                    "text": {"type": "string"},
                    "value": {"description": "JSON value to query."},
                    "json_text": {"type": "string"},
                    "input_path": {"type": "string", "description": "Allowed path to .surp, Surp text, or JSON input."},
                    "input_format": {"type": "string", "enum": ["surp_binary", "surp_text", "json"], "description": "Required when input_path is ambiguous."},
                    "limits_profile": {"type": "string", "enum": ["strict", "default", "unlimited"], "default": "strict"},
                    "limits": limits_schema(),
                    "result_format": {"type": "string", "enum": ["json", "typed", "text"], "default": "json"},
                    "indent": {"type": "integer", "minimum": 0, "maximum": 16, "default": 2}
                },
                "required": ["query"],
                "oneOf": [
                    {"required": ["data_base64"]},
                    {"required": ["text"]},
                    {"required": ["value"]},
                    {"required": ["json_text"]},
                    {"required": ["input_path"]}
                ],
                "additionalProperties": false
            }),
        ),
        tool(
            "surp_rfc001_parse_ctn",
            "Parse an RFC-001 CTN document into typed document metadata and values.",
            text_input_schema(vec![(
                "include_normalized_ctn",
                json!({"type": "boolean", "default": true}),
            )]),
        ),
        tool(
            "surp_rfc001_normalize_ctn",
            "Parse and canonicalize RFC-001 CTN text.",
            text_input_schema(vec![
                ("output_path", json!({"type": "string"})),
                (
                    "allow_overwrite",
                    json!({"type": "boolean", "default": false}),
                ),
                ("include_text", json!({"type": "boolean"})),
            ]),
        ),
        tool(
            "surp_rfc001_compile_ctn",
            "Compile RFC-001 CTN text into RFC-001 CBF bytes.",
            text_input_schema(vec![
                ("with_symtab", json!({"type": "boolean", "default": true})),
                (
                    "alignment",
                    json!({"type": "integer", "minimum": 0, "maximum": 255, "default": 0}),
                ),
                ("output_path", json!({"type": "string"})),
                (
                    "allow_overwrite",
                    json!({"type": "boolean", "default": false}),
                ),
                ("include_base64", json!({"type": "boolean"})),
            ]),
        ),
        tool(
            "surp_rfc001_decode_cbf",
            "Decode RFC-001 CBF bytes into header metadata, symbol table, typed document, and CTN text.",
            binary_input_schema(vec![(
                "include_ctn",
                json!({"type": "boolean", "default": true}),
            )]),
        ),
        tool(
            "surp_rfc001_cbf_to_ctn",
            "Decode RFC-001 CBF bytes and return canonical CTN text.",
            binary_input_schema(vec![
                ("output_path", json!({"type": "string"})),
                (
                    "allow_overwrite",
                    json!({"type": "boolean", "default": false}),
                ),
                ("include_text", json!({"type": "boolean"})),
            ]),
        ),
        tool(
            "surp_rfc001_inspect_cbf",
            "Inspect RFC-001 CBF header fields, symbol metadata, offsets, and checksum validity through full decode.",
            binary_input_schema(vec![]),
        ),
        tool(
            "surp_rfc001_query_ctn",
            "Run baseline RFC-001 CQL over CTN text and return typed values or CTN snippets.",
            text_input_schema(vec![
                ("query", json!({"type": "string"})),
                ("as_ctn", json!({"type": "boolean", "default": false})),
            ])
            .with_required(&["query"]),
        ),
        tool(
            "surp_rfc001_query_cbf",
            "Run baseline RFC-001 CQL over CBF bytes and return typed values or CTN snippets.",
            binary_input_schema(vec![
                ("query", json!({"type": "string"})),
                ("as_ctn", json!({"type": "boolean", "default": false})),
            ])
            .with_required(&["query"]),
        ),
    ]
}

pub fn resources() -> Vec<Json> {
    vec![
        resource(
            "surp://capabilities",
            "Surp MCP Capabilities",
            "Complete server capabilities and safety defaults.",
        ),
        resource(
            "surp://formats/v1",
            "Surp v1 Format",
            "Stable block-framed Surp binary and text notation behavior.",
        ),
        resource(
            "surp://formats/rfc001",
            "Surp RFC-001 Format",
            "CTN, CBF, and baseline CQL behavior currently implemented.",
        ),
        resource(
            "surp://security",
            "Surp MCP Security",
            "Path allowlists, payload limits, writes, and logging behavior.",
        ),
        resource(
            "surp://examples/v1",
            "Surp v1 Examples",
            "JSON, text, binary, inspect, validate, and query examples.",
        ),
        resource(
            "surp://examples/rfc001",
            "Surp RFC-001 Examples",
            "CTN, CBF, and CQL examples.",
        ),
    ]
}

pub fn resource_text(uri: &str) -> Option<&'static str> {
    match uri {
        "surp://capabilities" => Some(include_str!("../resources/capabilities.md")),
        "surp://formats/v1" => Some(include_str!("../resources/format_v1.md")),
        "surp://formats/rfc001" => Some(include_str!("../resources/format_rfc001.md")),
        "surp://security" => Some(include_str!("../resources/security.md")),
        "surp://examples/v1" => Some(include_str!("../resources/examples_v1.md")),
        "surp://examples/rfc001" => Some(include_str!("../resources/examples_rfc001.md")),
        _ => None,
    }
}

pub fn prompts() -> Vec<Json> {
    vec![
        json!({
            "name": "surp_convert_json_to_binary",
            "description": "Guide an agent through encoding JSON to v1 Surp and validating the result.",
            "arguments": [
                {"name": "json_summary", "description": "Short description of the JSON payload.", "required": false}
            ]
        }),
        json!({
            "name": "surp_inspect_binary",
            "description": "Guide an agent through inspecting and validating a Surp binary payload.",
            "arguments": [
                {"name": "format", "description": "surp_binary or rfc001_cbf", "required": false}
            ]
        }),
        json!({
            "name": "surp_query_rfc001",
            "description": "Guide an agent through compiling CTN/CBF and querying with baseline CQL.",
            "arguments": [
                {"name": "query_goal", "description": "What to extract from the document.", "required": false}
            ]
        }),
    ]
}

pub fn prompt(name: &str, args: Option<&Json>) -> Option<Json> {
    let arg = |key: &str| {
        args.and_then(|value| value.get(key))
            .and_then(Json::as_str)
            .unwrap_or("")
    };
    let text = match name {
        "surp_convert_json_to_binary" => format!(
            "Use surp_v1_encode_json to encode the JSON payload ({}) into Surp v1 binary. Then call surp_v1_inspect and surp_v1_validate on the returned base64 or output path. Keep v1 Surp distinct from RFC-001 CBF.",
            arg("json_summary")
        ),
        "surp_inspect_binary" => format!(
            "Inspect the {} payload. For v1 .surp use surp_v1_inspect and surp_v1_validate. For RFC-001 CBF use surp_rfc001_inspect_cbf or surp_rfc001_decode_cbf. Report checksum, trailer/header, compression, and decode errors.",
            arg("format")
        ),
        "surp_query_rfc001" => format!(
            "Use surp_rfc001_query_ctn for CTN text or surp_rfc001_query_cbf for CBF bytes. Goal: {}. CQL supports .field, [], [index], negative indexes, ['symbol], and [\"string\"].",
            arg("query_goal")
        ),
        _ => return None,
    };
    Some(json!({
        "description": name,
        "messages": [{
            "role": "user",
            "content": {"type": "text", "text": text}
        }]
    }))
}

fn tool(name: &str, description: &str, input_schema: Json) -> Json {
    json!({
        "name": name,
        "description": description,
        "inputSchema": input_schema,
        "outputSchema": {"type": "object"},
        "annotations": {
            "readOnlyHint": !name.contains("encode_json") && !name.contains("text_to_binary") && !name.contains("compile_ctn"),
            "destructiveHint": false,
            "idempotentHint": true,
            "openWorldHint": false
        }
    })
}

fn resource(uri: &str, name: &str, description: &str) -> Json {
    json!({
        "uri": uri,
        "name": name,
        "description": description,
        "mimeType": "text/markdown"
    })
}

fn object_schema(properties: Vec<(&str, Json)>) -> Json {
    let mut props = serde_json::Map::new();
    for (key, value) in properties {
        props.insert(key.to_string(), value);
    }
    json!({
        "type": "object",
        "properties": props,
        "additionalProperties": false
    })
}

fn binary_input_schema(extra: Vec<(&str, Json)>) -> Json {
    let mut schema = object_schema(vec![
        (
            "data_base64",
            json!({"type": "string", "description": "Inline base64-encoded binary data."}),
        ),
        (
            "input_path",
            json!({"type": "string", "description": "Allowed path to read binary data from."}),
        ),
    ]);
    add_properties(&mut schema, extra);
    schema["oneOf"] = json!([
        {"required": ["data_base64"]},
        {"required": ["input_path"]}
    ]);
    schema
}

fn text_input_schema(extra: Vec<(&str, Json)>) -> Json {
    let mut schema = object_schema(vec![
        (
            "text",
            json!({"type": "string", "description": "Inline UTF-8 text."}),
        ),
        (
            "input_path",
            json!({"type": "string", "description": "Allowed path to read UTF-8 text from."}),
        ),
    ]);
    add_properties(&mut schema, extra);
    schema["oneOf"] = json!([
        {"required": ["text"]},
        {"required": ["input_path"]}
    ]);
    schema
}

fn add_properties(schema: &mut Json, extra: Vec<(&str, Json)>) {
    let properties = schema
        .get_mut("properties")
        .and_then(Json::as_object_mut)
        .expect("object schema properties");
    for (key, value) in extra {
        properties.insert(key.to_string(), value);
    }
}

fn compression_schema() -> Json {
    json!({"type": "string", "enum": ["none", "lz4", "snappy", "zstd"], "default": "none"})
}

fn limits_schema() -> Json {
    json!({
        "type": "object",
        "properties": {
            "max_nesting_depth": {"type": "integer", "minimum": 1},
            "max_block_size": {"type": "integer", "minimum": 1},
            "max_items": {"type": "integer", "minimum": 1},
            "max_memory": {"type": "integer", "minimum": 1},
            "max_string_length": {"type": "integer", "minimum": 1},
            "max_decompression_ratio": {"type": "integer", "minimum": 1}
        },
        "additionalProperties": false
    })
}

trait WithRequired {
    fn with_required(self, required: &[&str]) -> Self;
}

impl WithRequired for Json {
    fn with_required(mut self, required: &[&str]) -> Self {
        self["required"] = Json::Array(
            required
                .iter()
                .map(|value| Json::String((*value).to_string()))
                .collect(),
        );
        self
    }
}
