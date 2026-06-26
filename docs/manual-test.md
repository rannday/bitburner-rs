# Manual Bitburner Test

`bitburner-core` is the reusable WASM-friendly protocol/path/sync crate.
`bitburner-api` is the native Remote API transport. `bitburner-cli` builds the
`bbrs` command used in this checklist.

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
