# Bitburner Remote API Tools

Rust tools for the Bitburner Remote API.

## Workspace

This repository is a Cargo workspace:

```text
crates/bitburner-api   reusable native Rust library for Bitburner Remote API
crates/bitburner-cli   CLI application that builds the bbrs binary
extensions/bitburner-zed Zed extension skeleton
```

`bitburner-api` owns typed errors, protocol structs, public data types, remote
path validation, reusable sync planning, response validation, and the native
blocking TCP/tungstenite Remote API client. It stays separate from the CLI so a
future MCP server, the local HTTP bridge, tests, and tools can reuse it.

`bitburner-cli` owns command parsing, filesystem walking, REPL behavior, and
the `bbrs` binary. It is the app boundary, hosts the local HTTP bridge, and may
use `anyhow`.

`extensions/bitburner-zed` is the Zed extension scaffold. It stays under
`extensions/` because it has Zed-specific metadata, WASM constraints, and a
separate build path. It does not depend on `bitburner-cli`. It currently does
not depend on `bitburner-api` because that crate is native-only.

Current Zed extension API does not expose TCP/WebSocket server or client APIs,
so the extension cannot directly speak to Bitburner Remote API. The supported
bridge architecture is:

```text
Zed extension -> HTTP localhost -> bbrs serve -> Bitburner Remote API WebSocket
```

Other future options remain process execution of `bbrs` or waiting for Zed to
expose socket/websocket APIs.

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

Default ports:

```text
Bitburner WebSocket listener: 127.0.0.1:12525
bbrs HTTP bridge:             127.0.0.1:12526
```

`bbrs serve` keeps the Remote API websocket open, starts the local HTTP bridge,
and starts a small command REPL. Use commands there:

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
bbrs serve [--addr <host:port>] [--http-addr <host:port>]
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

## HTTP Bridge

`bbrs serve` exposes a local JSON HTTP control API for editors and tools:

```text
GET  /health
GET  /servers
GET  /files?server=home
GET  /defs
POST /push
POST /sync
```

Manual checks:

```sh
curl http://127.0.0.1:12526/health
curl http://127.0.0.1:12526/servers
curl "http://127.0.0.1:12526/files?server=home"
curl http://127.0.0.1:12526/defs
```

PowerShell:

```powershell
Invoke-RestMethod http://127.0.0.1:12526/health
Invoke-RestMethod http://127.0.0.1:12526/servers
Invoke-RestMethod "http://127.0.0.1:12526/files?server=home"
Invoke-RestMethod http://127.0.0.1:12526/defs
```

Push a file:

```json
{
  "server": "home",
  "filename": "scripts/foo.js",
  "content": "export async function main(ns) {}"
}
```

Sync files from an editor/tool:

```json
{
  "server": "home",
  "remote_dir": "scripts",
  "files": [
    {
      "relative_path": "src/hack.js",
      "content": "export async function main(ns) {}"
    }
  ],
  "dry_run": true
}
```

The HTTP bridge binds to loopback by default. It is intended only for local
editor/tool integration. Do not bind it to a LAN/WAN interface unless you
understand the risk. No auth/token is implemented yet; future hardening can add
a random local token or config file.

## Development

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
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
