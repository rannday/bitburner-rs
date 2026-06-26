use bitburner_core::{DEFAULT_SERVER, SyncOptions, UploadableExtension, normalize_remote_path};
use zed_extension_api as zed;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 12525;
const DEFAULT_REMOTE_DIR: &str = "scripts";

struct BitburnerExtension;

impl zed::Extension for BitburnerExtension {
    fn new() -> Self {
        Self
    }

    fn run_slash_command(
        &self,
        command: zed::SlashCommand,
        _args: Vec<String>,
        worktree: Option<&zed::Worktree>,
    ) -> Result<zed::SlashCommandOutput, String> {
        if command.name != "bitburner" {
            return Err(format!("unsupported Bitburner slash command '{}'", command.name));
        }

        Ok(zed::SlashCommandOutput {
            text: extension_status(worktree),
            sections: Vec::new(),
        })
    }
}

fn extension_status(worktree: Option<&zed::Worktree>) -> String {
    let remote_dir = normalize_remote_path(DEFAULT_REMOTE_DIR)
        .unwrap_or_else(|_| DEFAULT_REMOTE_DIR.to_string());
    let extensions = UploadableExtension::ALL
        .iter()
        .map(|extension| format!(".{}", extension.as_str()))
        .collect::<Vec<_>>()
        .join(", ");
    let options = SyncOptions {
        remote_dir: Some(remote_dir.clone()),
        allowed_extensions: UploadableExtension::ALL.to_vec(),
    };
    let worktree_root = worktree.map_or_else(|| "<none>".to_string(), zed::Worktree::root_path);

    format!(
        "Bitburner Zed extension\n\
         server: {DEFAULT_SERVER}\n\
         remoteDir: {remote_dir}\n\
         host: {DEFAULT_HOST}\n\
         port: {DEFAULT_PORT}\n\
         uploadable extensions: {extensions}\n\
         worktree: {worktree_root}\n\
         sync planner: available through bitburner-core\n\
         configured remote dir: {}\n\
         remote transport: unavailable in zed_extension_api 0.7.0; no TCP or websocket API is exposed",
        options.remote_dir.as_deref().unwrap_or("")
    )
}

zed::register_extension!(BitburnerExtension);
