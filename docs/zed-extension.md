# Zed Extension

The Zed extension lives at `extensions/bitburner-zed`.

It stays outside `crates/` and outside the root Cargo workspace because it is a
Zed/WASM package with Zed-specific metadata and a separate check path. Normal
workspace checks cover the reusable Rust crates and CLI. The extension is
checked separately:

```sh
cargo check --manifest-path extensions/bitburner-zed/Cargo.toml
```

## Architecture

```text
crates/bitburner-api
  reusable native Rust library for Bitburner Remote API, typed errors,
  protocol/types, path rules, sync planning, response validation, and native
  websocket client

crates/bitburner-cli
  bbrs CLI/REPL and future local bridge, native filesystem walking,
  app-level anyhow boundary

extensions/bitburner-zed
  Zed extension scaffold, outside the root workspace
```

The extension should not depend on `bitburner-cli` internals. It currently does
not depend on `bitburner-api` because that crate owns native blocking
TCP/tungstenite code.

## Zed API Capability Check

The extension currently uses `zed_extension_api = "0.7.0"`.

| Capability | Result |
| --- | --- |
| TCP listener | Not exposed |
| TCP client | Not exposed |
| Websocket server | Not exposed |
| Websocket client | Not exposed |
| HTTP fetch | Exposed through `zed::http_client` |
| File/project filesystem APIs | Limited: `Worktree::root_path`, `Worktree::read_text_file(path)`, `which`, and shell env; no project file enumeration found |
| Settings APIs | Limited helpers for language, LSP, and context-server settings; no public generic custom `bitburner.*` helper found |
| Command/action APIs | No general editor command/action API found |
| Task APIs | DAP/task template helpers exist, not a general extension command surface for sync |
| Language-server APIs | Exposed |
| Slash-command APIs | Exposed for Assistant slash commands |

Current Zed extension API does not expose TCP/WebSocket server or client APIs,
so the extension cannot directly speak to Bitburner Remote API.

Future practical paths:

1. Zed extension -> local HTTP bridge in `bbrs serve` -> Bitburner Remote API
2. Zed extension -> process execution of `bbrs`
3. wait for Zed to expose socket/websocket APIs

Preferred future path: Zed extension -> local HTTP -> `bbrs serve` ->
WebSocket -> Bitburner.

## Current Extension Behavior

The extension registers with Zed and implements a minimal `/bitburner`
slash-command handler. It reports configured defaults and clearly says remote
transport is unavailable. It does not claim upload, download, sync, or
definition download works.

Desired future editor commands remain blocked by the current API surface:

```text
bitburner.syncCurrentProject
bitburner.pushCurrentFile
bitburner.pullCurrentFile
bitburner.downloadDefinitions
```

Do not implement these as shell-outs to `bbrs` for normal extension behavior.
The preferred path is a local HTTP bridge hosted by `bbrs serve`, with
`bitburner-api` continuing to own native Remote API access.

## Manual Testing

Workspace Rust checks do not require a live Bitburner connection:

```sh
cargo test --workspace --all-targets
```

Native runtime behavior is still manual-tested through:

```sh
cargo run -p bitburner-cli -- serve
```

Then connect Bitburner Remote API to `127.0.0.1:12525` and run REPL smoke
commands such as `servers`, `files home`, `sync home game_files scripts
--dry-run`, and `defs NetscriptDefinitions.d.ts`.
