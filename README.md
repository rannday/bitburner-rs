# Bitburner Remote API CLI

Rust CLI for the Bitburner Remote API.

## Install

```sh
cargo install --path .
```

## Start

```sh
bbrs serve
```

In Bitburner, open `Options -> Remote API`, set host `127.0.0.1` and port `12525`, then connect.

`bbrs serve` keeps the Remote API websocket open and starts a small command REPL. Use commands there:

```text
servers
files home
sync home game_files scripts --dry-run
sync home game_files scripts
ram home scripts/hacking/jit-batcher.js
defs NetscriptDefinitions.d.ts
quit
```

## Commands

Top-level CLI:

```text
bbrs serve [--addr <host:port>]
bbrs --help
bbrs --version
```

REPL commands inside `bbrs serve`:

```text
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
sync <server> <local-dir> [remote-dir] [--dry-run]
```

## Sync

Sync uploads `.js` files only for now.

It skips default generated, VCS, and editor directories:
`.git`, `target`, `node_modules`, `dist`, `build`, `.zed`, `.vscode`, `.idea`, `coverage`, `tmp`, and `temp`.

Remote paths use Bitburner forward slashes. Absolute remote paths and paths containing `..` are rejected.

## Windows Build Tools

Rust MSVC builds need the Visual Studio C++ tools installed.

If `link.exe` is found but `kernel32.lib` is missing, run `cargo` from a Developer PowerShell or through `vcvars64.bat` so the Windows SDK library paths are loaded.

## Future Work

Zed and MCP integration are deferred until the Rust CLI/server behavior is stable.
