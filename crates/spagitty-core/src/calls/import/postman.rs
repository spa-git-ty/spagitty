// SPDX-License-Identifier: GPL-3.0-or-later

//! Postman collections, v2.0 and v2.1, and Postman environments (FEAT-162,
//! proposal §7.3).
//!
//! Folders stay folders, `{{variables}}` stay as they are, and `:id` in a
//! path becomes `{{id}}`. Scripts are not run (§2, rule 5). The test-script
//! lines that only copy a response value into a variable —
//! `pm.environment.set("token", pm.response.json().data.accessToken)` —
//! become keep rules; every other line is kept in the call's notes, so
//! nothing a person wrote is lost, and Import says how many calls had some.

use serde_json::Value;

use super::super::model::{
    new_id, Auth, AuthType, Body, BodyType, Call, Environment, Field, KeepRule, KeyPlace, Pair,
    Variable,
};
use super::{key_of, path_of, Endpoint, Status};

/// Whether `value` is a Postman collection.
pub fn is_collection(value: &Value) -> bool {
    value["info"]["schema"]
        .as_str()
        .is_some_and(|s| s.contains("getpostman.com") || s.contains("postman"))
        && value["item"].is_array()
}

/// Whether `value` is a Postman environment.
pub fn is_environment(value: &Value) -> bool {
    value["values"].is_array()
        && (value["_postman_variable_scope"].as_str() == Some("environment")
            || value["name"].is_string())
        && !value["item"].is_array()
}

fn text(value: &Value) -> String {
    match value {
        Value::String(s) => s.clone(),
        Value::Null => String::new(),
        other => other.to_string(),
    }
}

/// `:id` path segments as `{{id}}`, everything else as written.
fn path_variables(url: &str) -> String {
    let (head, query) = match url.split_once('?') {
        Some((h, q)) => (h, Some(q)),
        None => (url, None),
    };
    let parts: Vec<String> = head
        .split('/')
        .map(|segment| match segment.strip_prefix(':') {
            Some(name) if !name.is_empty() && !segment.contains("//") => format!("{{{{{name}}}}}"),
            _ => segment.to_string(),
        })
        .collect();
    let head = parts.join("/");
    match query {
        Some(q) => format!("{head}?{q}"),
        None => head,
    }
}

fn url_of(url: &Value) -> String {
    let raw = match url {
        Value::String(s) => s.clone(),
        Value::Object(_) => url["raw"].as_str().map(str::to_string).unwrap_or_else(|| {
            let host = match &url["host"] {
                Value::Array(parts) => parts.iter().map(text).collect::<Vec<_>>().join("."),
                other => text(other),
            };
            let path = match &url["path"] {
                Value::Array(parts) => parts.iter().map(text).collect::<Vec<_>>().join("/"),
                other => text(other),
            };
            let protocol = url["protocol"]
                .as_str()
                .map(|p| format!("{p}://"))
                .unwrap_or_default();
            format!("{protocol}{host}/{path}")
        }),
        _ => String::new(),
    };
    path_variables(&raw)
}

/// `pm.response.json().data.items[0]["id"]` as `$.data.items[0]['id']`;
/// `alias` is a name the response's JSON was put in on an earlier line.
fn json_path(expression: &str, aliases: &[String]) -> Option<String> {
    let expression = expression.trim().trim_end_matches(';').trim();
    let rest = ["pm.response.json()", "JSON.parse(responseBody)"]
        .iter()
        .find_map(|start| expression.strip_prefix(start))
        .or_else(|| {
            aliases.iter().find_map(|alias| {
                expression
                    .strip_prefix(alias.as_str())
                    .filter(|r| r.is_empty() || r.starts_with(['.', '[']))
            })
        })?;
    let mut path = String::from("$");
    let mut chars = rest.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '.' => {
                let name: String = std::iter::from_fn(|| {
                    chars.next_if(|c| c.is_alphanumeric() || *c == '_' || *c == '$')
                })
                .collect();
                if name.is_empty() {
                    return None;
                }
                path.push('.');
                path.push_str(&name);
            }
            '[' => {
                let inner: String = std::iter::from_fn(|| chars.next_if(|c| *c != ']')).collect();
                chars.next();
                let inner = inner.trim();
                if inner.chars().all(|c| c.is_ascii_digit()) && !inner.is_empty() {
                    path.push_str(&format!("[{inner}]"));
                } else {
                    let key = inner.trim_matches(['"', '\'']);
                    path.push_str(&format!("['{key}']"));
                }
            }
            c if c.is_whitespace() => {}
            _ => return None,
        }
    }
    Some(path)
}

/// Keep rules from a test script's lines, and the lines that are not one.
pub fn rules_from(lines: &[String]) -> (Vec<KeepRule>, Vec<String>) {
    let mut rules = Vec::new();
    let mut rest = Vec::new();
    let mut aliases: Vec<String> = Vec::new();
    let setter = regex::Regex::new(
        r#"^\s*pm\.(?:environment|collectionVariables|globals|variables)\.set\(\s*["'`]([^"'`]+)["'`]\s*,\s*(.+?)\s*\)\s*;?\s*$"#,
    )
    .expect("a valid pattern");
    let alias = regex::Regex::new(r#"^\s*(?:var|let|const)\s+([A-Za-z_$][\w$]*)\s*=\s*(?:pm\.response\.json\(\)|JSON\.parse\(responseBody\))\s*;?\s*$"#)
        .expect("a valid pattern");
    for line in lines {
        if let Some(found) = alias.captures(line) {
            aliases.push(found[1].to_string());
            continue;
        }
        if let Some(found) = setter.captures(line) {
            if let Some(from) = json_path(&found[2], &aliases) {
                rules.push(KeepRule {
                    var: found[1].to_string(),
                    from,
                    every: true,
                    ..KeepRule::default()
                });
                continue;
            }
        }
        if !line.trim().is_empty() {
            rest.push(line.clone());
        }
    }
    (rules, rest)
}

fn pairs(list: &Value, key: &str) -> Vec<Pair> {
    list.as_array()
        .map(|items| {
            items
                .iter()
                .map(|item| Pair {
                    name: text(&item[key]),
                    value: text(&item["value"]),
                    on: item["disabled"].as_bool() != Some(true),
                })
                .collect()
        })
        .unwrap_or_default()
}

fn auth_of(auth: &Value) -> Option<Auth> {
    let kind = auth["type"].as_str()?;
    // v2.1 lists `[{ key, value }]`; v2.0 has an object.
    let find = |name: &str| -> Option<String> {
        match &auth[kind] {
            Value::Array(entries) => entries
                .iter()
                .find(|e| e["key"].as_str() == Some(name))
                .map(|e| text(&e["value"])),
            Value::Object(map) => map.get(name).map(text),
            _ => None,
        }
    };
    Some(match kind {
        "bearer" => Auth {
            kind: AuthType::Bearer,
            token: find("token"),
            ..Auth::default()
        },
        "basic" => Auth {
            kind: AuthType::Basic,
            user: find("username"),
            password: find("password"),
            ..Auth::default()
        },
        "apikey" => Auth {
            kind: AuthType::ApiKey,
            key: find("key"),
            value: find("value"),
            place: Some(if find("in").as_deref() == Some("query") {
                KeyPlace::Query
            } else {
                KeyPlace::Header
            }),
            ..Auth::default()
        },
        "noauth" => Auth::default(),
        _ => return None,
    })
}

fn body_of(body: &Value) -> Option<Body> {
    match body["mode"].as_str()? {
        "raw" => {
            let raw = text(&body["raw"]);
            let language = body["options"]["raw"]["language"].as_str().unwrap_or("");
            let kind = match language {
                "json" => BodyType::Json,
                "xml" => BodyType::Xml,
                _ if serde_json::from_str::<Value>(&raw).is_ok() => BodyType::Json,
                _ if raw.trim_start().starts_with('<') => BodyType::Xml,
                _ => BodyType::Raw,
            };
            Some(Body {
                kind,
                text: Some(raw),
                ..Body::default()
            })
        }
        "urlencoded" | "formdata" => {
            let multipart = body["mode"] == "formdata";
            let list = &body[if multipart { "formdata" } else { "urlencoded" }];
            let fields = list
                .as_array()
                .map(|items| {
                    items
                        .iter()
                        .map(|item| {
                            let file = item["type"].as_str() == Some("file");
                            Field {
                                name: text(&item["key"]),
                                value: if file { text(&item["src"]) } else { text(&item["value"]) },
                                file,
                                on: item["disabled"].as_bool() != Some(true),
                            }
                        })
                        .collect()
                })
                .unwrap_or_default();
            Some(Body {
                kind: if multipart { BodyType::Multipart } else { BodyType::Form },
                fields,
                ..Body::default()
            })
        }
        "graphql" => Some(Body {
            kind: BodyType::Json,
            text: Some(
                serde_json::to_string_pretty(&serde_json::json!({
                    "query": body["graphql"]["query"],
                    "variables": serde_json::from_str::<Value>(body["graphql"]["variables"].as_str().unwrap_or("{}")).unwrap_or(Value::Null),
                }))
                .unwrap_or_default(),
            ),
            ..Body::default()
        }),
        _ => None,
    }
}

fn script_lines(item: &Value, listen: &str) -> Vec<String> {
    item["event"]
        .as_array()
        .map(|events| {
            events
                .iter()
                .filter(|e| e["listen"].as_str() == Some(listen))
                .flat_map(|e| match &e["script"]["exec"] {
                    Value::Array(lines) => lines.iter().map(text).collect::<Vec<_>>(),
                    Value::String(s) => s.lines().map(str::to_string).collect(),
                    _ => Vec::new(),
                })
                .collect()
        })
        .unwrap_or_default()
}

/// One Postman item as a call, and whether it had script lines left over.
fn call_of(item: &Value) -> (Call, bool) {
    let request = &item["request"];
    // A request may be just its URL.
    let (method, url) = match request {
        Value::String(url) => ("GET".to_string(), path_variables(url)),
        _ => (
            request["method"]
                .as_str()
                .unwrap_or("GET")
                .to_ascii_uppercase(),
            url_of(&request["url"]),
        ),
    };
    let (keep, rest) = rules_from(&script_lines(item, "test"));
    let pre = script_lines(item, "prerequest");
    let mut notes = Vec::new();
    if !pre.iter().all(|l| l.trim().is_empty()) {
        notes.push(format!(
            "Postman pre-request script, not run:\n{}",
            pre.join("\n")
        ));
    }
    if !rest.is_empty() {
        notes.push(format!(
            "Postman test script, not run:\n{}",
            rest.join("\n")
        ));
    }
    let scripted = !notes.is_empty();
    let description = match &request["description"] {
        Value::String(s) => Some(s.clone()),
        Value::Object(o) => o.get("content").and_then(Value::as_str).map(str::to_string),
        _ => None,
    };
    if let Some(d) = description.filter(|d| !d.trim().is_empty()) {
        notes.insert(0, d);
    }
    let call = Call {
        id: new_id(),
        name: item["name"].as_str().unwrap_or("Request").to_string(),
        method: Some(method),
        url,
        headers: pairs(&request["header"], "key"),
        body: body_of(&request["body"]),
        auth: auth_of(&request["auth"]),
        keep,
        notes: (!notes.is_empty()).then(|| notes.join("\n\n")),
        ..Call::default()
    };
    (call, scripted)
}

/// A collection's calls, each with the folder it was in (folders inside
/// folders joined with ` / `), its variables, and how many calls had script
/// lines kept as notes.
pub fn read(file: &str, value: &Value) -> (Vec<Endpoint>, Vec<Variable>, usize) {
    let mut out = Vec::new();
    let mut scripted = 0;
    let collection_auth = auth_of(&value["auth"]);
    fn walk(
        file: &str,
        items: &Value,
        folder: Option<String>,
        auth: &Option<Auth>,
        out: &mut Vec<Endpoint>,
        scripted: &mut usize,
    ) {
        for item in items.as_array().into_iter().flatten() {
            if item["item"].is_array() {
                let name = item["name"].as_str().unwrap_or("Folder").to_string();
                let nested = match &folder {
                    Some(parent) => format!("{parent} / {name}"),
                    None => name,
                };
                let inner = auth_of(&item["auth"]).or_else(|| auth.clone());
                walk(file, &item["item"], Some(nested), &inner, out, scripted);
                continue;
            }
            let (mut call, had) = call_of(item);
            if call.auth.is_none() {
                call.auth = auth.clone().filter(|a| a.kind != AuthType::None);
            }
            if had {
                *scripted += 1;
            }
            let path = path_of(&call.url);
            out.push(Endpoint {
                id: format!("{file}#{}", out.len()),
                key: key_of(&call),
                source: format!("file:{file}"),
                group: folder.clone().unwrap_or_else(|| "Requests".into()),
                file: Some(file.to_string()),
                line: None,
                method: call.method().to_string(),
                path,
                hint: call
                    .notes
                    .as_ref()
                    .map(|_| "has a script".to_string())
                    .unwrap_or_default(),
                call,
                status: Status::New,
                declared: None,
                folder: folder.clone(),
            });
        }
    }
    walk(
        file,
        &value["item"],
        None,
        &collection_auth,
        &mut out,
        &mut scripted,
    );
    let vars = value["variable"]
        .as_array()
        .map(|list| {
            list.iter()
                .map(|v| Variable::plain(&text(&v["key"]), &text(&v["value"])))
                .collect()
        })
        .unwrap_or_default();
    (out, vars, scripted)
}

/// A Postman environment. A secret's value is returned beside it, for the
/// keychain; the environment holds only its name.
pub fn environment(value: &Value) -> (Environment, Vec<(String, String)>) {
    let name = value["name"].as_str().unwrap_or("imported");
    let mut env = Environment::new(&super::super::store::slug(name), Default::default());
    let mut secrets = Vec::new();
    for entry in value["values"].as_array().into_iter().flatten() {
        if entry["enabled"].as_bool() == Some(false) {
            continue;
        }
        let key = text(&entry["key"]);
        let val = text(&entry["value"]);
        if entry["type"].as_str() == Some("secret") {
            env.vars.push(Variable {
                name: key.clone(),
                secret: true,
                ..Variable::default()
            });
            if !val.is_empty() {
                secrets.push((key, val));
            }
        } else {
            env.vars.push(Variable::plain(&key, &val));
        }
    }
    (env, secrets)
}

#[cfg(test)]
mod tests {
    use super::*;

    const V21: &str = r#"{
      "info": {"name": "Orders", "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"},
      "auth": {"type": "bearer", "bearer": [{"key": "token", "value": "{{token}}"}]},
      "variable": [{"key": "tenant", "value": "acme"}],
      "item": [
        {"name": "Auth", "item": [
          {"name": "Log in",
           "event": [{"listen": "test", "script": {"exec": [
             "var json = pm.response.json();",
             "pm.environment.set(\"token\", json.data.accessToken);",
             "pm.collectionVariables.set('userId', pm.response.json().data.user[\"id\"]);",
             "pm.test('ok', function () { pm.response.to.have.status(200); });"
           ]}}],
           "request": {"method": "POST", "auth": {"type": "noauth"},
             "header": [{"key": "Accept", "value": "application/json"}, {"key": "X-Off", "value": "1", "disabled": true}],
             "body": {"mode": "raw", "raw": "{\"email\": \"{{email}}\"}", "options": {"raw": {"language": "json"}}},
             "url": {"raw": "{{baseUrl}}/auth/login", "host": ["{{baseUrl}}"], "path": ["auth", "login"]}}}
        ]},
        {"name": "Get user", "request": {"method": "GET", "url": "{{baseUrl}}/users/:id?full=true"}},
        {"name": "Upload", "request": {"method": "POST", "url": "{{baseUrl}}/files",
          "body": {"mode": "formdata", "formdata": [{"key": "doc", "type": "file", "src": "a.pdf"}, {"key": "note", "value": "x"}]}}}
      ]
    }"#;

    #[test]
    fn a_v21_collection_becomes_calls_in_its_folders() {
        let value: Value = serde_json::from_str(V21).unwrap();
        assert!(is_collection(&value));
        let (endpoints, vars, scripted) = read("orders.postman_collection.json", &value);
        assert_eq!(endpoints.len(), 3);
        assert_eq!(vars, vec![Variable::plain("tenant", "acme")]);
        assert_eq!(scripted, 1);

        let login = &endpoints[0].call;
        assert_eq!(endpoints[0].folder.as_deref(), Some("Auth"));
        assert_eq!(login.url, "{{baseUrl}}/auth/login");
        assert!(!login.headers[1].on);
        assert_eq!(login.body.as_ref().unwrap().kind, BodyType::Json);
        // `noauth` on the request wins over the collection's bearer.
        assert_eq!(login.auth.as_ref().unwrap().kind, AuthType::None);
        assert_eq!(
            login
                .keep
                .iter()
                .map(|r| (r.var.as_str(), r.from.as_str()))
                .collect::<Vec<_>>(),
            [
                ("token", "$.data.accessToken"),
                ("userId", "$.data.user['id']")
            ]
        );
        assert!(login.notes.as_deref().unwrap().contains("pm.test('ok'"));
        assert!(!login
            .notes
            .as_deref()
            .unwrap()
            .contains("pm.environment.set"));

        let user = &endpoints[1].call;
        assert_eq!(user.url, "{{baseUrl}}/users/{{id}}?full=true");
        assert_eq!(endpoints[1].key, "GET /users/{}");
        assert_eq!(
            user.auth.as_ref().unwrap().token.as_deref(),
            Some("{{token}}")
        );

        let upload = endpoints[2].call.body.as_ref().unwrap();
        assert_eq!(upload.kind, BodyType::Multipart);
        assert!(upload.fields[0].file);
        assert_eq!(upload.fields[0].value, "a.pdf");
    }

    #[test]
    fn a_v20_collection_reads_its_object_auth_and_url() {
        let v20 = r#"{"info": {"name": "Old", "schema": "https://schema.getpostman.com/json/collection/v2.0.0/collection.json"},
          "item": [{"name": "Ping", "request": {"method": "get", "auth": {"type": "basic", "basic": {"username": "u", "password": "{{pw}}"}},
            "url": {"protocol": "https", "host": ["api", "example", "com"], "path": ["v1", "ping"]}}}]}"#;
        let value: Value = serde_json::from_str(v20).unwrap();
        let (endpoints, _, _) = read("old.json", &value);
        assert_eq!(endpoints[0].call.url, "https://api.example.com/v1/ping");
        assert_eq!(endpoints[0].method, "GET");
        assert_eq!(
            endpoints[0].call.auth.as_ref().unwrap().password.as_deref(),
            Some("{{pw}}")
        );
    }

    #[test]
    fn an_environment_keeps_its_secrets_apart() {
        let value: Value = serde_json::from_str(
            r#"{"name": "Staging EU", "_postman_variable_scope": "environment", "values": [
              {"key": "baseUrl", "value": "https://staging", "enabled": true},
              {"key": "password", "value": "hunter2", "type": "secret", "enabled": true},
              {"key": "off", "value": "x", "enabled": false}]}"#,
        )
        .unwrap();
        assert!(is_environment(&value));
        let (env, secrets) = environment(&value);
        assert_eq!(env.name, "staging-eu");
        assert_eq!(env.vars.len(), 2);
        assert!(env.vars[1].secret && env.vars[1].value.is_none());
        assert_eq!(
            secrets,
            vec![("password".to_string(), "hunter2".to_string())]
        );
    }
}
