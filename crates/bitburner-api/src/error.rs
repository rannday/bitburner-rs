use std::fmt;

use crate::JsonRpcError;

#[derive(Debug)]
pub enum BitburnerError {
    InvalidPath(String),
    InvalidProtocol(String),
    JsonRpc(JsonRpcError),
    Json(serde_json::Error),
    Io(String),
    WebSocket(String),
}

pub type Result<T> = std::result::Result<T, BitburnerError>;

impl BitburnerError {
    pub fn invalid_path(message: impl Into<String>) -> Self {
        Self::InvalidPath(message.into())
    }

    pub fn invalid_protocol(message: impl Into<String>) -> Self {
        Self::InvalidProtocol(message.into())
    }

    pub fn io(message: impl Into<String>) -> Self {
        Self::Io(message.into())
    }

    pub fn websocket(message: impl Into<String>) -> Self {
        Self::WebSocket(message.into())
    }
}

impl fmt::Display for BitburnerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPath(message) => formatter.write_str(message),
            Self::InvalidProtocol(message) => formatter.write_str(message),
            Self::JsonRpc(error) => write!(
                formatter,
                "remote error {}: {}",
                error
                    .code
                    .map_or_else(|| "?".to_string(), |code| code.to_string()),
                error.message
            ),
            Self::Json(error) => write!(formatter, "{error}"),
            Self::Io(message) => formatter.write_str(message),
            Self::WebSocket(message) => formatter.write_str(message),
        }
    }
}

impl std::error::Error for BitburnerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            _ => None,
        }
    }
}

impl From<serde_json::Error> for BitburnerError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}
