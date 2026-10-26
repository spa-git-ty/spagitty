// SPDX-License-Identifier: GPL-3.0-or-later

//! Bruno's YAML collections (BUG-100): the OpenCollection format Bruno
//! writes now, in place of `.bru` files ([`super::bruno`]).
//!
//! A collection is a folder with `opencollection.yml`, a `folder.yml` in
//! each folder, one `.yml` file per request and `environments/*.yml`. A
//! request is `info` (`name`, `type: http`, `seq`), `http` (`method`,
//! `url`, `headers`, `params`, `body`, `auth`) and `runtime` (`scripts`,
//! `variables`, `assertions`). An entry with `disabled: true` is switched
//! off. `auth: inherit` takes the auth of the folder above, up to the
//! collection's `request.auth`; a request with no `auth` has none, as
//! Bruno leaves it out then. The collection's and the folders' headers go
//! before the request's own, as Bruno sends them.

use yaml_rust2::YamlLoader;

use super::super::model::{
    new_id, Auth, AuthType, Body, BodyType, Call, Environment, Field, KeepRule, KeyPlace, Pair,
    Variable,
};
use super::bruno::res_path;
use super::json::{from_yaml, J};
use super::{key_of, path_of, Endpoint, Status};

/// The collection's root file, by its name.
pub fn is_root_file(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name == "opencollection.yml" || name == "opencollection.yaml"
}

fn is_yaml(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    name.ends_with(".yml") || name.ends_with(".yaml")
}

fn load(text: &str) -> Option<J> {
    let docs = YamlLoader::load_from_str(text).ok()?;
    docs.first().map(from_yaml)
}

/// The text of `key`, or empty.
fn text(value: &J, key: &str) -> String {
    value.get(key).and_then(J::text).unwrap_or_default()
}

fn on(entry: &J) -> bool {
    !matches!(entry.get("disabled"), Some(J::Bool(true)))
        && !matches!(entry.get("enabled"), Some(J::Bool(false)))
}

/// What an `auth` says: its own, none, or the one above it.
#[derive(Debug, Clone, PartialEq)]
enum Said {
    Inherit,
    Own(Option<Auth>),
}

fn auth_of(value: Option<&J>) -> Said {
    let Some(value) = value else {
        return Said::Inherit;
    };
    let kind = match value {
        J::Str(mode) => mode.as_str(),
        _ => value.get("type").and_then(J::str).unwrap_or("none"),
    };
    Said::Own(match kind {
        "inherit" => return Said::Inherit,
        "bearer" => Some(Auth {
            kind: AuthType::Bearer,
            token: Some(text(value, "token")),
            ..Auth::default()
        }),
        "basic" => Some(Auth {
            kind: AuthType::Basic,
            user: Some(text(value, "username")),
            password: Some(text(value, "password")),
            ..Auth::default()
        }),
        "apikey" => Some(Auth {
            kind: AuthType::ApiKey,
            key: Some(text(value, "key")),
            value: Some(text(value, "value")),
            place: Some(
                if value.get("placement").and_then(J::str) == Some("queryparams") {
                    KeyPlace::Query
                } else {
                    KeyPlace::Header
                },
            ),
            ..Auth::default()
        }),
        _ => None,
    })
}

fn pairs(list: Option<&J>) -> Vec<Pair> {
    list.map(J::arr)
        .unwrap_or_default()
        .iter()
        .map(|entry| Pair {
            name: text(entry, "name"),
            value: text(entry, "value"),
            on: on(entry),
        })
        .filter(|pair| !pair.name.is_empty())
        .collect()
}

/// What a folder, or the collection, gives the requests under it.
#[derive(Debug, Clone)]
struct Level {
    name: Option<String>,
    seq: i64,
    auth: Said,
    headers: Vec<Pair>,
}

fn level(value: &J) -> Level {
    let request = value.get("request");
    Level {
        name: value
            .get("info")
            .map(|info| text(info, "name"))
            .filter(|n| !n.is_empty()),
        seq: seq_of(value),
        auth: auth_of(request.and_then(|r| r.get("auth"))),
        headers: pairs(request.and_then(|r| r.get("headers"))),
    }
}

fn seq_of(value: &J) -> i64 {
    value
        .get("info")
        .and_then(|info| info.get("seq"))
        .and_then(J::text)
        .and_then(|s| s.parse().ok())
        .unwrap_or(i64::MAX)
}

/// `:id` in the path filled from the path parameters, and the query
/// parameters that are on and not in the URL yet added to it.
fn url_with(url: &str, params: &[J]) -> String {
    let (mut path, mut query) = match url.split_once('?') {
        Some((path, query)) => (path.to_string(), query.to_string()),
        None => (url.to_string(), String::new()),
    };
    for param in params.iter().filter(|p| on(p)) {
        let name = text(param, "name");
        let value = text(param, "value");
        if name.is_empty() {
            continue;
        }
        if param.get("type").and_then(J::str) == Some("path") {
            let filled = if value.is_empty() {
                format!("{{{{{name}}}}}")
            } else {
                value
            };
            path = path
                .split('/')
                .map(|segment| {
                    if segment == format!(":{name}") {
                        filled.clone()
                    } else {
                        segment.to_string()
                    }
                })
                .collect::<Vec<_>>()
                .join("/");
            continue;
        }
        let there = query
            .split('&')
            .any(|part| part.split('=').next() == Some(name.as_str()));
        if !there {
            if !query.is_empty() {
                query.push('&');
            }
            query.push_str(&format!("{name}={value}"));
        }
    }
    if query.is_empty() {
        path
    } else {
        format!("{path}?{query}")
    }
}

fn fields(data: Option<&J>) -> Vec<Field> {
    data.map(J::arr)
        .unwrap_or_default()
        .iter()
        .map(|entry| {
            let file = entry.get("type").and_then(J::str) == Some("file");
            // A file part's value is a list of paths.
            let value = match entry.get("value") {
                Some(J::Arr(paths)) => paths.first().and_then(J::text).unwrap_or_default(),
                _ => text(entry, "value"),
            };
            Field {
                name: text(entry, "name"),
                value,
                file,
                on: on(entry),
            }
        })
        .filter(|field| !field.name.is_empty())
        .collect()
}

fn body_of(body: Option<&J>) -> Option<Body> {
    let body = body?;
    let data = body.get("data");
    let written = || data.and_then(J::text).filter(|t| !t.trim().is_empty());
    let with_text = |kind: BodyType, text: String| Body {
        kind,
        text: Some(text),
        ..Body::default()
    };
    match body.get("type").and_then(J::str)? {
        "json" => written().map(|t| with_text(BodyType::Json, t)),
        "xml" => written().map(|t| with_text(BodyType::Xml, t)),
        "text" | "sparql" => written().map(|t| with_text(BodyType::Raw, t)),
        "graphql" => {
            let data = data?;
            let query = text(data, "query");
            let variables = text(data, "variables");
            let variables = serde_json::from_str::<serde_json::Value>(&variables)
                .unwrap_or(serde_json::Value::Object(Default::default()));
            let json = serde_json::json!({ "query": query, "variables": variables });
            Some(with_text(
                BodyType::Json,
                serde_json::to_string_pretty(&json).unwrap_or_default(),
            ))
        }
        "form-urlencoded" => Some(Body {
            kind: BodyType::Form,
            fields: fields(data),
            ..Body::default()
        }),
        "multipart-form" => Some(Body {
            kind: BodyType::Multipart,
            fields: fields(data),
            ..Body::default()
        }),
        _ => None,
    }
}

/// One request file as a call, with `above` the folders over it from the
/// nearest out to the collection; `None` for a file that is not a request.
fn call_of(value: &J, above: &[&Level]) -> Option<Call> {
    let info = value.get("info")?;
    if !matches!(info.get("type").and_then(J::str), Some("http" | "graphql")) {
        return None;
    }
    let http = value.get("http").or_else(|| value.get("graphql"))?;
    let mut headers = pairs(http.get("headers"));
    // The collection's first, then each folder's, then the request's own.
    for level in above {
        let mut given: Vec<Pair> = level
            .headers
            .iter()
            .filter(|pair| {
                !headers
                    .iter()
                    .any(|own| own.name.eq_ignore_ascii_case(&pair.name))
            })
            .cloned()
            .collect();
        given.append(&mut headers);
        headers = given;
    }
    let mut auth = match http.get("auth") {
        None => Said::Own(None),
        given => auth_of(given),
    };
    for level in above {
        if auth != Said::Inherit {
            break;
        }
        auth = level.auth.clone();
    }
    let auth = match auth {
        Said::Own(auth) => auth,
        Said::Inherit => None,
    };

    let runtime = value.get("runtime");
    let mut notes = Vec::new();
    let mut keep = Vec::new();
    for var in runtime
        .and_then(|r| r.get("variables"))
        .map(J::arr)
        .unwrap_or_default()
    {
        let name = text(var, "name");
        let expression = text(var, "value");
        let after = var.get("scope").and_then(J::str) != Some("before-request");
        match res_path(&expression) {
            Some(from) if after && on(var) => keep.push(KeepRule {
                var: name,
                from,
                every: true,
                ..KeepRule::default()
            }),
            _ => notes.push(format!("{name}: {expression}")),
        }
    }
    for script in runtime
        .and_then(|r| r.get("scripts"))
        .map(J::arr)
        .unwrap_or_default()
    {
        let code = text(script, "code");
        if !code.trim().is_empty() {
            let kind = script.get("type").and_then(J::str).unwrap_or("script");
            notes.push(format!("Bruno {kind} script, not run:\n{}", code.trim()));
        }
    }
    let assertions: Vec<String> = runtime
        .and_then(|r| r.get("assertions"))
        .map(J::arr)
        .unwrap_or_default()
        .iter()
        .map(|a| format!("{} {}", text(a, "expression"), text(a, "value")))
        .collect();
    if !assertions.is_empty() {
        notes.push(format!(
            "Bruno assertions, not run:\n{}",
            assertions.join("\n")
        ));
    }

    Some(Call {
        id: new_id(),
        name: Some(text(info, "name"))
            .filter(|n| !n.is_empty())
            .unwrap_or_else(|| "Request".into()),
        method: Some(
            Some(text(http, "method"))
                .filter(|m| !m.is_empty())
                .unwrap_or_else(|| "GET".into())
                .to_ascii_uppercase(),
        ),
        url: url_with(
            &text(http, "url"),
            http.get("params").map(J::arr).unwrap_or_default(),
        ),
        headers,
        body: body_of(http.get("body")),
        auth,
        keep,
        notes: (!notes.is_empty()).then(|| notes.join("\n\n")),
        ..Call::default()
    })
}

/// One request file on its own, chosen from outside: nothing above it.
pub fn request(text: &str) -> Option<Call> {
    call_of(&load(text)?, &[])
}

/// An environment file: `name` and `variables`. A secret's value is not in
/// the file; Bruno keeps it on the computer.
fn environment(file_name: &str, value: &J) -> Option<Environment> {
    let variables = value.get("variables")?;
    let name = Some(self::text(value, "name"))
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| file_name.to_string());
    let mut env = Environment::new(&super::super::store::slug(&name), Default::default());
    for var in variables.arr().iter().filter(|v| on(v)) {
        let name = self::text(var, "name");
        if name.is_empty() {
            continue;
        }
        if matches!(var.get("secret"), Some(J::Bool(true))) {
            env.vars.push(Variable {
                name,
                secret: true,
                ..Variable::default()
            });
        } else {
            env.vars
                .push(Variable::plain(&name, &self::text(var, "value")));
        }
    }
    Some(env)
}

/// A collection's requests and environments. `files` are its YAML files by
/// their path from the folder that holds `opencollection.yml`, that file
/// among them. Returns the collection's name too.
pub fn read(root: &str, files: &[(String, String)]) -> (Vec<Endpoint>, Vec<Environment>, String) {
    let parsed: Vec<(&str, J)> = files
        .iter()
        .filter(|(relative, _)| is_yaml(relative))
        .filter_map(|(relative, text)| load(text).map(|value| (relative.as_str(), value)))
        .collect();
    let collection = parsed
        .iter()
        .find(|(relative, _)| is_root_file(relative))
        .map(|(_, value)| level(value));
    let name = collection
        .as_ref()
        .and_then(|c| c.name.clone())
        .or_else(|| root.rsplit('/').next().map(str::to_string))
        .filter(|n| !n.is_empty())
        .unwrap_or_else(|| "Bruno".into());
    let folders: Vec<(String, Level)> = parsed
        .iter()
        .filter_map(|(relative, value)| {
            let dir = relative.strip_suffix("/folder.yml")?;
            Some((dir.to_string(), level(value)))
        })
        .collect();
    let folder_at = |dir: &str| folders.iter().find(|(d, _)| d == dir).map(|(_, l)| l);

    let mut envs = Vec::new();
    let mut requests = Vec::new();
    for (relative, value) in &parsed {
        if let Some(file) = relative.strip_prefix("environments/") {
            let stem = file.rsplit_once('.').map_or(file, |(stem, _)| stem);
            envs.extend(environment(stem, value));
            continue;
        }
        let dirs: Vec<&str> = match relative.rsplit_once('/') {
            Some((dir, _)) => dir.split('/').collect(),
            None => Vec::new(),
        };
        // Nearest first, the collection last.
        let mut above: Vec<&Level> = (1..=dirs.len())
            .rev()
            .filter_map(|n| folder_at(&dirs[..n].join("/")))
            .collect();
        above.extend(collection.as_ref());
        let Some(call) = call_of(value, &above) else {
            continue;
        };
        // Bruno's order: folders and requests side by side, by `seq`.
        let mut order: Vec<(i64, String)> = (1..=dirs.len())
            .map(|n| {
                let seq = folder_at(&dirs[..n].join("/")).map_or(i64::MAX, |l| l.seq);
                (seq, dirs[n - 1].to_lowercase())
            })
            .collect();
        order.push((seq_of(value), call.name.to_lowercase()));
        let folder = (!dirs.is_empty()).then(|| {
            (1..=dirs.len())
                .map(|n| {
                    folder_at(&dirs[..n].join("/"))
                        .and_then(|l| l.name.clone())
                        .unwrap_or_else(|| dirs[n - 1].to_string())
                })
                .collect::<Vec<_>>()
                .join(" / ")
        });
        requests.push((order, relative.to_string(), folder, call));
    }
    requests.sort_by(|a, b| a.0.cmp(&b.0));

    let root_file = if root.is_empty() {
        "opencollection.yml".to_string()
    } else {
        format!("{root}/opencollection.yml")
    };
    let endpoints = requests
        .into_iter()
        .map(|(_, relative, folder, call)| {
            let file = if root.is_empty() {
                relative
            } else {
                format!("{root}/{relative}")
            };
            Endpoint {
                id: format!("{file}#0"),
                key: key_of(&call),
                source: format!("file:{root_file}"),
                group: folder.clone().unwrap_or_else(|| name.clone()),
                file: Some(file),
                line: None,
                method: call.method().to_string(),
                path: path_of(&call.url),
                hint: String::new(),
                call,
                status: Status::New,
                declared: None,
                folder,
            }
        })
        .collect();
    (endpoints, envs, name)
}

#[cfg(test)]
mod tests {
    use super::*;

    const ROOT: &str = "opencollection: 1.0.0\n\ninfo:\n  name: Bank API\nrequest:\n  auth:\n    type: bearer\n    token: \"{{token}}\"\n  headers:\n    - name: X-Channel\n      value: web\n";

    const FOLDER: &str =
        "info:\n  name: accounts\n  type: folder\n  seq: 2\n\nrequest:\n  auth: inherit\n";

    const ACCOUNT: &str = r#"info:
  name: account
  type: http
  seq: 9

http:
  method: GET
  url: "{{accountsUrl}}/accounts/:id"
  headers:
    - name: Accept
      value: application/json
    - name: X-Off
      value: "1"
      disabled: true
  params:
    - name: id
      value: "42"
      type: path
    - name: sort
      value: asc
      type: query
    - name: page
      value: "2"
      type: query
      disabled: true
  body:
    type: json
    data: ""
  auth: inherit

runtime:
  scripts:
    - type: after-response
      code: |-
        bru.setEnvVar("token", res.getBody().token);

settings:
  encodeUrl: true

examples:
  - name: Success
    response:
      status: 200
"#;

    // A SOAP body as Bruno writes it: one double-quoted, folded string.
    const SOAP: &str = "info:\n  name: Get Accounts\n  type: http\n  seq: 5\n\nhttp:\n  method: POST\n  url: \"{{ESB_URL}}/esb/party\"\n  headers:\n    - name: SOAPAction\n      value: '\"urn:party\"'\n  body:\n    type: xml\n    data: \"<soapenv:Envelope>\\r\n\n      \\    <soapenv:Body/>\\r\n\n      </soapenv:Envelope>\"\n  auth: inherit\n";

    const UPLOAD: &str = "info:\n  name: upload\n  type: http\n  seq: 1\n\nhttp:\n  method: POST\n  url: \"{{dobUrl}}/upload\"\n  body:\n    type: multipart-form\n    data:\n      - name: note\n        type: text\n        value: fff\n        disabled: true\n      - name: file\n        type: file\n        value:\n          - /tmp/a.jpg\n";

    const ENV: &str = "name: A Local Host\nvariables:\n  - name: baseUrl\n    value: http://localhost:3000\n  - name: token\n    value: \"\"\n    secret: true\n";

    fn files() -> Vec<(String, String)> {
        [
            ("opencollection.yml", ROOT),
            ("accounts/folder.yml", FOLDER),
            ("accounts/account.yml", ACCOUNT),
            ("Get Accounts.yml", SOAP),
            ("dob/upload.yml", UPLOAD),
            ("environments/A Local Host.yml", ENV),
        ]
        .map(|(path, text)| (path.to_string(), text.to_string()))
        .to_vec()
    }

    #[test]
    fn a_request_inherits_auth_and_headers_and_fills_its_parameters() {
        let (endpoints, _, name) = read("bank", &files());
        assert_eq!(name, "Bank API");
        let account = endpoints.iter().find(|e| e.call.name == "account").unwrap();
        let call = &account.call;
        assert_eq!(call.method.as_deref(), Some("GET"));
        assert_eq!(call.url, "{{accountsUrl}}/accounts/42?sort=asc");
        assert_eq!(
            call.headers,
            vec![
                Pair::new("X-Channel", "web"),
                Pair::new("Accept", "application/json"),
                Pair {
                    name: "X-Off".into(),
                    value: "1".into(),
                    on: false
                },
            ]
        );
        let auth = call.auth.as_ref().unwrap();
        assert_eq!(auth.kind, AuthType::Bearer);
        assert_eq!(auth.token.as_deref(), Some("{{token}}"));
        // An empty JSON body is no body.
        assert!(call.body.is_none());
        assert!(call.notes.as_deref().unwrap().contains("bru.setEnvVar"));
        assert_eq!(account.folder.as_deref(), Some("accounts"));
        assert_eq!(account.file.as_deref(), Some("bank/accounts/account.yml"));
        assert_eq!(account.source, "file:bank/opencollection.yml");
    }

    #[test]
    fn a_folded_xml_body_reads_as_bruno_wrote_it() {
        let (endpoints, _, _) = read("", &files());
        let soap = endpoints
            .iter()
            .find(|e| e.call.name == "Get Accounts")
            .unwrap();
        assert_eq!(soap.group, "Bank API");
        assert_eq!(
            soap.call.headers,
            vec![
                Pair::new("X-Channel", "web"),
                Pair::new("SOAPAction", "\"urn:party\"")
            ]
        );
        let body = soap.call.body.as_ref().unwrap();
        assert_eq!(body.kind, BodyType::Xml);
        assert_eq!(
            body.text.as_deref(),
            Some("<soapenv:Envelope>\r\n    <soapenv:Body/>\r\n</soapenv:Envelope>")
        );
    }

    #[test]
    fn multipart_parts_keep_their_files_and_switches() {
        let (endpoints, _, _) = read("", &files());
        let upload = endpoints.iter().find(|e| e.call.name == "upload").unwrap();
        // No `auth` at all is none, even under a collection with one.
        assert!(upload.call.auth.is_none());
        let body = upload.call.body.as_ref().unwrap();
        assert_eq!(body.kind, BodyType::Multipart);
        assert!(!body.fields[0].on);
        assert!(body.fields[1].file);
        assert_eq!(body.fields[1].value, "/tmp/a.jpg");
    }

    #[test]
    fn environments_come_in_with_their_secrets_empty() {
        let (endpoints, envs, _) = read("", &files());
        assert_eq!(endpoints.len(), 3);
        assert_eq!(envs.len(), 1);
        assert_eq!(envs[0].name, "a-local-host");
        assert_eq!(
            envs[0].vars[0].value.as_deref(),
            Some("http://localhost:3000")
        );
        assert!(envs[0].vars[1].secret);
        assert!(envs[0].vars[1].value.is_none());
    }

    #[test]
    fn requests_are_in_bruno_order() {
        let mut files = files();
        files.push((
            "accounts/first.yml".into(),
            ACCOUNT
                .replace("name: account", "name: first")
                .replace("seq: 9", "seq: 1"),
        ));
        let (endpoints, _, _) = read("", &files);
        let names: Vec<&str> = endpoints.iter().map(|e| e.call.name.as_str()).collect();
        assert_eq!(names, vec!["first", "account", "Get Accounts", "upload"]);
    }
}
