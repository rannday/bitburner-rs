use std::io::{self, Write};

use anyhow::Context;
use bitburner_api::{BitburnerApi, DEFAULT_SERVER};

use crate::AppResult;
use crate::args::{self, ReplCommand, SyncOptions, TopLevelCommand};
use crate::fs_sync::{self, SyncItem};
use crate::path::normalize_remote_file_path;

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
        TopLevelCommand::Serve { address } => crate::ws::serve(&address),
    }
}

pub fn execute_with_client<A>(command: ReplCommand, remote: &mut A) -> AppResult<CommandOutput>
where
    A: BitburnerApi + ?Sized,
{
    match command {
        ReplCommand::Help => Ok(CommandOutput::Text(repl_help_text())),
        ReplCommand::Servers => {
            let servers = remote.get_all_servers()?;
            Ok(CommandOutput::Text(format!(
                "{}\n",
                serde_json::to_string_pretty(&servers)?
            )))
        }
        ReplCommand::Files { server } => Ok(CommandOutput::Lines(remote.get_file_names(&server)?)),
        ReplCommand::Get {
            server,
            filename,
            local_path,
        } => {
            let filename = normalize_remote_file_arg(&filename)?;
            let content = remote.get_file(&server, &filename)?;
            if let Some(path) = local_path {
                write_text_file(&path, content)?;
                Ok(CommandOutput::Lines(vec![format!(
                    "Wrote {}",
                    path.display()
                )]))
            } else {
                Ok(CommandOutput::Text(content))
            }
        }
        ReplCommand::Push {
            server,
            remote_filename,
            local_path,
        } => {
            let remote_filename = normalize_remote_file_arg(&remote_filename)?;
            let content = std::fs::read_to_string(&local_path)
                .with_context(|| format!("read local file '{}'", local_path.display()))?;
            remote.push_file(&server, &remote_filename, &content)?;
            Ok(CommandOutput::Lines(vec![format!("OK {remote_filename}")]))
        }
        ReplCommand::Delete { server, filename } => {
            let filename = normalize_remote_file_arg(&filename)?;
            remote.delete_file(&server, &filename)?;
            Ok(CommandOutput::Lines(vec![format!("OK {filename}")]))
        }
        ReplCommand::Metadata { server, filename } => {
            let filename = normalize_remote_file_arg(&filename)?;
            let metadata = remote.get_file_metadata(&server, &filename)?;
            Ok(CommandOutput::Text(format!(
                "{}\n",
                serde_json::to_string_pretty(&metadata)?
            )))
        }
        ReplCommand::AllFiles { values } => {
            let (server, local_path) = all_files_values(values)?;
            let files = remote.get_all_files(&server)?;
            write_text_file(&local_path, serde_json::to_string_pretty(&files)?)?;
            Ok(CommandOutput::Lines(vec![format!(
                "Wrote {}",
                local_path.display()
            )]))
        }
        ReplCommand::AllMetadata { server } => {
            let metadata = remote.get_all_file_metadata(&server)?;
            Ok(CommandOutput::Text(format!(
                "{}\n",
                serde_json::to_string_pretty(&metadata)?
            )))
        }
        ReplCommand::Ram { server, filename } => Ok(CommandOutput::Lines(vec![
            remote
                .calculate_ram(&server, &normalize_remote_file_arg(&filename)?)?
                .to_string(),
        ])),
        ReplCommand::Defs { local_path } => {
            let content = remote.get_definition_file()?;
            if let Some(path) = local_path {
                write_text_file(&path, content)?;
                Ok(CommandOutput::Lines(vec![format!(
                    "Wrote {}",
                    path.display()
                )]))
            } else {
                Ok(CommandOutput::Text(content))
            }
        }
        ReplCommand::Save { local_path } => {
            let save = remote.get_save_file()?;
            write_text_file(&local_path, serde_json::to_string_pretty(&save)?)?;
            Ok(CommandOutput::Lines(vec![format!(
                "Wrote {}",
                local_path.display()
            )]))
        }
        ReplCommand::Sync(options) => execute_sync(options, remote),
    }
}

fn execute_sync<A>(options: SyncOptions, remote: &mut A) -> AppResult<CommandOutput>
where
    A: BitburnerApi + ?Sized,
{
    let plan = fs_sync::build_sync_plan(&options.local_dir, options.remote_dir.as_deref())?;
    let mut lines = sync_summary_lines(
        plan.len(),
        &options.local_dir,
        &options.server,
        options.remote_dir.as_deref(),
    );

    if !should_upload_sync(&plan, options.dry_run) {
        append_dry_or_empty_sync_plan(&mut lines, plan, &options);
        return Ok(CommandOutput::Lines(lines));
    }

    upload_sync_plan(&mut lines, plan, &options, remote)?;
    Ok(CommandOutput::Lines(lines))
}

fn append_dry_or_empty_sync_plan(
    lines: &mut Vec<String>,
    plan: Vec<SyncItem>,
    options: &SyncOptions,
) {
    if plan.is_empty() {
        lines.push("No uploadable .js files found.".to_string());
    } else {
        lines.extend(plan.into_iter().map(|item| {
            format!(
                "{} -> {}:{}",
                item.local_path.display(),
                options.server,
                item.remote_path
            )
        }));
    }
}

fn upload_sync_plan<A>(
    lines: &mut Vec<String>,
    plan: Vec<SyncItem>,
    options: &SyncOptions,
    remote: &mut A,
) -> AppResult<()>
where
    A: BitburnerApi + ?Sized,
{
    let synced = plan.len();
    for item in plan {
        let content = std::fs::read_to_string(&item.local_path)
            .with_context(|| format!("read local file '{}'", item.local_path.display()))?;
        remote.push_file(&options.server, &item.remote_path, &content)?;
        lines.push(format!(
            "OK {} -> {}:{}",
            item.local_path.display(),
            options.server,
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

fn all_files_values(values: Vec<String>) -> AppResult<(String, std::path::PathBuf)> {
    match values.as_slice() {
        [local_path] => Ok((DEFAULT_SERVER.to_string(), local_path.into())),
        [server, local_path] => Ok((server.clone(), local_path.into())),
        _ => anyhow::bail!("usage: all-files [server] <local-path>"),
    }
}

fn normalize_remote_file_arg(path: &str) -> AppResult<String> {
    normalize_remote_file_path(path).with_context(|| format!("invalid remote path '{path}'"))
}

fn write_text_file(path: &std::path::Path, content: String) -> AppResult<()> {
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
REPL commands:
  help
  quit | exit
  servers
  files [server]
  get <server> <filename> [local-path]
  push <server> <remote-filename> <local-path>
  delete <server> <filename>
  metadata <server> <filename>
  all-files [server] <local-path>
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
    use bitburner_api::{BitburnerFile, FileMetadata, SaveFile, ServerInfo};
    use std::path::PathBuf;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[derive(Default)]
    struct FakeApi {
        get_file_calls: Vec<(String, String)>,
        push_file_calls: Vec<(String, String, String)>,
    }

    impl BitburnerApi for FakeApi {
        fn push_file(&mut self, server: &str, filename: &str, content: &str) -> anyhow::Result<()> {
            self.push_file_calls.push((
                server.to_string(),
                filename.to_string(),
                content.to_string(),
            ));
            Ok(())
        }

        fn get_file(&mut self, server: &str, filename: &str) -> anyhow::Result<String> {
            self.get_file_calls
                .push((server.to_string(), filename.to_string()));
            Ok("content".to_string())
        }

        fn get_file_metadata(
            &mut self,
            _server: &str,
            _filename: &str,
        ) -> anyhow::Result<FileMetadata> {
            anyhow::bail!("unexpected get_file_metadata call")
        }

        fn delete_file(&mut self, _server: &str, _filename: &str) -> anyhow::Result<()> {
            anyhow::bail!("unexpected delete_file call")
        }

        fn get_file_names(&mut self, _server: &str) -> anyhow::Result<Vec<String>> {
            anyhow::bail!("unexpected get_file_names call")
        }

        fn get_all_files(&mut self, _server: &str) -> anyhow::Result<Vec<BitburnerFile>> {
            anyhow::bail!("unexpected get_all_files call")
        }

        fn get_all_file_metadata(&mut self, _server: &str) -> anyhow::Result<Vec<FileMetadata>> {
            anyhow::bail!("unexpected get_all_file_metadata call")
        }

        fn calculate_ram(&mut self, _server: &str, _filename: &str) -> anyhow::Result<f64> {
            anyhow::bail!("unexpected calculate_ram call")
        }

        fn get_definition_file(&mut self) -> anyhow::Result<String> {
            anyhow::bail!("unexpected get_definition_file call")
        }

        fn get_save_file(&mut self) -> anyhow::Result<SaveFile> {
            anyhow::bail!("unexpected get_save_file call")
        }

        fn get_all_servers(&mut self) -> anyhow::Result<Vec<ServerInfo>> {
            anyhow::bail!("unexpected get_all_servers call")
        }
    }

    #[test]
    fn empty_sync_plan_does_not_upload() {
        assert!(!should_upload_sync(&[], false));
    }

    #[test]
    fn help_is_returned_as_output() {
        assert!(repl_help_text().contains("sync <server> <local-dir>"));
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
