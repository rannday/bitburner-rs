use bitburner_api::{BitburnerApi, BitburnerError, SyncItem};
use serde::Serialize;

use crate::AppResult;
use crate::connection::bitburner_error_invalidates_connection;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct SyncUploadItem {
    pub relative_path: String,
    pub remote_path: String,
}

#[derive(Debug)]
pub(crate) struct SyncUploadError {
    pub uploaded: Vec<SyncUploadItem>,
    pub failed: SyncUploadFailure,
    source: Option<Box<BitburnerError>>,
}

impl SyncUploadError {
    pub(crate) fn invalidates_connection(&self) -> bool {
        self.source
            .as_ref()
            .is_some_and(|source| bitburner_error_invalidates_connection(source))
    }

    pub(crate) fn into_anyhow(self) -> anyhow::Error {
        let message = format!(
            "sync upload failed for {} -> {}: {}",
            self.failed.relative_path, self.failed.remote_path, self.failed.error
        );
        match self.source {
            Some(source) => anyhow::Error::new(*source).context(message),
            None => anyhow::anyhow!(message),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub(crate) struct SyncUploadFailure {
    pub relative_path: String,
    pub remote_path: String,
    pub error: String,
}

pub(crate) fn sync_upload_item(item: &SyncItem) -> SyncUploadItem {
    SyncUploadItem {
        relative_path: item.relative_path.to_string_lossy().replace('\\', "/"),
        remote_path: item.remote_path.clone(),
    }
}

pub(crate) fn upload_sync_items<A>(
    api: &mut A,
    server: &str,
    plan: Vec<SyncItem>,
    mut read_content: impl FnMut(&SyncItem) -> AppResult<String>,
) -> std::result::Result<Vec<SyncUploadItem>, SyncUploadError>
where
    A: BitburnerApi + ?Sized,
{
    let mut uploaded = Vec::new();
    for item in plan {
        let content = match read_content(&item) {
            Ok(content) => content,
            Err(err) => return Err(sync_upload_error(uploaded, &item, err, None)),
        };
        if let Err(err) = api.push_file(server, &item.remote_path, &content) {
            let message = err.to_string();
            return Err(sync_upload_error(uploaded, &item, message, Some(err)));
        }
        uploaded.push(sync_upload_item(&item));
    }
    Ok(uploaded)
}

fn sync_upload_error(
    uploaded: Vec<SyncUploadItem>,
    item: &SyncItem,
    err: impl std::fmt::Display,
    source: Option<BitburnerError>,
) -> SyncUploadError {
    SyncUploadError {
        uploaded,
        failed: SyncUploadFailure {
            relative_path: item.relative_path.to_string_lossy().replace('\\', "/"),
            remote_path: item.remote_path.clone(),
            error: err.to_string(),
        },
        source: source.map(Box::new),
    }
}
