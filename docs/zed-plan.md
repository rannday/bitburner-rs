# Zed Integration Plan

`bbrs` stays the core tool. Zed should stay a thin wrapper.

First useful Zed flow:

```text
bbrs sync <workspace-folder> [remote-dir] --server home
```

Zed should pass the active worktree or workspace root as `local-dir`.

Zed settings should expose:

- `server`
- `remoteDir`
- `clean`
- maybe uploadable file extensions

Later MCP flow:

```text
bbrs mcp
```

`bbrs mcp` will later expose sync and Remote API tools to Zed Agent through MCP.

Do not make the CLI depend on Zed.

Do not put Bitburner game/client scripts in this repo yet. This repo owns the Rust CLI and future editor integration.
