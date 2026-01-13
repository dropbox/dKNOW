//! `DoclingViz` MCP Server
//!
//! MCP (Model Context Protocol) server for AI-assisted PDF extraction correction.
//! Enables AI agents like Claude and GPT to view, analyze, and correct PDF extractions.

// Clippy pedantic allows:
// - f64 to f32 casts for bounding box coordinates (precision acceptable)
// - Similar variable names in tool handling (json_args, json_response)
// - Tool list and call functions are necessarily large
// - Tool call state parameter consumed for extraction
#![allow(clippy::cast_possible_truncation)]
#![allow(clippy::similar_names)]
#![allow(clippy::too_many_lines)]
#![allow(clippy::needless_pass_by_value)]

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};

mod corrections;
mod state;
mod tools;

use state::ServerState;

/// MCP JSON-RPC request
#[derive(Debug, Deserialize)]
struct McpRequest {
    #[allow(
        dead_code,
        reason = "required by MCP protocol for deserialization, always '2.0'"
    )]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

/// MCP JSON-RPC response
#[derive(Debug, Serialize)]
struct McpResponse {
    jsonrpc: String,
    id: Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<McpError>,
}

/// MCP error object
#[derive(Debug, Serialize)]
struct McpError {
    code: i32,
    message: String,
}

fn main() {
    // Initialize logging to stderr (stdout is for JSON-RPC)
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info"))
        .target(env_logger::Target::Stderr)
        .init();

    log::info!("DoclingViz MCP Server starting...");

    let mut state = ServerState::new();
    let stdin = io::stdin();
    let stdout = io::stdout();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                log::error!("Failed to read stdin: {e}");
                break;
            }
        };

        // Skip empty lines
        if line.is_empty() {
            continue;
        }

        // Parse JSON-RPC request
        let request: McpRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                log::error!("Invalid JSON: {e} - line: {line}");
                continue;
            }
        };

        log::debug!("Received request: method={}", request.method);

        // Handle request
        let response = handle_request(&mut state, request);

        // Write response
        let mut stdout = stdout.lock();
        if let Err(e) = serde_json::to_writer(&mut stdout, &response) {
            log::error!("Failed to write response: {e}");
            break;
        }
        if let Err(e) = writeln!(stdout) {
            log::error!("Failed to write newline: {e}");
            break;
        }
        if let Err(e) = stdout.flush() {
            log::error!("Failed to flush stdout: {e}");
            break;
        }
    }

    log::info!("DoclingViz MCP Server shutting down");
}

/// Handle an MCP request and return a response
fn handle_request(state: &mut ServerState, request: McpRequest) -> McpResponse {
    let id = request.id.unwrap_or(Value::Null);

    match request.method.as_str() {
        // Protocol handshake
        "initialize" => McpResponse {
            jsonrpc: "2.0".into(),
            id,
            result: Some(json!({
                "protocolVersion": "2024-11-05",
                "serverInfo": {
                    "name": "docling-mcp",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "capabilities": {
                    "tools": {}
                }
            })),
            error: None,
        },

        // Client notification that initialization is complete
        "notifications/initialized" => McpResponse {
            jsonrpc: "2.0".into(),
            id,
            result: Some(json!({})),
            error: None,
        },

        // List available tools
        "tools/list" => McpResponse {
            jsonrpc: "2.0".into(),
            id,
            result: Some(json!({
                "tools": tools::list_tools()
            })),
            error: None,
        },

        // Call a tool
        "tools/call" => {
            let params = request.params.unwrap_or(Value::Null);
            let tool_name = params.get("name").and_then(|v| v.as_str()).unwrap_or("");
            let arguments = params.get("arguments").cloned().unwrap_or(Value::Null);

            log::info!("Calling tool: {tool_name} with args: {arguments}");

            match tools::call_tool(state, tool_name, arguments) {
                Ok(result) => McpResponse {
                    jsonrpc: "2.0".into(),
                    id,
                    result: Some(json!({
                        "content": [{
                            "type": "text",
                            "text": serde_json::to_string_pretty(&result).unwrap_or_default()
                        }]
                    })),
                    error: None,
                },
                Err(e) => {
                    log::error!("Tool {tool_name} failed: {e}");
                    McpResponse {
                        jsonrpc: "2.0".into(),
                        id,
                        result: None,
                        error: Some(McpError {
                            code: -32000,
                            message: e,
                        }),
                    }
                }
            }
        }

        // Unknown method
        _ => {
            log::warn!("Unknown method: {}", request.method);
            McpResponse {
                jsonrpc: "2.0".into(),
                id,
                result: None,
                error: Some(McpError {
                    code: -32601,
                    message: format!("Method not found: {}", request.method),
                }),
            }
        }
    }
}
