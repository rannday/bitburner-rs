mod args;
mod cli;
mod error;
pub mod fs_sync;
pub mod path;
pub mod remote;
mod ws;

pub use error::AppResult;
pub use remote::{
    BitburnerApi, BitburnerFile, DEFAULT_ADDRESS, DEFAULT_REQUEST_TIMEOUT, DEFAULT_SERVER,
    FileMetadata, JsonRpcError, JsonRpcRequest, JsonRpcResponse, RemoteClient, SaveFile,
    ServerInfo,
};

pub fn run_cli() -> AppResult<()> {
    cli::run()
}
