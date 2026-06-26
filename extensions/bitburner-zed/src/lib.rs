use zed_extension_api as zed;

const BRIDGE_URL: &str = "http://127.0.0.1:12526";
const DEFAULT_SERVER: &str = "home";
const DEFAULT_REMOTE_DIR: &str = "scripts";
const UPLOADABLE_EXTENSIONS: [&str; 5] = [".js", ".ts", ".txt", ".script", ".ns"];
const HELP: &str = "Usage:\n\
                   /bitburner\n\
                   /bitburner status\n\
                   /bitburner push <worktree-path> [remote-path]\n\
                   \n\
                   Defaults:\n\
                   bridge URL: http://127.0.0.1:12526\n\
                   server: home\n\
                   remote dir: scripts";

struct BitburnerExtension;

impl zed::Extension for BitburnerExtension {
    fn new() -> Self {
        Self
    }

    fn run_slash_command(
        &self,
        command: zed::SlashCommand,
        args: Vec<String>,
        worktree: Option<&zed::Worktree>,
    ) -> Result<zed::SlashCommandOutput, String> {
        if command.name != "bitburner" {
            return Err(format!(
                "unsupported Bitburner slash command '{}'",
                command.name
            ));
        }

        let text = match args.first().map(String::as_str) {
            None | Some("") | Some("status") => extension_status(worktree),
            Some("help") => HELP.to_string(),
            Some("push") => push_worktree_file(&args[1..], worktree)?,
            Some(other) => {
                return Err(format!(
                    "unsupported /bitburner subcommand '{other}'. Try /bitburner help"
                ));
            }
        };

        Ok(zed::SlashCommandOutput {
            text,
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
         bridge URL: {BRIDGE_URL}\n\
         server: {DEFAULT_SERVER}\n\
         remoteDir: {DEFAULT_REMOTE_DIR}\n\
         uploadable extensions: {extensions}\n\
         worktree: {worktree_root}\n\
         push: /bitburner push <worktree-path> [remote-path]\n\
         current-file push: not exposed by this Zed extension API\n\
         remote transport: use local HTTP bridge; no direct TCP or websocket API is exposed"
    )
}

fn bridge_health() -> Result<bool, String> {
    let request = zed::http_client::HttpRequest::builder()
        .method(zed::http_client::HttpMethod::Get)
        .url(format!("{BRIDGE_URL}/health"))
        .build()?;
    let response = request.fetch()?;
    let value: zed::serde_json::Value = zed::serde_json::from_slice(&response.body)
        .map_err(|err| format!("invalid bridge health response: {err}"))?;

    Ok(value
        .get("bitburner_connected")
        .and_then(zed::serde_json::Value::as_bool)
        .unwrap_or(false))
}

fn push_worktree_file(args: &[String], worktree: Option<&zed::Worktree>) -> Result<String, String> {
    let worktree = worktree.ok_or_else(|| "no worktree is available".to_string())?;
    let local_path = args
        .first()
        .ok_or_else(|| "usage: /bitburner push <worktree-path> [remote-path]".to_string())?;
    let content = worktree
        .read_text_file(local_path)
        .map_err(|err| format!("read {local_path}: {err}"))?;
    let remote_path = args.get(1).cloned().unwrap_or_else(|| {
        let trimmed = local_path.trim_start_matches(['/', '\\']);
        format!("{DEFAULT_REMOTE_DIR}/{}", trimmed.replace('\\', "/"))
    });

    let body = zed::serde_json::json!({
        "server": DEFAULT_SERVER,
        "filename": remote_path,
        "content": content,
    });
    let request = zed::http_client::HttpRequest::builder()
        .method(zed::http_client::HttpMethod::Post)
        .url(format!("{BRIDGE_URL}/push"))
        .header("Content-Type", "application/json")
        .body(body.to_string().into_bytes())
        .build()?;
    let response = request.fetch()?;
    let value: zed::serde_json::Value = zed::serde_json::from_slice(&response.body)
        .map_err(|err| format!("invalid bridge push response: {err}"))?;
    if let Some(error) = value.get("error").and_then(zed::serde_json::Value::as_str) {
        return Err(format!("bridge push failed: {error}"));
    }
    if value
        .get("ok")
        .and_then(zed::serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        Ok(format!(
            "Pushed {local_path} to {DEFAULT_SERVER}:{remote_path} through {BRIDGE_URL}"
        ))
    } else {
        Err(format!("unexpected bridge push response: {value}"))
    }
}

zed::register_extension!(BitburnerExtension);
