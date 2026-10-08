// SPDX-License-Identifier: GPL-3.0-or-later

//! Model providers, reached with the person's own key (2.0, agents).
//!
//! Spagitty has no model of its own, ships no key and keeps no bill. A remote
//! agent is a model the person pays for, reached over its provider's HTTP API;
//! Spagitty runs the loop around it (`assign::remote` in the farm crate) and
//! this module only speaks each provider's wire format: one message out, one
//! answer back, with the tool calls it asked for.
//!
//! Four kinds of endpoint: Anthropic, OpenAI, Google, and anything that speaks
//! the OpenAI-compatible chat API — routers such as OpenRouter, and local
//! servers such as Ollama and LM Studio, given by base URL.
//!
//! The key is the caller's to fetch from the keychain at request time and
//! pass in. It goes in a header and nowhere else, and is never written to an
//! error ([`http::redact`]).

pub mod http;

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::{Error, Result};

/// Which wire format.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Provider {
    Anthropic,
    OpenAi,
    Google,
    /// The OpenAI-compatible chat API at a base URL the person gives.
    Compatible,
}

impl Provider {
    pub fn label(self) -> &'static str {
        match self {
            Provider::Anthropic => "Anthropic",
            Provider::OpenAi => "OpenAI",
            Provider::Google => "Google",
            Provider::Compatible => "OpenAI-compatible",
        }
    }

    pub fn default_base(self) -> &'static str {
        match self {
            Provider::Anthropic => "https://api.anthropic.com",
            Provider::OpenAi => "https://api.openai.com/v1",
            Provider::Google => "https://generativelanguage.googleapis.com",
            Provider::Compatible => "http://localhost:11434/v1",
        }
    }
}

/// Where to send, and as whom.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Endpoint {
    pub provider: Provider,
    /// Empty for the provider's own.
    pub base: String,
    pub model: String,
    /// Empty for an endpoint that needs none, such as a local server.
    pub key: String,
}

impl Endpoint {
    fn base(&self) -> String {
        let base = if self.base.trim().is_empty() {
            self.provider.default_base()
        } else {
            self.base.trim()
        };
        base.trim_end_matches('/').to_string()
    }

    /// The name errors are reported under: the provider, or the host for a
    /// compatible endpoint.
    pub fn name(&self) -> String {
        match self.provider {
            Provider::Compatible => self
                .base()
                .split("://")
                .nth(1)
                .and_then(|rest| rest.split('/').next())
                .unwrap_or("the endpoint")
                .to_string(),
            other => other.label().to_string(),
        }
    }

    /// Does everything sent here stay on this machine?
    pub fn is_local(&self) -> bool {
        http::is_local(&self.base())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Role {
    User,
    Assistant,
}

/// One part of a message.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum Part {
    Text {
        text: String,
    },
    /// The model asks for a tool.
    Call {
        id: String,
        name: String,
        input: Value,
    },
    /// Spagitty's answer to a call.
    #[serde(rename_all = "camelCase")]
    Result {
        id: String,
        name: String,
        content: String,
        is_error: bool,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Message {
    pub role: Role,
    pub parts: Vec<Part>,
}

impl Message {
    pub fn user(text: impl Into<String>) -> Message {
        Message {
            role: Role::User,
            parts: vec![Part::Text { text: text.into() }],
        }
    }
}

/// A tool the model may call: a name, what it does, and a JSON schema for its
/// input — a plain object schema, which every provider here accepts.
#[derive(Debug, Clone, PartialEq)]
pub struct Tool {
    pub name: &'static str,
    pub description: &'static str,
    pub schema: Value,
}

/// Why the model stopped.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Stop {
    /// It is done with this turn.
    Done,
    /// It wants tools answered.
    Tools,
    /// It ran out of room for its answer.
    Length,
    /// Anything else the provider reported, such as a refusal.
    Other,
}

/// One answer.
#[derive(Debug, Clone, PartialEq)]
pub struct Turn {
    pub parts: Vec<Part>,
    pub stop: Stop,
    pub input_tokens: u64,
    pub output_tokens: u64,
}

impl Turn {
    pub fn text(&self) -> String {
        self.parts
            .iter()
            .filter_map(|part| match part {
                Part::Text { text } => Some(text.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    pub fn calls(&self) -> Vec<(String, String, Value)> {
        self.parts
            .iter()
            .filter_map(|part| match part {
                Part::Call { id, name, input } => Some((id.clone(), name.clone(), input.clone())),
                _ => None,
            })
            .collect()
    }
}

/// Send one exchange and read the answer.
pub fn complete(
    endpoint: &Endpoint,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
    max_tokens: u32,
) -> Result<Turn> {
    let (url, headers, body) = request(endpoint, system, messages, tools, max_tokens);
    let headers: Vec<(&str, &str)> = headers.iter().map(|(n, v)| (*n, v.as_str())).collect();
    let answered = http::post(&url, &headers, &body.to_string(), &endpoint.name())?;
    let value = parsed(endpoint, &answered)?;
    read_turn(endpoint.provider, &value).ok_or_else(|| Error::Model {
        provider: endpoint.name(),
        detail: "answered with something that is not a reply".into(),
    })
}

/// The models a provider offers, when it lists them.
pub fn models(endpoint: &Endpoint) -> Result<Vec<String>> {
    let base = endpoint.base();
    let url = match endpoint.provider {
        Provider::Anthropic => format!("{base}/v1/models?limit=100"),
        Provider::OpenAi | Provider::Compatible => format!("{base}/models"),
        Provider::Google => format!("{base}/v1beta/models?pageSize=200"),
    };
    let headers = auth(endpoint);
    let headers: Vec<(&str, &str)> = headers.iter().map(|(n, v)| (*n, v.as_str())).collect();
    let answered = http::get(&url, &headers, &endpoint.name())?;
    let value = parsed(endpoint, &answered)?;
    Ok(read_models(endpoint.provider, &value))
}

/// One small request, and how long it took: what *Test* shows.
pub fn test(endpoint: &Endpoint) -> Result<Duration> {
    let started = Instant::now();
    complete(
        endpoint,
        "Answer with one word.",
        &[Message::user("Say ok.")],
        &[],
        16,
    )?;
    Ok(started.elapsed())
}

fn auth(endpoint: &Endpoint) -> Vec<(&'static str, String)> {
    let key = endpoint.key.trim();
    let mut headers = Vec::new();
    match endpoint.provider {
        Provider::Anthropic => {
            headers.push(("anthropic-version", "2023-06-01".to_string()));
            if !key.is_empty() {
                headers.push(("x-api-key", key.to_string()));
            }
        }
        Provider::Google => {
            if !key.is_empty() {
                headers.push(("x-goog-api-key", key.to_string()));
            }
        }
        Provider::OpenAi | Provider::Compatible => {
            if !key.is_empty() {
                headers.push(("Authorization", format!("Bearer {key}")));
            }
        }
    }
    headers
}

/// The URL, headers and body of one exchange. Pure, so each wire format has a
/// test without a network.
pub fn request(
    endpoint: &Endpoint,
    system: &str,
    messages: &[Message],
    tools: &[Tool],
    max_tokens: u32,
) -> (String, Vec<(&'static str, String)>, Value) {
    let base = endpoint.base();
    let headers = auth(endpoint);
    match endpoint.provider {
        Provider::Anthropic => {
            let mut body = json!({
                "model": endpoint.model,
                "max_tokens": max_tokens,
                "system": system,
                "messages": messages.iter().map(anthropic_message).collect::<Vec<_>>(),
            });
            if !tools.is_empty() {
                body["tools"] = tools
                    .iter()
                    .map(|t| json!({"name": t.name, "description": t.description, "input_schema": t.schema}))
                    .collect();
            }
            (format!("{base}/v1/messages"), headers, body)
        }
        Provider::OpenAi | Provider::Compatible => {
            let mut wire = vec![json!({"role": "system", "content": system})];
            for message in messages {
                wire.extend(openai_messages(message));
            }
            let mut body = json!({ "model": endpoint.model, "messages": wire });
            // OpenAI's own API renamed the cap; compatible servers know the
            // old name.
            let cap = if endpoint.provider == Provider::OpenAi {
                "max_completion_tokens"
            } else {
                "max_tokens"
            };
            body[cap] = json!(max_tokens);
            if !tools.is_empty() {
                body["tools"] = tools
                    .iter()
                    .map(|t| {
                        json!({"type": "function", "function": {
                            "name": t.name, "description": t.description, "parameters": t.schema
                        }})
                    })
                    .collect();
            }
            (format!("{base}/chat/completions"), headers, body)
        }
        Provider::Google => {
            let mut body = json!({
                "systemInstruction": {"parts": [{"text": system}]},
                "contents": messages.iter().map(google_message).collect::<Vec<_>>(),
                "generationConfig": {"maxOutputTokens": max_tokens},
            });
            if !tools.is_empty() {
                body["tools"] = json!([{ "functionDeclarations": tools
                    .iter()
                    .map(|t| json!({"name": t.name, "description": t.description, "parameters": t.schema}))
                    .collect::<Vec<_>>() }]);
            }
            let model = endpoint.model.trim_start_matches("models/");
            (
                format!("{base}/v1beta/models/{model}:generateContent"),
                headers,
                body,
            )
        }
    }
}

fn anthropic_message(message: &Message) -> Value {
    let content: Vec<Value> = message
        .parts
        .iter()
        .map(|part| match part {
            Part::Text { text } => json!({"type": "text", "text": text}),
            Part::Call { id, name, input } => {
                json!({"type": "tool_use", "id": id, "name": name, "input": input})
            }
            Part::Result {
                id,
                content,
                is_error,
                ..
            } => json!({"type": "tool_result", "tool_use_id": id, "content": content, "is_error": is_error}),
        })
        .collect();
    json!({"role": role(message.role), "content": content})
}

fn openai_messages(message: &Message) -> Vec<Value> {
    let mut out = Vec::new();
    let text: Vec<&str> = message
        .parts
        .iter()
        .filter_map(|p| match p {
            Part::Text { text } => Some(text.as_str()),
            _ => None,
        })
        .collect();
    let calls: Vec<Value> = message
        .parts
        .iter()
        .filter_map(|p| match p {
            Part::Call { id, name, input } => Some(json!({
                "id": id, "type": "function",
                "function": {"name": name, "arguments": input.to_string()}
            })),
            _ => None,
        })
        .collect();
    if !text.is_empty() || !calls.is_empty() {
        let mut entry = json!({"role": role(message.role), "content": text.join("\n")});
        if !calls.is_empty() {
            entry["tool_calls"] = Value::Array(calls);
        }
        out.push(entry);
    }
    for part in &message.parts {
        if let Part::Result { id, content, .. } = part {
            out.push(json!({"role": "tool", "tool_call_id": id, "content": content}));
        }
    }
    out
}

fn google_message(message: &Message) -> Value {
    let parts: Vec<Value> = message
        .parts
        .iter()
        .map(|part| match part {
            Part::Text { text } => json!({"text": text}),
            Part::Call { name, input, .. } => {
                json!({"functionCall": {"name": name, "args": input}})
            }
            Part::Result { name, content, .. } => {
                json!({"functionResponse": {"name": name, "response": {"content": content}}})
            }
        })
        .collect();
    let role = match message.role {
        Role::User => "user",
        Role::Assistant => "model",
    };
    json!({"role": role, "parts": parts})
}

fn role(role: Role) -> &'static str {
    match role {
        Role::User => "user",
        Role::Assistant => "assistant",
    }
}

/// The answer as JSON, or the provider's own error sentence.
fn parsed(endpoint: &Endpoint, answered: &http::Answered) -> Result<Value> {
    let value: Value = serde_json::from_str(&answered.body).unwrap_or(Value::Null);
    if (200..300).contains(&answered.status) && !value.is_null() {
        return Ok(value);
    }
    let said = error_sentence(&value);
    let detail = match (answered.status, said) {
        (_, Some(said)) => said,
        (401 | 403, None) => "refused the key".to_string(),
        (404, None) => "has no such model, or no such endpoint".to_string(),
        (429, None) => "is limiting requests; try again later".to_string(),
        (status, None) if status >= 500 => format!("had a problem of its own ({status})"),
        (status, None) => format!("answered {status}"),
    };
    Err(Error::Model {
        provider: endpoint.name(),
        detail: http::redact(&detail, &[("key", endpoint.key.as_str())]),
    })
}

/// The sentence a provider puts in its error body, whichever shape it uses.
pub fn error_sentence(value: &Value) -> Option<String> {
    let error = value.get("error")?;
    let message = error
        .get("message")
        .and_then(Value::as_str)
        .or_else(|| error.as_str())?;
    Some(message.trim().to_string())
}

/// Read one answer in a provider's shape.
pub fn read_turn(provider: Provider, value: &Value) -> Option<Turn> {
    let count = |v: Option<&Value>| v.and_then(Value::as_u64).unwrap_or(0);
    match provider {
        Provider::Anthropic => {
            let content = value.get("content")?.as_array()?;
            let parts = content
                .iter()
                .filter_map(|block| match block.get("type")?.as_str()? {
                    "text" => Some(Part::Text {
                        text: block.get("text")?.as_str()?.to_string(),
                    }),
                    "tool_use" => Some(Part::Call {
                        id: block.get("id")?.as_str()?.to_string(),
                        name: block.get("name")?.as_str()?.to_string(),
                        input: block.get("input").cloned().unwrap_or(json!({})),
                    }),
                    _ => None,
                })
                .collect();
            let stop = match value.get("stop_reason").and_then(Value::as_str) {
                Some("tool_use") => Stop::Tools,
                Some("max_tokens") => Stop::Length,
                Some("end_turn") | Some("stop_sequence") => Stop::Done,
                _ => Stop::Other,
            };
            let usage = value.get("usage");
            Some(Turn {
                parts,
                stop,
                input_tokens: count(usage.and_then(|u| u.get("input_tokens"))),
                output_tokens: count(usage.and_then(|u| u.get("output_tokens"))),
            })
        }
        Provider::OpenAi | Provider::Compatible => {
            let choice = value.get("choices")?.as_array()?.first()?;
            let message = choice.get("message")?;
            let mut parts = Vec::new();
            if let Some(text) = message.get("content").and_then(Value::as_str) {
                if !text.is_empty() {
                    parts.push(Part::Text {
                        text: text.to_string(),
                    });
                }
            }
            for call in message
                .get("tool_calls")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let function = call.get("function")?;
                let arguments = function
                    .get("arguments")
                    .and_then(Value::as_str)
                    .unwrap_or("{}");
                parts.push(Part::Call {
                    id: call
                        .get("id")
                        .and_then(Value::as_str)
                        .unwrap_or("call")
                        .to_string(),
                    name: function.get("name")?.as_str()?.to_string(),
                    input: serde_json::from_str(arguments).unwrap_or(json!({})),
                });
            }
            let has_calls = parts.iter().any(|p| matches!(p, Part::Call { .. }));
            let stop = match choice.get("finish_reason").and_then(Value::as_str) {
                _ if has_calls => Stop::Tools,
                Some("stop") => Stop::Done,
                Some("length") => Stop::Length,
                Some("tool_calls") => Stop::Tools,
                _ => Stop::Other,
            };
            let usage = value.get("usage");
            Some(Turn {
                parts,
                stop,
                input_tokens: count(usage.and_then(|u| u.get("prompt_tokens"))),
                output_tokens: count(usage.and_then(|u| u.get("completion_tokens"))),
            })
        }
        Provider::Google => {
            let candidate = value.get("candidates")?.as_array()?.first()?;
            let mut parts = Vec::new();
            for (index, part) in candidate
                .get("content")
                .and_then(|c| c.get("parts"))
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .enumerate()
            {
                if let Some(text) = part.get("text").and_then(Value::as_str) {
                    parts.push(Part::Text {
                        text: text.to_string(),
                    });
                } else if let Some(call) = part.get("functionCall") {
                    parts.push(Part::Call {
                        // Google gives a call no id; one is made up so the
                        // answer can be matched to it.
                        id: format!("call-{index}"),
                        name: call.get("name")?.as_str()?.to_string(),
                        input: call.get("args").cloned().unwrap_or(json!({})),
                    });
                }
            }
            let has_calls = parts.iter().any(|p| matches!(p, Part::Call { .. }));
            let stop = match candidate.get("finishReason").and_then(Value::as_str) {
                _ if has_calls => Stop::Tools,
                Some("STOP") => Stop::Done,
                Some("MAX_TOKENS") => Stop::Length,
                _ => Stop::Other,
            };
            let usage = value.get("usageMetadata");
            Some(Turn {
                parts,
                stop,
                input_tokens: count(usage.and_then(|u| u.get("promptTokenCount"))),
                output_tokens: count(usage.and_then(|u| u.get("candidatesTokenCount"))),
            })
        }
    }
}

/// The model names in a provider's list.
pub fn read_models(provider: Provider, value: &Value) -> Vec<String> {
    let mut names: Vec<String> = match provider {
        Provider::Google => value
            .get("models")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter(|model| {
                model
                    .get("supportedGenerationMethods")
                    .and_then(Value::as_array)
                    .is_some_and(|methods| methods.iter().any(|m| m == "generateContent"))
            })
            .filter_map(|model| model.get("name")?.as_str())
            .map(|name| name.trim_start_matches("models/").to_string())
            .collect(),
        _ => value
            .get("data")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|model| model.get("id")?.as_str().map(str::to_string))
            .collect(),
    };
    names.sort();
    names.dedup();
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    fn endpoint(provider: Provider) -> Endpoint {
        Endpoint {
            provider,
            base: String::new(),
            model: "m-1".into(),
            key: "secret-key-123".into(),
        }
    }

    fn exchange() -> Vec<Message> {
        vec![
            Message::user("Review this."),
            Message {
                role: Role::Assistant,
                parts: vec![
                    Part::Text {
                        text: "Reading.".into(),
                    },
                    Part::Call {
                        id: "c1".into(),
                        name: "read_file".into(),
                        input: json!({"path": "a.rs"}),
                    },
                ],
            },
            Message {
                role: Role::User,
                parts: vec![Part::Result {
                    id: "c1".into(),
                    name: "read_file".into(),
                    content: "fn main() {}".into(),
                    is_error: false,
                }],
            },
        ]
    }

    fn tools() -> Vec<Tool> {
        vec![Tool {
            name: "read_file",
            description: "Read a file.",
            schema: json!({"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]}),
        }]
    }

    #[test]
    fn anthropic_is_asked_in_its_own_shape() {
        let (url, headers, body) = request(
            &endpoint(Provider::Anthropic),
            "sys",
            &exchange(),
            &tools(),
            1000,
        );
        assert_eq!(url, "https://api.anthropic.com/v1/messages");
        assert!(headers.contains(&("x-api-key", "secret-key-123".into())));
        assert!(headers.contains(&("anthropic-version", "2023-06-01".into())));
        assert_eq!(body["system"], "sys");
        assert_eq!(body["messages"][1]["content"][1]["type"], "tool_use");
        assert_eq!(body["messages"][2]["content"][0]["tool_use_id"], "c1");
        assert_eq!(body["tools"][0]["input_schema"]["type"], "object");
    }

    #[test]
    fn openai_is_asked_in_its_own_shape() {
        let (url, headers, body) = request(
            &endpoint(Provider::OpenAi),
            "sys",
            &exchange(),
            &tools(),
            1000,
        );
        assert_eq!(url, "https://api.openai.com/v1/chat/completions");
        assert!(headers.contains(&("Authorization", "Bearer secret-key-123".into())));
        assert_eq!(body["messages"][0]["role"], "system");
        assert_eq!(
            body["messages"][2]["tool_calls"][0]["function"]["arguments"],
            r#"{"path":"a.rs"}"#
        );
        assert_eq!(body["messages"][3]["role"], "tool");
        assert_eq!(body["max_completion_tokens"], 1000);
        assert_eq!(body["tools"][0]["function"]["name"], "read_file");
    }

    #[test]
    fn a_compatible_endpoint_uses_its_base_and_the_old_cap() {
        let mut local = endpoint(Provider::Compatible);
        local.base = "http://localhost:11434/v1/".into();
        local.key.clear();
        let (url, headers, body) = request(&local, "sys", &[Message::user("hi")], &[], 10);
        assert_eq!(url, "http://localhost:11434/v1/chat/completions");
        assert!(headers.is_empty(), "no key, no header");
        assert_eq!(body["max_tokens"], 10);
        assert!(local.is_local());
        assert_eq!(local.name(), "localhost:11434");
    }

    #[test]
    fn google_is_asked_in_its_own_shape() {
        let (url, headers, body) = request(
            &endpoint(Provider::Google),
            "sys",
            &exchange(),
            &tools(),
            1000,
        );
        assert_eq!(
            url,
            "https://generativelanguage.googleapis.com/v1beta/models/m-1:generateContent"
        );
        assert!(headers.contains(&("x-goog-api-key", "secret-key-123".into())));
        assert_eq!(body["contents"][1]["role"], "model");
        assert_eq!(
            body["contents"][1]["parts"][1]["functionCall"]["name"],
            "read_file"
        );
        assert_eq!(
            body["contents"][2]["parts"][0]["functionResponse"]["name"],
            "read_file"
        );
        assert_eq!(
            body["tools"][0]["functionDeclarations"][0]["name"],
            "read_file"
        );
    }

    #[test]
    fn each_answer_is_read_with_its_tool_calls_and_tokens() {
        let anthropic = json!({
            "content": [{"type": "text", "text": "Hm."}, {"type": "tool_use", "id": "t1", "name": "read_file", "input": {"path": "a"}}],
            "stop_reason": "tool_use", "usage": {"input_tokens": 10, "output_tokens": 3}
        });
        let turn = read_turn(Provider::Anthropic, &anthropic).unwrap();
        assert_eq!(turn.stop, Stop::Tools);
        assert_eq!(turn.calls()[0].1, "read_file");
        assert_eq!((turn.input_tokens, turn.output_tokens), (10, 3));

        let openai = json!({
            "choices": [{"message": {"content": null, "tool_calls": [{"id": "x", "type": "function", "function": {"name": "search", "arguments": "{\"pattern\":\"fn\"}"}}]}, "finish_reason": "tool_calls"}],
            "usage": {"prompt_tokens": 7, "completion_tokens": 2}
        });
        let turn = read_turn(Provider::OpenAi, &openai).unwrap();
        assert_eq!(turn.calls()[0].2, json!({"pattern": "fn"}));
        assert_eq!(turn.stop, Stop::Tools);

        let google = json!({
            "candidates": [{"content": {"parts": [{"text": "Done."}]}, "finishReason": "STOP"}],
            "usageMetadata": {"promptTokenCount": 5, "candidatesTokenCount": 1}
        });
        let turn = read_turn(Provider::Google, &google).unwrap();
        assert_eq!(turn.text(), "Done.");
        assert_eq!(turn.stop, Stop::Done);
    }

    #[test]
    fn a_providers_own_error_sentence_is_what_is_said() {
        let body = json!({"type": "error", "error": {"type": "authentication_error", "message": "invalid x-api-key"}});
        assert_eq!(error_sentence(&body).as_deref(), Some("invalid x-api-key"));
        let answered = http::Answered {
            status: 401,
            body: body.to_string(),
        };
        let error = parsed(&endpoint(Provider::Anthropic), &answered).unwrap_err();
        assert_eq!(error.to_string(), "Anthropic: invalid x-api-key");
        let silent = http::Answered {
            status: 401,
            body: String::new(),
        };
        assert_eq!(
            parsed(&endpoint(Provider::OpenAi), &silent)
                .unwrap_err()
                .to_string(),
            "OpenAI: refused the key"
        );
    }

    #[test]
    fn model_lists_are_read_from_each_shape() {
        let listed = json!({"data": [{"id": "b"}, {"id": "a"}]});
        assert_eq!(read_models(Provider::Anthropic, &listed), ["a", "b"]);
        let google = json!({"models": [
            {"name": "models/gemini-x", "supportedGenerationMethods": ["generateContent"]},
            {"name": "models/embed", "supportedGenerationMethods": ["embedContent"]}
        ]});
        assert_eq!(read_models(Provider::Google, &google), ["gemini-x"]);
    }
}
