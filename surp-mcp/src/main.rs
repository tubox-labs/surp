fn main() {
    if let Err(err) = surp_mcp::run_stdio() {
        eprintln!(
            "{}",
            serde_json::json!({
                "level": "error",
                "target": "surp_mcp",
                "event": "server_exit",
                "error": err.to_string(),
            })
        );
        std::process::exit(1);
    }
}
