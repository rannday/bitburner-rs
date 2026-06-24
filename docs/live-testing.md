# Live Bitburner Remote API Testing

This checklist validates the Rust Remote API server against a real Bitburner client.

## Build

```sh
cargo test
cargo run -- serve
```

The server should print that it is listening on `127.0.0.1:12525` and waiting for Bitburner to connect.

## Connect Bitburner

In Bitburner:

1. Open `Options -> Remote API`.
2. Set host to `127.0.0.1`.
3. Set port to `12525`.
4. Connect.

The CLI should print `client connected from ...` and continue accepting REPL commands.

## Smoke test commands

Run these in the `bbrs>` REPL:

```text
servers
files home
defs NetscriptDefinitions.d.ts
save save-file.json
sync home game_files scripts --dry-run
sync home game_files scripts
```

Expected results:

- `servers` prints pretty JSON with at least `home`.
- `files home` lists remote filenames.
- `defs NetscriptDefinitions.d.ts` writes the Netscript definition file locally.
- `save save-file.json` writes the save file JSON locally.
- `sync ... --dry-run` prints the planned `.js` uploads without modifying the game.
- `sync ...` uploads the planned `.js` files and overwrites matching remote filenames.

## File round-trip test

Create a local test file:

```sh
mkdir -p game_files/manual-test
cat > game_files/manual-test/hello.js <<'JS'
export async function main(ns) {
  ns.tprint("hello from bbrs");
}
JS
```

Then run:

```text
sync home game_files/manual-test scripts/manual-test --dry-run
sync home game_files/manual-test scripts/manual-test
files home
get home scripts/manual-test/hello.js downloaded-hello.js
ram home scripts/manual-test/hello.js
delete home scripts/manual-test/hello.js
```

Expected results:

- The dry run maps `hello.js` to `scripts/manual-test/hello.js`.
- The real sync uploads the file.
- `get` writes `downloaded-hello.js` and its contents match the original file.
- `ram` prints a numeric RAM value.
- `delete` removes the remote test file.

## Reconnect test

1. Leave `bbrs serve` running.
2. Disconnect Bitburner Remote API.
3. Reconnect Bitburner Remote API.
4. Run `servers` again.

Expected result: the new connection replaces the previous connection, the old websocket is explicitly closed, and REPL commands continue working.

## Notes

- Sync intentionally uploads `.js` files only.
- Sync intentionally overwrites remote files with matching filenames.
- There is no remote cleanup command. Old files are left in-game unless manually deleted.
- The REPL executes one command at a time. Connection state is not locked while a remote command is running, so reconnects are not blocked by a long request.
