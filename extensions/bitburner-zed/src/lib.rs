use zed_extension_api as zed;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: u16 = 12525;
const DEFAULT_SERVER: &str = "home";
const DEFAULT_REMOTE_DIR: &str = "scripts";
const UPLOADABLE_EXTENSIONS: [&str; 5] = [".js", ".ts", ".txt", ".script", ".ns"];

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
    let extensions = UPLOADABLE_EXTENSIONS.join(", ");
    let worktree_root = worktree.map_or_else(|| "<none>".to_string(), zed::Worktree::root_path);

    format!(
        "Bitburner Zed extension\n\
         server: {DEFAULT_SERVER}\n\
         remoteDir: {DEFAULT_REMOTE_DIR}\n\
         host: {DEFAULT_HOST}\n\
         port: {DEFAULT_PORT}\n\
         uploadable extensions: {extensions}\n\
         worktree: {worktree_root}\n\
         sync planner: unavailable in extension crate; bitburner-api is native-only\n\
         remote transport: unavailable in zed_extension_api 0.7.0; no TCP or websocket API is exposed"
    )
}

zed::register_extension!(BitburnerExtension);
