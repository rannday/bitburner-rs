# Zed Integration Plan

`bbrs` stays the core tool. Zed should stay a thin wrapper.

Current Zed extension docs support extension features such as languages,
debuggers, themes, snippets, slash commands, indexed docs providers, and MCP
servers. They do not expose a general custom editor command/task API for an
extension to add a "sync Bitburner" command directly.

So v0.2 should not pretend to be a native command integration. The honest and
useful Zed path is a project task that invokes the installed or local `bbrs`
binary.

## Roadmap

- v0.1: CLI works.
- v0.2: Zed task invokes `bbrs`.
- v0.3: Optional MCP/Agent integration.
- Future: daemon mode or IPC for repeated syncs.

First useful Zed wrapper flow:

```text
bbrs sync <workspace-root> <remote-dir> --server <server> --addr <addr>
```

Zed should pass the active worktree or workspace root as `local-dir`.

Manual `.zed/tasks.json` example:

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

Zed settings should expose:

- `server`, default `home`
- `remote_dir`, default empty
- `addr`, default `127.0.0.1:12525`
- `clean`, default `false`

Later MCP flow:

```text
bbrs mcp
```

`bbrs mcp` will later expose sync and Remote API tools to Zed Agent through MCP.

Do not make the CLI depend on Zed.

Do not put Bitburner game/client scripts in this repo yet. This repo owns the Rust CLI and future editor integration.
