use std::env;
use std::path::PathBuf;

use crate::error::{AppError, AppResult};
use crate::remote::DEFAULT_ADDRESS;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command {
    Help,
    Version,
    Serve {
        address: String,
    },
    Files {
        server: String,
    },
    Get {
        server: String,
        filename: String,
        local_path: Option<PathBuf>,
    },
    Push {
        server: String,
        remote_filename: String,
        local_path: PathBuf,
    },
    Delete {
        server: String,
        filename: String,
    },
    Metadata {
        server: String,
        filename: String,
    },
    AllFiles {
        server: String,
        local_path: PathBuf,
    },
    AllMetadata {
        server: String,
    },
    Ram {
        server: String,
        filename: String,
    },
    Defs {
        local_path: Option<PathBuf>,
    },
    Save {
        local_path: PathBuf,
    },
    Sync(SyncOptions),
    Clean {
        server: String,
    },
    Mcp,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SyncOptions {
    pub server: String,
    pub address: String,
    pub local_dir: PathBuf,
    pub remote_dir: Option<String>,
    pub clean: bool,
    pub dry_run: bool,
}

pub fn parse_env() -> AppResult<Command> {
    parse(env::args().skip(1))
}

pub fn parse<I, S>(args: I) -> AppResult<Command>
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    let mut args: Vec<String> = args.into_iter().map(Into::into).collect();
    if args.is_empty() {
        return Ok(Command::Help);
    }

    let command = args.remove(0);
    match command.as_str() {
        "help" | "-h" | "--help" => Ok(Command::Help),
        "version" | "-V" | "--version" => Ok(Command::Version),
        "serve" => parse_serve(command, args),
        "mcp" => expect_no_args(&command, args).map(|_| Command::Mcp),
        "files" => parse_optional_server(command, args).map(|server| Command::Files { server }),
        "get" => parse_get(command, args),
        "push" => parse_push(command, args),
        "delete" => parse_two(command, args, "server", "filename")
            .map(|(server, filename)| Command::Delete { server, filename }),
        "metadata" => parse_two(command, args, "server", "filename")
            .map(|(server, filename)| Command::Metadata { server, filename }),
        "all-files" => parse_all_files(command, args),
        "all-metadata" => {
            parse_optional_server(command, args).map(|server| Command::AllMetadata { server })
        }
        "ram" => parse_two(command, args, "server", "filename")
            .map(|(server, filename)| Command::Ram { server, filename }),
        "defs" => parse_defs(command, args),
        "save" => parse_one(command, args, "local-path").map(|local_path| Command::Save {
            local_path: PathBuf::from(local_path),
        }),
        "sync" => parse_sync(command, args).map(Command::Sync),
        "clean" => parse_optional_server(command, args).map(|server| Command::Clean { server }),
        _ => Err(AppError::Usage(format!(
            "unknown command '{command}'; run 'bbrs help'"
        ))),
    }
}

fn parse_get(command: String, args: Vec<String>) -> AppResult<Command> {
    if args.len() != 2 && args.len() != 3 {
        return Err(usage(&command, "<server> <filename> [local-path]"));
    }
    Ok(Command::Get {
        server: args[0].clone(),
        filename: args[1].clone(),
        local_path: args.get(2).map(PathBuf::from),
    })
}

fn parse_push(command: String, args: Vec<String>) -> AppResult<Command> {
    if args.len() != 3 {
        return Err(usage(&command, "<server> <remote-filename> <local-path>"));
    }
    Ok(Command::Push {
        server: args[0].clone(),
        remote_filename: args[1].clone(),
        local_path: PathBuf::from(&args[2]),
    })
}

fn parse_all_files(command: String, args: Vec<String>) -> AppResult<Command> {
    match args.len() {
        1 => Ok(Command::AllFiles {
            server: "home".to_string(),
            local_path: PathBuf::from(&args[0]),
        }),
        2 => Ok(Command::AllFiles {
            server: args[0].clone(),
            local_path: PathBuf::from(&args[1]),
        }),
        _ => Err(usage(&command, "[server] <local-path>")),
    }
}

fn parse_defs(command: String, args: Vec<String>) -> AppResult<Command> {
    match args.len() {
        0 => Ok(Command::Defs { local_path: None }),
        1 => Ok(Command::Defs {
            local_path: Some(PathBuf::from(&args[0])),
        }),
        _ => Err(usage(&command, "[local-path]")),
    }
}

fn parse_serve(command: String, args: Vec<String>) -> AppResult<Command> {
    let mut address = DEFAULT_ADDRESS.to_string();
    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "--addr" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err(usage(&command, "[--addr <host:port>]"));
                };
                address = value.clone();
            }
            _ if arg.starts_with("--") => {
                return Err(AppError::Usage(format!("unknown serve flag '{arg}'")));
            }
            _ => return Err(usage(&command, "[--addr <host:port>]")),
        }
        index += 1;
    }

    Ok(Command::Serve { address })
}

fn parse_sync(command: String, args: Vec<String>) -> AppResult<SyncOptions> {
    let mut clean = false;
    let mut dry_run = false;
    let mut server = "home".to_string();
    let mut address = DEFAULT_ADDRESS.to_string();
    let mut positional = Vec::new();
    let mut index = 0;

    while index < args.len() {
        let arg = &args[index];
        match arg.as_str() {
            "--clean" => clean = true,
            "--dry-run" => dry_run = true,
            "--server" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err(sync_usage(&command));
                };
                server = value.clone();
            }
            "--addr" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    return Err(sync_usage(&command));
                };
                address = value.clone();
            }
            _ if arg.starts_with("--") => {
                return Err(AppError::Usage(format!("unknown sync flag '{arg}'")));
            }
            _ => positional.push(arg.clone()),
        }
        index += 1;
    }

    if positional.len() > 2 {
        return Err(sync_usage(&command));
    }

    Ok(SyncOptions {
        server,
        address,
        local_dir: positional
            .first()
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(".")),
        remote_dir: positional.get(1).cloned(),
        clean,
        dry_run,
    })
}

fn sync_usage(command: &str) -> AppError {
    usage(
        command,
        "[local-dir] [remote-dir] [--server <server>] [--addr <host:port>] [--clean] [--dry-run]",
    )
}

fn parse_optional_server(command: String, args: Vec<String>) -> AppResult<String> {
    match args.len() {
        0 => Ok("home".to_string()),
        1 => Ok(args[0].clone()),
        _ => Err(usage(&command, "[server]")),
    }
}

fn parse_one(command: String, args: Vec<String>, name: &str) -> AppResult<String> {
    if args.len() != 1 {
        return Err(usage(&command, name));
    }
    Ok(args[0].clone())
}

fn parse_two(
    command: String,
    args: Vec<String>,
    first: &str,
    second: &str,
) -> AppResult<(String, String)> {
    if args.len() != 2 {
        return Err(usage(&command, &format!("<{first}> <{second}>")));
    }
    Ok((args[0].clone(), args[1].clone()))
}

fn expect_no_args(command: &str, args: Vec<String>) -> AppResult<()> {
    if args.is_empty() {
        Ok(())
    } else {
        Err(usage(command, ""))
    }
}

fn usage(command: &str, usage: &str) -> AppError {
    let suffix = if usage.is_empty() {
        String::new()
    } else {
        format!(" {usage}")
    };
    AppError::Usage(format!("usage: bbrs {command}{suffix}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_sync_with_flags() {
        let command = parse([
            "sync",
            ".",
            "scripts",
            "--server",
            "home",
            "--clean",
            "--dry-run",
        ])
        .expect("parse");

        assert_eq!(
            command,
            Command::Sync(SyncOptions {
                server: "home".to_string(),
                address: DEFAULT_ADDRESS.to_string(),
                local_dir: PathBuf::from("."),
                remote_dir: Some("scripts".to_string()),
                clean: true,
                dry_run: true,
            })
        );
    }

    #[test]
    fn parses_files_default_server() {
        assert_eq!(
            parse(["files"]).expect("parse"),
            Command::Files {
                server: "home".to_string()
            }
        );
    }

    #[test]
    fn rejects_unknown_sync_flag() {
        let err = parse(["sync", ".", "--wat"]).expect_err("error");
        assert_eq!(err.to_string(), "unknown sync flag '--wat'");
    }

    #[test]
    fn parses_sync_defaults() {
        assert_eq!(
            parse(["sync"]).expect("parse"),
            Command::Sync(SyncOptions {
                server: "home".to_string(),
                address: DEFAULT_ADDRESS.to_string(),
                local_dir: PathBuf::from("."),
                remote_dir: None,
                clean: false,
                dry_run: false,
            })
        );
    }

    #[test]
    fn parses_sync_with_server() {
        assert_eq!(
            parse(["sync", "src", "remote", "--server", "n00dles"]).expect("parse"),
            Command::Sync(SyncOptions {
                server: "n00dles".to_string(),
                address: DEFAULT_ADDRESS.to_string(),
                local_dir: PathBuf::from("src"),
                remote_dir: Some("remote".to_string()),
                clean: false,
                dry_run: false,
            })
        );
    }

    #[test]
    fn parses_sync_with_addr() {
        assert_eq!(
            parse(["sync", "--addr", "127.0.0.1:12525"]).expect("parse"),
            Command::Sync(SyncOptions {
                server: "home".to_string(),
                address: "127.0.0.1:12525".to_string(),
                local_dir: PathBuf::from("."),
                remote_dir: None,
                clean: false,
                dry_run: false,
            })
        );
    }

    #[test]
    fn parses_serve_with_addr() {
        assert_eq!(
            parse(["serve", "--addr", "127.0.0.1:12525"]).expect("parse"),
            Command::Serve {
                address: "127.0.0.1:12525".to_string()
            }
        );
    }

    #[test]
    fn rejects_missing_sync_addr_value() {
        let err = parse(["sync", "--addr"]).expect_err("error");
        assert_eq!(
            err.to_string(),
            "usage: bbrs sync [local-dir] [remote-dir] [--server <server>] [--addr <host:port>] [--clean] [--dry-run]"
        );
    }

    #[test]
    fn rejects_missing_serve_addr_value() {
        let err = parse(["serve", "--addr"]).expect_err("error");
        assert_eq!(err.to_string(), "usage: bbrs serve [--addr <host:port>]");
    }
}
