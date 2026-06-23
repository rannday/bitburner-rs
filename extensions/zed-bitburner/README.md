# Bitburner Zed Extension

Thin Zed scaffold for the `bbrs` CLI.

This extension does not implement sync logic. The core CLI remains the source of
truth:

```text
bbrs sync <workspace-root> [remote-dir] --server home --addr 127.0.0.1:12525
```

## Current Status

The Rust entrypoint is intentionally minimal and buildable with
`zed_extension_api`. It only registers an extension type.

Current Zed extension docs do not expose a general API for extensions to add
custom editor commands or task definitions. Zed tasks are configured by users in
`tasks.json`, so the practical v0.2 integration is a documented task that
invokes `bbrs`.

## Manual Zed Task

Add this to `.zed/tasks.json` in a project you want to sync:

```json
[
  {
    "label": "Bitburner: sync workspace",
    "command": "bbrs",
    "args": [
      "sync",
      "$ZED_WORKTREE_ROOT",
      "--server",
      "home",
      "--addr",
      "127.0.0.1:12525"
    ],
    "reveal": "always",
    "hide": "never",
    "allow_concurrent_runs": false,
    "use_new_terminal": false
  }
]
```

For a remote directory, insert it after `$ZED_WORKTREE_ROOT`:

```json
"args": [
  "sync",
  "$ZED_WORKTREE_ROOT",
  "scripts",
  "--server",
  "home",
  "--addr",
  "127.0.0.1:12525"
]
```

## Intended Settings

- `server`: remote Bitburner server, default `home`
- `remote_dir`: remote path prefix, default empty
- `addr`: Remote API listen address, default `127.0.0.1:12525`
- `clean`: pass `--clean` when supported, default `false`

## Roadmap

- v0.1: CLI works.
- v0.2: Zed task invokes `bbrs`.
- v0.3: Optional MCP/Agent integration.
- Future: daemon mode or IPC for repeated syncs.

This crate is not added to the root Cargo workspace. Keep the CLI independent
from Zed.
