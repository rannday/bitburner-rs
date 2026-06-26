# Bitburner Zed Extension

Thin Zed extension scaffold for Bitburner editor integration.

This extension is named `bitburner-zed` to match its directory and Rust package:

- directory: `extensions/bitburner-zed`
- package: `bitburner-zed`
- Zed extension id: `bitburner`
- display name: `Bitburner`

The extension stays under `extensions/`, not `crates/`, because it has
Zed-specific metadata, WASM constraints, and a separate check/build path.

## Current Status

The extension depends on `bitburner-core` for reusable WASM-friendly
protocol/types/path/sync/client logic. It does not depend on `bitburner-cli`,
and it should not depend on `bitburner-api` while that crate remains
native/blocking.

The current `zed_extension_api` version is `0.7.0`.

| Capability | Status |
| --- | --- |
| TCP listener | Not exposed |
| TCP client | Not exposed |
| Websocket server | Not exposed |
| Websocket client | Not exposed |
| HTTP fetch | Exposed through `zed::http_client` |
| File/project filesystem APIs | Limited: worktree root and `read_text_file(path)`, no file enumeration found |
| Settings APIs | Limited to supported categories such as language, LSP, and context server settings |
| Command/action APIs | No general editor command/action API found |
| Task APIs | Task/DAP template helpers exist, not a general sync command surface |
| Language-server APIs | Exposed |
| Slash-command APIs | Exposed for Assistant slash commands |

Because Zed does not expose TCP or websocket APIs here, direct Bitburner Remote
API communication from the extension is blocked. `bitburner-core` is usable by
the extension, but `bitburner-api` remains native-only.

The extension currently registers a minimal `/bitburner` slash-command handler
that reports defaults and the transport limitation. It does not upload,
download, or sync files.

## Intended Defaults

- `bitburner.server`: `home`
- `bitburner.remoteDir`: `scripts`
- `bitburner.host`: `127.0.0.1`
- `bitburner.port`: `12525`

Custom `bitburner.*` settings are not wired yet because the Rust API wrapper
does not expose a generic custom-extension settings helper in this version.

## Check

This crate is not added to the root Cargo workspace. Check it separately from
the repo root:

```sh
cargo check --manifest-path extensions/bitburner-zed/Cargo.toml
```

No live Bitburner integration tests are included. Runtime behavior should be
manual-tested once Zed exposes a usable transport or another supported
integration path is added.
