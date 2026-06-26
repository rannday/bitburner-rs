use std::net::TcpStream;

use serde_json::Value;
use tungstenite::protocol::Message;
use tungstenite::{WebSocket, accept};

use crate::{BitburnerError, DEFAULT_REQUEST_TIMEOUT, JsonRpcRequest, JsonRpcResponse, Result};

pub trait BitburnerTransport {
    fn send_request_value(&mut self, request: JsonRpcRequest) -> Result<JsonRpcResponse<Value>>;
}

pub struct NativeWebSocketTransport {
    socket: WebSocket<TcpStream>,
}

impl NativeWebSocketTransport {
    pub fn from_stream(stream: TcpStream) -> Result<Self> {
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

    pub fn close(&mut self) -> Result<()> {
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
