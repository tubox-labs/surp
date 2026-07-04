use std::io::{self, BufRead, Write};

use serde::{Deserialize, Serialize};
use serde_json::{Value as Json, json};
use thiserror::Error;

use crate::ops;
use crate::registry;
use crate::security::SecurityConfig;

#[derive(Debug, Error)]
pub enum ProtocolError {
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("security configuration error: {0}")]
    Security(#[from] crate::security::SecurityError),
}

/// Maximum bracket/brace nesting depth accepted in a raw RPC message before
/// it is handed to `serde_json`. `serde_json`'s recursive-descent parser has
/// no built-in depth limit and will blow the call stack on sufficiently deep
/// input (e.g. a long run of `[[[[...`); pre-scanning for depth lets us
/// reject such payloads with a clean error instead of crashing the process.
const MAX_JSON_NESTING_DEPTH: usize = 128;

#[derive(Debug, Deserialize)]
struct RpcMessage {
    #[allow(dead_code)]
    jsonrpc: Option<String>,
    id: Option<Json>,
    method: Option<String>,
    params: Option<Json>,
}

#[derive(Debug, Serialize)]
struct RpcResponse {
    jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    id: Option<Json>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Json>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<RpcError>,
}

#[derive(Debug, Serialize)]
struct RpcError {
    code: i64,
    message: String,
}

pub fn run_stdio() -> Result<(), ProtocolError> {
    let security = SecurityConfig::from_env()?;
    log_event("info", "server_start", json!({}));

    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Some(response) = handle_message(&line, &security) {
            serde_json::to_writer(&mut stdout, &response)?;
            stdout.write_all(b"\n")?;
            stdout.flush()?;
        }
    }
    log_event("info", "server_stop", json!({}));
    Ok(())
}

pub fn handle_message(raw: &str, security: &SecurityConfig) -> Option<Json> {
    // Reject oversized or pathologically nested messages before they are
    // ever handed to serde_json. Both checks run ahead of any
    // SecurityConfig payload-size check applied to parsed tool arguments,
    // since those only cover already-parsed values and can't protect
    // against a single huge/deep raw line.
    if raw.len() > security.max_line_bytes {
        return Some(response_error(
            None,
            -32700,
            format!(
                "message is {} bytes, exceeding the configured maximum of {} bytes",
                raw.len(),
                security.max_line_bytes
            ),
        ));
    }
    if json_nesting_depth_exceeds(raw, MAX_JSON_NESTING_DEPTH) {
        return Some(response_error(
            None,
            -32700,
            format!(
                "message nesting depth exceeds the configured maximum of {MAX_JSON_NESTING_DEPTH}"
            ),
        ));
    }

    let parsed: Result<RpcMessage, _> = serde_json::from_str(raw);
    let message = match parsed {
        Ok(message) => message,
        Err(err) => return Some(response_error(None, -32700, format!("parse error: {err}"))),
    };

    let id = message.id.clone();
    let Some(method) = message.method.as_deref() else {
        return Some(response_error(id, -32600, "missing method".to_string()));
    };

    if id.is_none() && method.starts_with("notifications/") {
        return None;
    }

    let result = match method {
        "initialize" => Ok(initialize_result(message.params.as_ref())),
        "ping" => Ok(json!({})),
        "tools/list" => Ok(json!({"tools": registry::tools()})),
        "tools/call" => handle_tool_call(message.params.as_ref(), security),
        "resources/list" => Ok(json!({"resources": registry::resources()})),
        "resources/read" => handle_resource_read(message.params.as_ref()),
        "prompts/list" => Ok(json!({"prompts": registry::prompts()})),
        "prompts/get" => handle_prompt_get(message.params.as_ref()),
        _ => Err(RpcError {
            code: -32601,
            message: format!("method not found: {method}"),
        }),
    };

    Some(match result {
        Ok(result) => response_result(id, result),
        Err(err) => response_error(id, err.code, err.message),
    })
}

/// Cheaply pre-scans raw JSON text for excessive `{`/`[` nesting depth
/// without doing a full parse. String contents (including escaped quotes)
/// are skipped so brackets inside string literals don't affect the count.
/// This is a best-effort guard: it only needs to catch pathological input
/// before it reaches `serde_json`, not to fully validate JSON syntax.
fn json_nesting_depth_exceeds(text: &str, max_depth: usize) -> bool {
    let mut depth: usize = 0;
    let mut in_string = false;
    let mut escaped = false;
    for ch in text.chars() {
        if in_string {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }
        match ch {
            '"' => in_string = true,
            '{' | '[' => {
                depth += 1;
                if depth > max_depth {
                    return true;
                }
            }
            '}' | ']' => depth = depth.saturating_sub(1),
            _ => {}
        }
    }
    false
}

fn initialize_result(params: Option<&Json>) -> Json {
    let requested_protocol = params
        .and_then(|params| params.get("protocolVersion"))
        .and_then(Json::as_str)
        .unwrap_or(registry::PROTOCOL_VERSION);

    json!({
        "protocolVersion": requested_protocol,
        "capabilities": {
            "tools": {"listChanged": false},
            "resources": {"subscribe": false, "listChanged": false},
            "prompts": {"listChanged": false},
            "logging": {}
        },
        "serverInfo": {
            "name": registry::SERVER_NAME,
            "version": registry::SERVER_VERSION
        }
    })
}

fn handle_tool_call(params: Option<&Json>, security: &SecurityConfig) -> Result<Json, RpcError> {
    let params = params.and_then(Json::as_object).ok_or_else(|| RpcError {
        code: -32602,
        message: "tools/call params must be an object".to_string(),
    })?;
    let name = params
        .get("name")
        .and_then(Json::as_str)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "tools/call params.name must be a string".to_string(),
        })?;
    let arguments = params
        .get("arguments")
        .cloned()
        .unwrap_or_else(|| json!({}));

    log_event("info", "tool_call", json!({"tool": name}));
    match ops::call_tool(name, &arguments, security) {
        Ok(outcome) => Ok(json!({
            "content": [{"type": "text", "text": outcome.text}],
            "structuredContent": outcome.structured,
            "isError": false
        })),
        Err(err) => {
            log_event(
                "warn",
                "tool_error",
                json!({"tool": name, "error": err.to_string()}),
            );
            Ok(json!({
                "content": [{"type": "text", "text": format!("Surp tool error: {err}")}],
                "structuredContent": {"error": err.to_string()},
                "isError": true
            }))
        }
    }
}

fn handle_resource_read(params: Option<&Json>) -> Result<Json, RpcError> {
    let uri = params
        .and_then(|params| params.get("uri"))
        .and_then(Json::as_str)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "resources/read params.uri must be a string".to_string(),
        })?;
    let text = registry::resource_text(uri).ok_or_else(|| RpcError {
        code: -32602,
        message: format!("unknown resource URI '{uri}'"),
    })?;
    Ok(json!({
        "contents": [{
            "uri": uri,
            "mimeType": "text/markdown",
            "text": text
        }]
    }))
}

fn handle_prompt_get(params: Option<&Json>) -> Result<Json, RpcError> {
    let params = params.and_then(Json::as_object).ok_or_else(|| RpcError {
        code: -32602,
        message: "prompts/get params must be an object".to_string(),
    })?;
    let name = params
        .get("name")
        .and_then(Json::as_str)
        .ok_or_else(|| RpcError {
            code: -32602,
            message: "prompts/get params.name must be a string".to_string(),
        })?;
    registry::prompt(name, params.get("arguments")).ok_or_else(|| RpcError {
        code: -32602,
        message: format!("unknown prompt '{name}'"),
    })
}

fn response_result(id: Option<Json>, result: Json) -> Json {
    serde_json::to_value(RpcResponse {
        jsonrpc: "2.0",
        id,
        result: Some(result),
        error: None,
    })
    .expect("response serializes")
}

fn response_error(id: Option<Json>, code: i64, message: String) -> Json {
    serde_json::to_value(RpcResponse {
        jsonrpc: "2.0",
        id,
        result: None,
        error: Some(RpcError { code, message }),
    })
    .expect("response serializes")
}

fn log_event(level: &str, event: &str, fields: Json) {
    let mut payload = json!({
        "level": level,
        "target": "surp_mcp",
        "event": event,
    });
    if let (Some(base), Some(extra)) = (payload.as_object_mut(), fields.as_object()) {
        for (key, value) in extra {
            base.insert(key.clone(), value.clone());
        }
    }
    eprintln!("{payload}");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_returns_capabilities() {
        let security = SecurityConfig::from_env().unwrap();
        let response = handle_message(
            r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"test"}}"#,
            &security,
        )
        .unwrap();
        assert_eq!(response["result"]["protocolVersion"], "test");
        assert!(response["result"]["capabilities"]["tools"].is_object());
    }

    #[test]
    fn tools_call_reports_tool_errors_in_result() {
        let security = SecurityConfig::from_env().unwrap();
        let response = handle_message(
            r#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"missing","arguments":{}}}"#,
            &security,
        )
        .unwrap();
        assert_eq!(response["result"]["isError"], true);
    }

    #[test]
    fn oversized_line_is_rejected_before_parsing() {
        let mut security = SecurityConfig::from_env().unwrap();
        security.max_line_bytes = 64;

        // Build a line far exceeding the cap; if this were fed to
        // serde_json::from_str unchecked it would still just be a large
        // allocation/parse, but the point is it must never get that far.
        let huge_string = "a".repeat(10 * 1024 * 1024);
        let raw = format!(r#"{{"jsonrpc":"2.0","id":1,"method":"ping","params":"{huge_string}"}}"#);
        assert!(raw.len() > security.max_line_bytes);

        let response = handle_message(&raw, &security).expect("clean error response");
        assert_eq!(response["error"]["code"], -32700);
        assert!(
            response["error"]["message"]
                .as_str()
                .unwrap()
                .contains("exceeding the configured maximum"),
        );
        assert!(response["result"].is_null());
    }

    #[test]
    fn deeply_nested_message_is_rejected_before_parsing() {
        let security = SecurityConfig::from_env().unwrap();
        let depth = MAX_JSON_NESTING_DEPTH + 100;
        let mut raw = String::new();
        raw.push_str(r#"{"jsonrpc":"2.0","id":1,"method":"ping","params":"#);
        raw.push_str(&"[".repeat(depth));
        raw.push_str(&"]".repeat(depth));
        raw.push('}');

        let response = handle_message(&raw, &security).expect("clean error response");
        assert_eq!(response["error"]["code"], -32700);
        assert!(
            response["error"]["message"]
                .as_str()
                .unwrap()
                .contains("nesting depth"),
        );
    }
}
