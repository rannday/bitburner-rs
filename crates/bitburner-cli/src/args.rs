use std::path::PathBuf;

use clap::{Parser, Subcommand};

use bitburner_api::{DEFAULT_ADDRESS, DEFAULT_SERVER};

use crate::http_bridge::DEFAULT_HTTP_ADDRESS;

#[derive(Debug, Parser)]
#[command(
    name = "bbrs",
    version,
    about = "Bitburner Remote API CLI",
    disable_help_subcommand = true
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: TopLevelCommand,
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum TopLevelCommand {
    /// Start the Remote API websocket server and command REPL.
    Serve {
        /// Local address for Bitburner Remote API to connect to.
        #[arg(long = "addr", default_value = DEFAULT_ADDRESS)]
        address: String,
        /// Local HTTP control API address for editor/tool integrations.
        #[arg(long = "http-addr", default_value = DEFAULT_HTTP_ADDRESS)]
        http_address: String,
    },
}

#[derive(Debug, Parser)]
#[command(
    name = "bbrs-repl",
    about = "Bitburner Remote API REPL commands",
    disable_help_subcommand = true
)]
pub struct ReplCli {
    #[command(subcommand)]
    pub command: ReplCommand,
}

#[derive(Debug, Clone, PartialEq, Eq, Subcommand)]
pub enum ReplCommand {
    /// Print REPL command help.
    Help,
    /// Print all known Bitburner servers as pretty JSON.
    Servers,
    /// List files on a Bitburner server.
    Files {
        #[arg(default_value = DEFAULT_SERVER)]
        server: String,
    },
    /// Read a remote file, optionally writing it locally.
    Get {
        server: String,
        filename: String,
        local_path: Option<PathBuf>,
    },
    /// Upload a local file to Bitburner.
    Push {
        server: String,
        remote_filename: String,
        local_path: PathBuf,
    },
    /// Delete a remote file.
    Delete { server: String, filename: String },
    /// Print remote file metadata as pretty JSON.
    Metadata { server: String, filename: String },
    /// Write all remote files as pretty JSON.
    AllFiles {
        #[arg(long, default_value = DEFAULT_SERVER)]
        server: String,
        local_path: PathBuf,
    },
    /// Print all remote file metadata as pretty JSON.
    AllMetadata {
        #[arg(default_value = DEFAULT_SERVER)]
        server: String,
    },
    /// Calculate script RAM usage.
    Ram { server: String, filename: String },
    /// Read Netscript definition file, optionally writing it locally.
    Defs { local_path: Option<PathBuf> },
    /// Write the Bitburner save file JSON.
    Save { local_path: PathBuf },
    /// Upload local Bitburner script/text files recursively.
    Sync(SyncOptions),
}

#[derive(Debug, Clone, PartialEq, Eq, Parser)]
pub struct SyncOptions {
    pub server: String,
    pub local_dir: PathBuf,
    pub remote_dir: Option<String>,
    /// Print the sync plan without uploading files.
    #[arg(long)]
    pub dry_run: bool,
}

pub fn parse_env() -> Cli {
    Cli::parse()
}

#[cfg(test)]
pub fn parse_from<I, T>(args: I) -> Result<Cli, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    Cli::try_parse_from(args)
}

pub fn parse_repl_from<I, T>(args: I) -> Result<ReplCli, clap::Error>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    ReplCli::try_parse_from(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn top_level_rejects_sync() {
        let err = parse_from(["bbrs", "sync", "home", "game_files", "scripts", "--dry-run"])
            .expect_err("error");

        assert_eq!(err.kind(), clap::error::ErrorKind::InvalidSubcommand);
    }

    #[test]
    fn repl_parses_sync_dry_run() {
        let cli = parse_repl_from(["bbrs", "sync", "home", "game_files", "scripts", "--dry-run"])
            .expect("parse");

        assert_eq!(
            cli.command,
            ReplCommand::Sync(SyncOptions {
                server: "home".to_string(),
                local_dir: PathBuf::from("game_files"),
                remote_dir: Some("scripts".to_string()),
                dry_run: true,
            })
        );
    }

    #[test]
    fn top_level_rejects_servers_command() {
        let err = parse_from(["bbrs", "servers"]).expect_err("error");

        assert_eq!(err.kind(), clap::error::ErrorKind::InvalidSubcommand);
    }

    #[test]
    fn repl_parses_servers_command() {
        let cli = parse_repl_from(["bbrs", "servers"]).expect("parse");

        assert_eq!(cli.command, ReplCommand::Servers);
    }

    #[test]
    fn repl_parses_all_files_without_server() {
        let cli = parse_repl_from(["bbrs", "all-files", "files.json"]).expect("parse");

        assert_eq!(
            cli.command,
            ReplCommand::AllFiles {
                server: "home".to_string(),
                local_path: PathBuf::from("files.json"),
            }
        );
    }

    #[test]
    fn repl_parses_all_files_with_server() {
        let cli = parse_repl_from(["bbrs", "all-files", "--server", "home", "files.json"])
            .expect("parse");

        assert_eq!(
            cli.command,
            ReplCommand::AllFiles {
                server: "home".to_string(),
                local_path: PathBuf::from("files.json"),
            }
        );
    }

    #[test]
    fn parses_serve_with_addr() {
        let cli = parse_from(["bbrs", "serve", "--addr", "127.0.0.1:12526"]).expect("parse");

        assert_eq!(
            cli.command,
            TopLevelCommand::Serve {
                address: "127.0.0.1:12526".to_string(),
                http_address: DEFAULT_HTTP_ADDRESS.to_string(),
            }
        );
    }

    #[test]
    fn parses_serve_with_http_addr() {
        let cli = parse_from(["bbrs", "serve", "--http-addr", "127.0.0.1:13000"]).expect("parse");

        assert_eq!(
            cli.command,
            TopLevelCommand::Serve {
                address: DEFAULT_ADDRESS.to_string(),
                http_address: "127.0.0.1:13000".to_string(),
            }
        );
    }
}
