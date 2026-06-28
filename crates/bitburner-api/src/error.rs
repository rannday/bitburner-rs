use std::fmt;

use crate::JsonRpcError;

#[derive(Debug)]
pub enum BitburnerError {
    InvalidPath(String),
    InvalidProtocol(String),
    JsonRpc(JsonRpcError),
    Json(serde_json::Error),
    Io {
        context: String,
        source: Box<std::io::Error>,
    },
    WebSocket {
        context: String,
        source: Box<tungstenite::Error>,
    },
}

pub type Result<T> = std::result::Result<T, BitburnerError>;

impl BitburnerError {
    pub fn invalid_path(message: impl Into<String>) -> Self {
        Self::InvalidPath(message.into())
    }

    pub fn invalid_protocol(message: impl Into<String>) -> Self {
        Self::InvalidProtocol(message.into())
    }

    pub fn io(context: impl Into<String>, source: std::io::Error) -> Self {
        Self::Io {
            context: context.into(),
            source: Box::new(source),
        }
    }

    pub fn websocket(context: impl Into<String>, source: tungstenite::Error) -> Self {
        Self::WebSocket {
            context: context.into(),
            source: Box::new(source),
        }
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
            Self::Io { context, source } => write!(formatter, "{context}: {source}"),
            Self::WebSocket { context, source } => write!(formatter, "{context}: {source}"),
        }
    }
}

impl std::error::Error for BitburnerError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Json(error) => Some(error),
            Self::Io { source, .. } => Some(source.as_ref()),
            Self::WebSocket { source, .. } => Some(source.as_ref()),
            _ => None,
        }
    }
}

impl From<serde_json::Error> for BitburnerError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

#[cfg(test)]
mod tests {
    use std::error::Error as _;

    use super::*;

    #[test]
    fn io_error_preserves_context_and_source() {
        let err = BitburnerError::io(
            "read websocket response",
            std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe broke"),
        );

        let display = err.to_string();
        assert!(display.contains("read websocket response"));
        assert!(display.contains("pipe broke"));
        assert_eq!(
            err.source().expect("source").to_string(),
            "pipe broke".to_string()
        );
    }

    #[test]
    fn websocket_error_preserves_context_and_source() {
        let err = BitburnerError::websocket(
            "send websocket request",
            tungstenite::Error::Io(std::io::Error::new(
                std::io::ErrorKind::ConnectionReset,
                "connection reset",
            )),
        );

        let display = err.to_string();
        assert!(display.contains("send websocket request"));
        assert!(display.contains("connection reset"));
        assert!(
            err.source()
                .expect("source")
                .to_string()
                .contains("connection reset")
        );
    }
}
