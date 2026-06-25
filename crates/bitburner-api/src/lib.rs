use std::time::Duration;

mod client;
mod protocol;
mod types;

pub use client::{BitburnerApi, RemoteClient};
pub use protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
pub use types::{BitburnerFile, FileMetadata, SaveFile, ServerInfo};

pub const DEFAULT_ADDRESS: &str = "127.0.0.1:12525";
pub const DEFAULT_SERVER: &str = "home";
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

pub type Result<T> = anyhow::Result<T>;
