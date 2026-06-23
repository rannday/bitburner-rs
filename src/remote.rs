use std::net::{TcpListener, TcpStream};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tungstenite::protocol::Message;
use tungstenite::{WebSocket, accept};

use crate::error::{AppError, AppResult};

pub const DEFAULT_ADDRESS: &str = "127.0.0.1:12525";
pub const DEFAULT_SERVER: &str = "home";

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

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileMetadata {
    pub filename: String,
    #[serde(default)]
    pub server: Option<String>,
    #[serde(default)]
    pub ram: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BitburnerFile {
    pub filename: String,
    #[serde(default)]
    pub content: String,
    #[serde(default)]
    pub server: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SaveFile {
    pub data: Value,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ServerInfo {
    pub data: Value,
}

pub struct RemoteClient {
    socket: WebSocket<TcpStream>,
    next_id: u64,
}

impl RemoteClient {
    pub fn listen(address: &str) -> AppResult<Self> {
        let listener = TcpListener::bind(address)?;
        println!("listening on {address}");
        println!("waiting for Bitburner Remote API client");
        let (stream, peer) = listener.accept()?;
        println!("client connected from {peer}");
        let socket = accept(stream)
            .map_err(|err| AppError::Remote(format!("websocket handshake failed: {err}")))?;
        println!("websocket connected");
        Ok(Self { socket, next_id: 1 })
    }

    pub fn push_file(&mut self, server: &str, filename: &str, content: &str) -> AppResult<Value> {
        self.request(
            "pushFile",
            Some(json!({
              "filename": filename,
              "content": content,
              "server": normalize_server(server),
            })),
        )
    }

    pub fn get_file(&mut self, server: &str, filename: &str) -> AppResult<String> {
        self.request(
            "getFile",
            Some(json!({
              "filename": filename,
              "server": normalize_server(server),
            })),
        )
    }

    pub fn get_file_metadata(&mut self, server: &str, filename: &str) -> AppResult<FileMetadata> {
        self.request(
            "getFileMetadata",
            Some(json!({
              "filename": filename,
              "server": normalize_server(server),
            })),
        )
    }

    pub fn delete_file(&mut self, server: &str, filename: &str) -> AppResult<Value> {
        self.request(
            "deleteFile",
            Some(json!({
              "filename": filename,
              "server": normalize_server(server),
            })),
        )
    }

    pub fn get_file_names(&mut self, server: &str) -> AppResult<Vec<String>> {
        self.request(
            "getFileNames",
            Some(json!({
              "server": normalize_server(server),
            })),
        )
    }

    pub fn get_all_files(&mut self, server: &str) -> AppResult<Vec<BitburnerFile>> {
        self.request(
            "getAllFiles",
            Some(json!({
              "server": normalize_server(server),
            })),
        )
    }

    pub fn get_all_file_metadata(&mut self, server: &str) -> AppResult<Vec<FileMetadata>> {
        self.request(
            "getAllFileMetadata",
            Some(json!({
              "server": normalize_server(server),
            })),
        )
    }

    pub fn calculate_ram(&mut self, server: &str, filename: &str) -> AppResult<f64> {
        self.request(
            "calculateRam",
            Some(json!({
              "filename": filename,
              "server": normalize_server(server),
            })),
        )
    }

    pub fn get_definition_file(&mut self) -> AppResult<String> {
        self.request("getDefinitionFile", None)
    }

    pub fn get_save_file(&mut self) -> AppResult<SaveFile> {
        self.request("getSaveFile", None)
    }

    #[allow(dead_code)]
    pub fn get_all_servers(&mut self) -> AppResult<Vec<ServerInfo>> {
        self.request("getAllServers", None)
    }

    pub fn clean_server(&mut self, server: &str) -> AppResult<()> {
        let _ = server;
        Err(AppError::NotImplemented(
            "clean is TODO: needs conservative delete policy on top of getFileNames/deleteFile"
                .to_string(),
        ))
    }

    pub fn build_request(&mut self, method: &str, params: Option<Value>) -> JsonRpcRequest {
        let id = self.next_id;
        self.next_id += 1;
        JsonRpcRequest {
            jsonrpc: "2.0",
            id,
            method: method.to_string(),
            params,
        }
    }

    pub fn request<T>(&mut self, method: &str, params: Option<Value>) -> AppResult<T>
    where
        T: DeserializeOwned,
    {
        let request = self.build_request(method, params);
        let request_id = request.id;
        let text = serde_json::to_string(&request)?;
        self.socket.send(Message::Text(text.into()))?;

        loop {
            let message = self.socket.read()?;
            let text = match message {
                Message::Text(text) => text,
                Message::Binary(bytes) => String::from_utf8(bytes.to_vec())
                    .map_err(|err| {
                        AppError::Remote(format!("invalid utf-8 websocket response: {err}"))
                    })?
                    .into(),
                Message::Ping(bytes) => {
                    self.socket.send(Message::Pong(bytes))?;
                    continue;
                }
                Message::Pong(_) => continue,
                Message::Close(_) => {
                    return Err(AppError::Remote(
                        "websocket closed before response arrived".to_string(),
                    ));
                }
                Message::Frame(_) => continue,
            };

            let response: JsonRpcResponse<T> = serde_json::from_str(&text)?;
            if response.id != request_id {
                continue;
            }
            if response.jsonrpc != "2.0" {
                return Err(AppError::Remote(format!(
                    "invalid jsonrpc version '{}'",
                    response.jsonrpc
                )));
            }
            if let Some(error) = response.error {
                return Err(AppError::Remote(format!(
                    "remote error {}: {}",
                    error
                        .code
                        .map_or_else(|| "?".to_string(), |code| code.to_string()),
                    error.message
                )));
            }
            return response
                .result
                .ok_or_else(|| AppError::Remote("response missing result".to_string()));
        }
    }
}

#[cfg(test)]
pub fn build_request_for_test(method: &str, params: Option<Value>) -> JsonRpcRequest {
    JsonRpcRequest {
        jsonrpc: "2.0",
        id: 1,
        method: method.to_string(),
        params,
    }
}

fn normalize_server(server: &str) -> &str {
    if server.is_empty() {
        DEFAULT_SERVER
    } else {
        server
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_json_uses_object_params() {
        let request = build_request_for_test(
            "pushFile",
            Some(json!({
              "filename": "scripts/main.js",
              "content": "export async function main() {}",
              "server": "home",
            })),
        );

        let actual = serde_json::to_value(request).expect("json");

        assert_eq!(
            actual,
            json!({
              "jsonrpc": "2.0",
              "id": 1,
              "method": "pushFile",
              "params": {
                "filename": "scripts/main.js",
                "content": "export async function main() {}",
                "server": "home",
              },
            })
        );
    }

    #[test]
    fn response_json_parses_result() {
        let response: JsonRpcResponse<String> =
            serde_json::from_str(r#"{"jsonrpc":"2.0","id":7,"result":"ok"}"#).expect("response");

        assert_eq!(response.jsonrpc, "2.0");
        assert_eq!(response.id, 7);
        assert_eq!(response.result.as_deref(), Some("ok"));
        assert!(response.error.is_none());
    }

    #[test]
    fn response_json_parses_error() {
        let response: JsonRpcResponse<String> = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":7,"error":{"code":-32000,"message":"bad file"}}"#,
        )
        .expect("response");

        assert_eq!(response.error.expect("error").message, "bad file");
    }
}
