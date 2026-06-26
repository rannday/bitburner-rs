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
crates/bitburner-core
  WASM-friendly protocol, types, typed errors, path rules, sync planning,
  generic JSON-RPC client, and transport trait

crates/bitburner-api
  native blocking TCP/tungstenite transport and RemoteClient wrapper

crates/bitburner-cli
  bbrs CLI/REPL, native filesystem walking, app-level anyhow boundary

extensions/bitburner-zed
  Zed extension scaffold, outside the root workspace, depends on bitburner-core
```

The extension should use `bitburner-core` directly. It should not depend on
`bitburner-cli` internals. It should not depend on `bitburner-api` unless that
crate gains a WASM-compatible transport.

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

That means `bitburner-core` is usable by the extension, but direct Bitburner
Remote API communication from Zed remains blocked unless Zed exposes a suitable
TCP/websocket transport API or the project adds another supported bridge.

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
The preferred path is a Zed-compatible transport that implements
`bitburner_core::BitburnerTransport`.

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
