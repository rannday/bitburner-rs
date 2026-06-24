use std::net::{TcpListener, TcpStream};
use std::time::Duration;

use anyhow::{Context, bail};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tungstenite::protocol::Message;
use tungstenite::{WebSocket, accept};

use crate::error::AppResult;

pub const DEFAULT_ADDRESS: &str = "127.0.0.1:12525";
pub const DEFAULT_SERVER: &str = "home";
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

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
    pub atime: String,
    pub btime: String,
    pub mtime: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BitburnerFile {
    pub filename: String,
    pub content: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SaveFile {
    pub identifier: String,
    pub binary: bool,
    pub save: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerInfo {
    pub hostname: String,
    #[serde(rename = "hasAdminRights")]
    pub has_admin_rights: bool,
    #[serde(rename = "purchasedByPlayer")]
    pub purchased_by_player: bool,
}

pub trait BitburnerApi {
    fn push_file(&mut self, server: &str, filename: &str, content: &str) -> AppResult<()>;
    fn get_file(&mut self, server: &str, filename: &str) -> AppResult<String>;
    fn get_file_metadata(&mut self, server: &str, filename: &str) -> AppResult<FileMetadata>;
    fn delete_file(&mut self, server: &str, filename: &str) -> AppResult<()>;
    fn get_file_names(&mut self, server: &str) -> AppResult<Vec<String>>;
    fn get_all_files(&mut self, server: &str) -> AppResult<Vec<BitburnerFile>>;
    fn get_all_file_metadata(&mut self, server: &str) -> AppResult<Vec<FileMetadata>>;
    fn calculate_ram(&mut self, server: &str, filename: &str) -> AppResult<f64>;
    fn get_definition_file(&mut self) -> AppResult<String>;
    fn get_save_file(&mut self) -> AppResult<SaveFile>;
    fn get_all_servers(&mut self) -> AppResult<Vec<ServerInfo>>;
}

pub struct RemoteClient {
    socket: WebSocket<TcpStream>,
    next_id: u64,
}

impl RemoteClient {
    pub fn listen(address: &str) -> AppResult<Self> {
        let listener = TcpListener::bind(address)
            .with_context(|| format!("bind websocket server on {address}"))?;
        let (stream, _) = listener
            .accept()
            .with_context(|| format!("accept websocket connection on {address}"))?;
        Self::from_stream(stream)
    }

    pub fn from_stream(stream: TcpStream) -> AppResult<Self> {
        stream
            .set_read_timeout(Some(DEFAULT_REQUEST_TIMEOUT))
            .context("set websocket read timeout")?;
        stream
            .set_write_timeout(Some(DEFAULT_REQUEST_TIMEOUT))
            .context("set websocket write timeout")?;
        let socket = accept(stream).context("websocket handshake failed")?;
        Ok(Self { socket, next_id: 1 })
    }

    pub fn close(&mut self) -> AppResult<()> {
        self.socket.close(None).context("close websocket")
    }

    pub fn push_file(&mut self, server: &str, filename: &str, content: &str) -> AppResult<()> {
        let result: String = self.request(
            "pushFile",
            Some(json!({
              "filename": filename,
              "content": content,
              "server": normalize_server(server),
            })),
        )?;
        validate_ok("pushFile", &result)
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

    pub fn delete_file(&mut self, server: &str, filename: &str) -> AppResult<()> {
        let result: String = self.request(
            "deleteFile",
            Some(json!({
              "filename": filename,
              "server": normalize_server(server),
            })),
        )?;
        validate_ok("deleteFile", &result)
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

    pub fn get_all_servers(&mut self) -> AppResult<Vec<ServerInfo>> {
        self.request("getAllServers", None)
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
        let text = serde_json::to_string(&request)
            .with_context(|| format!("serialize {method} request"))?;
        self.socket
            .send(Message::Text(text))
            .with_context(|| format!("send {method} request"))?;

        loop {
            let message = self
                .socket
                .read()
                .with_context(|| format!("{method} timed out or failed waiting for response"))?;
            let text = match message {
                Message::Text(text) => text,
                Message::Binary(bytes) => String::from_utf8(bytes.to_vec()).with_context(|| {
                    format!("{method} returned invalid utf-8 websocket response")
                })?,
                Message::Ping(bytes) => {
                    self.socket
                        .send(Message::Pong(bytes))
                        .with_context(|| format!("send pong while waiting for {method}"))?;
                    continue;
                }
                Message::Pong(_) => continue,
                Message::Close(_) => {
                    bail!("websocket closed before {method} response arrived");
                }
                Message::Frame(_) => continue,
            };

            let response: JsonRpcResponse<T> =
                serde_json::from_str(&text).with_context(|| format!("parse {method} response"))?;
            if response.id != request_id {
                continue;
            }
            if response.jsonrpc != "2.0" {
                bail!("invalid jsonrpc version '{}'", response.jsonrpc);
            }
            if let Some(error) = response.error {
                bail!(
                    "remote error {}: {}",
                    error
                        .code
                        .map_or_else(|| "?".to_string(), |code| code.to_string()),
                    error.message
                );
            }
            return response
                .result
                .with_context(|| format!("{method} response missing result"));
        }
    }
}

impl BitburnerApi for RemoteClient {
    fn push_file(&mut self, server: &str, filename: &str, content: &str) -> AppResult<()> {
        RemoteClient::push_file(self, server, filename, content)
    }

    fn get_file(&mut self, server: &str, filename: &str) -> AppResult<String> {
        RemoteClient::get_file(self, server, filename)
    }

    fn get_file_metadata(&mut self, server: &str, filename: &str) -> AppResult<FileMetadata> {
        RemoteClient::get_file_metadata(self, server, filename)
    }

    fn delete_file(&mut self, server: &str, filename: &str) -> AppResult<()> {
        RemoteClient::delete_file(self, server, filename)
    }

    fn get_file_names(&mut self, server: &str) -> AppResult<Vec<String>> {
        RemoteClient::get_file_names(self, server)
    }

    fn get_all_files(&mut self, server: &str) -> AppResult<Vec<BitburnerFile>> {
        RemoteClient::get_all_files(self, server)
    }

    fn get_all_file_metadata(&mut self, server: &str) -> AppResult<Vec<FileMetadata>> {
        RemoteClient::get_all_file_metadata(self, server)
    }

    fn calculate_ram(&mut self, server: &str, filename: &str) -> AppResult<f64> {
        RemoteClient::calculate_ram(self, server, filename)
    }

    fn get_definition_file(&mut self) -> AppResult<String> {
        RemoteClient::get_definition_file(self)
    }

    fn get_save_file(&mut self) -> AppResult<SaveFile> {
        RemoteClient::get_save_file(self)
    }

    fn get_all_servers(&mut self) -> AppResult<Vec<ServerInfo>> {
        RemoteClient::get_all_servers(self)
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

fn validate_ok(method: &str, result: &str) -> AppResult<()> {
    if result == "OK" {
        Ok(())
    } else {
        bail!("{method} returned unexpected result '{result}'")
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

    #[test]
    fn metadata_parses_official_shape() {
        let response: JsonRpcResponse<FileMetadata> = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":1,"result":{"filename":"a.js","atime":"1","btime":"2","mtime":"3"}}"#,
        )
        .expect("response");

        assert_eq!(
            response.result.expect("metadata"),
            FileMetadata {
                filename: "a.js".to_string(),
                atime: "1".to_string(),
                btime: "2".to_string(),
                mtime: "3".to_string(),
            }
        );
    }

    #[test]
    fn save_file_parses_official_shape() {
        let response: JsonRpcResponse<SaveFile> = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":1,"result":{"identifier":"bitburner","binary":false,"save":"abc"}}"#,
        )
        .expect("response");

        assert_eq!(
            response.result.expect("save"),
            SaveFile {
                identifier: "bitburner".to_string(),
                binary: false,
                save: "abc".to_string(),
            }
        );
    }

    #[test]
    fn servers_parse_official_shape() {
        let response: JsonRpcResponse<Vec<ServerInfo>> = serde_json::from_str(
            r#"{"jsonrpc":"2.0","id":1,"result":[{"hostname":"home","hasAdminRights":true,"purchasedByPlayer":false}]}"#,
        )
        .expect("response");

        assert_eq!(
            response.result.expect("servers"),
            vec![ServerInfo {
                hostname: "home".to_string(),
                has_admin_rights: true,
                purchased_by_player: false,
            }]
        );
    }

    #[test]
    fn ok_result_validation_rejects_non_ok() {
        let err = validate_ok("pushFile", "NOPE").expect_err("error");

        assert!(err.to_string().contains("pushFile"));
    }
}
