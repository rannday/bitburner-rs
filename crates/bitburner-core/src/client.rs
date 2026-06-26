use serde::de::DeserializeOwned;
use serde_json::{Value, json};

use crate::{
    BitburnerError, BitburnerFile, BitburnerTransport, DEFAULT_SERVER, FileMetadata,
    JsonRpcRequest, JsonRpcResponse, Result, SaveFile, ServerInfo,
};

pub struct BitburnerClient<T> {
    transport: T,
    next_id: u64,
}

impl<T> BitburnerClient<T> {
    pub fn new(transport: T) -> Self {
        Self {
            transport,
            next_id: 1,
        }
    }

    pub fn transport_mut(&mut self) -> &mut T {
        &mut self.transport
    }
}

impl<T: BitburnerTransport> BitburnerClient<T> {
    pub fn push_file(&mut self, server: &str, filename: &str, content: &str) -> Result<()> {
        let result: String = self.request(
            "pushFile",
            Some(json!({
              "filename": filename,
              "content": content,
              "server": normalize_server(server),
            })),
        )?;
        validate_ok("pushFile", &result)
    }

    pub fn get_file(&mut self, server: &str, filename: &str) -> Result<String> {
        self.request(
            "getFile",
            Some(json!({
              "filename": filename,
              "server": normalize_server(server),
            })),
        )
    }

    pub fn get_file_metadata(&mut self, server: &str, filename: &str) -> Result<FileMetadata> {
        self.request(
            "getFileMetadata",
            Some(json!({
              "filename": filename,
              "server": normalize_server(server),
            })),
        )
    }

    pub fn delete_file(&mut self, server: &str, filename: &str) -> Result<()> {
        let result: String = self.request(
            "deleteFile",
            Some(json!({
              "filename": filename,
              "server": normalize_server(server),
            })),
        )?;
        validate_ok("deleteFile", &result)
    }

    pub fn get_file_names(&mut self, server: &str) -> Result<Vec<String>> {
        self.request(
            "getFileNames",
            Some(json!({
              "server": normalize_server(server),
            })),
        )
    }

    pub fn get_all_files(&mut self, server: &str) -> Result<Vec<BitburnerFile>> {
        self.request(
            "getAllFiles",
            Some(json!({
              "server": normalize_server(server),
            })),
        )
    }

    pub fn get_all_file_metadata(&mut self, server: &str) -> Result<Vec<FileMetadata>> {
        self.request(
            "getAllFileMetadata",
            Some(json!({
              "server": normalize_server(server),
            })),
        )
    }

    pub fn calculate_ram(&mut self, server: &str, filename: &str) -> Result<f64> {
        self.request(
            "calculateRam",
            Some(json!({
              "filename": filename,
              "server": normalize_server(server),
            })),
        )
    }

    pub fn get_definition_file(&mut self) -> Result<String> {
        self.request("getDefinitionFile", None)
    }

    pub fn get_save_file(&mut self) -> Result<SaveFile> {
        self.request("getSaveFile", None)
    }

    pub fn get_all_servers(&mut self) -> Result<Vec<ServerInfo>> {
        self.request("getAllServers", None)
    }

    pub fn build_request(&mut self, method: &str, params: Option<Value>) -> JsonRpcRequest {
        let id = self.next_id;
        self.next_id += 1;
        JsonRpcRequest {
            jsonrpc: "2.0",
            id,
            method: method.to_string(),
            params,
        }
    }

    pub fn request<ValueType>(&mut self, method: &str, params: Option<Value>) -> Result<ValueType>
    where
        ValueType: DeserializeOwned,
    {
        let value = self.request_value(method, params)?;
        serde_json::from_value(value).map_err(|err| {
            BitburnerError::invalid_protocol(format!("decode {method} result: {err}"))
        })
    }

    pub fn request_value(&mut self, method: &str, params: Option<Value>) -> Result<Value> {
        let request = self.build_request(method, params);
        let request_id = request.id;
        let response = self.transport.send_request_value(request)?;
        validate_response_value(method, request_id, response)
    }
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
        let mut client = BitburnerClient::new(MockTransport::default());
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
        let mut client = BitburnerClient::new(transport);

        client
            .push_file("", "scripts/main.js", "export async function main() {}")
            .expect("push");
        assert_eq!(
            client.get_file("n00dles", "scripts/main.js").expect("file"),
            "content"
        );

        let requests = &client.transport_mut().requests;
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
        let mut client = BitburnerClient::new(transport);

        assert_eq!(
            client.get_file_names("home").expect("names"),
            vec!["a.js".to_string(), "b.ts".to_string()]
        );
        assert_eq!(client.calculate_ram("home", "a.js").expect("ram"), 1.75);
        assert_eq!(
            client.get_save_file().expect("save"),
            SaveFile {
                identifier: "bitburner".to_string(),
                binary: false,
                save: "abc".to_string(),
            }
        );
        assert_eq!(
            client.get_all_servers().expect("servers"),
            vec![ServerInfo {
                hostname: "home".to_string(),
                has_admin_rights: true,
                purchased_by_player: false,
            }]
        );
    }

    #[test]
    fn remote_error_response_becomes_typed_error() {
        let mut client = BitburnerClient::new(MockTransport::with_response(MockTransport::error(
            1, "bad file",
        )));

        let err = client.get_file("home", "missing.js").expect_err("error");

        assert!(matches!(err, BitburnerError::JsonRpc(_)));
        assert!(err.to_string().contains("remote error -32000: bad file"));
    }

    #[test]
    fn mismatched_response_id_fails() {
        let mut client = BitburnerClient::new(MockTransport::with_response(MockTransport::ok(
            2,
            json!("content"),
        )));

        let err = client.get_file("home", "a.js").expect_err("error");

        assert!(err.to_string().contains("did not match request id 1"));
    }

    #[test]
    fn response_with_both_result_and_error_fails() {
        let mut client = BitburnerClient::new(MockTransport::with_response(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: 1,
            result: Some(json!("content")),
            error: Some(JsonRpcError {
                code: Some(-32000),
                message: "bad".to_string(),
                data: None,
            }),
        }));

        let err = client.get_file("home", "a.js").expect_err("error");

        assert!(err.to_string().contains("both result and error"));
    }

    #[test]
    fn response_with_neither_result_nor_error_fails() {
        let mut client = BitburnerClient::new(MockTransport::with_response(JsonRpcResponse {
            jsonrpc: "2.0".to_string(),
            id: 1,
            result: None,
            error: None,
        }));

        let err = client.get_file("home", "a.js").expect_err("error");

        assert!(err.to_string().contains("neither result nor error"));
    }

    #[test]
    fn invalid_jsonrpc_version_fails() {
        let mut client = BitburnerClient::new(MockTransport::with_response(JsonRpcResponse {
            jsonrpc: "1.0".to_string(),
            id: 1,
            result: Some(json!("content")),
            error: None,
        }));

        let err = client.get_file("home", "a.js").expect_err("error");

        assert!(err.to_string().contains("invalid jsonrpc version"));
    }

    #[test]
    fn invalid_result_shape_fails() {
        let mut client = BitburnerClient::new(MockTransport::with_response(MockTransport::ok(
            1,
            json!({"not":"a string"}),
        )));

        let err = client.get_file("home", "a.js").expect_err("error");

        assert!(err.to_string().contains("decode getFile result"));
    }

    #[test]
    fn ok_result_validation_rejects_non_ok() {
        let mut client = BitburnerClient::new(MockTransport::with_response(MockTransport::ok(
            1,
            json!("NOPE"),
        )));

        let err = client
            .push_file("home", "a.js", "content")
            .expect_err("error");

        assert!(err.to_string().contains("pushFile"));
    }
}
