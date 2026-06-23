# Bitburner Zed Extension

Thin Zed wrapper for the `bbrs` CLI.

This scaffold does not rewrite sync logic. The core CLI remains the source of truth. Future Zed commands should invoke an installed or local `bbrs` binary like:

```text
bbrs sync <workspace-root> <remote-dir> --server <server> --addr <addr>
```

## Intended Settings

- `server`: remote Bitburner server, default `home`
- `remote_dir`: remote path prefix, default empty
- `addr`: Remote API listen address, default `127.0.0.1:12525`
- `clean`: pass `--clean` when supported, default `false`

## Current Status

This pass is a scaffold only. It includes extension metadata and a tiny Rust library for building future `bbrs sync` arguments.

It is not added to the root Cargo package or workspace. Zed extensions compile to WASM and require Zed-specific extension APIs before this can run inside Zed. Those APIs are intentionally not wired here yet.
