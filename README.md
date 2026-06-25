# Bitburner Remote API Tools

Rust tools for the Bitburner Remote API.

## Workspace

This repository is a Cargo workspace:

```text
crates/bitburner-api   reusable Bitburner Remote API library
crates/bitburner-cli   CLI application that builds the bbrs binary
```

`bitburner-api` owns the Remote API client, protocol structs, public data
types, and constants.

`bitburner-cli` owns command parsing, sync planning, REPL behavior, and the
`bbrs` binary. Future Zed integration should depend on `bitburner-api`
directly or shell out to `bbrs`; it should not depend on CLI internals.

## Install

```sh
cargo install --path crates/bitburner-cli
```

## Start

```sh
bbrs serve
```

From a checkout:

```sh
cargo run -p bitburner-cli -- serve
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

## Development

```sh
cargo test --workspace
cargo run -p bitburner-cli -- serve
cargo build -p bitburner-cli
```
