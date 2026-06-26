# HTTP API

`bbrs serve` starts a local JSON HTTP bridge for editors and small tools.

Default bind address: `127.0.0.1:12526`

This bridge is intended for local editor/tool integration. It has no auth/token
support by design right now. Binding it to a non-loopback address such as
`0.0.0.0:12526`, `[::]:12526`, or a LAN IP exposes file and script control
operations to other clients on the network.

All responses are JSON. Error responses use:

```json
{ "error": "message" }
```

Common error statuses:

| Status | Meaning |
| --- | --- |
| `400` | Invalid JSON, invalid path, or ambiguous request data |
| `404` | Unknown route |
| `405` | Wrong method for a known route |
| `413` | Request body exceeds the bridge body limit |
| `500` | Bridge or Bitburner command failed |
| `503` | Bitburner Remote API is not connected |

## `GET /health`

Purpose: Check whether the bridge is running and whether Bitburner is connected.

Request query: none.

Request body: none.

Example request:

```sh
curl http://127.0.0.1:12526/health
```

Response schema:

```json
{
  "ok": true,
  "bitburner_connected": true,
  "version": "0.1.0"
}
```

Example response:

```json
{
  "ok": true,
  "bitburner_connected": false,
  "version": "0.1.0"
}
```

Errors: `405` for non-GET methods.

## `GET /servers`

Purpose: List known Bitburner servers.

Request query: none.

Request body: none.

Example request:

```sh
curl http://127.0.0.1:12526/servers
```

Response schema:

```json
[
  {
    "hostname": "home",
    "hasAdminRights": true,
    "purchasedByPlayer": true
  }
]
```

Example response:

```json
[
  {
    "hostname": "home",
    "hasAdminRights": true,
    "purchasedByPlayer": true
  }
]
```

Errors: `405`, `500`, `503`.

## `GET /files?server=home`

Purpose: List filenames on one Bitburner server.

Request query:

| Name | Type | Default | Notes |
| --- | --- | --- | --- |
| `server` | string | `home` | Percent-decoded. Empty values default to `home`. Unknown query parameters are ignored. |

Request body: none.

Example request:

```sh
curl "http://127.0.0.1:12526/files?server=home"
```

Response schema:

```json
["scripts/foo.js"]
```

Example response:

```json
["scripts/foo.js", "scripts/batcher.js"]
```

Errors: `405`, `500`, `503`.

## `GET /defs`

Purpose: Export Bitburner Netscript definitions.

Request query: none.

Request body: none.

Example request:

```sh
curl http://127.0.0.1:12526/defs
```

Response schema:

```json
{
  "filename": "NetscriptDefinitions.d.ts",
  "content": "..."
}
```

Example response:

```json
{
  "filename": "NetscriptDefinitions.d.ts",
  "content": "export interface NS {}"
}
```

Errors: `405`, `500`, `503`.

## `POST /push`

Purpose: Upload one text file to Bitburner.

Request query: none.

Request body schema:

```json
{
  "server": "home",
  "filename": "scripts/foo.js",
  "content": "export async function main(ns) {}"
}
```

Fields:

| Name | Type | Required | Notes |
| --- | --- | --- | --- |
| `server` | string | no | Defaults to `home` when omitted or empty. |
| `filename` | string | yes | Remote Bitburner path. Absolute paths and `..` are rejected. |
| `content` | string | yes | File content to upload. |

Example request:

```sh
curl -X POST http://127.0.0.1:12526/push \
  -H "Content-Type: application/json" \
  -d '{"server":"home","filename":"scripts/foo.js","content":"export async function main(ns) {}"}'
```

Response schema:

```json
{
  "ok": true,
  "filename": "scripts/foo.js"
}
```

Example response:

```json
{
  "ok": true,
  "filename": "scripts/foo.js"
}
```

Errors: `400`, `405`, `413`, `500`, `503`.

## `POST /sync`

Purpose: Plan or upload multiple text files to Bitburner.

Normal sync overwrites existing remote Bitburner files. That is expected.
Duplicate `relative_path` values inside one `/sync` request are rejected as
ambiguous before any upload calls are made.

Request query: none.

Request body schema:

```json
{
  "server": "home",
  "remote_dir": "scripts",
  "files": [
    {
      "relative_path": "src/foo.js",
      "content": "export async function main(ns) {}"
    }
  ],
  "dry_run": false
}
```

Fields:

| Name | Type | Required | Notes |
| --- | --- | --- | --- |
| `server` | string | no | Defaults to `home` when omitted or empty. |
| `remote_dir` | string or null | no | Optional remote prefix. |
| `files` | array | yes | Worktree-relative file entries. |
| `files[].relative_path` | string | yes | Must be unique within the request. Uploadable extensions are `.js`, `.ts`, `.txt`, `.script`, and `.ns`. |
| `files[].content` | string | yes | File content to upload. |
| `dry_run` | boolean | no | Defaults to `false`. When true, returns the plan without uploading. |

Dry-run example request:

```sh
curl -X POST http://127.0.0.1:12526/sync \
  -H "Content-Type: application/json" \
  -d '{"server":"home","remote_dir":"scripts","files":[{"relative_path":"src/foo.js","content":"export async function main(ns) {}"}],"dry_run":true}'
```

Dry-run response schema:

```json
{
  "ok": true,
  "dry_run": true,
  "planned": [
    {
      "relative_path": "src/foo.js",
      "remote_path": "scripts/src/foo.js"
    }
  ]
}
```

Upload response schema:

```json
{
  "ok": true,
  "dry_run": false,
  "uploaded": [
    {
      "relative_path": "src/foo.js",
      "remote_path": "scripts/src/foo.js"
    }
  ]
}
```

Duplicate path error example:

```json
{
  "error": "duplicate sync relative_path 'src/foo.js'"
}
```

Errors: `400`, `405`, `413`, `500`, `503`.
