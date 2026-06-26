mod error;
mod path;
mod protocol;
mod sync;
mod types;

pub use error::{BitburnerError, Result};
pub use path::{
    join_remote_paths, normalize_remote_file_path, normalize_remote_path, path_to_forward_slashes,
    relative_remote_path, remote_path_to_local,
};
pub use protocol::{JsonRpcError, JsonRpcRequest, JsonRpcResponse};
pub use sync::{
    LocalFileEntry, SyncItem, SyncOptions, UploadableExtension, UploadableFileKind,
    build_sync_plan_from_entries, is_uploadable_path, is_uploadable_path_with_extensions,
};
pub use types::{BitburnerFile, FileMetadata, SaveFile, ServerInfo};

pub const DEFAULT_SERVER: &str = "home";
