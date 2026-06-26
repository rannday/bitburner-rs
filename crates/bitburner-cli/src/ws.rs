use std::io::{self, BufRead, Write};
use std::net::TcpListener;
use std::thread;

use anyhow::Context;
use bitburner_api::RemoteClient;

use crate::AppResult;
use crate::args;
use crate::cli::{execute_with_client, print_repl_help};
use crate::connection::{SharedConnection, SharedConnectionError};

const REPL_PROMPT: &str = "bbrs> ";

fn print_async_status(message: impl std::fmt::Display) {
    let mut stdout = io::stdout();
    let _ = writeln!(stdout, "\n{message}");
    let _ = write!(stdout, "{REPL_PROMPT}");
    let _ = stdout.flush();
}

fn startup_banner(address: &str, http_address: &str) -> String {
    format!(
        "Starting Bitburner Remote Server version {}\nListening on {address}\nHTTP bridge on {http_address}\nType `help` for usage\n\n",
        env!("CARGO_PKG_VERSION")
    )
}

pub fn serve(address: &str, http_address: &str) -> AppResult<()> {
    let listener = TcpListener::bind(address)
        .with_context(|| format!("bind websocket server on {address}"))?;
    let current = SharedConnection::default();
    crate::http_bridge::spawn_http_server(http_address, current.clone())?;
    print!("{}", startup_banner(address, http_address));

    let accept_current = current.clone();

    thread::spawn(move || accept_loop(listener, accept_current));

    repl(current)
}

fn accept_loop(listener: TcpListener, current: SharedConnection) {
    for incoming in listener.incoming() {
        match incoming {
            Ok(stream) => {
                let peer = stream
                    .peer_addr()
                    .map_or_else(|_| "<unknown>".to_string(), |addr| addr.to_string());
                print_async_status(format_args!("client connected from {peer}"));

                match RemoteClient::from_stream(stream) {
                    Ok(client) => replace_connection(&current, client),
                    Err(err) => print_async_status(format_args!("error: {err:#}")),
                }
            }
            Err(err) => {
                print_async_status(format_args!(
                    "error: accept websocket connection failed: {err}"
                ));
                return;
            }
        }
    }
}

fn replace_connection(current: &SharedConnection, client: RemoteClient) {
    let previous = current.replace(client);
    if let Some(mut previous) = previous {
        print_async_status("replacing previous Bitburner connection");
        let _ = previous.close();
    }
}

fn repl(current: SharedConnection) -> AppResult<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("{REPL_PROMPT}");
        stdout.flush().context("flush prompt")?;

        let mut line = String::new();
        let read = stdin.lock().read_line(&mut line).context("read stdin")?;
        if read == 0 {
            return Ok(());
        }

        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        if line == "exit" || line == "quit" {
            return Ok(());
        }

        let words = match parse_repl_words(line) {
            Ok(words) => words,
            Err(err) => {
                eprintln!("error: {err:#}");
                continue;
            }
        };
        let parse_args = std::iter::once("bbrs".to_string()).chain(words);
        let cli = match args::parse_repl_from(parse_args) {
            Ok(cli) => cli,
            Err(err) => {
                eprintln!("{err}");
                continue;
            }
        };

        if matches!(cli.command, args::ReplCommand::Help) {
            print_repl_help();
            continue;
        }

        let result = execute_repl_command(&current, cli.command);

        match result {
            Ok(output) => output.print()?,
            Err(err) => eprintln!("error: {err:#}"),
        }
    }
}

fn execute_repl_command(
    current: &SharedConnection,
    command: args::ReplCommand,
) -> AppResult<crate::cli::CommandOutput> {
    current
        .with_client(|remote| execute_with_client(command, remote))
        .map_err(|err| match err {
            SharedConnectionError::NotConnected => anyhow::anyhow!(
                "Bitburner is not connected. In Bitburner, open Options -> Remote API and connect to the bbrs address."
            ),
            SharedConnectionError::State(message) => anyhow::anyhow!(message),
            SharedConnectionError::Command(err) => err,
        })
}

pub fn parse_repl_words(line: &str) -> AppResult<Vec<String>> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars().peekable();
    let mut quote: Option<char> = None;

    while let Some(ch) = chars.next() {
        match quote {
            Some(quote_char) if ch == quote_char => quote = None,
            Some(quote_char) if ch == '\\' => match chars.peek().copied() {
                Some(next) if next == quote_char || next == '\\' => {
                    current.push(chars.next().expect("peeked char"));
                }
                _ => current.push(ch),
            },
            Some(_) => current.push(ch),
            None if ch == '"' || ch == '\'' => quote = Some(ch),
            None if ch.is_whitespace() => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            None => current.push(ch),
        }
    }

    if let Some(quote_char) = quote {
        anyhow::bail!("unterminated quote {quote_char}");
    }

    if !current.is_empty() {
        words.push(current);
    }

    Ok(words)
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::args::{self, ReplCommand, SyncOptions};

    use super::*;

    fn parse_line(line: &str) -> ReplCommand {
        let words = parse_repl_words(line).expect("split line");
        args::parse_repl_from(std::iter::once("bbrs".to_string()).chain(words))
            .expect("parse repl command")
            .command
    }

    #[test]
    fn startup_banner_is_short() {
        assert_eq!(
            startup_banner("127.0.0.1:12525", "127.0.0.1:12526"),
            format!(
                "Starting Bitburner Remote Server version {}\nListening on 127.0.0.1:12525\nHTTP bridge on 127.0.0.1:12526\nType `help` for usage\n\n",
                env!("CARGO_PKG_VERSION")
            )
        );
    }

    #[test]
    fn parses_quoted_push_path() {
        assert_eq!(
            parse_line(r#"push home scripts/foo.js "local path/with spaces/foo.js""#),
            ReplCommand::Push {
                server: "home".to_string(),
                remote_filename: "scripts/foo.js".to_string(),
                local_path: PathBuf::from("local path/with spaces/foo.js"),
            }
        );
    }

    #[test]
    fn parses_unquoted_windows_push_path() {
        assert_eq!(
            parse_line(
                r#"push home contracts/spiral-matrix.js C:\Users\Rann\bb\contracts\spiral-matrix.js"#
            ),
            ReplCommand::Push {
                server: "home".to_string(),
                remote_filename: "contracts/spiral-matrix.js".to_string(),
                local_path: PathBuf::from(r"C:\Users\Rann\bb\contracts\spiral-matrix.js"),
            }
        );
    }

    #[test]
    fn parses_unquoted_windows_get_path() {
        assert_eq!(
            parse_line(r#"get home scripts/foo.js C:\Users\Rann\out\foo.js"#),
            ReplCommand::Get {
                server: "home".to_string(),
                filename: "scripts/foo.js".to_string(),
                local_path: Some(PathBuf::from(r"C:\Users\Rann\out\foo.js")),
            }
        );
    }

    #[test]
    fn parses_unquoted_windows_sync_path() {
        assert_eq!(
            parse_line(r#"sync home C:\Users\Rann\game_files scripts --dry-run"#),
            ReplCommand::Sync(SyncOptions {
                server: "home".to_string(),
                local_dir: PathBuf::from(r"C:\Users\Rann\game_files"),
                remote_dir: Some("scripts".to_string()),
                dry_run: true,
            })
        );
    }

    #[test]
    fn parses_quoted_windows_path_with_spaces() {
        assert_eq!(
            parse_line(
                r#"push home contracts/spiral-matrix.js "C:\Users\Rann\bb contracts\spiral matrix.js""#
            ),
            ReplCommand::Push {
                server: "home".to_string(),
                remote_filename: "contracts/spiral-matrix.js".to_string(),
                local_path: PathBuf::from(r"C:\Users\Rann\bb contracts\spiral matrix.js"),
            }
        );
    }

    #[test]
    fn keeps_forward_slashes_in_remote_paths() {
        assert_eq!(
            parse_line(r#"push home old/foo.js C:\Users\Rann\foo.js"#),
            ReplCommand::Push {
                server: "home".to_string(),
                remote_filename: "old/foo.js".to_string(),
                local_path: PathBuf::from(r"C:\Users\Rann\foo.js"),
            }
        );
    }

    #[test]
    fn remote_path_layer_normalizes_backslashes() {
        assert_eq!(
            bitburner_api::normalize_remote_file_path(r"contracts\spiral-matrix.js")
                .expect("remote path"),
            "contracts/spiral-matrix.js"
        );
    }

    #[test]
    fn parses_quoted_get_path() {
        assert_eq!(
            parse_line(r#"get home scripts/foo.js "out path/foo.js""#),
            ReplCommand::Get {
                server: "home".to_string(),
                filename: "scripts/foo.js".to_string(),
                local_path: Some(PathBuf::from("out path/foo.js")),
            }
        );
    }

    #[test]
    fn parses_quoted_sync_path() {
        assert_eq!(
            parse_line(r#"sync home "game files" scripts --dry-run"#),
            ReplCommand::Sync(SyncOptions {
                server: "home".to_string(),
                local_dir: PathBuf::from("game files"),
                remote_dir: Some("scripts".to_string()),
                dry_run: true,
            })
        );
    }

    #[test]
    fn rejects_unterminated_quote() {
        let err = parse_repl_words(r#"get home scripts/foo.js "out path/foo.js"#)
            .expect_err("unterminated quote");

        assert!(err.to_string().contains("unterminated quote"));
    }

    #[test]
    fn supports_quoted_quote_escape() {
        assert_eq!(
            parse_repl_words(r#"get home "scripts/quo\"te.js""#).expect("parse"),
            vec![
                "get".to_string(),
                "home".to_string(),
                "scripts/quo\"te.js".to_string()
            ]
        );
    }
}
