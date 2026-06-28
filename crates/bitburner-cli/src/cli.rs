use std::io::{self, Write};
use std::path::{Path, PathBuf};

use anyhow::Context;
use bitburner_api::{BitburnerApi, SyncItem, default_server_name, normalize_remote_file_path};

use crate::AppResult;
use crate::args::{self, ReplCommand, SyncOptions, TopLevelCommand};
use crate::fs_sync;
use crate::sync_upload::upload_sync_items;

#[derive(Debug, Clone, PartialEq)]
pub enum CommandOutput {
    Text(String),
    Lines(Vec<String>),
}

impl CommandOutput {
    pub fn print(&self) -> AppResult<()> {
        match self {
            CommandOutput::Text(text) => {
                print!("{text}");
                io::stdout().flush().context("flush stdout")
            }
            CommandOutput::Lines(lines) => {
                for line in lines {
                    println!("{line}");
                }
                Ok(())
            }
        }
    }
}

pub fn run() -> AppResult<()> {
    let cli = args::parse_env();

    match cli.command {
        TopLevelCommand::Serve {
            address,
            http_address,
        } => crate::ws::serve(&address, &http_address),
    }
}

pub fn execute_with_client<A>(command: ReplCommand, remote: &mut A) -> AppResult<CommandOutput>
where
    A: BitburnerApi + ?Sized,
{
    match command {
        ReplCommand::Help => Ok(CommandOutput::Text(repl_help_text())),
        ReplCommand::Servers => cmd_servers(remote),
        ReplCommand::Files { server } => cmd_files(remote, server),
        ReplCommand::Get {
            server,
            filename,
            local_path,
        } => cmd_get(remote, server, filename, local_path),
        ReplCommand::Push {
            server,
            remote_filename,
            local_path,
        } => cmd_push(remote, server, remote_filename, local_path),
        ReplCommand::Delete { server, filename } => cmd_delete(remote, server, filename),
        ReplCommand::Metadata { server, filename } => cmd_metadata(remote, server, filename),
        ReplCommand::AllFiles { server, local_path } => cmd_all_files(remote, server, local_path),
        ReplCommand::AllMetadata { server } => cmd_all_metadata(remote, server),
        ReplCommand::Ram { server, filename } => cmd_ram(remote, server, filename),
        ReplCommand::Defs { local_path } => cmd_defs(remote, local_path),
        ReplCommand::Save { local_path } => cmd_save(remote, local_path),
        ReplCommand::Sync(options) => execute_sync(options, remote),
    }
}

fn cmd_servers<A>(remote: &mut A) -> AppResult<CommandOutput>
where
    A: BitburnerApi + ?Sized,
{
    pretty_json_output(&remote.get_all_servers()?)
}

fn cmd_files<A>(remote: &mut A, server: String) -> AppResult<CommandOutput>
where
    A: BitburnerApi + ?Sized,
{
    Ok(CommandOutput::Lines(
        remote.get_file_names(default_server_name(Some(&server)))?,
    ))
}

fn cmd_get<A>(
    remote: &mut A,
    server: String,
    filename: String,
    local_path: Option<PathBuf>,
) -> AppResult<CommandOutput>
where
    A: BitburnerApi + ?Sized,
{
    let filename = normalize_remote_file_arg(&filename)?;
    let content = remote.get_file(default_server_name(Some(&server)), &filename)?;
    content_or_write(local_path, content)
}

fn cmd_push<A>(
    remote: &mut A,
    server: String,
    remote_filename: String,
    local_path: PathBuf,
) -> AppResult<CommandOutput>
where
    A: BitburnerApi + ?Sized,
{
    let remote_filename = normalize_remote_file_arg(&remote_filename)?;
    let content = std::fs::read_to_string(&local_path)
        .with_context(|| format!("read local file '{}'", local_path.display()))?;
    remote.push_file(
        default_server_name(Some(&server)),
        &remote_filename,
        &content,
    )?;
    Ok(ok_file(&remote_filename))
}

fn cmd_delete<A>(remote: &mut A, server: String, filename: String) -> AppResult<CommandOutput>
where
    A: BitburnerApi + ?Sized,
{
    let filename = normalize_remote_file_arg(&filename)?;
    remote.delete_file(default_server_name(Some(&server)), &filename)?;
    Ok(ok_file(&filename))
}

fn cmd_metadata<A>(remote: &mut A, server: String, filename: String) -> AppResult<CommandOutput>
where
    A: BitburnerApi + ?Sized,
{
    let filename = normalize_remote_file_arg(&filename)?;
    pretty_json_output(&remote.get_file_metadata(default_server_name(Some(&server)), &filename)?)
}

fn cmd_all_files<A>(remote: &mut A, server: String, local_path: PathBuf) -> AppResult<CommandOutput>
where
    A: BitburnerApi + ?Sized,
{
    let files = remote.get_all_files(default_server_name(Some(&server)))?;
    write_text_file(&local_path, serde_json::to_string_pretty(&files)?)?;
    Ok(wrote(&local_path))
}

fn cmd_all_metadata<A>(remote: &mut A, server: String) -> AppResult<CommandOutput>
where
    A: BitburnerApi + ?Sized,
{
    pretty_json_output(&remote.get_all_file_metadata(default_server_name(Some(&server)))?)
}

fn cmd_ram<A>(remote: &mut A, server: String, filename: String) -> AppResult<CommandOutput>
where
    A: BitburnerApi + ?Sized,
{
    Ok(CommandOutput::Lines(vec![
        remote
            .calculate_ram(
                default_server_name(Some(&server)),
                &normalize_remote_file_arg(&filename)?,
            )?
            .to_string(),
    ]))
}

fn cmd_defs<A>(remote: &mut A, local_path: Option<PathBuf>) -> AppResult<CommandOutput>
where
    A: BitburnerApi + ?Sized,
{
    content_or_write(local_path, remote.get_definition_file()?)
}

fn cmd_save<A>(remote: &mut A, local_path: PathBuf) -> AppResult<CommandOutput>
where
    A: BitburnerApi + ?Sized,
{
    write_text_file(
        &local_path,
        serde_json::to_string_pretty(&remote.get_save_file()?)?,
    )?;
    Ok(wrote(&local_path))
}

fn execute_sync<A>(options: SyncOptions, remote: &mut A) -> AppResult<CommandOutput>
where
    A: BitburnerApi + ?Sized,
{
    let plan = fs_sync::build_sync_plan(&options.local_dir, options.remote_dir.as_deref())?;
    let server = default_server_name(Some(&options.server)).to_string();
    let mut lines = sync_summary_lines(
        plan.len(),
        &options.local_dir,
        &server,
        options.remote_dir.as_deref(),
    );

    if !should_upload_sync(&plan, options.dry_run) {
        append_dry_or_empty_sync_plan(&mut lines, plan, &options);
        return Ok(CommandOutput::Lines(lines));
    }

    upload_sync_plan(&mut lines, plan, &server, remote)?;
    Ok(CommandOutput::Lines(lines))
}

fn append_dry_or_empty_sync_plan(
    lines: &mut Vec<String>,
    plan: Vec<SyncItem>,
    options: &SyncOptions,
) {
    if plan.is_empty() {
        lines.push("No uploadable files found.".to_string());
    } else {
        lines.extend(plan.into_iter().map(|item| {
            format!(
                "{} -> {}:{}",
                item.source_path.display(),
                default_server_name(Some(&options.server)),
                item.remote_path
            )
        }));
    }
}

fn upload_sync_plan<A>(
    lines: &mut Vec<String>,
    plan: Vec<SyncItem>,
    server: &str,
    remote: &mut A,
) -> AppResult<()>
where
    A: BitburnerApi + ?Sized,
{
    let synced = plan.len();
    let plan_for_output = plan.clone();
    upload_sync_items(remote, server, plan, |item| {
        std::fs::read_to_string(&item.source_path)
            .with_context(|| format!("read local file '{}'", item.source_path.display()))
    })
    .map_err(|err| err.into_anyhow())?;
    for item in plan_for_output {
        lines.push(format!(
            "OK {} -> {}:{}",
            item.source_path.display(),
            server,
            item.remote_path
        ));
    }
    lines.push(format!("Synced {synced} file(s)."));
    Ok(())
}

fn sync_summary_lines(
    file_count: usize,
    local_root: &std::path::Path,
    server: &str,
    remote_dir: Option<&str>,
) -> Vec<String> {
    let mut lines = vec![
        format!("Planned files: {file_count}"),
        format!("Local root: {}", local_root.display()),
        format!("Remote server: {server}"),
    ];
    if let Some(remote_dir) = remote_dir {
        lines.push(format!("Remote dir: {remote_dir}"));
    }
    lines
}

fn should_upload_sync(plan: &[SyncItem], dry_run: bool) -> bool {
    !dry_run && !plan.is_empty()
}

fn normalize_remote_file_arg(path: &str) -> AppResult<String> {
    normalize_remote_file_path(path).with_context(|| format!("invalid remote path '{path}'"))
}

fn content_or_write(local_path: Option<PathBuf>, content: String) -> AppResult<CommandOutput> {
    if let Some(path) = local_path {
        write_text_file(&path, content)?;
        Ok(wrote(&path))
    } else {
        Ok(CommandOutput::Text(content))
    }
}

fn wrote(path: &Path) -> CommandOutput {
    CommandOutput::Lines(vec![format!("Wrote {}", path.display())])
}

fn ok_file(path: &str) -> CommandOutput {
    CommandOutput::Lines(vec![format!("OK {path}")])
}

fn pretty_json_output<T: serde::Serialize>(value: &T) -> AppResult<CommandOutput> {
    Ok(CommandOutput::Text(format!(
        "{}\n",
        serde_json::to_string_pretty(value)?
    )))
}

fn write_text_file(path: &Path, content: String) -> AppResult<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("create local directory '{}'", parent.display()))?;
    }
    std::fs::write(path, content).with_context(|| format!("write local file '{}'", path.display()))
}

pub fn repl_help_text() -> String {
    "\
Usage:
  help
  quit | exit
  servers
  files [server]
  get <server> <filename> [local-path]
  push <server> <remote-filename> <local-path>
  delete <server> <filename>
  metadata <server> <filename>
  all-files [--server <server>] <local-path>
  all-metadata [server]
  ram <server> <filename>
  defs [local-path]
  save <local-path>
  sync <server> <local-dir> [remote-dir] [--dry-run]\n"
        .to_string()
}

pub fn print_repl_help() {
    print!("{}", repl_help_text());
}

#[cfg(test)]
mod tests {
    use super::*;
    use bitburner_api::{
        BitburnerError, BitburnerFile, FileMetadata, Result, SaveFile, ServerInfo,
    };
    use serde_json::Value;
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Default)]
    struct FakeApi {
        get_file_calls: Vec<(String, String)>,
        push_file_calls: Vec<(String, String, String)>,
    }

    impl BitburnerApi for FakeApi {
        fn request_value(&mut self, _method: &str, _params: Option<Value>) -> Result<Value> {
            unexpected("request_value")
        }

        fn push_file(&mut self, server: &str, filename: &str, content: &str) -> Result<()> {
            self.push_file_calls.push((
                server.to_string(),
                filename.to_string(),
                content.to_string(),
            ));
            Ok(())
        }

        fn get_file(&mut self, server: &str, filename: &str) -> Result<String> {
            self.get_file_calls
                .push((server.to_string(), filename.to_string()));
            Ok("content".to_string())
        }

        fn get_file_metadata(&mut self, _server: &str, _filename: &str) -> Result<FileMetadata> {
            unexpected("get_file_metadata")
        }

        fn delete_file(&mut self, _server: &str, _filename: &str) -> Result<()> {
            unexpected("delete_file")
        }

        fn get_file_names(&mut self, _server: &str) -> Result<Vec<String>> {
            unexpected("get_file_names")
        }

        fn get_all_files(&mut self, _server: &str) -> Result<Vec<BitburnerFile>> {
            unexpected("get_all_files")
        }

        fn get_all_file_metadata(&mut self, _server: &str) -> Result<Vec<FileMetadata>> {
            unexpected("get_all_file_metadata")
        }

        fn calculate_ram(&mut self, _server: &str, _filename: &str) -> Result<f64> {
            unexpected("calculate_ram")
        }

        fn get_definition_file(&mut self) -> Result<String> {
            unexpected("get_definition_file")
        }

        fn get_save_file(&mut self) -> Result<SaveFile> {
            unexpected("get_save_file")
        }

        fn get_all_servers(&mut self) -> Result<Vec<ServerInfo>> {
            unexpected("get_all_servers")
        }
    }

    fn unexpected<T>(method: &str) -> Result<T> {
        Err(BitburnerError::invalid_protocol(format!(
            "unexpected {method} call"
        )))
    }

    #[test]
    fn empty_sync_plan_does_not_upload() {
        assert!(!should_upload_sync(&[], false));
    }

    #[test]
    fn help_is_returned_as_output() {
        let help = repl_help_text();

        assert!(help.contains("Usage:"));
        assert!(help.contains("  help"));
        assert!(help.contains("  servers"));
        assert!(help.contains("  files [server]"));
        assert!(help.contains("  sync <server> <local-dir> [remote-dir] [--dry-run]"));
        assert!(!help.contains("REPL commands:"));
    }

    #[test]
    fn write_text_file_creates_parent_directories() {
        let root = temp_root("bbrs-cli-write");
        let path = root.join("nested").join("out.txt");

        write_text_file(&path, "content".to_string()).expect("write");

        assert_eq!(std::fs::read_to_string(&path).expect("read"), "content");

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn get_normalizes_remote_backslashes() {
        let mut remote = FakeApi::default();

        let output = execute_with_client(
            ReplCommand::Get {
                server: "home".to_string(),
                filename: r"contracts\spiral-matrix.js".to_string(),
                local_path: None,
            },
            &mut remote,
        )
        .expect("get");

        assert_eq!(output, CommandOutput::Text("content".to_string()));
        assert_eq!(
            remote.get_file_calls,
            vec![("home".to_string(), "contracts/spiral-matrix.js".to_string())]
        );
    }

    #[test]
    fn push_normalizes_remote_backslashes() {
        let root = temp_root("bbrs-cli-push");
        std::fs::create_dir_all(&root).expect("mkdir root");
        let local_path = root.join("spiral-matrix.js");
        std::fs::write(&local_path, "export async function main() {}").expect("write");
        let mut remote = FakeApi::default();

        let output = execute_with_client(
            ReplCommand::Push {
                server: "home".to_string(),
                remote_filename: r"contracts\spiral-matrix.js".to_string(),
                local_path,
            },
            &mut remote,
        )
        .expect("push");

        assert_eq!(
            output,
            CommandOutput::Lines(vec!["OK contracts/spiral-matrix.js".to_string()])
        );
        assert_eq!(remote.push_file_calls.len(), 1);
        assert_eq!(remote.push_file_calls[0].1, "contracts/spiral-matrix.js");

        std::fs::remove_dir_all(root).expect("cleanup");
    }

    #[test]
    fn rejects_invalid_remote_file_path_before_api_call() {
        let mut remote = FakeApi::default();

        let err = execute_with_client(
            ReplCommand::Get {
                server: "home".to_string(),
                filename: "scripts/../foo.js".to_string(),
                local_path: None,
            },
            &mut remote,
        )
        .expect_err("error");

        assert!(err.to_string().contains("invalid remote path"));
        assert!(remote.get_file_calls.is_empty());
    }

    fn temp_root(name: &str) -> PathBuf {
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("time")
            .as_nanos();
        std::env::temp_dir().join(format!("{name}-{stamp}"))
    }
}
