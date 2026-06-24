# Zed Integration Plan

`bbrs` stays the core tool. Zed should stay a thin wrapper.

Current Zed extension docs support extension features such as languages,
debuggers, themes, snippets, slash commands, indexed docs providers, and MCP
servers. They do not expose a general custom editor command/task API for an
extension to add a "sync Bitburner" command directly.

Zed integration is deferred. The supported workflow today is `bbrs serve`
followed by REPL commands after Bitburner connects.

## Roadmap

- v0.1: CLI works.
- v0.2: Define a supported Zed integration path.
- v0.3: MCP exposes Bitburner tools to Zed Agent.
- Future: daemon mode or IPC for repeated syncs.

Do not ship a sync task example until sync has a supported non-interactive
entrypoint or MCP integration.

Later MCP flow:

```text
bbrs mcp
```

`bbrs mcp` will later expose sync and Remote API tools to Zed Agent through MCP. See [mcp-plan.md](mcp-plan.md).

Do not make the CLI depend on Zed.

Do not put Bitburner game/client scripts in this repo yet. This repo owns the Rust CLI and future editor integration.
