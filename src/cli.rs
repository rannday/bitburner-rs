use std::io::{self, Write};

use anyhow::Context;

use crate::args::{self, ReplCommand, SyncOptions, TopLevelCommand};
use crate::error::AppResult;
use crate::fs_sync::{self, SyncItem};
use crate::remote::{DEFAULT_SERVER, RemoteClient};

pub fn run() -> AppResult<()> {
    let cli = args::parse_env();

    match cli.command {
        TopLevelCommand::Serve { address } => crate::ws::serve(&address),
    }
}

pub fn execute_with_client(command: ReplCommand, remote: &mut RemoteClient) -> AppResult<()> {
    match command {
        ReplCommand::Help => {
            print_repl_help();
            Ok(())
        }
        ReplCommand::Servers => {
            let servers = remote.get_all_servers()?;
            println!("{}", serde_json::to_string_pretty(&servers)?);
            Ok(())
        }
        ReplCommand::Files { server } => {
            for file in remote.get_file_names(&server)? {
                println!("{file}");
            }
            Ok(())
        }
        ReplCommand::Get {
            server,
            filename,
            local_path,
        } => {
            let content = remote.get_file(&server, &filename)?;
            if let Some(path) = local_path {
                std::fs::write(&path, content)
                    .with_context(|| format!("write file '{}'", path.display()))?;
            } else {
                print!("{content}");
                io::stdout().flush().context("flush stdout")?;
            }
            Ok(())
        }
        ReplCommand::Push {
            server,
            remote_filename,
            local_path,
        } => {
            let content = std::fs::read_to_string(&local_path)
                .with_context(|| format!("read file '{}'", local_path.display()))?;
            remote.push_file(&server, &remote_filename, &content)?;
            println!("OK {remote_filename}");
            Ok(())
        }
        ReplCommand::Delete { server, filename } => {
            remote.delete_file(&server, &filename)?;
            println!("OK {filename}");
            Ok(())
        }
        ReplCommand::Metadata { server, filename } => {
            let metadata = remote.get_file_metadata(&server, &filename)?;
            println!("{}", serde_json::to_string_pretty(&metadata)?);
            Ok(())
        }
        ReplCommand::AllFiles { values } => {
            let (server, local_path) = all_files_values(values)?;
            let files = remote.get_all_files(&server)?;
            std::fs::write(&local_path, serde_json::to_string_pretty(&files)?)
                .with_context(|| format!("write file '{}'", local_path.display()))?;
            Ok(())
        }
        ReplCommand::AllMetadata { server } => {
            let metadata = remote.get_all_file_metadata(&server)?;
            println!("{}", serde_json::to_string_pretty(&metadata)?);
            Ok(())
        }
        ReplCommand::Ram { server, filename } => {
            println!("{}", remote.calculate_ram(&server, &filename)?);
            Ok(())
        }
        ReplCommand::Defs { local_path } => {
            let content = remote.get_definition_file()?;
            if let Some(path) = local_path {
                std::fs::write(&path, content)
                    .with_context(|| format!("write file '{}'", path.display()))?;
            } else {
                print!("{content}");
                io::stdout().flush().context("flush stdout")?;
            }
            Ok(())
        }
        ReplCommand::Save { local_path } => {
            let save = remote.get_save_file()?;
            std::fs::write(&local_path, serde_json::to_string_pretty(&save)?)
                .with_context(|| format!("write file '{}'", local_path.display()))?;
            Ok(())
        }
        ReplCommand::Sync(options) => execute_sync(options, remote),
    }
}

fn execute_sync(options: SyncOptions, remote: &mut RemoteClient) -> AppResult<()> {
    let plan = build_and_print_sync_plan(&options)?;
    if !should_upload_sync(&plan, options.dry_run) {
        print_dry_or_empty_sync_plan(plan, &options);
        return Ok(());
    }

    upload_sync_plan(plan, &options, remote)
}

fn build_and_print_sync_plan(options: &SyncOptions) -> AppResult<Vec<SyncItem>> {
    let plan = fs_sync::build_sync_plan(&options.local_dir, options.remote_dir.as_deref())?;
    print_sync_summary(
        plan.len(),
        &options.local_dir,
        &options.server,
        options.remote_dir.as_deref(),
    );
    Ok(plan)
}

fn print_dry_or_empty_sync_plan(plan: Vec<SyncItem>, options: &SyncOptions) {
    if plan.is_empty() {
        println!("No uploadable .js files found.");
    } else {
        for item in plan {
            println!(
                "{} -> {}:{}",
                item.local_path.display(),
                options.server,
                item.remote_path
            );
        }
    }
}

fn upload_sync_plan(
    plan: Vec<SyncItem>,
    options: &SyncOptions,
    remote: &mut RemoteClient,
) -> AppResult<()> {
    let synced = plan.len();
    for item in plan {
        let content = std::fs::read_to_string(&item.local_path)
            .with_context(|| format!("read file '{}'", item.local_path.display()))?;
        remote.push_file(&options.server, &item.remote_path, &content)?;
        println!(
            "OK {} -> {}:{}",
            item.local_path.display(),
            options.server,
            item.remote_path
        );
    }
    println!("Synced {synced} file(s).");
    Ok(())
}

fn print_sync_summary(
    file_count: usize,
    local_root: &std::path::Path,
    server: &str,
    remote_dir: Option<&str>,
) {
    println!("Planned files: {file_count}");
    println!("Local root: {}", local_root.display());
    println!("Remote server: {server}");
    if let Some(remote_dir) = remote_dir {
        println!("Remote dir: {remote_dir}");
    }
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

pub fn print_repl_help() {
    println!(
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
  sync <server> <local-dir> [remote-dir] [--dry-run]"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_sync_plan_does_not_upload() {
        assert!(!should_upload_sync(&[], false));
    }
}
