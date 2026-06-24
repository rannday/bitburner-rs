use std::io::{self, BufRead, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};
use std::thread;

use anyhow::Context;

use crate::args;
use crate::cli::{execute_with_client, print_repl_help};
use crate::error::AppResult;
use crate::remote::RemoteClient;

pub fn serve(address: &str) -> AppResult<()> {
    let listener = TcpListener::bind(address)
        .with_context(|| format!("bind websocket server on {address}"))?;
    println!("listening on {address}");
    println!("waiting for Bitburner Remote API client");

    let current = Arc::new(Mutex::new(None));
    let accept_current = Arc::clone(&current);

    thread::spawn(move || {
        for incoming in listener.incoming() {
            match incoming {
                Ok(stream) => {
                    let peer = stream
                        .peer_addr()
                        .map_or_else(|_| "<unknown>".to_string(), |addr| addr.to_string());
                    println!("client connected from {peer}");

                    match RemoteClient::from_stream(stream) {
                        Ok(client) => match accept_current.lock() {
                            Ok(mut slot) => {
                                if slot.is_some() {
                                    println!("replacing previous Bitburner connection");
                                }
                                *slot = Some(client);
                            }
                            Err(_) => {
                                eprintln!("error: connection state mutex poisoned");
                                return;
                            }
                        },
                        Err(err) => eprintln!("error: {err:#}"),
                    }
                }
                Err(err) => {
                    eprintln!("error: accept websocket connection failed: {err}");
                    return;
                }
            }
        }
    });

    println!(
        "ready. enter commands like `servers`, `files home`, or `sync home game_files scripts`."
    );
    print_repl_help();
    repl(current)
}

fn repl(current: Arc<Mutex<Option<RemoteClient>>>) -> AppResult<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    loop {
        print!("bbrs> ");
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

        let result = {
            let mut guard = current
                .lock()
                .map_err(|_| anyhow::anyhow!("connection state mutex poisoned"))?;
            match guard.as_mut() {
                Some(remote) => execute_with_client(cli.command, remote),
                None => Err(anyhow::anyhow!("Bitburner is not connected")),
            }
        };

        if let Err(err) = result {
            eprintln!("error: {err:#}");
        }
    }
}

pub fn parse_repl_words(line: &str) -> AppResult<Vec<String>> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut chars = line.chars();
    let mut quote: Option<char> = None;
    let mut escaping = false;

    for ch in chars.by_ref() {
        if escaping {
            current.push(ch);
            escaping = false;
            continue;
        }

        if ch == '\\' {
            escaping = true;
            continue;
        }

        match quote {
            Some(quote_char) if ch == quote_char => quote = None,
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

    if escaping {
        current.push('\\');
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
    fn supports_backslash_escape() {
        assert_eq!(
            parse_repl_words(r#"files home\ server"#).expect("parse"),
            vec!["files".to_string(), "home server".to_string()]
        );
    }
}
