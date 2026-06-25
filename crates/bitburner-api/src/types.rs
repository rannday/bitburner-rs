use serde::{Deserialize, Serialize};

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
