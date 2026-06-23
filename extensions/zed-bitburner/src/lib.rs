pub const DEFAULT_SERVER: &str = "home";
pub const DEFAULT_REMOTE_DIR: &str = "";
pub const DEFAULT_ADDR: &str = "127.0.0.1:12525";
pub const DEFAULT_CLEAN: bool = false;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncSettings {
    pub server: String,
    pub remote_dir: String,
    pub addr: String,
    pub clean: bool,
}

impl Default for SyncSettings {
    fn default() -> Self {
        Self {
            server: DEFAULT_SERVER.to_string(),
            remote_dir: DEFAULT_REMOTE_DIR.to_string(),
            addr: DEFAULT_ADDR.to_string(),
            clean: DEFAULT_CLEAN,
        }
    }
}

pub fn build_sync_args(workspace_root: &str, settings: &SyncSettings) -> Vec<String> {
    let mut args = vec![
        "sync".to_string(),
        workspace_root.to_string(),
        settings.remote_dir.clone(),
        "--server".to_string(),
        settings.server.clone(),
        "--addr".to_string(),
        settings.addr.clone(),
    ];

    if settings.remote_dir.is_empty() {
        args.remove(2);
    }

    if settings.clean {
        args.push("--clean".to_string());
    }

    args
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_sync_args_with_defaults() {
        assert_eq!(
            build_sync_args("C:\\repo", &SyncSettings::default()),
            vec![
                "sync",
                "C:\\repo",
                "--server",
                "home",
                "--addr",
                "127.0.0.1:12525",
            ]
        );
    }

    #[test]
    fn builds_sync_args_with_remote_dir_and_clean() {
        let settings = SyncSettings {
            remote_dir: "scripts".to_string(),
            clean: true,
            ..SyncSettings::default()
        };

        assert_eq!(
            build_sync_args("C:\\repo", &settings),
            vec![
                "sync",
                "C:\\repo",
                "scripts",
                "--server",
                "home",
                "--addr",
                "127.0.0.1:12525",
                "--clean",
            ]
        );
    }
}
