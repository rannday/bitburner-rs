use std::net::TcpStream;

use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::transport::{BitburnerTransport, NativeWebSocketTransport};
use crate::{
    BitburnerError, BitburnerFile, DEFAULT_SERVER, FileMetadata, JsonRpcRequest, JsonRpcResponse,
    Result, SaveFile, ServerInfo,
};

pub trait BitburnerApi {
    fn request_value(&mut self, method: &str, params: Option<Value>) -> Result<Value>;

    fn push_file(&mut self, server: &str, filename: &str, content: &str) -> Result<()> {
        let result: String = request(
            self,
            "pushFile",
            Some(json!({
              "filename": filename,
              "content": content,
              "server": normalize_server(server),
            })),
        )?;
        validate_ok("pushFile", &result)
    }

    fn get_file(&mut self, server: &str, filename: &str) -> Result<String> {
        request(
            self,
            "getFile",
            Some(json!({
              "filename": filename,
              "server": normalize_server(server),
            })),
        )
    }

    fn get_file_metadata(&mut self, server: &str, filename: &str) -> Result<FileMetadata> {
        request(
            self,
            "getFileMetadata",
            Some(json!({
              "filename": filename,
              "server": normalize_server(server),
            })),
        )
    }

    fn delete_file(&mut self, server: &str, filename: &str) -> Result<()> {
        let result: String = request(
            self,
            "deleteFile",
            Some(json!({
              "filename": filename,
              "server": normalize_server(server),
            })),
        )?;
        validate_ok("deleteFile", &result)
    }

    fn get_file_names(&mut self, server: &str) -> Result<Vec<String>> {
        request(
            self,
            "getFileNames",
            Some(json!({
              "server": normalize_server(server),
            })),
        )
    }

    fn get_all_files(&mut self, server: &str) -> Result<Vec<BitburnerFile>> {
        request(
            self,
            "getAllFiles",
            Some(json!({
              "server": normalize_server(server),
            })),
        )
    }

    fn get_all_file_metadata(&mut self, server: &str) -> Result<Vec<FileMetadata>> {
        request(
            self,
            "getAllFileMetadata",
            Some(json!({
              "server": normalize_server(server),
            })),
        )
    }

    fn calculate_ram(&mut self, server: &str, filename: &str) -> Result<f64> {
        request(
            self,
            "calculateRam",
            Some(json!({
              "filename": filename,
              "server": normalize_server(server),
            })),
        )
    }

    fn get_definition_file(&mut self) -> Result<String> {
        request(self, "getDefinitionFile", None)
    }

    fn get_save_file(&mut self) -> Result<SaveFile> {
        request(self, "getSaveFile", None)
    }

    fn get_all_servers(&mut self) -> Result<Vec<ServerInfo>> {
        request(self, "getAllServers", None)
    }
}

pub struct RemoteClient {
    client: JsonRpcClient<NativeWebSocketTransport>,
}

impl RemoteClient {
    pub fn from_stream(stream: TcpStream) -> Result<Self> {
        let transport = NativeWebSocketTransport::from_stream(stream)?;
        Ok(Self {
            client: JsonRpcClient::new(transport),
        })
    }

    pub fn close(&mut self) -> Result<()> {
        self.client.transport_mut().close()
    }
}

impl BitburnerApi for RemoteClient {
    fn request_value(&mut self, method: &str, params: Option<Value>) -> Result<Value> {
        self.client.request_value(method, params)
    }
}

struct JsonRpcClient<T> {
    transport: T,
    next_id: u64,
}

impl<T> JsonRpcClient<T> {
    fn new(transport: T) -> Self {
        Self {
            transport,
            next_id: 1,
        }
    }

    fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }
}

impl<T: BitburnerTransport> JsonRpcClient<T> {
    fn build_request(&mut self, method: &str, params: Option<Value>) -> JsonRpcRequest {
        let id = self.next_id;
        self.next_id += 1;
        JsonRpcRequest {
            jsonrpc: "2.0",
            id,
            method: method.to_string(),
            params,
        }
    }

    fn request_value(&mut self, method: &str, params: Option<Value>) -> Result<Value> {
        let request = self.build_request(method, params);
        let request_id = request.id;
        let response = self.transport.send_request_value(request)?;
        validate_response_value(method, request_id, response)
    }
}

fn request<Api, ValueType>(api: &mut Api, method: &str, params: Option<Value>) -> Result<ValueType>
where
    Api: BitburnerApi + ?Sized,
    ValueType: DeserializeOwned,
{
    let value = api.request_value(method, params)?;
    serde_json::from_value(value)
        .map_err(|err| BitburnerError::invalid_protocol(format!("decode {method} result: {err}")))
}

fn normalize_server(server: &str) -> &str {
    if server.is_empty() {
        DEFAULT_SERVER
    } else {
        server
    }
}

fn validate_ok(method: &str, result: &str) -> Result<()> {
    if result == "OK" {
        Ok(())
    } else {
        Err(BitburnerError::invalid_protocol(format!(
            "{method} returned unexpected result '{result}'"
        )))
    }
}

fn validate_response_value(
    method: &str,
    request_id: u64,
    response: JsonRpcResponse<Value>,
) -> Result<Value> {
    if response.id != request_id {
        return Err(BitburnerError::invalid_protocol(format!(
            "{method} response id {} did not match request id {request_id}",
            response.id
        )));
    }

    if response.jsonrpc != "2.0" {
        return Err(BitburnerError::invalid_protocol(format!(
            "invalid jsonrpc version '{}'",
            response.jsonrpc
        )));
    }

    match (response.result, response.error) {
        (Some(_), Some(_)) => Err(BitburnerError::invalid_protocol(format!(
            "{method} response has both result and error"
        ))),
        (None, None) => Err(BitburnerError::invalid_protocol(format!(
            "{method} response has neither result nor error"
        ))),
        (None, Some(error)) => Err(BitburnerError::JsonRpc(error)),
        (Some(result), None) => Ok(result),
    }
}

#[cfg(test)]
mod tests {
    use std::collections::VecDeque;

    use super::*;
    use crate::{JsonRpcError, SaveFile, ServerInfo};

    #[derive(Default)]
    struct MockTransport {
        responses: VecDeque<JsonRpcResponse<Value>>,
        requests: Vec<JsonRpcRequest>,
    }

    struct MockApi {
        client: JsonRpcClient<MockTransport>,
    }

    impl MockApi {
        fn new(transport: MockTransport) -> Self {
            Self {
                client: JsonRpcClient::new(transport),
            }
        }

        fn with_response(response: JsonRpcResponse<Value>) -> Self {
            Self::new(MockTransport::with_response(response))
        }
    }

    impl BitburnerApi for MockApi {
        fn request_value(&mut self, method: &str, params: Option<Value>) -> Result<Value> {
            self.client.request_value(method, params)
        }
    }

    impl MockTransport {
        fn with_response(response: JsonRpcResponse<Value>) -> Self {
            Self {
                responses: VecDeque::from([response]),
                requests: Vec::new(),
            }
        }

        fn ok(id: u64, result: Value) -> JsonRpcResponse<Value> {
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: Some(result),
                error: None,
            }
        }

        fn error(id: u64, message: &str) -> JsonRpcResponse<Value> {
            JsonRpcResponse {
                jsonrpc: "2.0".to_string(),
                id,
                result: None,
                error: Some(JsonRpcError {
                    code: Some(-32000),
                    message: message.to_string(),
                    data: None,
                }),
            }
        }
    }

    impl BitburnerTransport for MockTransport {
        fn send_request_value(
            &mut self,
            request: JsonRpcRequest,
        ) -> Result<JsonRpcResponse<Value>> {
            self.requests.push(request);
            self.responses
                .pop_front()
                .ok_or_else(|| BitburnerError::invalid_protocol("mock transport has no response"))
        }
    }

    #[test]
    fn request_json_uses_object_params() {
        let mut client = JsonRpcClient::new(MockTransport::default());
        let request = client.build_request(
            "pushFile",
            Some(json!({
              "filename": "scripts/main.js",
              "content": "export async function main() {}",
              "server": "home",
            })),
        );

        let actual = serde_json::to_value(request).expect("json");

        assert_eq!(
            actual,
            json!({
              "jsonrpc": "2.0",
              "id": 1,
              "method": "pushFile",
              "params": {
                "filename": "scripts/main.js",
                "content": "export async function main() {}",
                "server": "home",
              },
            })
        );
    }

    #[test]
    fn request_ids_increment_and_methods_keep_params() {
        let transport = MockTransport {
            responses: VecDeque::from([
                MockTransport::ok(1, json!("OK")),
                MockTransport::ok(2, json!("content")),
            ]),
            requests: Vec::new(),
        };
        let mut api = MockApi::new(transport);

        api.push_file("", "scripts/main.js", "export async function main() {}")
            .expect("push");
        assert_eq!(
            api.get_file("n00dles", "scripts/main.js").expect("file"),
            "content"
        );

        let requests = &api.client.transport_mut().requests;
        assert_eq!(requests[0].id, 1);
        assert_eq!(requests[0].method, "pushFile");
        assert_eq!(
            requests[0].params.as_ref().expect("params")["server"],
            json!("home")
        );
        assert_eq!(
            requests[0].params.as_ref().expect("params")["filename"],
            json!("scripts/main.js")
        );
        assert_eq!(requests[1].id, 2);
        assert_eq!(requests[1].method, "getFile");
        assert_eq!(
            requests[1].params.as_ref().expect("params")["server"],
            json!("n00dles")
        );
    }

    #[test]
    fn typed_methods_decode_successful_responses() {
        let transport = MockTransport {
            responses: VecDeque::from([
                MockTransport::ok(1, json!(["a.js", "b.ts"])),
                MockTransport::ok(2, json!(1.75)),
                MockTransport::ok(
                    3,
                    json!({"identifier":"bitburner","binary":false,"save":"abc"}),
                ),
                MockTransport::ok(
                    4,
                    json!([{"hostname":"home","hasAdminRights":true,"purchasedByPlayer":false}]),
                ),
            ]),
            requests: Vec::new(),
        };
        let mut api = MockApi::new(transport);

        assert_eq!(
            api.get_file_names("home").expect("names"),
            vec!["a.js".to_string(), "b.ts".to_string()]
        );
        assert_eq!(api.calculate_ram("home", "a.js").expect("ram"), 1.75);
        assert_eq!(
            api.get_save_file().expect("save"),
            SaveFile {
                identifier: "bitburner".to_string(),
                binary: false,
                save: "abc".to_string(),
            }
        );
        assert_eq!(
            api.get_all_servers().expect("servers"),
            vec![ServerInfo {
                hostname: "home".to_string(),
                has_admin_rights: true,
                purchased_by_player: false,
            }]
        );
    }

    #[test]
    fn remote_error_response_becomes_typed_error() {
        let mut api = MockApi::with_response(MockTransport::error(1, "bad file"));

        let err = api.get_file("home", "missing.js").expect_err("error");

        assert!(matches!(err, BitburnerError::JsonRpc(_)));
        assert!(err.to_string().contains("remote error -32000: bad file"));
    }

    #[test]
    fn mismatched_response_id_fails() {
        let mut api = MockApi::with_response(MockTransport::ok(2, json!("content")));

        let err = api.get_file("home", "a.js").expect_err("error");

        assert!(err.to_string().contains("did not match request id 1"));
    }

    #[test]
    fn response_with_both_result_and_error_fails() {
        let mut api = MockApi::with_response(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: 1,
            result: Some(json!("content")),
            error: Some(JsonRpcError {
                code: Some(-32000),
                message: "bad".to_string(),
                data: None,
            }),
        });

        let err = api.get_file("home", "a.js").expect_err("error");

        assert!(err.to_string().contains("both result and error"));
    }

    #[test]
    fn response_with_neither_result_nor_error_fails() {
        let mut api = MockApi::with_response(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: 1,
            result: None,
            error: None,
        });

        let err = api.get_file("home", "a.js").expect_err("error");

        assert!(err.to_string().contains("neither result nor error"));
    }

    #[test]
    fn invalid_jsonrpc_version_fails() {
        let mut api = MockApi::with_response(JsonRpcResponse {
            jsonrpc: "1.0".to_string(),
            id: 1,
            result: Some(json!("content")),
            error: None,
        });

        let err = api.get_file("home", "a.js").expect_err("error");

        assert!(err.to_string().contains("invalid jsonrpc version"));
    }

    #[test]
    fn invalid_result_shape_fails() {
        let mut api = MockApi::with_response(MockTransport::ok(1, json!({"not":"a string"})));

        let err = api.get_file("home", "a.js").expect_err("error");

        assert!(err.to_string().contains("decode getFile result"));
    }

    #[test]
    fn ok_result_validation_rejects_non_ok() {
        let mut api = MockApi::with_response(MockTransport::ok(1, json!("NOPE")));

        let err = api.push_file("home", "a.js", "content").expect_err("error");

        assert!(err.to_string().contains("pushFile"));
    }
}
