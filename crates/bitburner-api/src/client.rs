use std::net::TcpStream;

use bitburner_core::{
    BitburnerClient, BitburnerError, BitburnerFile, BitburnerTransport, FileMetadata,
    JsonRpcRequest, JsonRpcResponse, Result, SaveFile, ServerInfo,
};
use serde_json::Value;
use tungstenite::protocol::Message;
use tungstenite::{WebSocket, accept};

use crate::DEFAULT_REQUEST_TIMEOUT;

pub trait BitburnerApi {
    fn push_file(&mut self, server: &str, filename: &str, content: &str) -> Result<()>;
    fn get_file(&mut self, server: &str, filename: &str) -> Result<String>;
    fn get_file_metadata(&mut self, server: &str, filename: &str) -> Result<FileMetadata>;
    fn delete_file(&mut self, server: &str, filename: &str) -> Result<()>;
    fn get_file_names(&mut self, server: &str) -> Result<Vec<String>>;
    fn get_all_files(&mut self, server: &str) -> Result<Vec<BitburnerFile>>;
    fn get_all_file_metadata(&mut self, server: &str) -> Result<Vec<FileMetadata>>;
    fn calculate_ram(&mut self, server: &str, filename: &str) -> Result<f64>;
    fn get_definition_file(&mut self) -> Result<String>;
    fn get_save_file(&mut self) -> Result<SaveFile>;
    fn get_all_servers(&mut self) -> Result<Vec<ServerInfo>>;
}

pub struct NativeWebSocketTransport {
    socket: WebSocket<TcpStream>,
}

impl NativeWebSocketTransport {
    fn from_stream(stream: TcpStream) -> Result<Self> {
        stream
            .set_read_timeout(Some(DEFAULT_REQUEST_TIMEOUT))
            .map_err(|err| BitburnerError::io(format!("set websocket read timeout: {err}")))?;
        stream
            .set_write_timeout(Some(DEFAULT_REQUEST_TIMEOUT))
            .map_err(|err| BitburnerError::io(format!("set websocket write timeout: {err}")))?;
        let socket = accept(stream).map_err(|err| {
            BitburnerError::websocket(format!(
                "websocket handshake failed; check that Bitburner Remote API is connecting to this address: {err}"
            ))
        })?;
        Ok(Self { socket })
    }

    fn close(&mut self) -> Result<()> {
        self.socket
            .close(None)
            .map_err(|err| BitburnerError::websocket(format!("close websocket: {err}")))
    }
}

impl BitburnerTransport for NativeWebSocketTransport {
    fn send_request_value(&mut self, request: JsonRpcRequest) -> Result<JsonRpcResponse<Value>> {
        let method = request.method.clone();
        let text = serde_json::to_string(&request).map_err(|err| {
            BitburnerError::invalid_protocol(format!("serialize {method} request: {err}"))
        })?;
        self.socket
            .send(Message::Text(text))
            .map_err(|err| BitburnerError::websocket(format!("send {method} request: {err}")))?;

        loop {
            let message = self.socket.read().map_err(|err| {
                BitburnerError::websocket(format!(
                    "{method} timed out or failed waiting for response: {err}"
                ))
            })?;
            let text = match message {
                Message::Text(text) => text,
                Message::Binary(bytes) => String::from_utf8(bytes.to_vec()).map_err(|err| {
                    BitburnerError::invalid_protocol(format!(
                        "{method} returned invalid utf-8 websocket response: {err}"
                    ))
                })?,
                Message::Ping(bytes) => {
                    self.socket.send(Message::Pong(bytes)).map_err(|err| {
                        BitburnerError::websocket(format!(
                            "send pong while waiting for {method}: {err}"
                        ))
                    })?;
                    continue;
                }
                Message::Pong(_) => continue,
                Message::Close(_) => {
                    return Err(BitburnerError::websocket(format!(
                        "websocket closed before {method} response arrived"
                    )));
                }
                Message::Frame(_) => continue,
            };

            return serde_json::from_str(&text).map_err(|err| {
                BitburnerError::invalid_protocol(format!("parse {method} response: {err}"))
            });
        }
    }
}

pub struct RemoteClient {
    client: BitburnerClient<NativeWebSocketTransport>,
}

impl RemoteClient {
    pub fn from_stream(stream: TcpStream) -> Result<Self> {
        let transport = NativeWebSocketTransport::from_stream(stream)?;
        Ok(Self {
            client: BitburnerClient::new(transport),
        })
    }

    pub fn close(&mut self) -> Result<()> {
        self.client.transport_mut().close()
    }

    pub fn push_file(&mut self, server: &str, filename: &str, content: &str) -> Result<()> {
        self.client.push_file(server, filename, content)
    }

    pub fn get_file(&mut self, server: &str, filename: &str) -> Result<String> {
        self.client.get_file(server, filename)
    }

    pub fn get_file_metadata(&mut self, server: &str, filename: &str) -> Result<FileMetadata> {
        self.client.get_file_metadata(server, filename)
    }

    pub fn delete_file(&mut self, server: &str, filename: &str) -> Result<()> {
        self.client.delete_file(server, filename)
    }

    pub fn get_file_names(&mut self, server: &str) -> Result<Vec<String>> {
        self.client.get_file_names(server)
    }

    pub fn get_all_files(&mut self, server: &str) -> Result<Vec<BitburnerFile>> {
        self.client.get_all_files(server)
    }

    pub fn get_all_file_metadata(&mut self, server: &str) -> Result<Vec<FileMetadata>> {
        self.client.get_all_file_metadata(server)
    }

    pub fn calculate_ram(&mut self, server: &str, filename: &str) -> Result<f64> {
        self.client.calculate_ram(server, filename)
    }

    pub fn get_definition_file(&mut self) -> Result<String> {
        self.client.get_definition_file()
    }

    pub fn get_save_file(&mut self) -> Result<SaveFile> {
        self.client.get_save_file()
    }

    pub fn get_all_servers(&mut self) -> Result<Vec<ServerInfo>> {
        self.client.get_all_servers()
    }
}

impl BitburnerApi for RemoteClient {
    fn push_file(&mut self, server: &str, filename: &str, content: &str) -> Result<()> {
        RemoteClient::push_file(self, server, filename, content)
    }

    fn get_file(&mut self, server: &str, filename: &str) -> Result<String> {
        RemoteClient::get_file(self, server, filename)
    }

    fn get_file_metadata(&mut self, server: &str, filename: &str) -> Result<FileMetadata> {
        RemoteClient::get_file_metadata(self, server, filename)
    }

    fn delete_file(&mut self, server: &str, filename: &str) -> Result<()> {
        RemoteClient::delete_file(self, server, filename)
    }

    fn get_file_names(&mut self, server: &str) -> Result<Vec<String>> {
        RemoteClient::get_file_names(self, server)
    }

    fn get_all_files(&mut self, server: &str) -> Result<Vec<BitburnerFile>> {
        RemoteClient::get_all_files(self, server)
    }

    fn get_all_file_metadata(&mut self, server: &str) -> Result<Vec<FileMetadata>> {
        RemoteClient::get_all_file_metadata(self, server)
    }

    fn calculate_ram(&mut self, server: &str, filename: &str) -> Result<f64> {
        RemoteClient::calculate_ram(self, server, filename)
    }

    fn get_definition_file(&mut self) -> Result<String> {
        RemoteClient::get_definition_file(self)
    }

    fn get_save_file(&mut self) -> Result<SaveFile> {
        RemoteClient::get_save_file(self)
    }

    fn get_all_servers(&mut self) -> Result<Vec<ServerInfo>> {
        RemoteClient::get_all_servers(self)
    }
}
