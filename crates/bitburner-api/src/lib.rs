use std::time::Duration;

mod client;
mod error;
mod path;
mod protocol;
mod sync;
mod transport;
mod types;

pub use client::{BitburnerApi, RemoteClient};
pub use error::{BitburnerError, Result};
pub use path::{
    join_remote_paths, normalize_remote_file_path, normalize_remote_path, path_to_forward_slashes,
    relative_remote_path, remote_path_to_local,
};
pub use protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
pub use sync::{
    DEFAULT_IGNORED_DIR_NAMES, LocalFileEntry, SyncItem, SyncOptions, UploadableExtension,
    UploadableFileKind, build_sync_plan_from_entries, is_default_ignored_dir_name,
    is_uploadable_path, is_uploadable_path_with_extensions,
};
pub use transport::NativeWebSocketTransport;
pub use types::{BitburnerFile, FileMetadata, SaveFile, ServerInfo};

pub const DEFAULT_ADDRESS: &str = "127.0.0.1:12525";
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
pub const DEFAULT_SERVER: &str = "home";
