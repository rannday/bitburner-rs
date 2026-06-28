use std::io::{self, Write};
use std::net::{SocketAddr, TcpListener};
use std::thread;

use anyhow::Context;
use bitburner_api::RemoteClient;
use rustyline::DefaultEditor;
use rustyline::error::ReadlineError;

use crate::AppResult;
use crate::args;
use crate::cli::{execute_with_client, print_repl_help};
use crate::connection::{SharedConnection, SharedConnectionError};

const REPL_PROMPT: &str = "bbrs> ";

fn print_async_status(message: impl std::fmt::Display) {
    let mut stdout = io::stdout();
    let _ = writeln!(stdout, "\n{message}");
    let _ = stdout.flush();
}

fn startup_banner(address: &str, http_address: &str) -> String {
    format!(
        "Starting Bitburner Remote Server version {}\nListening on {address}\nHTTP bridge on {http_address}\nType `help` for usage\n\n",
        env!("CARGO_PKG_VERSION")
    )
}

pub fn serve(address: &str, http_address: &str) -> AppResult<()> {
    warn_if_non_loopback("websocket listener", address);
    warn_if_non_loopback("HTTP bridge", http_address);

    let listener = TcpListener::bind(address)
        .with_context(|| format!("bind websocket server on {address}"))?;
    let current = SharedConnection::default();
    crate::http_bridge::spawn_http_server(http_address, current.clone())?;
    print!("{}", startup_banner(address, http_address));

    let accept_current = current.clone();

    thread::spawn(move || accept_loop(listener, accept_current));

    repl(current)
}

fn warn_if_non_loopback(label: &str, address: &str) {
    if is_non_loopback_bind_address(address) {
        eprintln!(
            "warning: {label} is not bound to loopback ({address}). bbrs is intended for local use; remote clients may be able to control Bitburner files/scripts. No auth is implemented."
        );
    }
}

fn is_non_loopback_bind_address(address: &str) -> bool {
    !is_loopback_bind_address(address)
}

fn is_loopback_bind_address(address: &str) -> bool {
    if let Ok(addr) = address.parse::<SocketAddr>() {
        return addr.ip().is_loopback();
    }

    host_from_address(address).is_some_and(|host| host.eq_ignore_ascii_case("localhost"))
}

fn host_from_address(address: &str) -> Option<&str> {
    if let Some(rest) = address.strip_prefix('[') {
        let (host, _) = rest.split_once(']')?;
        return Some(host);
    }

    address.rsplit_once(':').map(|(host, _)| host)
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
    if current.replace(client) {
        print_async_status("replacing previous Bitburner connection");
    }
}

fn repl(current: SharedConnection) -> AppResult<()> {
    let mut editor = DefaultEditor::new().context("initialize REPL line editor")?;
    let mut last_history_line: Option<String> = None;

    loop {
        let line = match editor.readline(REPL_PROMPT) {
            Ok(line) => line,
            Err(ReadlineError::Interrupted) => {
                println!();
                continue;
            }
            Err(ReadlineError::Eof) => return Ok(()),
            Err(err) => return Err(anyhow::anyhow!("read stdin: {err}")),
        };

        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        add_repl_history(&mut editor, &mut last_history_line, line);
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

fn add_repl_history(
    editor: &mut DefaultEditor,
    last_history_line: &mut Option<String>,
    line: &str,
) {
    if should_add_repl_history(line, last_history_line.as_deref()) {
        let _ = editor.add_history_entry(line);
        *last_history_line = Some(line.trim().to_string());
    }
}

fn should_add_repl_history(line: &str, last_history_line: Option<&str>) -> bool {
    let line = line.trim();
    !line.is_empty() && last_history_line != Some(line)
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
    fn repl_history_skips_blank_lines() {
        assert!(!should_add_repl_history("   ", None));
    }

    #[test]
    fn repl_history_skips_duplicate_consecutive_command() {
        assert!(!should_add_repl_history("servers", Some("servers")));
    }

    #[test]
    fn repl_history_adds_normal_command() {
        assert!(should_add_repl_history("servers", None));
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
    fn loopback_bind_addresses_do_not_warn() {
        assert!(!is_non_loopback_bind_address("127.0.0.1:12526"));
        assert!(!is_non_loopback_bind_address("localhost:12526"));
        assert!(!is_non_loopback_bind_address("[::1]:12526"));
    }

    #[test]
    fn non_loopback_bind_addresses_warn() {
        assert!(is_non_loopback_bind_address("0.0.0.0:12526"));
        assert!(is_non_loopback_bind_address("[::]:12526"));
        assert!(is_non_loopback_bind_address("192.168.1.50:12526"));
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
