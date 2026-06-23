# MCP Integration Plan

`bbrs mcp` is planned as the future Zed Agent integration path. The working product today is still the CLI and the documented Zed task that calls `bbrs sync`.

## Why MCP

MCP gives Zed Agent a tool interface instead of a single shell task. That should let the agent dry-run syncs, upload files, inspect Bitburner files, calculate RAM, and fetch definitions through explicit tools with structured JSON input and output.

The core CLI remains the source of truth. MCP should wrap the same sync and Remote API behavior, not fork it.

## Server Mode

Planned stdio server:

```text
bbrs mcp
```

Do not implement JSON-RPC-over-stdio until the CLI behavior and tool shapes are stable.

The Bitburner Remote API still requires the Bitburner client to connect to:

```text
ws://127.0.0.1:12525
```

If a different address is needed, tools should accept an `addr` field matching the CLI `--addr <host:port>` option.

## Planned Tools

### bitburner_sync

Input:

```json
{
  "local_dir": ".",
  "remote_dir": "scripts",
  "server": "home",
  "addr": "127.0.0.1:12525",
  "clean": false
}
```

Output:

```json
{
  "synced": 2,
  "files": [
    {
      "local_path": "src/foo.js",
      "remote_path": "scripts/src/foo.js"
    }
  ]
}
```

### bitburner_sync_dry_run

Input:

```json
{
  "local_dir": ".",
  "remote_dir": "scripts",
  "server": "home"
}
```

Output:

```json
{
  "planned": 2,
  "files": [
    {
      "local_path": "src/foo.js",
      "remote_path": "scripts/src/foo.js"
    }
  ]
}
```

### bitburner_push_file

Input:

```json
{
  "server": "home",
  "remote_path": "scripts/foo.js",
  "content": "export async function main(ns) {}",
  "addr": "127.0.0.1:12525"
}
```

Output:

```json
{
  "ok": true,
  "remote_path": "scripts/foo.js"
}
```

### bitburner_get_file

Input:

```json
{
  "server": "home",
  "remote_path": "scripts/foo.js",
  "addr": "127.0.0.1:12525"
}
```

Output:

```json
{
  "remote_path": "scripts/foo.js",
  "content": "export async function main(ns) {}"
}
```

### bitburner_list_files

Input:

```json
{
  "server": "home",
  "addr": "127.0.0.1:12525"
}
```

Output:

```json
{
  "server": "home",
  "files": ["scripts/foo.js"]
}
```

### bitburner_calculate_ram

Input:

```json
{
  "server": "home",
  "remote_path": "scripts/foo.js",
  "addr": "127.0.0.1:12525"
}
```

Output:

```json
{
  "remote_path": "scripts/foo.js",
  "ram": 1.6
}
```

### bitburner_get_definitions

Input:

```json
{
  "addr": "127.0.0.1:12525"
}
```

Output:

```json
{
  "content": "declare interface NS { }"
}
```
