use serde::{Serialize, Deserialize};
use serde_json::Value;

/// JSON-RPC 2.0 request.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: String,
    pub id: Option<Value>,
    pub method: String,
    #[serde(default)]
    pub params: Option<Value>,
}

/// JSON-RPC 2.0 response.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    pub jsonrpc: String,
    pub id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Create a success response.
    pub fn success(id: Option<Value>, result: Value) -> Self {
        JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response.
    pub fn error(id: Option<Value>, code: i32, message: String) -> Self {
        JsonRpcResponse {
            jsonrpc: "2.0".into(),
            id,
            result: None,
            error: Some(JsonRpcError {
                code,
                message,
                data: None,
            }),
        }
    }
}

/// JSON-RPC 2.0 error object.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// Standard JSON-RPC error codes.
#[allow(dead_code)]
pub const PARSE_ERROR: i32 = -32700;
pub const INVALID_REQUEST: i32 = -32600;
pub const METHOD_NOT_FOUND: i32 = -32601;
pub const INVALID_PARAMS: i32 = -32602;
pub const INTERNAL_ERROR: i32 = -32603;

/// MCP tool definition.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolDefinition {
    pub name: String,
    pub description: String,
    #[serde(rename = "inputSchema")]
    pub input_schema: Value,
}

/// MCP resource definition.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourceDefinition {
    pub uri: String,
    pub name: String,
    pub description: String,
    #[serde(rename = "mimeType")]
    pub mime_type: String,
}

/// MCP server capabilities.
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerCapabilities {
    pub tools: ToolsCapability,
    pub resources: ResourcesCapability,
}

/// Tools capability declaration.
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolsCapability {
    #[serde(rename = "listChanged")]
    pub list_changed: bool,
}

/// Resources capability declaration.
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResourcesCapability {
    #[serde(rename = "listChanged")]
    pub list_changed: bool,
    pub subscribe: bool,
}

/// Server info for MCP initialize response.
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ServerInfo {
    pub name: String,
    pub version: String,
}

/// Tool call result content item.
#[allow(dead_code)]
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ToolResultContent {
    #[serde(rename = "type")]
    pub content_type: String,
    pub text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_rpc_request_parse() {
        let json = r#"{"jsonrpc":"2.0","id":1,"method":"tools/list"}"#;
        let req: JsonRpcRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.jsonrpc, "2.0");
        assert_eq!(req.method, "tools/list");
        assert!(req.params.is_none());
    }

    #[test]
    fn test_json_rpc_response_success() {
        let resp = JsonRpcResponse::success(Some(Value::Number(1.into())), Value::String("ok".into()));
        assert_eq!(resp.jsonrpc, "2.0");
        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
    }

    #[test]
    fn test_json_rpc_response_error() {
        let resp = JsonRpcResponse::error(Some(Value::Number(1.into())), METHOD_NOT_FOUND, "Not found".into());
        assert_eq!(resp.jsonrpc, "2.0");
        assert!(resp.result.is_none());
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, METHOD_NOT_FOUND);
    }

    #[test]
    fn test_tool_definition_serialize() {
        let tool = ToolDefinition {
            name: "game/move_units".into(),
            description: "Move units to a target position".into(),
            input_schema: serde_json::json!({
                "type": "object",
                "properties": {
                    "unit_ids": { "type": "array", "items": { "type": "integer" } },
                    "target_x": { "type": "number" },
                    "target_y": { "type": "number" }
                },
                "required": ["unit_ids", "target_x", "target_y"]
            }),
        };
        let json = serde_json::to_string(&tool).unwrap();
        assert!(json.contains("game/move_units"));
        assert!(json.contains("inputSchema"));
    }

    #[test]
    fn test_resource_definition_serialize() {
        let resource = ResourceDefinition {
            uri: "game://state".into(),
            name: "Game State".into(),
            description: "Current game state".into(),
            mime_type: "application/json".into(),
        };
        let json = serde_json::to_string(&resource).unwrap();
        assert!(json.contains("game://state"));
        assert!(json.contains("mimeType"));
    }
}
