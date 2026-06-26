# Zed Integration Plan

`bitburner-core` is the reusable WASM-friendly protocol/types/path/sync/client
crate. `bitburner-api` is the native blocking Remote API transport. `bbrs` is
the CLI built by `bitburner-cli`.

Zed should use `bitburner-core` directly for reusable logic. It should not
depend on private `bitburner-cli` internals, and it should not depend on
`bitburner-api` while that crate remains native/blocking.

The extension package lives at `extensions/bitburner-zed`. It is not under
`crates/` and is not a root workspace member because it has Zed-specific
metadata, WASM constraints, and a separate check path.

Current `zed_extension_api = "0.7.0"` exposes HTTP fetch, language-server APIs,
Assistant slash commands, limited worktree file reads, and context-server APIs.
It does not expose a TCP listener, TCP client, websocket server, websocket
client, project file enumeration, or general editor command/action API.

Direct Bitburner Remote API communication from the Zed extension remains
blocked until Zed exposes a suitable transport API or another supported bridge
is added. Do not fake upload/download support and do not shell out to `bbrs` for
normal extension behavior.

The supported runtime workflow today is `bbrs serve` followed by REPL commands
after Bitburner connects. See [zed-extension.md](zed-extension.md) for the
current extension API capability notes.

## Roadmap

- v0.1: CLI works.
- v0.2: Keep `bitburner-core` reusable by the extension and wait for a supported transport path.
- v0.3: MCP exposes Bitburner tools to Zed Agent.
- Future: daemon mode or IPC for repeated syncs.

Do not ship a sync task example until sync has a supported non-interactive
entrypoint or MCP integration.

Later MCP flow:

```text
bbrs mcp
```

`bbrs mcp` will later expose sync and Remote API tools to Zed Agent through MCP. See [mcp-plan.md](mcp-plan.md).

Do not make the CLI depend on Zed. Do not make Zed integration depend on
private `bitburner-cli` internals.

Do not put Bitburner game/client scripts in this repo yet. This repo owns the
Remote API library, Rust CLI, and future editor integration.
