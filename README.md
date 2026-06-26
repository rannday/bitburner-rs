# Bitburner Remote API Tools

Rust tools for the Bitburner Remote API.

## Workspace

This repository is a Cargo workspace:

```text
crates/bitburner-core  reusable WASM-friendly protocol/types/path/sync logic
crates/bitburner-api   native Bitburner Remote API transport/client
crates/bitburner-cli   CLI application that builds the bbrs binary
extensions/bitburner-zed Zed extension skeleton
```

`bitburner-core` owns platform-neutral protocol structs, public data types,
remote path validation, typed errors, abstract sync planning, and the generic
JSON-RPC client/transport trait. It avoids native sockets and filesystem
walking so the Zed extension can use it directly.

`bitburner-api` owns the native blocking websocket/TCP Remote API client and
depends on `bitburner-core`. Its `RemoteClient` is native-only.

`bitburner-cli` owns command parsing, filesystem walking, REPL behavior, and
the `bbrs` binary.

`extensions/bitburner-zed` is the Zed extension scaffold. It stays under
`extensions/` because it has Zed-specific metadata, WASM constraints, and a
separate build path. It should use `bitburner-core`, not CLI internals. It
should not depend on `bitburner-api` unless a WASM-compatible transport is
added.

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

Sync uploads `.js`, `.ts`, `.txt`, `.script`, and `.ns` files.

It skips default generated, VCS, and editor directories:
`.git`, `target`, `node_modules`, `dist`, `build`, `.zed`, `.vscode`, `.idea`, `coverage`, `tmp`, and `temp`.
Ignored directory names are matched case-sensitively.

Remote paths use Bitburner forward slashes. Absolute remote paths and paths containing `..` are rejected.

Local paths use native OS path syntax. On Windows, unquoted local paths with
backslashes work in the REPL:

```text
push home contracts/spiral-matrix.js C:\Users\Rann\bb\contracts\spiral-matrix.js
get home scripts/foo.js C:\Users\Rann\out\foo.js
sync home C:\Users\Rann\game_files scripts --dry-run
```

Quote local paths that contain spaces:

```text
push home contracts/spiral-matrix.js "C:\Users\Rann\bb contracts\spiral matrix.js"
```

## Development

```sh
cargo test --workspace --all-targets
cargo run -p bitburner-cli -- serve
cargo build -p bitburner-cli
```

The Zed extension is intentionally not part of the root workspace yet. Check it
separately when needed:

```sh
cargo check --manifest-path extensions/bitburner-zed/Cargo.toml
```

Live Bitburner Remote API behavior is manual-tested; unit tests do not require
a running game client.
