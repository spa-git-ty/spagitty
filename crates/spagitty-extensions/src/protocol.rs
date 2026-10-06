// SPDX-License-Identifier: GPL-3.0-or-later

//! JSON-RPC 2.0, one compact message per line.
//!
//! `schemas/extensions/protocol.v1.md` is the standard; this module is the
//! host's reading of the framing. It knows nothing about methods — that is
//! [`crate::host`] — only what a well-formed message is and how to write one.

use serde_json::{json, Map, Value};

/// The largest message either side may send.
pub const MAX_MESSAGE_BYTES: usize = 1024 * 1024;

/// JSON-RPC's own codes and the protocol's.
pub mod code {
    pub const PARSE_ERROR: i64 = -32700;
    pub const INVALID_REQUEST: i64 = -32600;
    pub const METHOD_NOT_FOUND: i64 = -32601;
    pub const INVALID_PARAMS: i64 = -32602;
    pub const INTERNAL: i64 = -32603;
    pub const NOT_GRANTED: i64 = -32001;
    pub const BAD_HANDLE: i64 = -32002;
    pub const BAD_OPERATION: i64 = -32003;
    pub const NOT_ACTIVE: i64 = -32004;
    pub const TOOL_REFUSED: i64 = -32005;
    pub const TOOL_MISSING: i64 = -32006;
    pub const LIMIT: i64 = -32007;
    pub const CANCELLED: i64 = -32008;
    pub const UNSUPPORTED: i64 = -32010;
    pub const DECLINED: i64 = -32011;
    pub const UNCERTAIN: i64 = -32012;
}

/// A request id. The host's are integers; a worker's are strings.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Id {
    Number(u64),
    Text(String),
}

impl Id {
    fn to_value(&self) -> Value {
        match self {
            Id::Number(n) => json!(n),
            Id::Text(s) => json!(s),
        }
    }

    fn from_value(value: &Value) -> Option<Id> {
        match value {
            Value::Number(n) => n.as_u64().map(Id::Number),
            Value::String(s) if !s.is_empty() && s.len() <= 128 => Some(Id::Text(s.clone())),
            _ => None,
        }
    }
}

/// An error a request was answered with.
#[derive(Debug, Clone, PartialEq)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
    pub data: Option<Value>,
}

impl RpcError {
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        RpcError {
            code,
            message: message.into(),
            data: None,
        }
    }

    fn to_value(&self) -> Value {
        let mut error = json!({"code": self.code, "message": self.message});
        if let Some(data) = &self.data {
            error["data"] = data.clone();
        }
        error
    }
}

impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.message, self.code)
    }
}

/// One message, either direction.
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    Request {
        id: Id,
        method: String,
        params: Value,
    },
    Notification {
        method: String,
        params: Value,
    },
    Response {
        id: Id,
        result: Result<Value, RpcError>,
    },
}

/// Why a line could not be read as a message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FrameError {
    TooLong,
    NotJson(String),
    NotRpc(String),
}

impl std::fmt::Display for FrameError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FrameError::TooLong => write!(f, "a message was longer than {MAX_MESSAGE_BYTES} bytes"),
            FrameError::NotJson(detail) => write!(f, "a line on stdout was not JSON ({detail})"),
            FrameError::NotRpc(detail) => write!(f, "a message was not JSON-RPC 2.0 ({detail})"),
        }
    }
}

impl Message {
    pub fn request(id: u64, method: &str, params: Value) -> Message {
        Message::Request {
            id: Id::Number(id),
            method: method.to_string(),
            params,
        }
    }

    pub fn notification(method: &str, params: Value) -> Message {
        Message::Notification {
            method: method.to_string(),
            params,
        }
    }

    pub fn ok(id: Id, result: Value) -> Message {
        Message::Response {
            id,
            result: Ok(result),
        }
    }

    pub fn err(id: Id, error: RpcError) -> Message {
        Message::Response {
            id,
            result: Err(error),
        }
    }

    /// Read one line. A trailing `\r` is tolerated.
    pub fn parse(line: &str) -> Result<Message, FrameError> {
        let line = line.strip_suffix('\r').unwrap_or(line);
        if line.len() > MAX_MESSAGE_BYTES {
            return Err(FrameError::TooLong);
        }
        let value: Value =
            serde_json::from_str(line).map_err(|e| FrameError::NotJson(e.to_string()))?;
        let Some(object) = value.as_object() else {
            return Err(FrameError::NotRpc("not an object".into()));
        };
        if object.get("jsonrpc").and_then(Value::as_str) != Some("2.0") {
            return Err(FrameError::NotRpc("missing \"jsonrpc\": \"2.0\"".into()));
        }
        let params = object.get("params").cloned().unwrap_or(Value::Null);
        if !(params.is_null() || params.is_object() || params.is_array()) {
            return Err(FrameError::NotRpc(
                "params must be an object or a list".into(),
            ));
        }

        match (object.get("method"), object.get("id")) {
            (Some(method), id) => {
                let method = method
                    .as_str()
                    .filter(|m| !m.is_empty() && m.len() <= 128)
                    .ok_or_else(|| FrameError::NotRpc("method must be a string".into()))?
                    .to_string();
                match id {
                    None => Ok(Message::Notification { method, params }),
                    Some(id) => Ok(Message::Request {
                        id: Id::from_value(id)
                            .ok_or_else(|| FrameError::NotRpc("bad id".into()))?,
                        method,
                        params,
                    }),
                }
            }
            (None, Some(id)) => {
                let id = Id::from_value(id).ok_or_else(|| FrameError::NotRpc("bad id".into()))?;
                match (object.get("result"), object.get("error")) {
                    (Some(result), None) => Ok(Message::Response {
                        id,
                        result: Ok(result.clone()),
                    }),
                    (None, Some(error)) => {
                        let code = error.get("code").and_then(Value::as_i64).ok_or_else(|| {
                            FrameError::NotRpc("an error needs a numeric code".into())
                        })?;
                        let message = error
                            .get("message")
                            .and_then(Value::as_str)
                            .unwrap_or("")
                            .to_string();
                        Ok(Message::Response {
                            id,
                            result: Err(RpcError {
                                code,
                                message,
                                data: error.get("data").cloned(),
                            }),
                        })
                    }
                    _ => Err(FrameError::NotRpc(
                        "a response needs exactly one of result and error".into(),
                    )),
                }
            }
            (None, None) => Err(FrameError::NotRpc(
                "neither a request nor a response".into(),
            )),
        }
    }

    /// One line, without the newline.
    pub fn to_line(&self) -> String {
        let mut object = Map::new();
        object.insert("jsonrpc".into(), json!("2.0"));
        match self {
            Message::Request { id, method, params } => {
                object.insert("id".into(), id.to_value());
                object.insert("method".into(), json!(method));
                object.insert("params".into(), params.clone());
            }
            Message::Notification { method, params } => {
                object.insert("method".into(), json!(method));
                object.insert("params".into(), params.clone());
            }
            Message::Response { id, result } => {
                object.insert("id".into(), id.to_value());
                match result {
                    Ok(value) => object.insert("result".into(), value.clone()),
                    Err(error) => object.insert("error".into(), error.to_value()),
                };
            }
        }
        // serde_json escapes control characters, so a value can never carry a
        // raw newline into the framing.
        Value::Object(object).to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kind_of_message_round_trips() {
        for message in [
            Message::request(1, "extension.initialize", json!({"a": 1})),
            Message::notification("operation.progress", json!({"operationId": "op-1"})),
            Message::ok(Id::Text("w1".into()), json!({"value": null})),
            Message::err(Id::Number(9), RpcError::new(code::NOT_GRANTED, "no")),
        ] {
            let line = message.to_line();
            assert!(!line.contains('\n'));
            assert_eq!(Message::parse(&line).unwrap(), message);
        }
    }

    #[test]
    fn a_newline_inside_a_value_cannot_break_the_framing() {
        let line = Message::notification("log", json!({"message": "one\ntwo\r\n"})).to_line();
        assert!(!line.contains('\n'));
        assert!(Message::parse(&format!("{line}\r")).is_ok());
    }

    #[test]
    fn things_that_are_not_json_rpc_are_refused() {
        assert!(matches!(
            Message::parse("hello"),
            Err(FrameError::NotJson(_))
        ));
        assert!(matches!(Message::parse("[]"), Err(FrameError::NotRpc(_))));
        assert!(matches!(
            Message::parse(r#"{"id":1,"result":{}}"#),
            Err(FrameError::NotRpc(_))
        ));
        assert!(matches!(
            Message::parse(r#"{"jsonrpc":"2.0","id":1,"result":{},"error":{"code":1}}"#),
            Err(FrameError::NotRpc(_))
        ));
        assert!(matches!(
            Message::parse(r#"{"jsonrpc":"2.0","id":{},"method":"x"}"#),
            Err(FrameError::NotRpc(_))
        ));
        assert!(matches!(
            Message::parse(r#"{"jsonrpc":"2.0","method":"x","params":3}"#),
            Err(FrameError::NotRpc(_))
        ));
        assert!(matches!(
            Message::parse(r#"{"jsonrpc":"2.0"}"#),
            Err(FrameError::NotRpc(_))
        ));
        assert_eq!(
            Message::parse(&"x".repeat(MAX_MESSAGE_BYTES + 1)),
            Err(FrameError::TooLong)
        );
    }

    #[test]
    fn an_error_response_keeps_its_code_and_data() {
        let line =
            r#"{"jsonrpc":"2.0","id":"w4","error":{"code":-32001,"message":"no","data":{"x":1}}}"#;
        match Message::parse(line).unwrap() {
            Message::Response {
                id,
                result: Err(error),
            } => {
                assert_eq!(id, Id::Text("w4".into()));
                assert_eq!(error.code, code::NOT_GRANTED);
                assert_eq!(error.data, Some(json!({"x": 1})));
                assert!(error.to_string().contains("-32001"));
            }
            other => panic!("{other:?}"),
        }
    }
}
