// SPDX-License-Identifier: GPL-3.0-or-later

//! A saved call made into the request [`crate::calls::send`] sends.
//!
//! Every `{{name}}` is resolved, the auth becomes its header or query
//! parameter, the body is encoded, and the `Content-Type` is set when the
//! call did not set one. The same is built a second time with secrets and
//! kept values left as their names: that copy is what history records and
//! what the screen shows as "sent".

use std::path::Path;

use crate::calls::model::{AuthType, BodyType, Call, KeyPlace, Kind};
use crate::calls::send::Outgoing;
use crate::calls::vars::{Missing, Scope};

/// What was sent, as it may be shown and kept: secrets as `{{name}}`.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shown {
    pub method: String,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Built {
    pub out: Outgoing,
    pub shown: Shown,
}

/// Why a call could not be built.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Unbuilt {
    Missing(Missing),
    /// A multipart file that could not be read, in a sentence.
    File(String),
}

impl std::fmt::Display for Unbuilt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Unbuilt::Missing(missing) => missing.fmt(f),
            Unbuilt::File(why) => f.write_str(why),
        }
    }
}

impl From<Missing> for Unbuilt {
    fn from(missing: Missing) -> Self {
        Unbuilt::Missing(missing)
    }
}

/// Build `call` in `scope`. `every` is Settings › Calls' *Header on every
/// call*, added unless the call sets the same header. `root` is where a
/// multipart file's relative path starts. `envelope` replaces a SOAP call's
/// envelope when WS-Security has added to it.
pub fn build(
    call: &Call,
    scope: &Scope,
    every: &[(String, String)],
    root: &Path,
    envelope: Option<&str>,
) -> Result<Built, Unbuilt> {
    let real = one(call, scope, every, root, envelope, false)?;
    let shown = one(call, scope, every, root, envelope, true)?;
    Ok(Built {
        shown: Shown {
            method: shown.method,
            url: shown.url,
            headers: shown.headers,
            body: shown
                .body
                .map(|bytes| String::from_utf8_lossy(&bytes).into_owned()),
        },
        out: real,
    })
}

fn one(
    call: &Call,
    scope: &Scope,
    every: &[(String, String)],
    root: &Path,
    envelope: Option<&str>,
    redact: bool,
) -> Result<Outgoing, Unbuilt> {
    let fill = |text: &str| -> Result<String, Missing> {
        if redact {
            scope.redacted(text)
        } else {
            scope.resolve(text)
        }
    };

    let mut url = with_scheme(fill(call.url.trim())?);
    let mut headers: Vec<(String, String)> = Vec::new();
    for pair in call.headers.iter().filter(|pair| pair.on) {
        let name = fill(pair.name.trim())?;
        if name.is_empty() {
            continue;
        }
        headers.push((name, fill(&pair.value)?));
    }

    if let Some(auth) = &call.auth {
        let get = |value: &Option<String>| fill(value.as_deref().unwrap_or(""));
        match auth.kind {
            AuthType::None => {}
            AuthType::Bearer => set(
                &mut headers,
                "Authorization",
                format!("Bearer {}", get(&auth.token)?),
            ),
            AuthType::Basic => {
                let pair = format!("{}:{}", get(&auth.user)?, get(&auth.password)?);
                set(
                    &mut headers,
                    "Authorization",
                    format!("Basic {}", base64(pair.as_bytes())),
                );
            }
            AuthType::ApiKey => {
                let key = get(&auth.key)?;
                let value = get(&auth.value)?;
                if !key.is_empty() {
                    match auth.place.unwrap_or_default() {
                        KeyPlace::Header => set(&mut headers, &key, value),
                        KeyPlace::Query => {
                            url.push(if url.contains('?') { '&' } else { '?' });
                            url.push_str(&encode(&key));
                            url.push('=');
                            url.push_str(&encode(&value));
                        }
                    }
                }
            }
        }
    }

    let body = match call.kind {
        Kind::Soap => {
            let text = match envelope {
                Some(text) => text.to_string(),
                None => call.envelope.clone().unwrap_or_default(),
            };
            let soap12 = call.version.as_deref() == Some("1.2");
            let action = fill(call.action.as_deref().unwrap_or(""))?;
            if soap12 {
                let mut kind = "application/soap+xml; charset=utf-8".to_string();
                if !action.is_empty() {
                    kind.push_str(&format!("; action=\"{action}\""));
                }
                default(&mut headers, "Content-Type", kind);
            } else {
                default(
                    &mut headers,
                    "Content-Type",
                    "text/xml; charset=utf-8".into(),
                );
                default(&mut headers, "SOAPAction", format!("\"{action}\""));
            }
            Some(fill(&text)?.into_bytes())
        }
        Kind::Rest => match &call.body {
            None => None,
            Some(body) => match body.kind {
                BodyType::None => None,
                BodyType::Json | BodyType::Xml | BodyType::Raw => {
                    let kind = match body.kind {
                        BodyType::Json => "application/json".to_string(),
                        BodyType::Xml => "application/xml".to_string(),
                        _ => body
                            .content_type
                            .clone()
                            .unwrap_or_else(|| "text/plain".to_string()),
                    };
                    default(&mut headers, "Content-Type", kind);
                    Some(fill(body.text.as_deref().unwrap_or(""))?.into_bytes())
                }
                BodyType::Form => {
                    default(
                        &mut headers,
                        "Content-Type",
                        "application/x-www-form-urlencoded".into(),
                    );
                    let mut parts = Vec::new();
                    for field in body.fields.iter().filter(|field| field.on) {
                        parts.push(format!(
                            "{}={}",
                            encode(&fill(&field.name)?),
                            encode(&fill(&field.value)?)
                        ));
                    }
                    Some(parts.join("&").into_bytes())
                }
                BodyType::Multipart => {
                    let boundary = "----spagitty-calls-boundary-7MA4YWxkTrZu0gW";
                    // The person may have set multipart/form-data without a
                    // boundary; the one used is the one that counts.
                    headers.retain(|(name, _)| !name.eq_ignore_ascii_case("content-type"));
                    headers.push((
                        "Content-Type".into(),
                        format!("multipart/form-data; boundary={boundary}"),
                    ));
                    let mut bytes = Vec::new();
                    for field in body.fields.iter().filter(|field| field.on) {
                        let name = fill(&field.name)?;
                        let value = fill(&field.value)?;
                        bytes.extend_from_slice(format!("--{boundary}\r\n").as_bytes());
                        if field.file {
                            let path = root.join(&value);
                            let file_name = Path::new(&value)
                                .file_name()
                                .map(|n| n.to_string_lossy().into_owned())
                                .unwrap_or_else(|| value.clone());
                            bytes.extend_from_slice(
                                format!(
                                    "Content-Disposition: form-data; name=\"{}\"; filename=\"{}\"\r\nContent-Type: application/octet-stream\r\n\r\n",
                                    quoted(&name),
                                    quoted(&file_name)
                                )
                                .as_bytes(),
                            );
                            if redact {
                                bytes.extend_from_slice(format!("<{value}>").as_bytes());
                            } else {
                                let content = std::fs::read(&path).map_err(|e| {
                                    Unbuilt::File(format!("{value} could not be read: {e}"))
                                })?;
                                bytes.extend_from_slice(&content);
                            }
                        } else {
                            bytes.extend_from_slice(
                                format!(
                                    "Content-Disposition: form-data; name=\"{}\"\r\n\r\n",
                                    quoted(&name)
                                )
                                .as_bytes(),
                            );
                            bytes.extend_from_slice(value.as_bytes());
                        }
                        bytes.extend_from_slice(b"\r\n");
                    }
                    bytes.extend_from_slice(format!("--{boundary}--\r\n").as_bytes());
                    Some(bytes)
                }
            },
        },
    };

    for (name, value) in every {
        default(&mut headers, name, value.clone());
    }

    Ok(Outgoing {
        method: call.method().to_ascii_uppercase(),
        url,
        headers,
        body,
    })
}

/// A URL typed without a scheme is `http://`: what a browser's address bar
/// does, and what `localhost:8080/x` means.
fn with_scheme(url: String) -> String {
    let lower = url.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") || url.is_empty() {
        url
    } else {
        format!("http://{url}")
    }
}

fn set(headers: &mut Vec<(String, String)>, name: &str, value: String) {
    headers.retain(|(key, _)| !key.eq_ignore_ascii_case(name));
    headers.push((name.to_string(), value));
}

fn default(headers: &mut Vec<(String, String)>, name: &str, value: String) {
    if !headers
        .iter()
        .any(|(key, _)| key.eq_ignore_ascii_case(name))
    {
        headers.push((name.to_string(), value));
    }
}

fn quoted(text: &str) -> String {
    text.replace('\\', "\\\\").replace('"', "\\\"")
}

/// `text` percent-encoded for a query or a form: everything but the
/// unreserved characters.
pub fn encode(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// `text` with `%XX` and `+` decoded, as a form or a query is read.
pub fn decode(text: &str) -> String {
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while at < bytes.len() {
        match bytes[at] {
            b'+' => out.push(b' '),
            b'%' if at + 2 < bytes.len() => {
                let hex = std::str::from_utf8(&bytes[at + 1..at + 3]).unwrap_or("");
                match u8::from_str_radix(hex, 16) {
                    Ok(byte) => {
                        out.push(byte);
                        at += 2;
                    }
                    Err(_) => out.push(b'%'),
                }
            }
            other => out.push(other),
        }
        at += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

/// Standard base64, with padding.
pub fn base64(bytes: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for chunk in bytes.chunks(3) {
        let n = (chunk[0] as u32) << 16
            | (*chunk.get(1).unwrap_or(&0) as u32) << 8
            | *chunk.get(2).unwrap_or(&0) as u32;
        out.push(TABLE[(n >> 18) as usize & 63] as char);
        out.push(TABLE[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 {
            TABLE[(n >> 6) as usize & 63] as char
        } else {
            '='
        });
        out.push(if chunk.len() > 2 {
            TABLE[n as usize & 63] as char
        } else {
            '='
        });
    }
    out
}

/// Standard or URL-safe base64 read back; `None` when it is not base64.
pub fn unbase64(text: &str) -> Option<Vec<u8>> {
    let mut bits = 0u32;
    let mut held = 0;
    let mut out = Vec::new();
    for c in text
        .bytes()
        .filter(|c| !c.is_ascii_whitespace() && *c != b'=')
    {
        let value = match c {
            b'A'..=b'Z' => c - b'A',
            b'a'..=b'z' => c - b'a' + 26,
            b'0'..=b'9' => c - b'0' + 52,
            b'+' | b'-' => 62,
            b'/' | b'_' => 63,
            _ => return None,
        } as u32;
        bits = bits << 6 | value;
        held += 6;
        if held >= 8 {
            held -= 8;
            out.push((bits >> held) as u8);
            bits &= (1 << held) - 1;
        }
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calls::model::{Auth, Body, Field, Pair};
    use crate::calls::vars::Value;
    use std::collections::HashMap;

    fn scope() -> Scope {
        let mut layer = HashMap::new();
        for (name, value, private) in [
            ("baseUrl", "https://api.test", false),
            ("email", "mira@example.com", false),
            ("password", "hunter22", true),
            ("token", "eyJtoken", true),
        ] {
            layer.insert(
                name.to_string(),
                Value {
                    value: value.into(),
                    private,
                },
            );
        }
        Scope {
            environment: Some("staging".into()),
            layers: vec![layer],
        }
    }

    fn login() -> Call {
        Call {
            id: "c_1".into(),
            name: "Log in".into(),
            method: Some("post".into()),
            url: "{{baseUrl}}/auth/login".into(),
            headers: vec![Pair::new("Accept", "application/json")],
            body: Some(Body {
                kind: BodyType::Json,
                text: Some("{\"email\":\"{{email}}\",\"password\":\"{{password}}\"}".into()),
                ..Body::default()
            }),
            ..Call::default()
        }
    }

    #[test]
    fn a_json_call_is_resolved_and_typed() {
        let every = vec![("User-Agent".to_string(), "Spagitty/4.0.0".to_string())];
        let built = build(&login(), &scope(), &every, Path::new("."), None).unwrap();
        assert_eq!(built.out.method, "POST");
        assert_eq!(built.out.url, "https://api.test/auth/login");
        assert_eq!(
            built.out.body.as_deref(),
            Some(&b"{\"email\":\"mira@example.com\",\"password\":\"hunter22\"}"[..])
        );
        assert!(built
            .out
            .headers
            .contains(&("Content-Type".into(), "application/json".into())));
        assert!(built
            .out
            .headers
            .contains(&("User-Agent".into(), "Spagitty/4.0.0".into())));
        // History keeps the secret as its name.
        let shown = built.shown.body.unwrap();
        assert!(shown.contains("{{password}}"));
        assert!(!shown.contains("hunter22"));
    }

    #[test]
    fn auth_is_a_header_or_a_parameter() {
        let mut call = login();
        call.auth = Some(Auth {
            kind: AuthType::Bearer,
            token: Some("{{token}}".into()),
            ..Auth::default()
        });
        let built = build(&call, &scope(), &[], Path::new("."), None).unwrap();
        assert!(built
            .out
            .headers
            .contains(&("Authorization".into(), "Bearer eyJtoken".into())));
        assert!(built
            .shown
            .headers
            .contains(&("Authorization".into(), "Bearer {{token}}".into())));

        call.auth = Some(Auth {
            kind: AuthType::Basic,
            user: Some("{{email}}".into()),
            password: Some("{{password}}".into()),
            ..Auth::default()
        });
        let built = build(&call, &scope(), &[], Path::new("."), None).unwrap();
        let expected = format!("Basic {}", base64(b"mira@example.com:hunter22"));
        assert!(built
            .out
            .headers
            .contains(&("Authorization".into(), expected)));

        call.auth = Some(Auth {
            kind: AuthType::ApiKey,
            key: Some("api key".into()),
            value: Some("{{token}}".into()),
            place: Some(KeyPlace::Query),
            ..Auth::default()
        });
        let built = build(&call, &scope(), &[], Path::new("."), None).unwrap();
        assert_eq!(
            built.out.url,
            "https://api.test/auth/login?api%20key=eyJtoken"
        );
    }

    #[test]
    fn a_call_header_wins_over_the_one_on_every_call() {
        let mut call = login();
        call.headers.push(Pair::new("user-agent", "mine"));
        call.headers.push(Pair {
            on: false,
            ..Pair::new("X-Off", "1")
        });
        let every = vec![("User-Agent".to_string(), "Spagitty".to_string())];
        let built = build(&call, &scope(), &every, Path::new("."), None).unwrap();
        let agents: Vec<_> = built
            .out
            .headers
            .iter()
            .filter(|(name, _)| name.eq_ignore_ascii_case("user-agent"))
            .collect();
        assert_eq!(
            agents,
            vec![&("user-agent".to_string(), "mine".to_string())]
        );
        assert!(!built.out.headers.iter().any(|(name, _)| name == "X-Off"));
    }

    #[test]
    fn a_missing_name_stops_the_build() {
        let mut call = login();
        call.url = "{{host}}/x".into();
        let error = build(&call, &scope(), &[], Path::new("."), None).unwrap_err();
        assert_eq!(error.to_string(), "{{host}} has no value in staging");
    }

    #[test]
    fn forms_and_multipart_are_encoded() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "file body").unwrap();
        let mut call = login();
        call.body = Some(Body {
            kind: BodyType::Form,
            fields: vec![
                Field {
                    name: "q".into(),
                    value: "a b&c".into(),
                    file: false,
                    on: true,
                },
                Field {
                    name: "off".into(),
                    value: "x".into(),
                    file: false,
                    on: false,
                },
            ],
            ..Body::default()
        });
        let built = build(&call, &scope(), &[], dir.path(), None).unwrap();
        assert_eq!(built.out.body.as_deref(), Some(&b"q=a%20b%26c"[..]));

        call.body = Some(Body {
            kind: BodyType::Multipart,
            fields: vec![
                Field {
                    name: "who".into(),
                    value: "{{email}}".into(),
                    file: false,
                    on: true,
                },
                Field {
                    name: "doc".into(),
                    value: "a.txt".into(),
                    file: true,
                    on: true,
                },
            ],
            ..Body::default()
        });
        let built = build(&call, &scope(), &[], dir.path(), None).unwrap();
        let body = String::from_utf8(built.out.body.unwrap()).unwrap();
        assert!(body.contains("name=\"who\"\r\n\r\nmira@example.com"));
        assert!(body.contains("filename=\"a.txt\""));
        assert!(body.contains("file body"));
        assert!(built.shown.body.unwrap().contains("<a.txt>"));
    }

    #[test]
    fn soap_headers_follow_the_version() {
        let call = Call {
            kind: Kind::Soap,
            url: "{{baseUrl}}/ws".into(),
            version: Some("1.1".into()),
            action: Some("urn:Get".into()),
            envelope: Some("<e/>".into()),
            ..Call::default()
        };
        let built = build(&call, &scope(), &[], Path::new("."), None).unwrap();
        assert_eq!(built.out.method, "POST");
        assert!(built
            .out
            .headers
            .contains(&("SOAPAction".into(), "\"urn:Get\"".into())));
        assert!(built
            .out
            .headers
            .contains(&("Content-Type".into(), "text/xml; charset=utf-8".into())));
        let call = Call {
            version: Some("1.2".into()),
            ..call
        };
        let built = build(&call, &scope(), &[], Path::new("."), None).unwrap();
        assert!(built.out.headers.contains(&(
            "Content-Type".into(),
            "application/soap+xml; charset=utf-8; action=\"urn:Get\"".into()
        )));
    }

    #[test]
    fn a_url_without_a_scheme_is_http() {
        let mut call = login();
        call.url = "localhost:8080/x".into();
        let built = build(&call, &scope(), &[], Path::new("."), None).unwrap();
        assert_eq!(built.out.url, "http://localhost:8080/x");
    }

    #[test]
    fn base64_and_percent_round_trip() {
        for text in ["", "f", "fo", "foo", "foob", "fooba", "foobar"] {
            assert_eq!(unbase64(&base64(text.as_bytes())).unwrap(), text.as_bytes());
        }
        assert_eq!(base64(b"foobar"), "Zm9vYmFy");
        assert_eq!(decode(&encode("a b&c=d/é")), "a b&c=d/é");
        assert_eq!(decode("a+b%2"), "a b%2");
    }
}
