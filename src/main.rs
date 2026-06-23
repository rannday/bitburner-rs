mod args;
mod error;
mod fs_sync;
mod path;
mod remote;
mod ws;

use args::Command;
use error::{AppError, AppResult};
use fs_sync::SyncItem;
use remote::{DEFAULT_ADDRESS, RemoteClient};

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> AppResult<()> {
    let command = args::parse_env()?;

    match command {
        Command::Help => {
            print_help();
            Ok(())
        }
        Command::Version => {
            println!("bbrs {}", env!("CARGO_PKG_VERSION"));
            Ok(())
        }
        Command::Serve { address } => ws::serve(&address),
        Command::Mcp => {
            println!("MCP support is planned.");
            println!(
                "Current recommended Zed integration is a documented task calling `bbrs sync`."
            );
            Ok(())
        }
        Command::Files { server } => {
            let mut remote = RemoteClient::listen(DEFAULT_ADDRESS)?;
            for file in remote.get_file_names(&server)? {
                println!("{file}");
            }
            Ok(())
        }
        Command::Get {
            server,
            filename,
            local_path,
        } => {
            let mut remote = RemoteClient::listen(DEFAULT_ADDRESS)?;
            let content = remote.get_file(&server, &filename)?;
            if let Some(path) = local_path {
                std::fs::write(path, content)?;
            } else {
                print!("{content}");
            }
            Ok(())
        }
        Command::Push {
            server,
            remote_filename,
            local_path,
        } => {
            let content = std::fs::read_to_string(local_path)?;
            let mut remote = RemoteClient::listen(DEFAULT_ADDRESS)?;
            remote.push_file(&server, &remote_filename, &content)?;
            println!("{remote_filename}");
            Ok(())
        }
        Command::Delete { server, filename } => {
            let mut remote = RemoteClient::listen(DEFAULT_ADDRESS)?;
            remote.delete_file(&server, &filename)?;
            println!("{filename}");
            Ok(())
        }
        Command::Metadata { server, filename } => {
            let mut remote = RemoteClient::listen(DEFAULT_ADDRESS)?;
            let metadata = remote.get_file_metadata(&server, &filename)?;
            println!("{}", serde_json::to_string_pretty(&metadata)?);
            Ok(())
        }
        Command::AllFiles { server, local_path } => {
            let mut remote = RemoteClient::listen(DEFAULT_ADDRESS)?;
            let files = remote.get_all_files(&server)?;
            std::fs::write(local_path, serde_json::to_string_pretty(&files)?)?;
            Ok(())
        }
        Command::AllMetadata { server } => {
            let mut remote = RemoteClient::listen(DEFAULT_ADDRESS)?;
            let metadata = remote.get_all_file_metadata(&server)?;
            println!("{}", serde_json::to_string_pretty(&metadata)?);
            Ok(())
        }
        Command::Ram { server, filename } => {
            let mut remote = RemoteClient::listen(DEFAULT_ADDRESS)?;
            println!("{}", remote.calculate_ram(&server, &filename)?);
            Ok(())
        }
        Command::Defs { local_path } => {
            let mut remote = RemoteClient::listen(DEFAULT_ADDRESS)?;
            let content = remote.get_definition_file()?;
            if let Some(path) = local_path {
                std::fs::write(path, content)?;
            } else {
                print!("{content}");
            }
            Ok(())
        }
        Command::Save { local_path } => {
            let mut remote = RemoteClient::listen(DEFAULT_ADDRESS)?;
            let save = remote.get_save_file()?;
            std::fs::write(local_path, serde_json::to_string_pretty(&save)?)?;
            Ok(())
        }
        Command::Sync(options) => {
            let plan = fs_sync::build_sync_plan(&options.local_dir, options.remote_dir.as_deref())?;
            print_sync_summary(
                plan.len(),
                &options.local_dir,
                &options.server,
                options.remote_dir.as_deref(),
            );

            if !should_listen_for_sync(&plan, options.dry_run) {
                if plan.is_empty() {
                    println!("No uploadable files found.");
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
                return Ok(());
            }

            if options.clean {
                return Err(AppError::NotImplemented(
                    "sync --clean is TODO: dry-run works, upload works without clean".to_string(),
                ));
            }
            let synced = plan.len();
            let mut remote = RemoteClient::listen(&options.address)?;
            for item in plan {
                let content = std::fs::read_to_string(&item.local_path)?;
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
        Command::Clean { server } => {
            let mut remote = RemoteClient::listen(DEFAULT_ADDRESS)?;
            remote.clean_server(&server)
        }
    }
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

fn should_listen_for_sync(plan: &[SyncItem], dry_run: bool) -> bool {
    !dry_run && !plan.is_empty()
}

fn print_help() {
    println!("bbrs - Bitburner Remote API sync tool");
    println!();
    println!("Commands:");
    println!("  help");
    println!("  version");
    println!("  serve [--addr <host:port>]");
    println!("  files [server]");
    println!("  get <server> <filename> [local-path]");
    println!("  push <server> <remote-filename> <local-path>");
    println!("  delete <server> <filename>");
    println!("  metadata <server> <filename>");
    println!("  all-files [server] <local-path>");
    println!("  all-metadata [server]");
    println!("  ram <server> <filename>");
    println!("  defs [local-path]");
    println!("  save <local-path>");
    println!(
        "  sync [local-dir] [remote-dir] [--server <server>] [--addr <host:port>] [--clean] [--dry-run]"
    );
    println!("  clean [server]");
    println!("  mcp");
    println!();
    println!("Sync uploads .js, .ts, .txt, .script, and .json files only.");
    println!(
        "Sync skips default generated/VCS/editor dirs: .git, target, node_modules, dist, build, .zed, .vscode, .idea, coverage, tmp, temp."
    );
    println!("Remote paths preserve paths relative to <local-dir>, then prefix [remote-dir].");
    println!("Windows backslashes become Bitburner forward slashes.");
    println!(
        "Sync and serve listen on 127.0.0.1:12525 by default; override with --addr <host:port>."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_sync_plan_does_not_listen() {
        assert!(!should_listen_for_sync(&[], false));
    }
}
