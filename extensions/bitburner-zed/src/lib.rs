use zed_extension_api as zed;

const BRIDGE_HEALTH_URL: &str = "http://127.0.0.1:12526/health";
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
    let bridge_status = match bridge_health() {
        Ok(connected) => format!(
            "Bitburner bridge: running\n\
             Bitburner connected: {}",
            if connected { "yes" } else { "no" }
        ),
        Err(_) => "Bitburner bridge: unavailable\nStart: bbrs serve".to_string(),
    };

    format!(
        "{bridge_status}\n\
         bridge: {BRIDGE_HEALTH_URL}\n\
         server: {DEFAULT_SERVER}\n\
         remoteDir: {DEFAULT_REMOTE_DIR}\n\
         uploadable extensions: {extensions}\n\
         worktree: {worktree_root}\n\
         sync planner: not wired in the extension yet\n\
         remote transport: use local HTTP bridge; no direct TCP or websocket API is exposed"
    )
}

fn bridge_health() -> Result<bool, String> {
    let request = zed::http_client::HttpRequest::builder()
        .method(zed::http_client::HttpMethod::Get)
        .url(BRIDGE_HEALTH_URL)
        .build()?;
    let response = request.fetch()?;
    let value: zed::serde_json::Value = zed::serde_json::from_slice(&response.body)
        .map_err(|err| format!("invalid bridge health response: {err}"))?;

    Ok(value
        .get("bitburner_connected")
        .and_then(zed::serde_json::Value::as_bool)
        .unwrap_or(false))
}

zed::register_extension!(BitburnerExtension);
