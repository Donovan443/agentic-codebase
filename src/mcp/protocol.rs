//! JSON-RPC 2.0 protocol types for the MCP server.
//!
//! Defines the wire format for JSON-RPC messages used in the
//! Model Context Protocol server implementation.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC 2.0 request message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcRequest {
    /// Must be "2.0".
    pub jsonrpc: String,
    /// Request ID (number or string). Absent for notifications.
    #[serde(default)]
    pub id: Option<Value>,
    /// Method name.
    pub method: String,
    /// Method parameters (optional).
    #[serde(default)]
    pub params: Value,
}

impl JsonRpcRequest {
    /// Create a new JSON-RPC request.
    pub fn new(id: impl Into<Value>, method: impl Into<String>) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: Some(id.into()),
            method: method.into(),
            params: Value::Null,
        }
    }

    /// Create a new JSON-RPC request with parameters.
    pub fn with_params(id: impl Into<Value>, method: impl Into<String>, params: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id: Some(id.into()),
            method: method.into(),
            params,
        }
    }
}

/// JSON-RPC 2.0 error object.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    /// Error code.
    pub code: i32,
    /// Error message.
    pub message: String,
    /// Additional error data (optional).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    /// Standard error: Parse error (-32700).
    pub fn parse_error(detail: impl Into<String>) -> Self {
        Self {
            code: -32700,
            message: "Parse error".to_string(),
            data: Some(Value::String(detail.into())),
        }
    }

    /// Standard error: Invalid request (-32600).
    pub fn invalid_request(detail: impl Into<String>) -> Self {
        Self {
            code: -32600,
            message: "Invalid Request".to_string(),
            data: Some(Value::String(detail.into())),
        }
    }

    /// Standard error: Method not found (-32601).
    pub fn method_not_found(method: impl Into<String>) -> Self {
        Self {
            code: -32601,
            message: "Method not found".to_string(),
            data: Some(Value::String(method.into())),
        }
    }

    /// MCP error: Tool not found (-32803).
    pub fn tool_not_found(tool: impl Into<String>) -> Self {
        Self {
            code: -32803,
            message: "Tool not found".to_string(),
            data: Some(Value::String(tool.into())),
        }
    }

    /// Standard error: Invalid params (-32602).
    pub fn invalid_params(detail: impl Into<String>) -> Self {
        Self {
            code: -32602,
            message: "Invalid params".to_string(),
            data: Some(Value::String(detail.into())),
        }
    }

    /// Standard error: Internal error (-32603).
    pub fn internal_error(detail: impl Into<String>) -> Self {
        Self {
            code: -32603,
            message: "Internal error".to_string(),
            data: Some(Value::String(detail.into())),
        }
    }
}

/// JSON-RPC 2.0 response message.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcResponse {
    /// Must be "2.0".
    pub jsonrpc: String,
    /// Must match the request ID.
    pub id: Value,
    /// The result (present on success).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    /// The error (present on failure).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
}

impl JsonRpcResponse {
    /// Create a success response.
    pub fn success(id: Value, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    /// Create an error response.
    pub fn error(id: Value, error: JsonRpcError) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(error),
        }
    }

    /// Create a tool error response (isError: true per MCP spec).
    /// Use this for tool execution failures instead of `error()`.
    pub fn tool_error(id: Value, message: impl Into<String>) -> Self {
        Self::success(
            id,
            serde_json::json!({
                "content": [{"type": "text", "text": message.into()}],
                "isError": true
            }),
        )
    }

    /// If this is a JSON-RPC error response, convert it to a tool error
    /// (isError: true) per MCP spec. Protocol errors pass through unchanged.
    pub fn into_tool_error_if_needed(self) -> Self {
        if let Some(ref err) = self.error {
            // Only convert tool execution errors, not protocol-level errors.
            // Protocol errors: parse (-32700), invalid request (-32600),
            // method not found (-32601), tool not found (-32803).
            match err.code {
                -32700 | -32600 | -32601 | -32803 => self, // keep as JSON-RPC error
                _ => {
                    // Convert to isError: true
                    let msg = if let Some(ref data) = err.data {
                        format!("{}: {}", err.message, data)
                    } else {
                        err.message.clone()
                    };
                    Self::tool_error(self.id.clone(), msg)
                }
            }
        } else {
            self
        }
    }
}

/// Parse a raw JSON string into a JSON-RPC request.
///
/// Returns an error response if parsing fails.
#[allow(clippy::result_large_err)]
pub fn parse_request(raw: &str) -> Result<JsonRpcRequest, JsonRpcResponse> {
    let value: Value = serde_json::from_str(raw).map_err(|e| {
        JsonRpcResponse::error(Value::Null, JsonRpcError::parse_error(e.to_string()))
    })?;

    let request: JsonRpcRequest = serde_json::from_value(value).map_err(|e| {
        JsonRpcResponse::error(Value::Null, JsonRpcError::invalid_request(e.to_string()))
    })?;

    if request.jsonrpc != "2.0" {
        return Err(JsonRpcResponse::error(
            request.id.unwrap_or(Value::Null),
            JsonRpcError::invalid_request("jsonrpc must be \"2.0\""),
        ));
    }

    Ok(request)
}
