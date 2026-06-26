use std::collections::{HashMap, HashSet};
use std::io::Read;
use std::path::PathBuf;
use std::thread;

use anyhow::Context;
use bitburner_api::{
    BitburnerApi, DEFAULT_SERVER, LocalFileEntry, SyncOptions, UploadableExtension,
    UploadableFileKind, build_sync_plan_from_entries, normalize_remote_file_path,
};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tiny_http::{Header, Method, Request, Response, Server, StatusCode};
use url::Url;

use crate::AppResult;
use crate::connection::{SharedConnection, SharedConnectionError};

pub const DEFAULT_HTTP_ADDRESS: &str = "127.0.0.1:12526";
pub const MAX_HTTP_BODY_BYTES: u64 = 32 * 1024 * 1024;
const DEFINITION_FILENAME: &str = "NetscriptDefinitions.d.ts";
const NOT_CONNECTED_MESSAGE: &str = "Bitburner is not connected";

pub(crate) fn spawn_http_server(
    address: &str,
    current: SharedConnection,
) -> AppResult<thread::JoinHandle<()>> {
    let server = Server::http(address)
        .map_err(|err| anyhow::anyhow!("bind HTTP control server on {address}: {err}"))?;
    let handle = thread::spawn(move || serve_http(server, current));
    Ok(handle)
}

fn serve_http(server: Server, current: SharedConnection) {
    for request in server.incoming_requests() {
        let current = current.clone();
        thread::spawn(move || {
            if let Err(err) = respond(request, &current) {
                eprintln!("error: HTTP bridge request failed: {err:#}");
            }
        });
    }
}

fn respond(mut request: Request, current: &SharedConnection) -> AppResult<()> {
    let method = BridgeMethod::from_tiny(request.method());
    let target = request.url().to_string();

    let bridge_response = match method {
        BridgeMethod::Post => match read_limited_body(request.as_reader()) {
            Ok(body) => handle_bridge_request(
                current,
                BridgeRequest {
                    method,
                    target,
                    body,
                },
            ),
            Err(err) => error_response(err),
        },
        _ => handle_bridge_request(
            current,
            BridgeRequest {
                method,
                target,
                body: String::new(),
            },
        ),
    };

    write_json_response(request, bridge_response)
}

trait BridgeState {
    fn bitburner_connected(&self) -> bool;

    fn with_bitburner<T>(
        &self,
        command: impl FnOnce(&mut dyn BitburnerApi) -> AppResult<T>,
    ) -> Result<T, SharedConnectionError>;
}

impl BridgeState for SharedConnection {
    fn bitburner_connected(&self) -> bool {
        self.is_connected()
    }

    fn with_bitburner<T>(
        &self,
        command: impl FnOnce(&mut dyn BitburnerApi) -> AppResult<T>,
    ) -> Result<T, SharedConnectionError> {
        self.with_client(command)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct BridgeRequest {
    method: BridgeMethod,
    target: String,
    body: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BridgeMethod {
    Get,
    Post,
    Other,
}

impl BridgeMethod {
    fn from_tiny(method: &Method) -> Self {
        match method {
            Method::Get => Self::Get,
            Method::Post => Self::Post,
            _ => Self::Other,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
struct BridgeResponse {
    status: u16,
    body: Value,
}

#[derive(Debug, Clone)]
struct BridgeError {
    status: u16,
    message: String,
}

impl BridgeError {
    fn bad_request(message: impl Into<String>) -> Self {
        Self {
            status: 400,
            message: message.into(),
        }
    }

    fn not_connected() -> Self {
        Self {
            status: 503,
            message: NOT_CONNECTED_MESSAGE.to_string(),
        }
    }

    fn internal(message: impl Into<String>) -> Self {
        Self {
            status: 500,
            message: message.into(),
        }
    }

    fn payload_too_large() -> Self {
        Self {
            status: 413,
            message: format!("request body exceeds {MAX_HTTP_BODY_BYTES} bytes"),
        }
    }
}

fn handle_bridge_request<S: BridgeState>(state: &S, request: BridgeRequest) -> BridgeResponse {
    if request.method == BridgeMethod::Post && request.body.len() as u64 > MAX_HTTP_BODY_BYTES {
        return error_response(BridgeError::payload_too_large());
    }

    let target = parse_target(&request.target);
    let path = target.path.as_str();
    let result = match (request.method, path) {
        (BridgeMethod::Get, "/health") => Ok(health_response(state)),
        (BridgeMethod::Get, "/servers") => bitburner_json(state, |api| {
            Ok(serde_json::to_value(api.get_all_servers()?)?)
        }),
        (BridgeMethod::Get, "/files") => {
            let server = target
                .query_param("server")
                .filter(|server| !server.is_empty())
                .unwrap_or(DEFAULT_SERVER);
            bitburner_json(state, |api| {
                Ok(serde_json::to_value(api.get_file_names(server)?)?)
            })
        }
        (BridgeMethod::Get, "/defs") => bitburner_json(state, |api| {
            let content = api.get_definition_file()?;
            Ok(json!({
                "filename": DEFINITION_FILENAME,
                "content": content,
            }))
        }),
        (BridgeMethod::Post, "/push") => handle_push(state, &request.body),
        (BridgeMethod::Post, "/sync") => handle_sync(state, &request.body),
        (method, path) if known_path(path) && method != expected_method(path) => Err(BridgeError {
            status: 405,
            message: "method not allowed".to_string(),
        }),
        _ => Err(BridgeError {
            status: 404,
            message: "not found".to_string(),
        }),
    };

    match result {
        Ok(body) => BridgeResponse { status: 200, body },
        Err(err) => error_response(err),
    }
}

fn read_limited_body(reader: &mut dyn Read) -> Result<String, BridgeError> {
    let mut limited = reader.take(MAX_HTTP_BODY_BYTES + 1);
    let mut bytes = Vec::new();
    limited
        .read_to_end(&mut bytes)
        .map_err(|err| BridgeError::internal(format!("read HTTP request body: {err}")))?;
    if bytes.len() as u64 > MAX_HTTP_BODY_BYTES {
        return Err(BridgeError::payload_too_large());
    }
    String::from_utf8(bytes).map_err(|_| BridgeError::bad_request("request body is not UTF-8"))
}

fn write_json_response(request: Request, bridge_response: BridgeResponse) -> AppResult<()> {
    let body = serde_json::to_string(&bridge_response.body).context("encode HTTP response")?;
    let mut response =
        Response::from_string(body).with_status_code(StatusCode(bridge_response.status));
    if let Ok(header) = Header::from_bytes(&b"Content-Type"[..], &b"application/json"[..]) {
        response = response.with_header(header);
    }
    request
        .respond(response)
        .map_err(|err| anyhow::anyhow!("write HTTP response: {err}"))
}

fn error_response(err: BridgeError) -> BridgeResponse {
    BridgeResponse {
        status: err.status,
        body: json!({ "error": err.message }),
    }
}

fn health_response<S: BridgeState>(state: &S) -> Value {
    json!({
        "ok": true,
        "bitburner_connected": state.bitburner_connected(),
        "version": env!("CARGO_PKG_VERSION"),
    })
}

fn bitburner_json<S: BridgeState>(
    state: &S,
    command: impl FnOnce(&mut dyn BitburnerApi) -> AppResult<Value>,
) -> Result<Value, BridgeError> {
    state.with_bitburner(command).map_err(map_connection_error)
}

fn handle_push<S: BridgeState>(state: &S, body: &str) -> Result<Value, BridgeError> {
    let request: PushRequest = parse_json_body(body)?;
    let server = default_server(request.server.as_deref());
    let filename = normalize_remote_file_path(&request.filename)
        .map_err(|err| BridgeError::bad_request(err.to_string()))?;

    bitburner_json(state, |api| {
        api.push_file(&server, &filename, &request.content)?;
        Ok(json!({
            "ok": true,
            "filename": filename,
        }))
    })
}

fn handle_sync<S: BridgeState>(state: &S, body: &str) -> Result<Value, BridgeError> {
    let request: SyncRequest = parse_json_body(body)?;
    let dry_run = request.dry_run.unwrap_or(false);
    let server = default_server(request.server.as_deref());
    let mut content_by_path = HashMap::new();
    let mut seen_paths = HashSet::new();
    let entries = request
        .files
        .into_iter()
        .map(|file| {
            let relative_path = PathBuf::from(file.relative_path);
            if !seen_paths.insert(relative_path.clone()) {
                return Err(BridgeError::bad_request(format!(
                    "duplicate sync relative_path '{}'",
                    relative_path.to_string_lossy().replace('\\', "/")
                )));
            }
            content_by_path.insert(relative_path.clone(), file.content);
            Ok(LocalFileEntry {
                relative_path,
                content_kind: UploadableFileKind::Text,
            })
        })
        .collect::<Result<Vec<_>, BridgeError>>()?;

    let plan = build_sync_plan_from_entries(
        entries,
        &SyncOptions {
            remote_dir: request.remote_dir,
            allowed_extensions: UploadableExtension::ALL.to_vec(),
        },
    )
    .map_err(|err| BridgeError::bad_request(err.to_string()))?;
    let planned = plan
        .iter()
        .map(|item| SyncResponseItem {
            relative_path: item.relative_path.to_string_lossy().replace('\\', "/"),
            remote_path: item.remote_path.clone(),
        })
        .collect::<Vec<_>>();

    if dry_run {
        return Ok(json!({
            "ok": true,
            "dry_run": true,
            "planned": planned,
        }));
    }

    bitburner_json(state, |api| {
        for item in plan {
            let content = content_by_path.get(&item.relative_path).ok_or_else(|| {
                anyhow::anyhow!("missing sync content for {}", item.relative_path.display())
            })?;
            api.push_file(&server, &item.remote_path, content)?;
        }
        Ok(json!({
            "ok": true,
            "dry_run": false,
            "uploaded": planned,
        }))
    })
}

fn parse_json_body<T: for<'de> Deserialize<'de>>(body: &str) -> Result<T, BridgeError> {
    serde_json::from_str(body).map_err(|_| BridgeError::bad_request("invalid JSON request body"))
}

fn map_connection_error(err: SharedConnectionError) -> BridgeError {
    match err {
        SharedConnectionError::NotConnected => BridgeError::not_connected(),
        SharedConnectionError::State(message) => BridgeError::internal(message),
        SharedConnectionError::Command(err) => {
            eprintln!("error: Bitburner command failed: {err:#}");
            BridgeError::internal("Bitburner command failed")
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ParsedTarget {
    path: String,
    query: HashMap<String, String>,
}

impl ParsedTarget {
    fn query_param(&self, name: &str) -> Option<&str> {
        self.query.get(name).map(String::as_str)
    }
}

fn parse_target(target: &str) -> ParsedTarget {
    let fallback_path = target.split_once('?').map_or(target, |(path, _)| path);
    let Ok(url) = Url::parse(&format!("http://localhost{target}")) else {
        return ParsedTarget {
            path: fallback_path.to_string(),
            query: HashMap::new(),
        };
    };
    let query = url
        .query_pairs()
        .map(|(key, value)| (key.into_owned(), value.into_owned()))
        .collect::<HashMap<_, _>>();
    ParsedTarget {
        path: url.path().to_string(),
        query,
    }
}

fn default_server(server: Option<&str>) -> String {
    match server {
        Some(server) if !server.trim().is_empty() => server.to_string(),
        _ => DEFAULT_SERVER.to_string(),
    }
}

fn known_path(path: &str) -> bool {
    matches!(
        path,
        "/health" | "/servers" | "/files" | "/defs" | "/push" | "/sync"
    )
}

fn expected_method(path: &str) -> BridgeMethod {
    match path {
        "/push" | "/sync" => BridgeMethod::Post,
        _ => BridgeMethod::Get,
    }
}

#[derive(Debug, Deserialize)]
struct PushRequest {
    server: Option<String>,
    filename: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct SyncRequest {
    server: Option<String>,
    remote_dir: Option<String>,
    files: Vec<SyncRequestFile>,
    dry_run: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct SyncRequestFile {
    relative_path: String,
    content: String,
}

#[derive(Debug, Serialize)]
struct SyncResponseItem {
    relative_path: String,
    remote_path: String,
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use bitburner_api::{
        BitburnerError, BitburnerFile, FileMetadata, Result, SaveFile, ServerInfo,
    };

    use super::*;

    #[derive(Default)]
    struct FakeState {
        connected: bool,
        api: Mutex<FakeApi>,
    }

    impl FakeState {
        fn disconnected() -> Self {
            Self {
                connected: false,
                api: Mutex::new(FakeApi::default()),
            }
        }

        fn connected() -> Self {
            Self {
                connected: true,
                api: Mutex::new(FakeApi::default()),
            }
        }
    }

    impl BridgeState for FakeState {
        fn bitburner_connected(&self) -> bool {
            self.connected
        }

        fn with_bitburner<T>(
            &self,
            command: impl FnOnce(&mut dyn BitburnerApi) -> AppResult<T>,
        ) -> std::result::Result<T, SharedConnectionError> {
            if !self.connected {
                return Err(SharedConnectionError::NotConnected);
            }
            let mut api = self
                .api
                .lock()
                .map_err(|_| SharedConnectionError::State("fake mutex poisoned".to_string()))?;
            command(&mut *api).map_err(SharedConnectionError::Command)
        }
    }

    #[derive(Default)]
    struct FakeApi {
        file_name_servers: Vec<String>,
        push_calls: Vec<(String, String, String)>,
    }

    impl BitburnerApi for FakeApi {
        fn request_value(&mut self, _method: &str, _params: Option<Value>) -> Result<Value> {
            unexpected("request_value")
        }

        fn get_all_servers(&mut self) -> Result<Vec<ServerInfo>> {
            Ok(vec![ServerInfo {
                hostname: "home".to_string(),
                has_admin_rights: true,
                purchased_by_player: true,
            }])
        }

        fn get_file_names(&mut self, server: &str) -> Result<Vec<String>> {
            self.file_name_servers.push(server.to_string());
            Ok(vec!["scripts/foo.js".to_string()])
        }

        fn get_definition_file(&mut self) -> Result<String> {
            Ok("type NS = unknown;".to_string())
        }

        fn push_file(&mut self, server: &str, filename: &str, content: &str) -> Result<()> {
            self.push_calls.push((
                server.to_string(),
                filename.to_string(),
                content.to_string(),
            ));
            Ok(())
        }

        fn get_file(&mut self, _server: &str, _filename: &str) -> Result<String> {
            unexpected("get_file")
        }

        fn get_file_metadata(&mut self, _server: &str, _filename: &str) -> Result<FileMetadata> {
            unexpected("get_file_metadata")
        }

        fn delete_file(&mut self, _server: &str, _filename: &str) -> Result<()> {
            unexpected("delete_file")
        }

        fn get_all_files(&mut self, _server: &str) -> Result<Vec<BitburnerFile>> {
            unexpected("get_all_files")
        }

        fn get_all_file_metadata(&mut self, _server: &str) -> Result<Vec<FileMetadata>> {
            unexpected("get_all_file_metadata")
        }

        fn calculate_ram(&mut self, _server: &str, _filename: &str) -> Result<f64> {
            unexpected("calculate_ram")
        }

        fn get_save_file(&mut self) -> Result<SaveFile> {
            unexpected("get_save_file")
        }
    }

    fn request(method: BridgeMethod, target: &str, body: &str) -> BridgeRequest {
        BridgeRequest {
            method,
            target: target.to_string(),
            body: body.to_string(),
        }
    }

    fn handle(state: &FakeState, method: BridgeMethod, target: &str, body: &str) -> BridgeResponse {
        handle_bridge_request(state, request(method, target, body))
    }

    fn unexpected<T>(method: &str) -> Result<T> {
        Err(BitburnerError::invalid_protocol(format!(
            "unexpected {method} call"
        )))
    }

    #[test]
    fn health_reports_disconnected() {
        let response = handle(&FakeState::disconnected(), BridgeMethod::Get, "/health", "");

        assert_eq!(response.status, 200);
        assert_eq!(response.body["ok"], true);
        assert_eq!(response.body["bitburner_connected"], false);
    }

    #[test]
    fn health_reports_connected() {
        let response = handle(&FakeState::connected(), BridgeMethod::Get, "/health", "");

        assert_eq!(response.status, 200);
        assert_eq!(response.body["bitburner_connected"], true);
    }

    #[test]
    fn servers_returns_json() {
        let response = handle(&FakeState::connected(), BridgeMethod::Get, "/servers", "");

        assert_eq!(response.status, 200);
        assert_eq!(
            response.body,
            json!([{"hostname":"home","hasAdminRights":true,"purchasedByPlayer":true}])
        );
    }

    #[test]
    fn files_defaults_server_to_home() {
        let state = FakeState::connected();
        let response = handle(&state, BridgeMethod::Get, "/files", "");

        assert_eq!(response.status, 200);
        assert_eq!(response.body, json!(["scripts/foo.js"]));
        assert_eq!(
            state.api.lock().expect("api").file_name_servers,
            vec!["home".to_string()]
        );
    }

    #[test]
    fn files_parses_home_server_query() {
        let state = FakeState::connected();
        let response = handle(&state, BridgeMethod::Get, "/files?server=home", "");

        assert_eq!(response.status, 200);
        assert_eq!(
            state.api.lock().expect("api").file_name_servers,
            vec!["home".to_string()]
        );
    }

    #[test]
    fn files_empty_server_query_defaults_to_home() {
        let state = FakeState::connected();
        let response = handle(&state, BridgeMethod::Get, "/files?server=", "");

        assert_eq!(response.status, 200);
        assert_eq!(
            state.api.lock().expect("api").file_name_servers,
            vec!["home".to_string()]
        );
    }

    #[test]
    fn files_decodes_percent_encoded_server_query() {
        let state = FakeState::connected();
        let response = handle(&state, BridgeMethod::Get, "/files?server=some%20server", "");

        assert_eq!(response.status, 200);
        assert_eq!(
            state.api.lock().expect("api").file_name_servers,
            vec!["some server".to_string()]
        );
    }

    #[test]
    fn defs_returns_filename_and_content() {
        let response = handle(&FakeState::connected(), BridgeMethod::Get, "/defs", "");

        assert_eq!(response.status, 200);
        assert_eq!(response.body["filename"], DEFINITION_FILENAME);
        assert_eq!(response.body["content"], "type NS = unknown;");
    }

    #[test]
    fn push_normalizes_backslash_paths() {
        let state = FakeState::connected();
        let response = handle(
            &state,
            BridgeMethod::Post,
            "/push",
            r#"{"server":"","filename":"scripts\\foo.js","content":"main"}"#,
        );

        assert_eq!(response.status, 200);
        assert_eq!(response.body["filename"], "scripts/foo.js");
        assert_eq!(
            state.api.lock().expect("api").push_calls,
            vec![(
                "home".to_string(),
                "scripts/foo.js".to_string(),
                "main".to_string()
            )]
        );
    }

    #[test]
    fn push_rejects_parent_segments() {
        let state = FakeState::connected();
        let response = handle(
            &state,
            BridgeMethod::Post,
            "/push",
            r#"{"server":"home","filename":"scripts/../foo.js","content":"main"}"#,
        );

        assert_eq!(response.status, 400);
        assert!(
            response.body["error"]
                .as_str()
                .expect("error")
                .contains("..")
        );
        assert!(state.api.lock().expect("api").push_calls.is_empty());
    }

    #[test]
    fn oversized_post_body_returns_413() {
        let body = "x".repeat((MAX_HTTP_BODY_BYTES + 1) as usize);
        let response = handle(&FakeState::connected(), BridgeMethod::Post, "/push", &body);

        assert_eq!(response.status, 413);
        assert!(
            response.body["error"]
                .as_str()
                .expect("error")
                .contains("exceeds")
        );
    }

    #[test]
    fn sync_dry_run_returns_planned_remote_paths() {
        let response = handle(
            &FakeState::connected(),
            BridgeMethod::Post,
            "/sync",
            r#"{"server":"home","remote_dir":"scripts","files":[{"relative_path":"src/hack.js","content":"main"}],"dry_run":true}"#,
        );

        assert_eq!(response.status, 200);
        assert_eq!(response.body["dry_run"], true);
        assert_eq!(
            response.body["planned"],
            json!([{"relative_path":"src/hack.js","remote_path":"scripts/src/hack.js"}])
        );
    }

    #[test]
    fn sync_uploads_planned_files() {
        let state = FakeState::connected();
        let response = handle(
            &state,
            BridgeMethod::Post,
            "/sync",
            r#"{"server":"home","remote_dir":"scripts","files":[{"relative_path":"src/hack.js","content":"main"}]}"#,
        );

        assert_eq!(response.status, 200);
        assert_eq!(response.body["dry_run"], false);
        assert_eq!(
            response.body["uploaded"],
            json!([{"relative_path":"src/hack.js","remote_path":"scripts/src/hack.js"}])
        );
        assert_eq!(
            state.api.lock().expect("api").push_calls,
            vec![(
                "home".to_string(),
                "scripts/src/hack.js".to_string(),
                "main".to_string()
            )]
        );
    }

    #[test]
    fn sync_rejects_duplicate_relative_paths_without_uploading() {
        let state = FakeState::connected();
        let response = handle(
            &state,
            BridgeMethod::Post,
            "/sync",
            r#"{"server":"home","remote_dir":"scripts","files":[{"relative_path":"src/foo.js","content":"one"},{"relative_path":"src/foo.js","content":"two"}]}"#,
        );

        assert_eq!(response.status, 400);
        assert!(
            response.body["error"]
                .as_str()
                .expect("error")
                .contains("src/foo.js")
        );
        assert!(state.api.lock().expect("api").push_calls.is_empty());
    }

    #[test]
    fn sync_filters_unsupported_extensions() {
        let response = handle(
            &FakeState::connected(),
            BridgeMethod::Post,
            "/sync",
            r#"{"remote_dir":"scripts","files":[{"relative_path":"src/hack.js","content":"main"},{"relative_path":"src/data.json","content":"{}"}],"dry_run":true}"#,
        );

        assert_eq!(response.status, 200);
        assert_eq!(
            response.body["planned"],
            json!([{"relative_path":"src/hack.js","remote_path":"scripts/src/hack.js"}])
        );
    }

    #[test]
    fn unknown_route_returns_404() {
        let response = handle(&FakeState::connected(), BridgeMethod::Get, "/missing", "");

        assert_eq!(response.status, 404);
        assert_eq!(response.body, json!({"error":"not found"}));
    }

    #[test]
    fn wrong_method_on_known_route_returns_405() {
        let response = handle(&FakeState::connected(), BridgeMethod::Get, "/push", "");

        assert_eq!(response.status, 405);
        assert_eq!(response.body, json!({"error":"method not allowed"}));
    }

    #[test]
    fn invalid_json_returns_400() {
        let response = handle(&FakeState::connected(), BridgeMethod::Post, "/push", "{bad");

        assert_eq!(response.status, 400);
        assert_eq!(response.body, json!({"error":"invalid JSON request body"}));
    }
}
