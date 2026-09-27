use serde::Serialize;
use serde_json::Value;

use crate::error::{Error, Result};

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum StdinMessage {
    #[serde(rename = "user")]
    User {
        message: UserPayload,
        #[serde(skip_serializing_if = "Option::is_none")]
        parent_tool_use_id: Option<String>,
    },
    #[serde(rename = "control_request")]
    ControlRequest {
        request_id: String,
        request: ControlRequest,
    },
    #[serde(rename = "control_response")]
    ControlResponse { response: ControlResponse },
}

#[derive(Debug, Clone, Serialize)]
pub struct UserPayload {
    pub role: &'static str,
    pub content: Vec<TextBlock>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TextBlock {
    #[serde(rename = "type")]
    pub kind: &'static str,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "subtype")]
pub enum ControlRequest {
    #[serde(rename = "initialize")]
    Initialize {},
    #[serde(rename = "interrupt")]
    Interrupt,
}

#[derive(Debug, Clone, Serialize)]
pub struct ControlResponse {
    pub subtype: &'static str,
    pub request_id: String,
    pub response: PermissionDecision,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "behavior")]
pub enum PermissionDecision {
    #[serde(rename = "allow")]
    Allow {
        #[serde(rename = "updatedInput")]
        updated_input: Value,
    },
    #[serde(rename = "deny")]
    Deny { message: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanUseTool {
    pub request_id: String,
    pub tool_name: String,
    pub input: Value,
    pub tool_use_id: Option<String>,
    pub decision_reason: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event {
    Init {
        session_id: String,
    },
    Assistant {
        text: String,
    },
    CanUseTool(CanUseTool),
    Result {
        session_id: Option<String>,
        is_error: bool,
        subtype: Option<String>,
        result: Option<String>,
    },
    ControlResponse {
        request_id: String,
        ok: bool,
    },
    Other {
        type_name: String,
    },
}

pub fn encode_line(message: &StdinMessage) -> Result<String> {
    let mut line =
        serde_json::to_string(message).map_err(|error| Error::Engine(error.to_string()))?;
    line.push('\n');
    Ok(line)
}

pub fn user_message(text: impl Into<String>) -> StdinMessage {
    StdinMessage::User {
        message: UserPayload {
            role: "user",
            content: vec![TextBlock {
                kind: "text",
                text: text.into(),
            }],
        },
        parent_tool_use_id: None,
    }
}

pub fn initialize_request(request_id: impl Into<String>) -> StdinMessage {
    StdinMessage::ControlRequest {
        request_id: request_id.into(),
        request: ControlRequest::Initialize {},
    }
}

pub fn interrupt_request(request_id: impl Into<String>) -> StdinMessage {
    StdinMessage::ControlRequest {
        request_id: request_id.into(),
        request: ControlRequest::Interrupt,
    }
}

pub fn permission_response(
    request_id: impl Into<String>,
    decision: PermissionDecision,
) -> StdinMessage {
    StdinMessage::ControlResponse {
        response: ControlResponse {
            subtype: "success",
            request_id: request_id.into(),
            response: decision,
        },
    }
}

pub fn decode_event(line: &str) -> Result<Event> {
    let value: Value =
        serde_json::from_str(line).map_err(|error| Error::Engine(error.to_string()))?;
    let type_name = value
        .get("type")
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .to_string();

    match type_name.as_str() {
        "system" if value.get("subtype").and_then(Value::as_str) == Some("init") => {
            let session_id = value
                .get("session_id")
                .and_then(Value::as_str)
                .ok_or_else(|| Error::Engine("system init missing session_id".into()))?;
            Ok(Event::Init {
                session_id: session_id.to_string(),
            })
        }
        "assistant" => Ok(Event::Assistant {
            text: extract_assistant_text(&value),
        }),
        "result" => Ok(Event::Result {
            session_id: value
                .get("session_id")
                .and_then(Value::as_str)
                .map(str::to_string),
            is_error: value
                .get("is_error")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || value
                    .get("subtype")
                    .and_then(Value::as_str)
                    .is_some_and(|subtype| subtype.starts_with("error")),
            subtype: value
                .get("subtype")
                .and_then(Value::as_str)
                .map(str::to_string),
            result: value
                .get("result")
                .and_then(Value::as_str)
                .map(str::to_string),
        }),
        "control_request" => decode_control_request(&value),
        "control_response" => {
            let response = value.get("response").unwrap_or(&Value::Null);
            let request_id = response
                .get("request_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let ok = response.get("subtype").and_then(Value::as_str) == Some("success");
            Ok(Event::ControlResponse { request_id, ok })
        }
        _ => Ok(Event::Other { type_name }),
    }
}

fn decode_control_request(value: &Value) -> Result<Event> {
    let request_id = value
        .get("request_id")
        .and_then(Value::as_str)
        .ok_or_else(|| Error::Engine("control_request missing request_id".into()))?
        .to_string();
    let request = value
        .get("request")
        .ok_or_else(|| Error::Engine("control_request missing request".into()))?;
    let subtype = request.get("subtype").and_then(Value::as_str).unwrap_or("");

    match subtype {
        "can_use_tool" => Ok(Event::CanUseTool(CanUseTool {
            request_id,
            tool_name: request
                .get("tool_name")
                .and_then(Value::as_str)
                .unwrap_or("unknown")
                .to_string(),
            input: request.get("input").cloned().unwrap_or(Value::Null),
            tool_use_id: request
                .get("tool_use_id")
                .and_then(Value::as_str)
                .map(str::to_string),
            decision_reason: request
                .get("decision_reason")
                .and_then(Value::as_str)
                .map(str::to_string),
        })),
        other => Ok(Event::Other {
            type_name: format!("control_request:{other}"),
        }),
    }
}

fn extract_assistant_text(value: &Value) -> String {
    let Some(content) = value.pointer("/message/content").and_then(Value::as_array) else {
        return String::new();
    };

    content
        .iter()
        .filter_map(|block| {
            if block.get("type").and_then(Value::as_str) == Some("text") {
                block.get("text").and_then(Value::as_str)
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn encodes_user_message_as_ndjson() {
        let line = encode_line(&user_message("hello")).unwrap();
        assert!(line.ends_with('\n'));
        let value: Value = serde_json::from_str(line.trim_end()).unwrap();
        assert_eq!(value["type"], "user");
        assert_eq!(value["message"]["content"][0]["text"], "hello");
    }

    #[test]
    fn encodes_allow_with_updated_input() {
        let line = encode_line(&permission_response(
            "req_1",
            PermissionDecision::Allow {
                updated_input: json!({"command": "echo ok"}),
            },
        ))
        .unwrap();
        let value: Value = serde_json::from_str(line.trim_end()).unwrap();
        assert_eq!(value["type"], "control_response");
        assert_eq!(value["response"]["request_id"], "req_1");
        assert_eq!(value["response"]["response"]["behavior"], "allow");
        assert_eq!(
            value["response"]["response"]["updatedInput"]["command"],
            "echo ok"
        );
    }

    #[test]
    fn encodes_initialize() {
        let line = encode_line(&initialize_request("init-1")).unwrap();
        let value: Value = serde_json::from_str(line.trim_end()).unwrap();
        assert_eq!(value["type"], "control_request");
        assert_eq!(value["request"]["subtype"], "initialize");
    }

    #[test]
    fn encodes_interrupt() {
        let line = encode_line(&interrupt_request("int_1")).unwrap();
        let value: Value = serde_json::from_str(line.trim_end()).unwrap();
        assert_eq!(value["request"]["subtype"], "interrupt");
    }

    #[test]
    fn decodes_can_use_tool() {
        let event = decode_event(
            r#"{"type":"control_request","request_id":"req_abc","request":{"subtype":"can_use_tool","tool_name":"Bash","input":{"command":"echo spike"},"tool_use_id":"toolu_1"}}"#,
        )
        .unwrap();
        match event {
            Event::CanUseTool(req) => {
                assert_eq!(req.request_id, "req_abc");
                assert_eq!(req.tool_name, "Bash");
                assert_eq!(req.input["command"], "echo spike");
            }
            other => panic!("unexpected {other:?}"),
        }
    }

    #[test]
    fn decodes_init_and_result() {
        let init =
            decode_event(r#"{"type":"system","subtype":"init","session_id":"sess-9"}"#).unwrap();
        assert_eq!(
            init,
            Event::Init {
                session_id: "sess-9".into()
            }
        );
        let result = decode_event(
            r#"{"type":"result","subtype":"success","session_id":"sess-9","is_error":false,"result":"done"}"#,
        )
        .unwrap();
        match result {
            Event::Result {
                session_id,
                is_error,
                result,
                ..
            } => {
                assert_eq!(session_id.as_deref(), Some("sess-9"));
                assert!(!is_error);
                assert_eq!(result.as_deref(), Some("done"));
            }
            other => panic!("unexpected {other:?}"),
        }
    }
}
