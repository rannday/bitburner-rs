# Zed Integration Plan

`bitburner-api` is the reusable native protocol/types/errors/path/sync/client
crate. `bbrs` is the CLI built by `bitburner-cli`.

Zed should not depend on private `bitburner-cli` internals. It currently should
not depend on `bitburner-api` because that crate is native/blocking.

The extension package lives at `extensions/bitburner-zed`. It is not under
`crates/` and is not a root workspace member because it has Zed-specific
metadata, WASM constraints, and a separate check path.

Current `zed_extension_api = "0.7.0"` exposes HTTP fetch, language-server APIs,
Assistant slash commands, limited worktree file reads, and context-server APIs.
It does not expose a TCP listener, TCP client, websocket server, websocket
client, project file enumeration, or general editor command/action API.

Direct Bitburner Remote API communication from the Zed extension remains
blocked until Zed exposes a suitable transport API. The supported bridge path is
now local HTTP through `bbrs serve`. Do not fake upload/download support. Do not
add shell-out behavior.

Practical paths:

1. Zed extension -> local HTTP bridge in `bbrs serve` -> Bitburner Remote API
2. Zed extension -> process execution of `bbrs`
3. wait for Zed to expose socket/websocket APIs

Preferred path: Zed extension -> local HTTP -> `bbrs serve` -> WebSocket ->
Bitburner.

The supported runtime workflow today is `bbrs serve`, optional HTTP bridge
calls, and REPL commands after Bitburner connects. See
[zed-extension.md](zed-extension.md) for the current extension API capability
notes.

The HTTP bridge binds to loopback by default and is intended only for local
editor/tool integration. Do not bind it to a LAN/WAN interface unless you
understand the risk. No auth/token is implemented yet; future hardening can add
a random local token or config file.

## Roadmap

- v0.1: CLI works.
- v0.2: Local HTTP bridge health check from the Zed slash command.
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
