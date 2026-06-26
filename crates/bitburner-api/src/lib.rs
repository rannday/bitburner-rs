use std::time::Duration;

mod client;

pub use bitburner_core::{
    BitburnerError, BitburnerFile, DEFAULT_SERVER, FileMetadata, JsonRpcError, JsonRpcRequest,
    JsonRpcResponse, Result, SaveFile, ServerInfo,
};
pub use client::{BitburnerApi, NativeWebSocketTransport, RemoteClient};

pub const DEFAULT_ADDRESS: &str = "127.0.0.1:12525";
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
