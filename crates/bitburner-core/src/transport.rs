use serde_json::Value;

use crate::{JsonRpcRequest, JsonRpcResponse, Result};

pub trait BitburnerTransport {
    fn send_request_value(&mut self, request: JsonRpcRequest) -> Result<JsonRpcResponse<Value>>;
}
