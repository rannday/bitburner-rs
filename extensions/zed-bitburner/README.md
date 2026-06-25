# Bitburner Zed Extension

Thin Zed scaffold for the `bbrs` CLI.

This extension is only a scaffold. It does not implement sync logic and is not
wired into `bbrs serve` sync.

## Current Status

The Rust entrypoint is intentionally minimal and buildable with
`zed_extension_api`. It only registers an extension type.

Zed integration is deferred. Do not configure a Zed task for sync; sync is
currently a REPL command inside `bbrs serve`, not a top-level CLI command.

## Intended Settings

- `server`: remote Bitburner server, default `home`
- `remote_dir`: remote path prefix, default empty
- `addr`: Remote API listen address, default `127.0.0.1:12525`

## Roadmap

- v0.1: CLI works.
- v0.2: Define a supported Zed integration path.
- v0.3: Optional MCP/Agent integration.
- Future: daemon mode or IPC for repeated syncs.

This crate is not added to the root Cargo workspace. Keep the CLI independent
from Zed.
