# Zed Integration Plan

`bbrs` stays the core tool. Zed should stay a thin wrapper.

## Roadmap

- v0.1: CLI works.
- v0.2: Zed wrapper invokes `bbrs`.
- v0.3: Optional MCP/Agent integration.
- Future: daemon mode or IPC for repeated syncs.

First useful Zed wrapper flow:

```text
bbrs sync <workspace-root> <remote-dir> --server <server> --addr <addr>
```

Zed should pass the active worktree or workspace root as `local-dir`.

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
