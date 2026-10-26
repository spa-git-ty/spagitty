// SPDX-License-Identifier: GPL-3.0-or-later

//! `.http` and `.rest` files (FEAT-162, proposal §8.3), as the JetBrains
//! HTTP client and VS Code's REST Client write them.
//!
//! Requests are separated by `###`. `# @name x` names one; `@baseUrl = …`
//! at the top is a variable of the file. A request is its request line, its
//! headers to the first blank line, and its body to the next `###`. A
//! response handler that only copies a value —
//! `client.global.set("token", response.body.token)` — becomes a keep rule;
//! any other handler is kept in the notes.

use super::super::model::{new_id, Body, BodyType, Call, KeepRule, Pair, Variable};
use super::{key_of, path_of, Endpoint, Status};

const METHODS: [&str; 9] = [
    "GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS", "TRACE", "CONNECT",
];

/// The file's requests, and its file-level variables.
pub fn read(file: &str, text: &str) -> (Vec<Endpoint>, Vec<Variable>) {
    let setter = regex::Regex::new(r#"client\.global\.set\(\s*["']([^"']+)["']\s*,\s*response\.body((?:\.[\w$]+|\[\d+\]|\[["'][^"']+["']\])*)\s*\)"#)
        .expect("a valid pattern");
    let mut out = Vec::new();
    let mut vars = Vec::new();
    for (index, chunk) in text.split("\n###").enumerate() {
        let chunk = if index == 0 {
            chunk.trim_start_matches("###")
        } else {
            chunk
        };
        let mut lines = chunk.lines().peekable();
        // The rest of the `###` line may name the request.
        let mut name = if index == 0 {
            None
        } else {
            lines
                .next()
                .map(str::trim)
                .filter(|n| !n.is_empty())
                .map(str::to_string)
        };
        let mut request_line = None;
        for line in lines.by_ref() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Some(rest) = trimmed
                .strip_prefix('#')
                .or_else(|| trimmed.strip_prefix("//"))
            {
                if let Some(given) = rest.trim().strip_prefix("@name") {
                    name = Some(given.trim().trim_start_matches('=').trim().to_string());
                }
                continue;
            }
            if let Some(rest) = trimmed.strip_prefix('@') {
                if let Some((key, value)) = rest.split_once('=') {
                    vars.push(Variable::plain(key.trim(), value.trim()));
                    continue;
                }
            }
            request_line = Some(trimmed.to_string());
            break;
        }
        let Some(request_line) = request_line else {
            continue;
        };
        let mut words = request_line.split_whitespace();
        let first = words.next().unwrap_or("");
        let (method, url) = if METHODS.contains(&first.to_ascii_uppercase().as_str()) {
            (
                first.to_ascii_uppercase(),
                words.next().unwrap_or("").to_string(),
            )
        } else {
            ("GET".to_string(), first.to_string())
        };
        // A long URL may continue on indented `?` or `&` lines.
        let mut url = url;
        while let Some(next) = lines.peek() {
            let t = next.trim();
            if next.starts_with(char::is_whitespace) && (t.starts_with('?') || t.starts_with('&')) {
                url.push_str(t);
                lines.next();
            } else {
                break;
            }
        }
        let mut headers = Vec::new();
        for line in lines.by_ref() {
            if line.trim().is_empty() {
                break;
            }
            if let Some((key, value)) = line.split_once(':') {
                headers.push(Pair::new(key.trim(), value.trim()));
            }
        }
        let rest: Vec<&str> = lines.collect();
        let rest = rest.join("\n");
        let (body_text, handler) = match rest.find("> {%") {
            Some(at) => (rest[..at].trim().to_string(), Some(rest[at..].to_string())),
            None => (rest.trim().to_string(), None),
        };
        let mut keep = Vec::new();
        let mut notes = None;
        if let Some(handler) = handler {
            for found in setter.captures_iter(&handler) {
                let path = found[2].replace('"', "'");
                keep.push(KeepRule {
                    var: found[1].to_string(),
                    from: format!("${path}"),
                    every: true,
                    ..KeepRule::default()
                });
            }
            let left = setter.replace_all(&handler, "");
            let left = left
                .trim_start_matches("> {%")
                .trim_end()
                .trim_end_matches("%}")
                .trim()
                .trim_matches(';')
                .trim();
            if !left.is_empty() {
                notes = Some(format!("Response handler, not run:\n{}", handler.trim()));
            }
        }
        let content_type = headers
            .iter()
            .find(|h: &&Pair| h.name.eq_ignore_ascii_case("content-type"))
            .map(|h| h.value.to_ascii_lowercase())
            .unwrap_or_default();
        let body = (!body_text.is_empty()).then(|| Body {
            kind: if content_type.contains("json")
                || serde_json::from_str::<serde_json::Value>(&body_text).is_ok()
            {
                BodyType::Json
            } else if content_type.contains("xml") {
                BodyType::Xml
            } else {
                BodyType::Raw
            },
            text: Some(body_text),
            ..Body::default()
        });
        let call = Call {
            id: new_id(),
            name: name.unwrap_or_else(|| super::name_for(&method, &path_of(&url), None)),
            method: Some(method),
            url,
            headers,
            body,
            keep,
            notes,
            ..Call::default()
        };
        out.push(Endpoint {
            id: format!("{file}#{}", out.len()),
            key: key_of(&call),
            source: format!("file:{file}"),
            group: file.rsplit('/').next().unwrap_or(file).to_string(),
            file: Some(file.to_string()),
            line: None,
            method: call.method().to_string(),
            path: path_of(&call.url),
            hint: String::new(),
            call,
            status: Status::New,
            declared: None,
            folder: None,
        });
    }
    (out, vars)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requests_are_read_between_separators() {
        let text = "@baseUrl = http://localhost:8080\n@tenant = acme\n\n### Log in\nPOST {{baseUrl}}/auth/login HTTP/1.1\nContent-Type: application/json\n\n{\"email\": \"a@b.c\"}\n\n> {%\n  client.global.set(\"token\", response.body.data.accessToken);\n%}\n\n###\n# @name List orders\nGET {{baseUrl}}/orders\n    ?page=1\n    &size=20\nAccept: application/json\n\n###\n{{baseUrl}}/health\n";
        let (endpoints, vars) = read("api/orders.http", text);
        assert_eq!(vars.len(), 2);
        let got: Vec<(String, String, String)> = endpoints
            .iter()
            .map(|e| (e.call.name.clone(), e.method.clone(), e.call.url.clone()))
            .collect();
        assert_eq!(
            got,
            vec![
                (
                    "Log in".into(),
                    "POST".into(),
                    "{{baseUrl}}/auth/login".into()
                ),
                (
                    "List orders".into(),
                    "GET".into(),
                    "{{baseUrl}}/orders?page=1&size=20".into()
                ),
                (
                    "Get health".into(),
                    "GET".into(),
                    "{{baseUrl}}/health".into()
                ),
            ]
        );
        let login = &endpoints[0].call;
        assert_eq!(
            login.body.as_ref().unwrap().text.as_deref(),
            Some("{\"email\": \"a@b.c\"}")
        );
        assert_eq!(login.keep[0].from, "$.data.accessToken");
        assert!(login.notes.is_none());
        assert_eq!(endpoints[1].call.headers[0].name, "Accept");
    }
}
