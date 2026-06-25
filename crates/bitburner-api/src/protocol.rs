use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JsonRpcRequest {
    pub jsonrpc: &'static str,
    pub id: u64,
    pub method: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub params: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct JsonRpcResponse<T> {
    pub jsonrpc: String,
    pub id: u64,
    pub result: Option<T>,
    pub error: Option<JsonRpcError>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct JsonRpcError {
    pub code: Option<i64>,
    pub message: String,
    #[serde(default)]
    pub data: Option<Value>,
}
