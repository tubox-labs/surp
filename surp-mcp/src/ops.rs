use std::collections::BTreeMap;

use base64::Engine;
use base64::engine::general_purpose::STANDARD as BASE64;
use serde_json::{Map, Number, Value as Json, json};
use surp_core::block::BlockReader;
use surp_core::checksum::compute_xxh64;
use surp_core::rfc001;
use surp_core::wire::{BlockType, CompressionType};
use surp_core::{Decoder, Encoder, Limits, Value};
use thiserror::Error;

use crate::registry;
use crate::security::{SecurityConfig, SecurityError};

#[derive(Debug, Error)]
pub enum OpError {
    #[error("{0}")]
    Message(String),

    #[error("Surp operation failed: {0}")]
    Core(#[from] surp_core::SurpError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("base64 error: {0}")]
    Base64(#[from] base64::DecodeError),

    #[error("UTF-8 error: {0}")]
    Utf8(#[from] std::string::FromUtf8Error),

    #[error("security error: {0}")]
    Security(#[from] SecurityError),
}

#[derive(Debug, Clone)]
pub struct ToolOutcome {
    pub text: String,
    pub structured: Json,
}

pub fn call_tool(
    name: &str,
    arguments: &Json,
    security: &SecurityConfig,
) -> Result<ToolOutcome, OpError> {
    let args = object(arguments)?;
    match name {
        "surp_capabilities" => capabilities(security),
        "surp_v1_encode_json" => v1_encode_json(args, security),
        "surp_v1_decode" => v1_decode(args, security),
        "surp_v1_text_parse" => v1_text_parse(args, security),
        "surp_v1_text_pretty" => v1_text_pretty(args, security),
        "surp_v1_text_to_binary" => v1_text_to_binary(args, security),
        "surp_v1_binary_to_text" => v1_binary_to_text(args, security),
        "surp_v1_inspect" => v1_inspect(args, security),
        "surp_v1_validate" => v1_validate(args, security),
        "surp_v1_query" => v1_query(args, security),
        "surp_rfc001_parse_ctn" => rfc001_parse_ctn(args, security),
        "surp_rfc001_normalize_ctn" => rfc001_normalize_ctn(args, security),
        "surp_rfc001_compile_ctn" => rfc001_compile_ctn(args, security),
        "surp_rfc001_decode_cbf" => rfc001_decode_cbf(args, security),
        "surp_rfc001_cbf_to_ctn" => rfc001_cbf_to_ctn(args, security),
        "surp_rfc001_inspect_cbf" => rfc001_inspect_cbf(args, security),
        "surp_rfc001_query_ctn" => rfc001_query_ctn(args, security),
        "surp_rfc001_query_cbf" => rfc001_query_cbf(args, security),
        _ => Err(OpError::Message(format!("unknown Surp MCP tool '{name}'"))),
    }
}

fn capabilities(security: &SecurityConfig) -> Result<ToolOutcome, OpError> {
    let structured = json!({
        "server": {
            "name": registry::SERVER_NAME,
            "version": registry::SERVER_VERSION,
            "protocol_version": registry::PROTOCOL_VERSION,
        },
        "formats": {
            "v1": {
                "binary": "block-framed .surp",
                "text": "Surp v1 text notation",
                "capabilities": [
                    "encode JSON to binary",
                    "decode binary to JSON-compatible values",
                    "parse and pretty-print text notation",
                    "inspect block layout and checksums",
                    "validate trailer/checksums/decode integrity",
                    "basic structural path queries"
                ]
            },
            "rfc001": {
                "text": "CTN",
                "binary": "CBF .crb",
                "query": "baseline CQL path queries",
                "capabilities": [
                    "parse CTN",
                    "normalize CTN",
                    "compile CTN to CBF",
                    "decode CBF",
                    "inspect CBF header and symbol metadata",
                    "query CTN/CBF with implemented CQL subset"
                ],
                "known_gaps_not_exposed_as_tools": [
                    "full CSL",
                    "witness cryptography",
                    "full CQL pipelines",
                    "stream chunk framing",
                    "CPC RPC",
                    "database pages",
                    "CBF index generation"
                ]
            }
        },
        "compiled_features": compression_features(),
        "security": {
            "roots": security.roots().iter().map(|path| path.display().to_string()).collect::<Vec<_>>(),
            "max_input_bytes": security.max_input_bytes,
            "max_inline_bytes": security.max_inline_bytes,
            "max_text_bytes": security.max_text_bytes,
            "default_limits_profile": "strict",
        },
        "tool_count": registry::tools().len(),
        "resource_count": registry::resources().len(),
    });
    Ok(outcome(
        "Surp MCP exposes v1 Surp and RFC-001 CTN/CBF/CQL operations.",
        structured,
    ))
}

fn v1_encode_json(
    args: &Map<String, Json>,
    security: &SecurityConfig,
) -> Result<ToolOutcome, OpError> {
    let mut values = json_values(args, security)?;
    if bool_arg(args, "sort_keys", false)? {
        for value in &mut values {
            sort_json(value);
        }
    }
    let mut surp_values: Vec<Value> = values.iter().map(Value::from).collect();
    if bool_arg(args, "sort_keys", false)? {
        for value in &mut surp_values {
            sort_surp_value(value);
        }
    }
    let bytes = encode_values(
        &surp_values,
        bool_arg(args, "dedup", false)?,
        compression_arg(args)?,
    )?;
    let mut structured = binary_output(args, security, &bytes)?;
    structured["value_count"] = json!(surp_values.len());
    structured["inspect"] = analyze_blocks_json(&bytes)?;
    Ok(outcome(
        format!(
            "Encoded {} v1 Surp value(s), {} bytes.",
            surp_values.len(),
            bytes.len()
        ),
        structured,
    ))
}

fn v1_decode(args: &Map<String, Json>, security: &SecurityConfig) -> Result<ToolOutcome, OpError> {
    let input = read_binary(args, security)?;
    let limits = limits_arg(args)?;
    let mut decoder = Decoder::with_limits(&input.data, limits);
    let values = decoder.decode_all_owned()?;
    let json_values: Vec<Json> = values.iter().map(Json::from).collect();
    let typed_values: Vec<Json> = values.iter().map(v1_value_to_json).collect();
    let rendered = render_json_values(&json_values, json_style(args)?)?;

    let mut structured = json!({
        "source": input.source,
        "value_count": values.len(),
        "memory_used": decoder.memory_used(),
        "values": json_values,
        "typed_values": typed_values,
    });
    merge_object(
        &mut structured,
        text_output(args, security, "json_text", &rendered)?,
    );

    Ok(outcome(
        format!("Decoded {} v1 Surp value(s).", values.len()),
        structured,
    ))
}

fn v1_text_parse(
    args: &Map<String, Json>,
    security: &SecurityConfig,
) -> Result<ToolOutcome, OpError> {
    let input = read_text(args, security)?;
    let value = surp_core::text::parse(&input.text)?;
    let indent = usize_arg(args, "indent", 2)?;
    let normalized = surp_core::text::pretty_print(&value, indent);
    let structured = json!({
        "source": input.source,
        "value": Json::from(&value),
        "typed_value": v1_value_to_json(&value),
        "normalized_text": normalized,
    });
    Ok(outcome("Parsed v1 Surp text notation.", structured))
}

fn v1_text_pretty(
    args: &Map<String, Json>,
    security: &SecurityConfig,
) -> Result<ToolOutcome, OpError> {
    let json = single_json_value(args, security)?;
    let mut value = Value::from(&json);
    if bool_arg(args, "sort_keys", false)? {
        sort_surp_value(&mut value);
    }
    let text = surp_core::text::pretty_print(&value, usize_arg(args, "indent", 2)?);
    let mut structured = json!({
        "typed_value": v1_value_to_json(&value),
    });
    merge_object(&mut structured, text_output(args, security, "text", &text)?);
    Ok(outcome("Rendered v1 Surp text notation.", structured))
}

fn v1_text_to_binary(
    args: &Map<String, Json>,
    security: &SecurityConfig,
) -> Result<ToolOutcome, OpError> {
    let input = read_text(args, security)?;
    let value = surp_core::text::parse(&input.text)?;
    let bytes = encode_values(
        &[value],
        bool_arg(args, "dedup", false)?,
        compression_arg(args)?,
    )?;
    let mut structured = binary_output(args, security, &bytes)?;
    structured["source"] = json!(input.source);
    structured["inspect"] = analyze_blocks_json(&bytes)?;
    Ok(outcome(
        format!("Compiled v1 Surp text to {} bytes.", bytes.len()),
        structured,
    ))
}

fn v1_binary_to_text(
    args: &Map<String, Json>,
    security: &SecurityConfig,
) -> Result<ToolOutcome, OpError> {
    let input = read_binary(args, security)?;
    let limits = limits_arg(args)?;
    let mut decoder = Decoder::with_limits(&input.data, limits);
    let values = decoder.decode_all_owned()?;
    let text = values
        .iter()
        .map(|value| {
            surp_core::text::pretty_print(value, usize_arg(args, "indent", 2).unwrap_or(2))
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    let mut structured = json!({
        "source": input.source,
        "value_count": values.len(),
        "memory_used": decoder.memory_used(),
    });
    merge_object(&mut structured, text_output(args, security, "text", &text)?);
    Ok(outcome(
        format!("Rendered {} v1 Surp value(s) as text.", values.len()),
        structured,
    ))
}

fn v1_inspect(args: &Map<String, Json>, security: &SecurityConfig) -> Result<ToolOutcome, OpError> {
    let input = read_binary(args, security)?;
    let mut structured = analyze_blocks_json(&input.data)?;
    structured["source"] = json!(input.source);
    let valid = structured["trailer"]
        .get("payload_checksum_valid")
        .and_then(Json::as_bool)
        .unwrap_or(false)
        && structured["trailer"]
            .get("file_checksum_valid")
            .and_then(Json::as_bool)
            .unwrap_or(false)
        && structured["blocks"]
            .as_array()
            .unwrap_or(&Vec::new())
            .iter()
            .all(|block| block["payload_checksum_valid"].as_bool().unwrap_or(true));
    structured["valid_framing_checksums"] = json!(valid);
    Ok(outcome("Inspected v1 Surp block layout.", structured))
}

fn v1_validate(
    args: &Map<String, Json>,
    security: &SecurityConfig,
) -> Result<ToolOutcome, OpError> {
    let input = read_binary(args, security)?;
    let report = analyze_blocks_json(&input.data)?;
    let mut errors = Vec::new();

    let blocks = report["blocks"].as_array().cloned().unwrap_or_default();
    for block in &blocks {
        if block["payload_checksum_valid"].as_bool() == Some(false) {
            errors.push(format!(
                "block #{} payload checksum is invalid",
                block["index"]
            ));
        }
    }
    match report.get("trailer") {
        Some(trailer)
            if trailer["payload_checksum_valid"].as_bool() == Some(true)
                && trailer["file_checksum_valid"].as_bool() == Some(true) => {}
        Some(trailer) => {
            if trailer["payload_checksum_valid"].as_bool() != Some(true) {
                errors.push("trailer payload checksum is invalid".to_string());
            }
            if trailer["file_checksum_valid"].as_bool() != Some(true) {
                errors.push("trailer file checksum is invalid".to_string());
            }
        }
        None => errors.push("missing trailer block".to_string()),
    }

    let compressed_blocks: Vec<Json> = blocks
        .iter()
        .filter(|block| block["payload_checksum_valid"].is_null())
        .map(|block| block["index"].clone())
        .collect();

    if bool_arg(args, "checksums_only", false)? && !compressed_blocks.is_empty() {
        errors.push(format!(
            "compressed block(s) {compressed_blocks:?} require full decode for checksum validation"
        ));
    }

    let mut decoded_values = None;
    let mut memory_used = None;
    if errors.is_empty() && !bool_arg(args, "checksums_only", false)? {
        let mut decoder = Decoder::with_limits(&input.data, limits_arg(args)?);
        match decoder.decode_all_owned() {
            Ok(values) => {
                decoded_values = Some(values.len());
                memory_used = Some(decoder.memory_used());
            }
            Err(err) => errors.push(format!("decode validation failed: {err}")),
        }
    }

    let valid = errors.is_empty();
    let structured = json!({
        "source": input.source,
        "valid": valid,
        "errors": errors,
        "decoded_value_count": decoded_values,
        "memory_used": memory_used,
        "inspect": report,
    });
    Ok(outcome(
        if valid {
            "v1 Surp validation passed.".to_string()
        } else {
            "v1 Surp validation failed.".to_string()
        },
        structured,
    ))
}

fn v1_query(args: &Map<String, Json>, security: &SecurityConfig) -> Result<ToolOutcome, OpError> {
    let query = required_str(args, "query")?;
    let values = v1_query_input(args, security)?;
    let matches = query_v1_values(&values, query)?;
    let result_format = str_arg(args, "result_format")?.unwrap_or("json");
    let rendered = match result_format {
        "json" => Json::Array(matches.iter().map(Json::from).collect()),
        "typed" => Json::Array(matches.iter().map(v1_value_to_json).collect()),
        "text" => Json::Array(
            matches
                .iter()
                .map(|value| {
                    Json::String(surp_core::text::pretty_print(
                        value,
                        usize_arg(args, "indent", 2).unwrap_or(2),
                    ))
                })
                .collect(),
        ),
        other => {
            return Err(OpError::Message(format!(
                "unsupported result_format '{other}'"
            )));
        }
    };
    let structured = json!({
        "query": query,
        "input_value_count": values.len(),
        "match_count": matches.len(),
        "matches": rendered,
        "typed_matches": matches.iter().map(v1_value_to_json).collect::<Vec<_>>(),
    });
    Ok(outcome(
        format!("v1 query returned {} match(es).", matches.len()),
        structured,
    ))
}

fn rfc001_parse_ctn(
    args: &Map<String, Json>,
    security: &SecurityConfig,
) -> Result<ToolOutcome, OpError> {
    let input = read_text(args, security)?;
    let doc = rfc001::parse_document(&input.text)?;
    let mut structured = json!({
        "source": input.source,
        "document": rfc_document_to_json(&doc),
    });
    if bool_arg(args, "include_normalized_ctn", true)? {
        structured["normalized_ctn"] = json!(rfc001::format_document(&doc));
    }
    Ok(outcome("Parsed RFC-001 CTN document.", structured))
}

fn rfc001_normalize_ctn(
    args: &Map<String, Json>,
    security: &SecurityConfig,
) -> Result<ToolOutcome, OpError> {
    let input = read_text(args, security)?;
    let doc = rfc001::parse_document(&input.text)?;
    let text = rfc001::format_document(&doc);
    let mut structured = json!({
        "source": input.source,
        "document": rfc_document_to_json(&doc),
    });
    merge_object(&mut structured, text_output(args, security, "text", &text)?);
    Ok(outcome("Normalized RFC-001 CTN.", structured))
}

fn rfc001_compile_ctn(
    args: &Map<String, Json>,
    security: &SecurityConfig,
) -> Result<ToolOutcome, OpError> {
    let input = read_text(args, security)?;
    let doc = rfc001::parse_document(&input.text)?;
    let alignment = u8_arg(args, "alignment", 0)?;
    let bytes = rfc001::encode_document(
        &doc,
        rfc001::EncodeOptions {
            with_symtab: bool_arg(args, "with_symtab", true)?,
            alignment,
        },
    )?;
    let mut structured = binary_output(args, security, &bytes)?;
    structured["source"] = json!(input.source);
    structured["document"] = rfc_document_to_json(&doc);
    structured["header"] = json!({
        "magic": "SURP",
        "header_size": rfc001::CBF_HEADER_SIZE,
    });
    Ok(outcome(
        format!("Compiled RFC-001 CTN to {} CBF bytes.", bytes.len()),
        structured,
    ))
}

fn rfc001_decode_cbf(
    args: &Map<String, Json>,
    security: &SecurityConfig,
) -> Result<ToolOutcome, OpError> {
    let input = read_binary(args, security)?;
    let decoded = rfc001::decode_document(&input.data)?;
    let mut structured = rfc_decoded_to_json(&decoded);
    structured["source"] = json!(input.source);
    if !bool_arg(args, "include_ctn", true)? {
        if let Some(obj) = structured.as_object_mut() {
            obj.remove("ctn");
        }
    }
    Ok(outcome("Decoded RFC-001 CBF document.", structured))
}

fn rfc001_cbf_to_ctn(
    args: &Map<String, Json>,
    security: &SecurityConfig,
) -> Result<ToolOutcome, OpError> {
    let input = read_binary(args, security)?;
    let decoded = rfc001::decode_document(&input.data)?;
    let text = rfc001::format_document(&decoded.document);
    let mut structured = json!({
        "source": input.source,
        "header": rfc_header_to_json(&decoded.header),
        "symbol_count": decoded.symbols.len(),
    });
    merge_object(&mut structured, text_output(args, security, "text", &text)?);
    Ok(outcome("Rendered RFC-001 CBF as CTN.", structured))
}

fn rfc001_inspect_cbf(
    args: &Map<String, Json>,
    security: &SecurityConfig,
) -> Result<ToolOutcome, OpError> {
    let input = read_binary(args, security)?;
    let structured = match rfc001::decode_document(&input.data) {
        Ok(decoded) => json!({
            "source": input.source,
            "valid": true,
            "byte_length": input.data.len(),
            "header": rfc_header_to_json(&decoded.header),
            "symbols": decoded.symbols,
            "symbol_count": decoded.symbols.len(),
            "ctn_preview": rfc001::format_document(&decoded.document),
        }),
        Err(err) => json!({
            "source": input.source,
            "valid": false,
            "byte_length": input.data.len(),
            "error": err.to_string(),
        }),
    };
    Ok(outcome("Inspected RFC-001 CBF.", structured))
}

fn rfc001_query_ctn(
    args: &Map<String, Json>,
    security: &SecurityConfig,
) -> Result<ToolOutcome, OpError> {
    let query = required_str(args, "query")?;
    let input = read_text(args, security)?;
    let doc = rfc001::parse_document(&input.text)?;
    let bytes = rfc001::encode_document(&doc, rfc001::EncodeOptions::default())?;
    let decoded = rfc001::decode_document(&bytes)?;
    let root = decoded.document.effective_root()?;
    let results = rfc001::query(&root, query)?;
    rfc_query_outcome(
        query,
        input.source,
        &results,
        bool_arg(args, "as_ctn", false)?,
    )
}

fn rfc001_query_cbf(
    args: &Map<String, Json>,
    security: &SecurityConfig,
) -> Result<ToolOutcome, OpError> {
    let query = required_str(args, "query")?;
    let input = read_binary(args, security)?;
    let decoded = rfc001::decode_document(&input.data)?;
    let root = decoded.document.effective_root()?;
    let results = rfc001::query(&root, query)?;
    rfc_query_outcome(
        query,
        input.source,
        &results,
        bool_arg(args, "as_ctn", false)?,
    )
}

fn rfc_query_outcome(
    query: &str,
    source: String,
    results: &[rfc001::Value],
    as_ctn: bool,
) -> Result<ToolOutcome, OpError> {
    let values = if as_ctn {
        Json::Array(
            results
                .iter()
                .map(|value| Json::String(rfc001::format_value(value)))
                .collect(),
        )
    } else {
        Json::Array(results.iter().map(rfc_value_to_json).collect())
    };
    let structured = json!({
        "source": source,
        "query": query,
        "match_count": results.len(),
        "matches": values,
        "ctn_matches": results.iter().map(rfc001::format_value).collect::<Vec<_>>(),
    });
    Ok(outcome(
        format!("RFC-001 query returned {} match(es).", results.len()),
        structured,
    ))
}

#[derive(Debug)]
struct BinaryInput {
    source: String,
    data: Vec<u8>,
}

#[derive(Debug)]
struct TextInput {
    source: String,
    text: String,
}

fn outcome(text: impl Into<String>, structured: Json) -> ToolOutcome {
    ToolOutcome {
        text: text.into(),
        structured,
    }
}

fn object(value: &Json) -> Result<&Map<String, Json>, OpError> {
    value
        .as_object()
        .ok_or_else(|| OpError::Message("tool arguments must be a JSON object".to_string()))
}

fn required_str<'a>(args: &'a Map<String, Json>, key: &str) -> Result<&'a str, OpError> {
    str_arg(args, key)?.ok_or_else(|| OpError::Message(format!("missing required string '{key}'")))
}

fn str_arg<'a>(args: &'a Map<String, Json>, key: &str) -> Result<Option<&'a str>, OpError> {
    match args.get(key) {
        Some(Json::String(value)) => Ok(Some(value)),
        Some(Json::Null) | None => Ok(None),
        Some(_) => Err(OpError::Message(format!("'{key}' must be a string"))),
    }
}

fn bool_arg(args: &Map<String, Json>, key: &str, default: bool) -> Result<bool, OpError> {
    match args.get(key) {
        Some(Json::Bool(value)) => Ok(*value),
        Some(Json::Null) | None => Ok(default),
        Some(_) => Err(OpError::Message(format!("'{key}' must be a boolean"))),
    }
}

fn usize_arg(args: &Map<String, Json>, key: &str, default: usize) -> Result<usize, OpError> {
    match args.get(key) {
        Some(Json::Number(value)) => value
            .as_u64()
            .and_then(|value| usize::try_from(value).ok())
            .ok_or_else(|| OpError::Message(format!("'{key}' must be a non-negative integer"))),
        Some(Json::Null) | None => Ok(default),
        Some(_) => Err(OpError::Message(format!("'{key}' must be an integer"))),
    }
}

fn u8_arg(args: &Map<String, Json>, key: &str, default: u8) -> Result<u8, OpError> {
    let value = usize_arg(args, key, usize::from(default))?;
    u8::try_from(value).map_err(|_| OpError::Message(format!("'{key}' must be <= 255")))
}

fn read_binary(
    args: &Map<String, Json>,
    security: &SecurityConfig,
) -> Result<BinaryInput, OpError> {
    let data_base64 = str_arg(args, "data_base64")?;
    let input_path = str_arg(args, "input_path")?;
    match (data_base64, input_path) {
        (Some(_), Some(_)) => Err(OpError::Message(
            "provide exactly one of data_base64 or input_path".to_string(),
        )),
        (Some(data), None) => {
            let bytes = BASE64.decode(data)?;
            security.assert_payload_size(bytes.len())?;
            Ok(BinaryInput {
                source: "inline_base64".to_string(),
                data: bytes,
            })
        }
        (None, Some(path)) => Ok(BinaryInput {
            source: path.to_string(),
            data: security.read_file(path)?,
        }),
        (None, None) => Err(OpError::Message(
            "provide one of data_base64 or input_path".to_string(),
        )),
    }
}

fn read_text(args: &Map<String, Json>, security: &SecurityConfig) -> Result<TextInput, OpError> {
    let text = str_arg(args, "text")?;
    let input_path = str_arg(args, "input_path")?;
    match (text, input_path) {
        (Some(_), Some(_)) => Err(OpError::Message(
            "provide exactly one of text or input_path".to_string(),
        )),
        (Some(text), None) => {
            security.assert_text_size(text.len())?;
            Ok(TextInput {
                source: "inline_text".to_string(),
                text: text.to_string(),
            })
        }
        (None, Some(path)) => {
            let bytes = security.read_file(path)?;
            security.assert_text_size(bytes.len())?;
            Ok(TextInput {
                source: path.to_string(),
                text: String::from_utf8(bytes)?,
            })
        }
        (None, None) => Err(OpError::Message(
            "provide one of text or input_path".to_string(),
        )),
    }
}

fn json_values(args: &Map<String, Json>, security: &SecurityConfig) -> Result<Vec<Json>, OpError> {
    let mut sources = 0;
    sources += usize::from(args.contains_key("value"));
    sources += usize::from(args.contains_key("values"));
    sources += usize::from(args.contains_key("json_text"));
    sources += usize::from(args.contains_key("input_path"));
    if sources != 1 {
        return Err(OpError::Message(
            "provide exactly one of value, values, json_text, or input_path".to_string(),
        ));
    }

    if let Some(value) = args.get("value") {
        // Inline JSON arguments bypass the text/input_path size checks
        // below unless we measure them here too; otherwise a compromised
        // or malicious MCP client could pass an arbitrarily large or
        // deeply nested inline value straight through to
        // surp_core::Value construction.
        let size = serde_json::to_vec(value)?.len();
        security.assert_text_size(size)?;
        return Ok(vec![value.clone()]);
    }
    if let Some(values) = args.get("values") {
        let size = serde_json::to_vec(values)?.len();
        security.assert_text_size(size)?;
        return values
            .as_array()
            .cloned()
            .ok_or_else(|| OpError::Message("'values' must be an array".to_string()));
    }
    if let Some(text) = str_arg(args, "json_text")? {
        security.assert_text_size(text.len())?;
        return Ok(vec![serde_json::from_str(text)?]);
    }
    if let Some(path) = str_arg(args, "input_path")? {
        let bytes = security.read_file(path)?;
        security.assert_text_size(bytes.len())?;
        return Ok(vec![serde_json::from_slice(&bytes)?]);
    }
    unreachable!("source count already checked")
}

fn single_json_value(args: &Map<String, Json>, security: &SecurityConfig) -> Result<Json, OpError> {
    let mut values = json_values(args, security)?;
    if values.len() != 1 {
        return Err(OpError::Message(
            "this tool expects exactly one JSON value; use value instead of values".to_string(),
        ));
    }
    Ok(values.remove(0))
}

fn binary_output(
    args: &Map<String, Json>,
    security: &SecurityConfig,
    bytes: &[u8],
) -> Result<Json, OpError> {
    let output_path = str_arg(args, "output_path")?;
    let include_base64 = match args.get("include_base64") {
        Some(_) => bool_arg(args, "include_base64", false)?,
        None => output_path.is_none(),
    };
    let mut structured = json!({
        "byte_length": bytes.len(),
    });
    if let Some(path) = output_path {
        let written =
            security.write_file(path, bytes, bool_arg(args, "allow_overwrite", false)?)?;
        structured["output_path"] = json!(written.display().to_string());
    }
    if include_base64 {
        security.assert_inline_size(bytes.len())?;
        structured["data_base64"] = json!(BASE64.encode(bytes));
    }
    Ok(structured)
}

fn text_output(
    args: &Map<String, Json>,
    security: &SecurityConfig,
    text_key: &str,
    text: &str,
) -> Result<Json, OpError> {
    let output_path = str_arg(args, "output_path")?;
    let include_text = match args
        .get("include_text")
        .or_else(|| args.get("include_json_text"))
    {
        Some(Json::Bool(value)) => *value,
        Some(Json::Null) | None => output_path.is_none(),
        Some(_) => {
            return Err(OpError::Message(
                "include_text must be a boolean".to_string(),
            ));
        }
    };
    let mut structured = json!({
        "text_length": text.len(),
    });
    if let Some(path) = output_path {
        let written = security.write_file(
            path,
            text.as_bytes(),
            bool_arg(args, "allow_overwrite", false)?,
        )?;
        structured["output_path"] = json!(written.display().to_string());
    }
    if include_text {
        security.assert_inline_size(text.len())?;
        structured[text_key] = json!(text);
    }
    Ok(structured)
}

fn merge_object(target: &mut Json, source: Json) {
    let Some(target) = target.as_object_mut() else {
        return;
    };
    if let Some(source) = source.as_object() {
        for (key, value) in source {
            target.insert(key.clone(), value.clone());
        }
    }
}

fn compression_arg(args: &Map<String, Json>) -> Result<CompressionType, OpError> {
    let compression = str_arg(args, "compression")?.unwrap_or("none");
    let wire = match compression {
        "none" => CompressionType::None,
        "lz4" => {
            if !cfg!(feature = "lz4") {
                return Err(unsupported_compression("lz4"));
            }
            CompressionType::Lz4
        }
        "snappy" => {
            if !cfg!(feature = "snappy") {
                return Err(unsupported_compression("snappy"));
            }
            CompressionType::Snappy
        }
        "zstd" => {
            if !cfg!(feature = "zstd") {
                return Err(unsupported_compression("zstd"));
            }
            CompressionType::Zstd
        }
        other => {
            return Err(OpError::Message(format!(
                "unsupported compression '{other}'"
            )));
        }
    };
    Ok(wire)
}

fn unsupported_compression(name: &str) -> OpError {
    OpError::Message(format!(
        "compression '{name}' is not enabled in this surp-mcp build; rebuild with --features {name}"
    ))
}

fn compression_features() -> Json {
    json!({
        "none": true,
        "lz4": cfg!(feature = "lz4"),
        "snappy": cfg!(feature = "snappy"),
        "zstd": cfg!(feature = "zstd"),
    })
}

fn encode_values(
    values: &[Value],
    dedup: bool,
    compression: CompressionType,
) -> Result<Vec<u8>, OpError> {
    let mut encoder = Encoder::new();
    if dedup {
        encoder.enable_dedup();
    }
    encoder.set_compression(compression);
    for value in values {
        encoder.encode_value(value)?;
    }
    Ok(encoder.finish()?)
}

fn limits_arg(args: &Map<String, Json>) -> Result<Limits, OpError> {
    let mut limits = match str_arg(args, "limits_profile")?.unwrap_or("strict") {
        "strict" => Limits::strict(),
        "default" => Limits::default(),
        "unlimited" => Limits::unlimited(),
        other => {
            return Err(OpError::Message(format!(
                "unsupported limits_profile '{other}'"
            )));
        }
    };

    if let Some(custom) = args.get("limits").and_then(Json::as_object) {
        if let Some(value) = limit_field(custom, "max_nesting_depth")? {
            limits.max_nesting_depth = value;
        }
        if let Some(value) = limit_field(custom, "max_block_size")? {
            limits.max_block_size = value;
        }
        if let Some(value) = limit_field(custom, "max_items")? {
            limits.max_items = value;
        }
        if let Some(value) = limit_field(custom, "max_memory")? {
            limits.max_memory = value;
        }
        if let Some(value) = limit_field(custom, "max_string_length")? {
            limits.max_string_length = value;
        }
        if let Some(value) = limit_field(custom, "max_decompression_ratio")? {
            limits.max_decompression_ratio = value;
        }
    } else if args.contains_key("limits") {
        return Err(OpError::Message("'limits' must be an object".to_string()));
    }

    Ok(limits)
}

fn limit_field(fields: &Map<String, Json>, key: &str) -> Result<Option<usize>, OpError> {
    match fields.get(key) {
        Some(Json::Number(value)) => value
            .as_u64()
            .and_then(|value| usize::try_from(value).ok())
            .map(Some)
            .ok_or_else(|| OpError::Message(format!("limits.{key} must be a positive integer"))),
        Some(Json::Null) | None => Ok(None),
        Some(_) => Err(OpError::Message(format!("limits.{key} must be an integer"))),
    }
}

fn json_style(args: &Map<String, Json>) -> Result<&str, OpError> {
    let style = str_arg(args, "json_style")?.unwrap_or("pretty");
    match style {
        "pretty" | "compact" => Ok(style),
        other => Err(OpError::Message(format!(
            "unsupported json_style '{other}'"
        ))),
    }
}

fn render_json_values(values: &[Json], style: &str) -> Result<String, OpError> {
    let render_value = if values.len() == 1 {
        values[0].clone()
    } else {
        Json::Array(values.to_vec())
    };
    Ok(match style {
        "pretty" => serde_json::to_string_pretty(&render_value)?,
        "compact" => serde_json::to_string(&render_value)?,
        _ => unreachable!("validated style"),
    })
}

fn analyze_blocks_json(data: &[u8]) -> Result<Json, OpError> {
    let mut blocks = Vec::new();
    let mut trailer = Json::Null;
    let mut offset = 0usize;
    let mut index = 0usize;

    while offset < data.len() {
        let block_offset = offset;
        let (block, consumed) = BlockReader::parse(data, offset).map_err(|err| {
            OpError::Message(format!(
                "failed to parse block #{index} at offset {offset}: {err}"
            ))
        })?;
        offset += consumed;

        let payload_checksum_valid = if block.compression == CompressionType::None
            || block.block_type == BlockType::Trailer
        {
            Json::Bool(block.verify_checksum())
        } else {
            Json::Null
        };

        if block.block_type == BlockType::Trailer {
            let file_checksum_valid = if block.payload.len() == 8 {
                let mut expected_bytes = [0u8; 8];
                expected_bytes.copy_from_slice(block.payload);
                let expected = u64::from_le_bytes(expected_bytes);
                let actual = compute_xxh64(&data[..block_offset]);
                expected == actual
            } else {
                false
            };
            trailer = json!({
                "payload_checksum_valid": block.verify_checksum(),
                "file_checksum_valid": file_checksum_valid,
            });
        }

        blocks.push(json!({
            "index": index,
            "offset": block_offset,
            "block_type": block_type_name(block.block_type),
            "compression": compression_name(block.compression),
            "payload_len": block.payload.len(),
            "payload_checksum_valid": payload_checksum_valid,
        }));
        index += 1;
    }

    Ok(json!({
        "file_size": data.len(),
        "blocks": blocks,
        "trailer": trailer,
    }))
}

fn block_type_name(block_type: BlockType) -> &'static str {
    match block_type {
        BlockType::Data => "data",
        BlockType::Index => "index",
        BlockType::Schema => "schema",
        BlockType::StringDict => "string_dict",
        BlockType::Trailer => "trailer",
    }
}

fn compression_name(compression: CompressionType) -> &'static str {
    match compression {
        CompressionType::None => "none",
        CompressionType::Zstd => "zstd",
        CompressionType::Snappy => "snappy",
        CompressionType::Lz4 => "lz4",
    }
}

fn v1_value_to_json(value: &Value) -> Json {
    match value {
        Value::Null => json!({"kind": "null", "value": null}),
        Value::Bool(value) => json!({"kind": "bool", "value": value}),
        Value::UInt(value) => json!({"kind": "uint", "value": value}),
        Value::Int(value) => json!({"kind": "int", "value": value}),
        Value::Float(value) => json!({"kind": "float", "value": json_float(*value)}),
        Value::Str(value) => json!({"kind": "str", "value": value}),
        Value::Bytes(value) => {
            json!({"kind": "bytes", "data_base64": BASE64.encode(value), "byte_length": value.len()})
        }
        Value::Array(items) => json!({
            "kind": "array",
            "items": items.iter().map(v1_value_to_json).collect::<Vec<_>>()
        }),
        Value::Object(entries) => json!({
            "kind": "object",
            "entries": entries.iter().map(|(key, value)| {
                json!({"key": key, "value": v1_value_to_json(value)})
            }).collect::<Vec<_>>()
        }),
    }
}

fn rfc_decoded_to_json(decoded: &rfc001::DecodedDocument) -> Json {
    json!({
        "header": rfc_header_to_json(&decoded.header),
        "symbols": decoded.symbols,
        "symbol_count": decoded.symbols.len(),
        "document": rfc_document_to_json(&decoded.document),
        "ctn": rfc001::format_document(&decoded.document),
    })
}

fn rfc_header_to_json(header: &rfc001::CbfHeader) -> Json {
    json!({
        "magic": "SURP",
        "cbf_version": header.cbf_version,
        "ctn_version": header.ctn_version,
        "flags": header.flags,
        "alignment": header.alignment,
        "schema_hash_prefix_hex": hex_lower(&header.schema_hash_prefix),
        "root_offset": header.root_offset,
        "symtab_offset": header.symtab_offset,
        "index_offset": header.index_offset,
        "self_describing": header.self_describing(),
        "has_symtab": header.has_symtab(),
        "has_index": header.has_index(),
    })
}

fn rfc_document_to_json(document: &rfc001::Document) -> Json {
    json!({
        "annotations": document.annotations.iter().map(rfc_annotation_to_json).collect::<Vec<_>>(),
        "uses": document.uses,
        "bindings": document.bindings.iter().map(|binding| {
            json!({"name": binding.name, "value": rfc_value_to_json(&binding.value)})
        }).collect::<Vec<_>>(),
        "root": document.root.as_ref().map(rfc_value_to_json),
        "binding_names": document.binding_names(),
        "annotation_names": document.annotation_names(),
    })
}

fn rfc_annotation_to_json(annotation: &rfc001::Annotation) -> Json {
    json!({
        "name": annotation.name,
        "value": annotation.value.as_ref().map(rfc_scalar_to_json),
    })
}

fn rfc_value_to_json(value: &rfc001::Value) -> Json {
    match value {
        rfc001::Value::Scalar(scalar) => rfc_scalar_to_json(scalar),
        rfc001::Value::Product(product) => json!({
            "kind": "product",
            "type_name": product.type_name,
            "fields": product.fields.iter().map(|field| {
                json!({"name": field.name, "value": rfc_value_to_json(&field.value)})
            }).collect::<Vec<_>>(),
        }),
        rfc001::Value::Sum(sum) => json!({
            "kind": "sum",
            "type_name": sum.type_name,
            "variant": sum.variant,
            "payload": match &sum.payload {
                rfc001::SumPayload::Unit => json!({"kind": "unit"}),
                rfc001::SumPayload::Tuple(items) => json!({
                    "kind": "tuple",
                    "items": items.iter().map(rfc_value_to_json).collect::<Vec<_>>()
                }),
                rfc001::SumPayload::Struct(fields) => json!({
                    "kind": "struct",
                    "fields": fields.iter().map(|field| {
                        json!({"name": field.name, "value": rfc_value_to_json(&field.value)})
                    }).collect::<Vec<_>>()
                }),
            },
        }),
        rfc001::Value::Sequence(sequence) => json!({
            "kind": "sequence",
            "elem_type": sequence.elem_type,
            "items": sequence.items.iter().map(rfc_value_to_json).collect::<Vec<_>>(),
        }),
        rfc001::Value::Association(pairs) => json!({
            "kind": "association",
            "pairs": pairs.iter().map(|(key, value)| {
                json!([rfc_value_to_json(key), rfc_value_to_json(value)])
            }).collect::<Vec<_>>(),
        }),
        rfc001::Value::Reference(reference) => match reference {
            rfc001::Reference::Binding(name) => json!({
                "kind": "reference",
                "reference_kind": "binding",
                "name": name,
            }),
            rfc001::Reference::ById(value) => json!({
                "kind": "reference",
                "reference_kind": "by_id",
                "value": rfc_value_to_json(value),
            }),
        },
        rfc001::Value::Tensor(tensor) => json!({
            "kind": "tensor",
            "element_type": tensor.element_type,
            "shape": tensor.shape,
            "annotations": tensor.annotations.iter().map(rfc_annotation_to_json).collect::<Vec<_>>(),
            "data": match &tensor.data {
                rfc001::TensorData::DenseF64(values) => json!({
                    "kind": "dense_f64",
                    "values": values.iter().map(|value| json_float(*value)).collect::<Vec<_>>()
                }),
                rfc001::TensorData::DenseI64(values) => json!({"kind": "dense_i64", "values": values}),
                rfc001::TensorData::DenseU64(values) => json!({"kind": "dense_u64", "values": values}),
                rfc001::TensorData::BinaryBlob(bytes) => json!({
                    "kind": "binary_blob",
                    "bytes_base64": BASE64.encode(bytes),
                    "byte_length": bytes.len(),
                }),
            },
        }),
        rfc001::Value::Stream(stream) => json!({
            "kind": "stream",
            "item_type": stream.item_type,
            "annotations": stream.annotations.iter().map(rfc_annotation_to_json).collect::<Vec<_>>(),
        }),
        rfc001::Value::Opaque(opaque) => json!({
            "kind": "opaque",
            "type_tag": opaque.type_tag,
            "bytes_base64": BASE64.encode(&opaque.bytes),
            "byte_length": opaque.bytes.len(),
        }),
    }
}

fn rfc_scalar_to_json(scalar: &rfc001::Scalar) -> Json {
    match scalar {
        rfc001::Scalar::Null => json!({"kind": "scalar", "type": "null", "value": null}),
        rfc001::Scalar::Unit => json!({"kind": "scalar", "type": "unit", "value": null}),
        rfc001::Scalar::Bool(value) => json!({"kind": "scalar", "type": "bool", "value": value}),
        rfc001::Scalar::I64(value) => json!({"kind": "scalar", "type": "i64", "value": value}),
        rfc001::Scalar::U64(value) => json!({"kind": "scalar", "type": "u64", "value": value}),
        rfc001::Scalar::Vi64(value) => json!({"kind": "scalar", "type": "vi64", "value": value}),
        rfc001::Scalar::Vu64(value) => json!({"kind": "scalar", "type": "vu64", "value": value}),
        rfc001::Scalar::F32(value) => {
            json!({"kind": "scalar", "type": "f32", "value": json_float(f64::from(*value))})
        }
        rfc001::Scalar::F64(value) => {
            json!({"kind": "scalar", "type": "f64", "value": json_float(*value)})
        }
        rfc001::Scalar::Str(value) => json!({"kind": "scalar", "type": "str", "value": value}),
        rfc001::Scalar::Bytes(value) => json!({
            "kind": "scalar",
            "type": "bytes",
            "value_base64": BASE64.encode(value),
            "byte_length": value.len(),
        }),
        rfc001::Scalar::Sym(value) => json!({"kind": "scalar", "type": "sym", "value": value}),
        rfc001::Scalar::Tagged { tag, value } => {
            json!({"kind": "scalar", "type": "tagged", "tag": tag, "value": value})
        }
    }
}

fn json_float(value: f64) -> Json {
    Number::from_f64(value)
        .map(Json::Number)
        .unwrap_or_else(|| Json::String(value.to_string()))
}

fn hex_lower(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum QueryStep {
    Field(String),
    Flatten,
    Index(isize),
}

fn v1_query_input(
    args: &Map<String, Json>,
    security: &SecurityConfig,
) -> Result<Vec<Value>, OpError> {
    let input_format = str_arg(args, "input_format")?;
    if args.contains_key("data_base64") {
        let input = read_binary(args, security)?;
        let mut decoder = Decoder::with_limits(&input.data, limits_arg(args)?);
        return Ok(decoder.decode_all_owned()?);
    }
    if args.contains_key("text") {
        let input = read_text(args, security)?;
        return Ok(vec![surp_core::text::parse(&input.text)?]);
    }
    if args.contains_key("value") || args.contains_key("json_text") {
        return Ok(json_values(args, security)?
            .iter()
            .map(Value::from)
            .collect());
    }
    if let Some(path) = str_arg(args, "input_path")? {
        let bytes = security.read_file(path)?;
        return match input_format {
            Some("surp_binary") => {
                let mut decoder = Decoder::with_limits(&bytes, limits_arg(args)?);
                Ok(decoder.decode_all_owned()?)
            }
            Some("surp_text") => {
                let text = String::from_utf8(bytes)?;
                Ok(vec![surp_core::text::parse(&text)?])
            }
            Some("json") => {
                let json: Json = serde_json::from_slice(&bytes)?;
                Ok(vec![Value::from(&json)])
            }
            Some(other) => Err(OpError::Message(format!(
                "unsupported input_format '{other}'"
            ))),
            None => Err(OpError::Message(
                "input_format is required when using input_path for surp_v1_query".to_string(),
            )),
        };
    }
    Err(OpError::Message(
        "provide query input as data_base64, text, value, json_text, or input_path".to_string(),
    ))
}

fn query_v1_values(values: &[Value], expr: &str) -> Result<Vec<Value>, OpError> {
    let steps = parse_query_steps(expr)?;
    let mut nodes = values.to_vec();
    for step in steps {
        let mut next = Vec::new();
        for node in &nodes {
            apply_v1_step(node, &step, &mut next);
        }
        nodes = next;
    }
    Ok(nodes)
}

fn parse_query_steps(expr: &str) -> Result<Vec<QueryStep>, OpError> {
    let mut s = expr.trim();
    if s.is_empty() {
        return Err(OpError::Message("query expression is empty".to_string()));
    }
    if !s.starts_with('.') && !s.starts_with('[') {
        return Err(OpError::Message(
            "query expression must start with '.' or '['".to_string(),
        ));
    }

    let mut steps = Vec::new();
    while !s.is_empty() {
        if let Some(rest) = s.strip_prefix('.') {
            s = rest;
            let mut end = 0usize;
            for (idx, ch) in s.char_indices() {
                if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                    end = idx + ch.len_utf8();
                } else {
                    break;
                }
            }
            if end == 0 {
                return Err(OpError::Message(format!("expected field name near '.{s}'")));
            }
            steps.push(QueryStep::Field(s[..end].to_string()));
            s = &s[end..];
            continue;
        }

        if s.starts_with('[') {
            let end = s
                .find(']')
                .ok_or_else(|| OpError::Message("unterminated bracket selector".to_string()))?;
            let inner = s[1..end].trim();
            if inner.is_empty() {
                steps.push(QueryStep::Flatten);
            } else {
                let index = inner.parse::<isize>().map_err(|_| {
                    OpError::Message(format!("unsupported v1 bracket selector [{inner}]"))
                })?;
                steps.push(QueryStep::Index(index));
            }
            s = &s[end + 1..];
            continue;
        }

        return Err(OpError::Message(format!(
            "unexpected query token near '{s}'"
        )));
    }

    Ok(steps)
}

fn apply_v1_step(node: &Value, step: &QueryStep, out: &mut Vec<Value>) {
    match step {
        QueryStep::Field(name) => {
            if let Value::Object(entries) = node {
                for (key, value) in entries {
                    if key == name {
                        out.push(value.clone());
                    }
                }
            }
        }
        QueryStep::Flatten => match node {
            Value::Array(items) => out.extend(items.iter().cloned()),
            Value::Object(entries) => out.extend(entries.iter().map(|(_, value)| value.clone())),
            _ => {}
        },
        QueryStep::Index(index) => {
            if let Value::Array(items) = node {
                if items.is_empty() {
                    return;
                }
                let len = items.len() as isize;
                let idx = if *index < 0 { len + *index } else { *index };
                if idx >= 0 {
                    if let Some(value) = usize::try_from(idx).ok().and_then(|idx| items.get(idx)) {
                        out.push(value.clone());
                    }
                }
            }
        }
    }
}

fn sort_json(value: &mut Json) {
    match value {
        Json::Array(items) => {
            for item in items {
                sort_json(item);
            }
        }
        Json::Object(map) => {
            for value in map.values_mut() {
                sort_json(value);
            }
            let sorted = map
                .iter()
                .map(|(key, value)| (key.clone(), value.clone()))
                .collect::<BTreeMap<_, _>>();
            map.clear();
            for (key, value) in sorted {
                map.insert(key, value);
            }
        }
        _ => {}
    }
}

fn sort_surp_value(value: &mut Value) {
    match value {
        Value::Array(items) => {
            for item in items {
                sort_surp_value(item);
            }
        }
        Value::Object(entries) => {
            for (_, value) in entries.iter_mut() {
                sort_surp_value(value);
            }
            entries.sort_by(|a, b| a.0.cmp(&b.0));
        }
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn query_v1_nested_values() {
        let value = Value::Object(vec![(
            "users".into(),
            Value::Array(vec![
                Value::Object(vec![("name".into(), Value::Str("Alice".into()))]),
                Value::Object(vec![("name".into(), Value::Str("Bob".into()))]),
            ]),
        )]);

        let matches = query_v1_values(&[value], ".users[].name").unwrap();
        assert_eq!(
            matches,
            vec![Value::Str("Alice".into()), Value::Str("Bob".into())]
        );
    }

    #[test]
    fn v1_typed_bytes_are_base64() {
        let value = Value::Bytes(vec![0, 1, 2]);
        let typed = v1_value_to_json(&value);
        assert_eq!(typed["kind"], "bytes");
        assert_eq!(typed["data_base64"], "AAEC");
    }

    #[test]
    fn rfc_scalar_bytes_are_json_safe() {
        let value = rfc001::Scalar::Bytes(vec![0xde, 0xad]);
        let encoded = rfc_scalar_to_json(&value);
        assert_eq!(encoded["value_base64"], "3q0=");
    }

    #[test]
    fn inline_value_exceeding_max_text_bytes_is_rejected() {
        let mut security = SecurityConfig::from_env().unwrap();
        security.max_text_bytes = 16;

        let args = json!({"value": "x".repeat(1024)});
        let args = args.as_object().unwrap();
        let err = json_values(args, &security).expect_err("oversized inline value must be rejected");
        assert!(matches!(err, OpError::Security(SecurityError::PayloadTooLarge { .. })));
    }

    #[test]
    fn inline_values_array_exceeding_max_text_bytes_is_rejected() {
        let mut security = SecurityConfig::from_env().unwrap();
        security.max_text_bytes = 16;

        let args = json!({"values": vec!["x".repeat(1024); 5]});
        let args = args.as_object().unwrap();
        let err =
            json_values(args, &security).expect_err("oversized inline values must be rejected");
        assert!(matches!(err, OpError::Security(SecurityError::PayloadTooLarge { .. })));
    }

    #[test]
    fn call_tool_rejects_oversized_inline_value_end_to_end() {
        let mut security = SecurityConfig::from_env().unwrap();
        security.max_text_bytes = 16;

        let args = json!({"value": "x".repeat(1024)});
        let result = call_tool("surp_v1_encode_json", &args, &security);
        assert!(result.is_err(), "expected oversized inline value to be rejected");
    }
}
