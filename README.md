# Bitburner Remote API Tool

## Install

Install the CLI from this repository:

```sh
cargo install --path .
```

## Use

Preview the sync plan:

```sh
bbrs sync . scripts --server home --dry-run
```

Upload to Bitburner:

```sh
bbrs sync . scripts --server home
```

## Sync

`bbrs sync [local-dir] [remote-dir] [--server <server>] [--addr <host:port>] [--clean] [--dry-run]`

Sync uploads `.js`, `.ts`, `.txt`, `.script`, and `.json` files only.
It skips default generated, VCS, and editor directories:
`.git`, `target`, `node_modules`, `dist`, `build`, `.zed`, `.vscode`, `.idea`, `coverage`, `tmp`, and `temp`.

Sync listens on `127.0.0.1:12525` by default. Use `--addr <host:port>` to override it.

## Windows Build Tools

Rust MSVC builds need the Visual Studio C++ tools installed.

If `link.exe` is found but `kernel32.lib` is missing, run `cargo` from a Developer PowerShell or through `vcvars64.bat` so the Windows SDK library paths are loaded.

## Zed Extension

https://zed.dev/docs/extensions/developing-extensions
