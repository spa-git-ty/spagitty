// SPDX-License-Identifier: GPL-3.0-or-later

//! Bruno collections (FEAT-162, proposal §8.3): a folder with `bruno.json`,
//! one `.bru` file per request, and `environments/*.bru`.
//!
//! A `.bru` file is blocks of `name {` … `}`: `meta`, a method block
//! (`get`, `post`, …) with the URL, `headers`, `query`, `auth:bearer`,
//! `body:json`, `vars:post-response`, scripts. A key-value line starting
//! with `~` is switched off. `vars:post-response` lines that read
//! `res.body…` become keep rules; scripts go into the notes.

use super::super::model::{
    new_id, Auth, AuthType, Body, BodyType, Call, Environment, Field, KeepRule, Pair, Variable,
};
use super::{key_of, path_of, Endpoint, Status};

const METHODS: [&str; 7] = ["get", "post", "put", "patch", "delete", "head", "options"];

/// The blocks of a `.bru` file, by name, with their lines.
fn blocks(text: &str) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    let mut lines = text.lines();
    while let Some(line) = lines.next() {
        let Some(name) = line.trim_end().strip_suffix('{').map(str::trim) else {
            continue;
        };
        if name.is_empty() || line.starts_with(char::is_whitespace) {
            continue;
        }
        let mut body = Vec::new();
        for inner in lines.by_ref() {
            if inner.trim_end() == "}" && !inner.starts_with(char::is_whitespace) {
                break;
            }
            body.push(inner.to_string());
        }
        out.push((name.to_string(), body));
    }
    out
}

/// `key: value` lines, with whether each is on.
fn pairs(lines: &[String]) -> Vec<(String, String, bool)> {
    lines
        .iter()
        .filter_map(|line| {
            let line = line.trim();
            let (on, line) = match line.strip_prefix('~') {
                Some(rest) => (false, rest),
                None => (true, line),
            };
            let (key, value) = line.split_once(':')?;
            Some((key.trim().to_string(), value.trim().to_string(), on))
        })
        .collect()
}

/// A body block's text, with the two spaces Bruno indents it by taken off.
fn dedent(lines: &[String]) -> String {
    lines
        .iter()
        .map(|line| line.strip_prefix("  ").unwrap_or(line))
        .collect::<Vec<_>>()
        .join("\n")
        .trim()
        .to_string()
}

/// `res.body.data.token` as `$.data.token`.
fn res_path(expression: &str) -> Option<String> {
    let rest = expression.trim().strip_prefix("res.body")?;
    if !rest.is_empty() && !rest.starts_with(['.', '[']) {
        return None;
    }
    Some(format!("${rest}"))
}

/// One request file as a call, or `None` for one that is not a request.
pub fn request(text: &str) -> Option<Call> {
    let blocks = blocks(text);
    let find = |name: &str| {
        blocks
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, l)| l.as_slice())
    };
    let meta = pairs(find("meta").unwrap_or(&[]));
    let (method, spec) = blocks.iter().find(|(n, _)| METHODS.contains(&n.as_str()))?;
    let spec = pairs(spec);
    let get = |list: &[(String, String, bool)], key: &str| {
        list.iter()
            .find(|(k, _, _)| k == key)
            .map(|(_, v, _)| v.clone())
    };
    let url = get(&spec, "url").unwrap_or_default();
    let headers = pairs(find("headers").unwrap_or(&[]))
        .into_iter()
        .map(|(name, value, on)| Pair { name, value, on })
        .collect();
    let body = match get(&spec, "body").as_deref() {
        Some("json") => find("body:json").map(|l| Body {
            kind: BodyType::Json,
            text: Some(dedent(l)),
            ..Body::default()
        }),
        Some("xml") => find("body:xml").map(|l| Body {
            kind: BodyType::Xml,
            text: Some(dedent(l)),
            ..Body::default()
        }),
        Some("text") => find("body:text").map(|l| Body {
            kind: BodyType::Raw,
            text: Some(dedent(l)),
            ..Body::default()
        }),
        Some(form @ ("formUrlEncoded" | "multipartForm")) => {
            let block = if form == "formUrlEncoded" {
                "body:form-urlencoded"
            } else {
                "body:multipart-form"
            };
            find(block).map(|l| Body {
                kind: if form == "formUrlEncoded" {
                    BodyType::Form
                } else {
                    BodyType::Multipart
                },
                fields: pairs(l)
                    .into_iter()
                    .map(|(name, value, on)| {
                        let file = value.starts_with("@file(");
                        Field {
                            name,
                            value: if file {
                                value
                                    .trim_start_matches("@file(")
                                    .trim_end_matches(')')
                                    .to_string()
                            } else {
                                value
                            },
                            file,
                            on,
                        }
                    })
                    .collect(),
                ..Body::default()
            })
        }
        _ => None,
    };
    let auth = match get(&spec, "auth").as_deref() {
        Some("bearer") => Some(Auth {
            kind: AuthType::Bearer,
            token: get(&pairs(find("auth:bearer").unwrap_or(&[])), "token"),
            ..Auth::default()
        }),
        Some("basic") => {
            let basic = pairs(find("auth:basic").unwrap_or(&[]));
            Some(Auth {
                kind: AuthType::Basic,
                user: get(&basic, "username"),
                password: get(&basic, "password"),
                ..Auth::default()
            })
        }
        _ => None,
    };
    let mut notes = Vec::new();
    let keep = pairs(find("vars:post-response").unwrap_or(&[]))
        .into_iter()
        .filter_map(|(var, expression, on)| match res_path(&expression) {
            Some(from) if on => Some(KeepRule {
                var,
                from,
                every: true,
                ..KeepRule::default()
            }),
            _ => {
                notes.push(format!("{var}: {expression}"));
                None
            }
        })
        .collect();
    for (name, lines) in &blocks {
        if name.starts_with("script:") || name == "tests" {
            notes.push(format!("Bruno {name}, not run:\n{}", dedent(lines)));
        }
    }
    Some(Call {
        id: new_id(),
        name: get(&meta, "name").unwrap_or_else(|| "Request".into()),
        method: Some(method.to_ascii_uppercase()),
        url,
        headers,
        body,
        auth,
        keep,
        notes: (!notes.is_empty()).then(|| notes.join("\n\n")),
        ..Call::default()
    })
}

/// An environment file: `vars { … }` and `vars:secret [ … ]`.
pub fn environment(name: &str, text: &str) -> Environment {
    let mut env = Environment::new(&super::super::store::slug(name), Default::default());
    for (block, lines) in blocks(text) {
        if block == "vars" {
            for (key, value, _) in pairs(&lines) {
                env.vars.push(Variable::plain(&key, &value));
            }
        }
    }
    // `vars:secret [ a, b ]` is a list, not a block of pairs.
    if let Some(start) = text.find("vars:secret [") {
        let rest = &text[start + "vars:secret [".len()..];
        let list = rest.split(']').next().unwrap_or("");
        for name in list
            .split([',', '\n'])
            .map(str::trim)
            .filter(|n| !n.is_empty())
        {
            env.vars.push(Variable {
                name: name.to_string(),
                secret: true,
                ..Variable::default()
            });
        }
    }
    env
}

/// A Bruno collection's requests: `files` are its `.bru` files, by their
/// path from the folder that holds `bruno.json`.
pub fn read(root: &str, files: &[(String, String)]) -> (Vec<Endpoint>, Vec<Environment>) {
    let mut out = Vec::new();
    let mut envs = Vec::new();
    for (relative, text) in files {
        let file = if root.is_empty() {
            relative.clone()
        } else {
            format!("{root}/{relative}")
        };
        if let Some(name) = relative.strip_prefix("environments/") {
            envs.push(environment(name.trim_end_matches(".bru"), text));
            continue;
        }
        let Some(call) = request(text) else { continue };
        let folder = relative
            .rsplit_once('/')
            .map(|(dir, _)| dir.replace('/', " / "));
        out.push(Endpoint {
            id: format!("{file}#0"),
            key: key_of(&call),
            source: format!("file:{root}/bruno.json"),
            group: folder.clone().unwrap_or_else(|| "Requests".into()),
            file: Some(file),
            line: None,
            method: call.method().to_string(),
            path: path_of(&call.url),
            hint: String::new(),
            call,
            status: Status::New,
            declared: None,
            folder,
        });
    }
    (out, envs)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BRU: &str = "meta {\n  name: Log in\n  type: http\n  seq: 1\n}\n\npost {\n  url: {{baseUrl}}/auth/login\n  body: json\n  auth: bearer\n}\n\nheaders {\n  Accept: application/json\n  ~X-Off: 1\n}\n\nauth:bearer {\n  token: {{token}}\n}\n\nbody:json {\n  {\n    \"email\": \"{{email}}\"\n  }\n}\n\nvars:post-response {\n  token: res.body.data.accessToken\n  when: Date.now()\n}\n\nscript:post-response {\n  console.log(res.status)\n}\n";

    #[test]
    fn a_bru_file_is_a_call() {
        let call = request(BRU).unwrap();
        assert_eq!(call.name, "Log in");
        assert_eq!(call.method.as_deref(), Some("POST"));
        assert_eq!(call.url, "{{baseUrl}}/auth/login");
        assert_eq!(
            call.headers,
            vec![
                Pair::new("Accept", "application/json"),
                Pair {
                    name: "X-Off".into(),
                    value: "1".into(),
                    on: false
                }
            ]
        );
        assert_eq!(call.auth.unwrap().token.as_deref(), Some("{{token}}"));
        assert_eq!(
            call.body.unwrap().text.as_deref(),
            Some("{\n  \"email\": \"{{email}}\"\n}")
        );
        assert_eq!(call.keep.len(), 1);
        assert_eq!(call.keep[0].from, "$.data.accessToken");
        let notes = call.notes.unwrap();
        assert!(notes.contains("when: Date.now()") && notes.contains("console.log"));
    }

    #[test]
    fn a_collection_reads_its_folders_and_environments() {
        let files = vec![
            ("auth/login.bru".to_string(), BRU.to_string()),
            (
                "environments/Local.bru".to_string(),
                "vars {\n  baseUrl: http://localhost:3000\n}\nvars:secret [\n  password\n]\n"
                    .to_string(),
            ),
        ];
        let (endpoints, envs) = read("api", &files);
        assert_eq!(endpoints[0].folder.as_deref(), Some("auth"));
        assert_eq!(endpoints[0].file.as_deref(), Some("api/auth/login.bru"));
        assert_eq!(envs[0].name, "local");
        assert_eq!(envs[0].vars.len(), 2);
        assert!(envs[0].vars[1].secret);
    }
}
