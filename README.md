# Bitburner Remote API Tool

## Sync

`bbrs sync [local-dir] [remote-dir] [--server <server>] [--addr <host:port>] [--clean] [--dry-run]`

Sync uploads `.js`, `.ts`, `.txt`, `.script`, and `.json` files only.
It skips default generated, VCS, and editor directories:
`.git`, `target`, `node_modules`, `dist`, `build`, `.zed`, `.vscode`, `.idea`, `coverage`, `tmp`, and `temp`.

Sync listens on `127.0.0.1:12525` by default. Use `--addr <host:port>` to override it.

## ZED Extension
https://zed.dev/docs/extensions/developing-extensions
