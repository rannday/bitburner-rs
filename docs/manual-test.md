# Manual Bitburner Test

`bitburner-api` is the reusable native protocol/path/sync/client crate.
`bitburner-cli` builds the `bbrs` command and hosts the local HTTP bridge used
in this checklist.

Start the server from the repo:

```sh
cargo run -p bitburner-cli -- serve
```

Or, after installing:

```sh
bbrs serve
```

In Bitburner, open `Options -> Remote API`, set host `127.0.0.1` and port
`12525`, then connect.

Default ports:

```text
Bitburner WebSocket listener: 127.0.0.1:12525
bbrs HTTP bridge:             127.0.0.1:12526
```

In the `bbrs serve` REPL, run:

```text
servers
files home
sync home game_files scripts --dry-run
sync home game_files scripts
ram home scripts/example.js
defs NetscriptDefinitions.d.ts
quit
```

Sync uploads `.js`, `.ts`, `.txt`, `.script`, and `.ns` files.
Remote paths use forward slashes. Local Windows paths can use backslashes; quote
local paths that contain spaces.

## HTTP Bridge

While `bbrs serve` is running:

```sh
curl http://127.0.0.1:12526/health
curl http://127.0.0.1:12526/servers
curl "http://127.0.0.1:12526/files?server=home"
curl http://127.0.0.1:12526/defs
```

PowerShell:

```powershell
Invoke-RestMethod http://127.0.0.1:12526/health
Invoke-RestMethod http://127.0.0.1:12526/servers
Invoke-RestMethod "http://127.0.0.1:12526/files?server=home"
Invoke-RestMethod http://127.0.0.1:12526/defs
```

`/health` works without a Bitburner connection. The other endpoints return
HTTP 503 until Bitburner connects.

`POST /push` body:

```json
{
  "server": "home",
  "filename": "scripts/foo.js",
  "content": "export async function main(ns) {}"
}
```

`POST /sync` body:

```json
{
  "server": "home",
  "remote_dir": "scripts",
  "files": [
    {
      "relative_path": "src/hack.js",
      "content": "export async function main(ns) {}"
    }
  ],
  "dry_run": true
}
```

The HTTP bridge binds to loopback by default and has no auth/token yet. Do not
bind it to a LAN/WAN interface unless you understand the risk.
