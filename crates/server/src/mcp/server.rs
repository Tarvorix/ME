use std::sync::Arc;
use tokio::sync::Mutex;
use axum::{
    Router,
    routing::{get, post},
    extract::State,
    http::StatusCode,
    Json,
    response::sse::{Event, Sse},
};
use tower_http::cors::{CorsLayer, Any};
use serde_json::{json, Value};
use tracing::info;
use futures_util::stream::{self, Stream};
use std::convert::Infallible;

use super::types::*;
use super::tools;
use super::resources;

use machine_empire_core::protocol::MatchConfig;

/// Shared state for the MCP server.
/// Holds a reference to the game world and match config for a connected player.
pub struct McpState {
    /// The game world — accessed through the match runner.
    /// In a real deployment this would be a channel to the match runner,
    /// but for now we hold an optional snapshot or direct reference.
    pub world: Option<Arc<Mutex<WorldSnapshot>>>,
    /// Match config.
    pub config: MatchConfig,
    /// Player ID for the connected agent.
    pub player_id: u8,
    /// Current tick.
    pub tick: u32,
    /// Command sender to the match runner.
    pub cmd_tx: Option<tokio::sync::mpsc::UnboundedSender<machine_empire_core::command::Command>>,
}

/// A snapshot of the world state for MCP queries.
/// In production this would be refreshed every tick.
pub struct WorldSnapshot {
    /// A direct reference to the game's World.
    /// This is updated by the match runner.
    pub game: machine_empire_core::game::Game,
}

impl McpState {
    pub fn new(config: MatchConfig, player_id: u8) -> Self {
        McpState {
            world: None,
            config,
            player_id,
            tick: 0,
            cmd_tx: None,
        }
    }
}

/// Type alias for shared MCP state.
pub type SharedMcpState = Arc<Mutex<McpState>>;

/// Create the Axum router for the MCP server.
pub fn create_router(state: SharedMcpState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    Router::new()
        .route("/mcp", post(handle_rpc))
        .route("/mcp/sse", get(handle_sse))
        .route("/health", get(health_check))
        .layer(cors)
        .with_state(state)
}

/// Health check endpoint.
async fn health_check() -> &'static str {
    "Machine Empire MCP Server OK"
}

/// Handle JSON-RPC 2.0 requests on POST /mcp.
async fn handle_rpc(
    State(state): State<SharedMcpState>,
    Json(request): Json<JsonRpcRequest>,
) -> (StatusCode, Json<JsonRpcResponse>) {
    let response = process_request(request, state).await;
    (StatusCode::OK, Json(response))
}

/// Process a single JSON-RPC request.
async fn process_request(request: JsonRpcRequest, state: SharedMcpState) -> JsonRpcResponse {
    if request.jsonrpc != "2.0" {
        return JsonRpcResponse::error(
            request.id,
            INVALID_REQUEST,
            "Invalid JSON-RPC version".into(),
        );
    }

    match request.method.as_str() {
        "initialize" => handle_initialize(request.id),
        "tools/list" => handle_tools_list(request.id),
        "tools/call" => handle_tools_call(request.id, request.params, state).await,
        "resources/list" => handle_resources_list(request.id),
        "resources/read" => handle_resources_read(request.id, request.params, state).await,
        _ => JsonRpcResponse::error(
            request.id,
            METHOD_NOT_FOUND,
            format!("Method not found: {}", request.method),
        ),
    }
}

/// Handle MCP initialize request.
fn handle_initialize(id: Option<Value>) -> JsonRpcResponse {
    JsonRpcResponse::success(id, json!({
        "protocolVersion": "2024-11-05",
        "capabilities": {
            "tools": { "listChanged": false },
            "resources": { "listChanged": false, "subscribe": false }
        },
        "serverInfo": {
            "name": "machine-empire",
            "version": "0.1.0"
        }
    }))
}

/// Handle tools/list request.
fn handle_tools_list(id: Option<Value>) -> JsonRpcResponse {
    let tool_list = tools::list_tools();
    JsonRpcResponse::success(id, json!({ "tools": tool_list }))
}

/// Handle tools/call request.
async fn handle_tools_call(
    id: Option<Value>,
    params: Option<Value>,
    state: SharedMcpState,
) -> JsonRpcResponse {
    let params = match params {
        Some(p) => p,
        None => return JsonRpcResponse::error(id, INVALID_PARAMS, "Missing params".into()),
    };

    let tool_name = match params.get("name").and_then(|v| v.as_str()) {
        Some(name) => name.to_string(),
        None => return JsonRpcResponse::error(id, INVALID_PARAMS, "Missing tool name".into()),
    };

    let arguments = params.get("arguments").cloned().unwrap_or(json!({}));

    let mcp_state = state.lock().await;
    let player_id = mcp_state.player_id;

    match tools::execute_tool(&tool_name, &arguments, player_id) {
        Ok((cmd, description)) => {
            // If there's a command, send it to the match runner
            if let Some(command) = cmd {
                if let Some(tx) = &mcp_state.cmd_tx {
                    if tx.send(command).is_err() {
                        return JsonRpcResponse::error(
                            id,
                            INTERNAL_ERROR,
                            "Failed to send command to match".into(),
                        );
                    }
                } else {
                    return JsonRpcResponse::error(
                        id,
                        INTERNAL_ERROR,
                        "No active match connection".into(),
                    );
                }
            }

            JsonRpcResponse::success(id, json!({
                "content": [{
                    "type": "text",
                    "text": description
                }]
            }))
        }
        Err((code, message)) => {
            JsonRpcResponse::error(id, code, message)
        }
    }
}

/// Handle resources/list request.
fn handle_resources_list(id: Option<Value>) -> JsonRpcResponse {
    let resource_list = resources::list_resources();
    JsonRpcResponse::success(id, json!({ "resources": resource_list }))
}

/// Handle resources/read request.
async fn handle_resources_read(
    id: Option<Value>,
    params: Option<Value>,
    state: SharedMcpState,
) -> JsonRpcResponse {
    let params = match params {
        Some(p) => p,
        None => return JsonRpcResponse::error(id, INVALID_PARAMS, "Missing params".into()),
    };

    let uri = match params.get("uri").and_then(|v| v.as_str()) {
        Some(u) => u.to_string(),
        None => return JsonRpcResponse::error(id, INVALID_PARAMS, "Missing resource URI".into()),
    };

    let mcp_state = state.lock().await;
    let player_id = mcp_state.player_id;
    let config = mcp_state.config.clone();
    let tick = mcp_state.tick;

    // Read from the world snapshot if available
    if let Some(world_ref) = &mcp_state.world {
        let world_lock = world_ref.lock().await;
        match resources::read_resource(&uri, &world_lock.game.world, player_id, &config, tick) {
            Ok(data) => {
                JsonRpcResponse::success(id, json!({
                    "contents": [{
                        "uri": uri,
                        "mimeType": "application/json",
                        "text": serde_json::to_string_pretty(&data).unwrap_or_default()
                    }]
                }))
            }
            Err(msg) => JsonRpcResponse::error(id, INTERNAL_ERROR, msg),
        }
    } else {
        JsonRpcResponse::error(id, INTERNAL_ERROR, "No active game world".into())
    }
}

/// SSE endpoint for MCP notifications.
async fn handle_sse(
    State(_state): State<SharedMcpState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    // Send a simple connected event
    let stream = stream::once(async {
        Ok(Event::default()
            .event("connected")
            .data(json!({"status": "connected", "server": "machine-empire"}).to_string()))
    });

    Sse::new(stream)
}

/// Start the MCP server on the given port.
pub async fn run(state: SharedMcpState, port: u16) {
    let app = create_router(state);
    let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{}", port))
        .await
        .expect("Failed to bind MCP server");

    info!("MCP server listening on http://0.0.0.0:{}", port);

    axum::serve(listener, app)
        .await
        .expect("MCP server failed");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_initialize_response() {
        let resp = handle_initialize(Some(Value::Number(1.into())));
        assert!(resp.result.is_some());
        let result = resp.result.unwrap();
        assert_eq!(result["protocolVersion"], "2024-11-05");
        assert!(result["capabilities"]["tools"].is_object());
        assert!(result["capabilities"]["resources"].is_object());
        assert_eq!(result["serverInfo"]["name"], "machine-empire");
    }

    #[test]
    fn test_tools_list_response() {
        let resp = handle_tools_list(Some(Value::Number(1.into())));
        assert!(resp.result.is_some());
        let result = resp.result.unwrap();
        let tools = result["tools"].as_array().unwrap();
        assert_eq!(tools.len(), 7);
    }

    #[test]
    fn test_resources_list_response() {
        let resp = handle_resources_list(Some(Value::Number(1.into())));
        assert!(resp.result.is_some());
        let result = resp.result.unwrap();
        let resources = result["resources"].as_array().unwrap();
        assert_eq!(resources.len(), 9);
    }

    #[tokio::test]
    async fn test_unknown_method() {
        let state = Arc::new(Mutex::new(McpState::new(MatchConfig::default(), 0)));
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(Value::Number(1.into())),
            method: "nonexistent/method".into(),
            params: None,
        };

        let resp = process_request(request, state).await;
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, METHOD_NOT_FOUND);
    }

    #[tokio::test]
    async fn test_invalid_jsonrpc_version() {
        let state = Arc::new(Mutex::new(McpState::new(MatchConfig::default(), 0)));
        let request = JsonRpcRequest {
            jsonrpc: "1.0".into(),
            id: Some(Value::Number(1.into())),
            method: "tools/list".into(),
            params: None,
        };

        let resp = process_request(request, state).await;
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, INVALID_REQUEST);
    }

    #[tokio::test]
    async fn test_tools_call_no_match() {
        let state = Arc::new(Mutex::new(McpState::new(MatchConfig::default(), 0)));
        let request = JsonRpcRequest {
            jsonrpc: "2.0".into(),
            id: Some(Value::Number(1.into())),
            method: "tools/call".into(),
            params: Some(json!({
                "name": "game/produce_unit",
                "arguments": { "unit_type": 0 }
            })),
        };

        let resp = process_request(request, state).await;
        // Should fail because there's no active match connection
        assert!(resp.error.is_some());
    }
}
