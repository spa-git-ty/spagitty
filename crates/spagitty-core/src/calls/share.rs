// SPDX-License-Identifier: GPL-3.0-or-later

//! Share (FEAT-164, proposal §5.5 and §7.3): a collection as one file
//! (Spagitty's own, or Postman 2.1), as a zip with its environments and
//! example responses, or as a commit.
//!
//! Every writer here goes through the same rule as the calls folder: a
//! secret or kept value is never written, only its name. A zip's
//! `README.md` lists what each person has to fill in.
//!
//! The Postman file is what Postman reads: folders, requests, auth,
//! collection variables, and each keep rule as the `pm.environment.set`
//! line that does the same in Postman. A SOAP call is a raw POST of its
//! envelope with its SOAP headers.

use serde_json::{json, Map, Value};

use super::model::{
    to_text, AuthType, BodyType, Call, Collection, Environment, Item, KeyPlace, Kind, Variable,
    VERSION,
};
use super::store::slug;
use super::zip;
use crate::Result;

/// A collection as Spagitty writes it, with its own secrets emptied.
pub fn spagitty(collection: &Collection) -> String {
    let mut written = collection.clone();
    written.v = VERSION;
    written.vars = written.vars.iter().map(Variable::for_file).collect();
    to_text(&written)
}

/// `$.data.items[0]['id']` as `pm.response.json().data.items[0]["id"]`;
/// `None` for an XPath, which Postman scripts cannot read as simply.
pub fn postman_path(path: &str) -> Option<String> {
    let rest = path.strip_prefix('$')?;
    let mut out = String::from("pm.response.json()");
    let mut chars = rest.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '.' => {
                out.push('.');
                while let Some(next) = chars.next_if(|c| *c != '.' && *c != '[') {
                    out.push(next);
                }
            }
            '[' => {
                let inner: String = std::iter::from_fn(|| chars.next_if(|c| *c != ']')).collect();
                chars.next();
                let inner = inner.trim();
                if inner.chars().all(|c| c.is_ascii_digit()) && !inner.is_empty() {
                    out.push_str(&format!("[{inner}]"));
                } else {
                    let key = inner.trim_matches(['\'', '"']).replace('"', "\\\"");
                    out.push_str(&format!("[\"{key}\"]"));
                }
            }
            _ => return None,
        }
    }
    Some(out)
}

fn url(raw: &str) -> Value {
    let (base, query) = raw.split_once('?').unwrap_or((raw, ""));
    let mut parts: Vec<&str> = base.split('/').collect();
    let host = if parts.first().is_some_and(|p| p.starts_with("{{")) || !base.contains("://") {
        parts.remove(0).to_string()
    } else {
        let scheme_host = base.split("://").nth(1).unwrap_or("");
        parts.drain(..3);
        scheme_host.split('/').next().unwrap_or("").to_string()
    };
    let mut url = json!({
        "raw": raw,
        "host": [host],
        "path": parts.iter().filter(|p| !p.is_empty()).collect::<Vec<_>>(),
    });
    if let Some((scheme, _)) = base.split_once("://") {
        url["protocol"] = json!(scheme);
    }
    if !query.is_empty() {
        url["query"] = Value::Array(
            query
                .split('&')
                .filter(|p| !p.is_empty())
                .map(|p| {
                    let (k, v) = p.split_once('=').unwrap_or((p, ""));
                    json!({ "key": k, "value": v })
                })
                .collect(),
        );
    }
    url
}

fn request(call: &Call) -> Value {
    let mut headers: Vec<Value> = call
        .headers
        .iter()
        .map(|h| {
            let mut header = json!({ "key": h.name, "value": h.value });
            if !h.on {
                header["disabled"] = json!(true);
            }
            header
        })
        .collect();
    let mut body = Value::Null;
    let method = if call.kind == Kind::Soap {
        "POST".to_string()
    } else {
        call.method().to_ascii_uppercase()
    };
    if call.kind == Kind::Soap {
        let action = call.action.clone().unwrap_or_default();
        if call.version.as_deref() == Some("1.2") {
            let mut kind = "application/soap+xml; charset=utf-8".to_string();
            if !action.is_empty() {
                kind.push_str(&format!("; action=\"{action}\""));
            }
            headers.push(json!({ "key": "Content-Type", "value": kind }));
        } else {
            headers.push(json!({ "key": "Content-Type", "value": "text/xml; charset=utf-8" }));
            headers.push(json!({ "key": "SOAPAction", "value": format!("\"{action}\"") }));
        }
        body = json!({ "mode": "raw", "raw": call.envelope.clone().unwrap_or_default(), "options": { "raw": { "language": "xml" } } });
    } else if let Some(b) = &call.body {
        body = match b.kind {
            BodyType::Json | BodyType::Xml | BodyType::Raw => {
                let language = match b.kind {
                    BodyType::Json => "json",
                    BodyType::Xml => "xml",
                    _ => "text",
                };
                if let (BodyType::Raw, Some(ct)) = (b.kind, &b.content_type) {
                    headers.push(json!({ "key": "Content-Type", "value": ct }));
                }
                json!({ "mode": "raw", "raw": b.text.clone().unwrap_or_default(), "options": { "raw": { "language": language } } })
            }
            BodyType::Form => json!({
                "mode": "urlencoded",
                "urlencoded": b.fields.iter().map(|f| {
                    let mut field = json!({ "key": f.name, "value": f.value });
                    if !f.on { field["disabled"] = json!(true); }
                    field
                }).collect::<Vec<_>>(),
            }),
            BodyType::Multipart => json!({
                "mode": "formdata",
                "formdata": b.fields.iter().map(|f| {
                    let mut field = if f.file {
                        json!({ "key": f.name, "type": "file", "src": f.value })
                    } else {
                        json!({ "key": f.name, "type": "text", "value": f.value })
                    };
                    if !f.on { field["disabled"] = json!(true); }
                    field
                }).collect::<Vec<_>>(),
            }),
            BodyType::None => Value::Null,
        };
    }
    let mut out = json!({ "method": method, "header": headers, "url": url(&call.url) });
    if !body.is_null() {
        out["body"] = body;
    }
    if let Some(auth) = &call.auth {
        let entry = |key: &str, value: &Option<String>| json!({ "key": key, "value": value.clone().unwrap_or_default(), "type": "string" });
        out["auth"] = match auth.kind {
            AuthType::None => json!({ "type": "noauth" }),
            AuthType::Bearer => {
                json!({ "type": "bearer", "bearer": [entry("token", &auth.token)] })
            }
            AuthType::Basic => {
                json!({ "type": "basic", "basic": [entry("username", &auth.user), entry("password", &auth.password)] })
            }
            AuthType::ApiKey => json!({
                "type": "apikey",
                "apikey": [
                    entry("key", &auth.key),
                    entry("value", &auth.value),
                    entry("in", &Some(if auth.place == Some(KeyPlace::Query) { "query".into() } else { "header".into() })),
                ],
            }),
        };
    }
    let mut description = call.notes.clone().unwrap_or_default();
    if call.kind == Kind::Soap
        && call
            .ws_security
            .as_ref()
            .is_some_and(|w| w.mode != super::model::WsMode::None)
    {
        if !description.is_empty() {
            description.push_str("\n\n");
        }
        description
            .push_str("Spagitty adds a WS-Security username token to this call; Postman does not.");
    }
    if !description.is_empty() {
        out["description"] = json!(description);
    }
    out
}

fn item(call: &Call) -> Value {
    let mut out = json!({ "name": call.name, "request": request(call) });
    let mut lines: Vec<String> = Vec::new();
    for rule in &call.keep {
        match postman_path(&rule.from) {
            Some(expression) => lines.push(format!(
                "pm.environment.set(\"{}\", {expression});",
                rule.var
            )),
            None => lines.push(format!(
                "// Spagitty keeps {} from {}, an XPath this script does not read.",
                rule.var, rule.from
            )),
        }
    }
    if !lines.is_empty() {
        out["event"] =
            json!([{ "listen": "test", "script": { "type": "text/javascript", "exec": lines } }]);
    }
    out
}

fn items(list: &[Item]) -> Vec<Value> {
    list.iter()
        .map(|entry| match entry {
            Item::Folder(folder) => json!({ "name": folder.folder, "item": items(&folder.items) }),
            Item::Call(call) => item(call),
        })
        .collect()
}

/// A collection as a Postman 2.1 collection.
pub fn postman(collection: &Collection) -> String {
    let mut out = Map::new();
    out.insert(
        "info".into(),
        json!({
            "name": collection.name,
            "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json",
        }),
    );
    out.insert("item".into(), Value::Array(items(&collection.items)));
    let vars: Vec<Value> = collection
        .vars
        .iter()
        .map(Variable::for_file)
        .map(|v| json!({ "key": v.name, "value": v.value.unwrap_or_default() }))
        .collect();
    if !vars.is_empty() {
        out.insert("variable".into(), Value::Array(vars));
    }
    let mut text = serde_json::to_string_pretty(&Value::Object(out)).unwrap_or_default();
    text.push('\n');
    text
}

/// An example response, by the call it answered.
#[derive(Debug, Clone)]
pub struct Example {
    pub call: String,
    pub media_type: String,
    pub body: Vec<u8>,
}

/// What each person has to fill in once they have the zip.
pub fn readme(collection: &Collection, environments: &[Environment]) -> String {
    let mut out = format!(
        "# {}\n\nA Calls collection from Spagitty. Open Calls, choose *Import*, and choose this zip.\n\n",
        collection.name
    );
    out.push_str("Secrets and values kept from responses were left out: they never leave the computer they are on.\n");
    let mut any = false;
    for env in environments {
        let missing: Vec<String> = env
            .vars
            .iter()
            .filter(|v| v.secret || v.kept || v.value.as_deref().unwrap_or("").is_empty())
            .map(|v| {
                let kind = if v.secret {
                    "secret"
                } else if v.kept {
                    "kept from a response: send the call that fills it"
                } else {
                    "empty"
                };
                format!("- `{}` ({kind})", v.name)
            })
            .collect();
        if !missing.is_empty() {
            if !any {
                out.push_str("\n## To fill in\n");
                any = true;
            }
            out.push_str(&format!("\n### {}\n\n{}\n", env.name, missing.join("\n")));
        }
    }
    if !any {
        out.push_str("\nNothing to fill in.\n");
    }
    out
}

fn extension(media_type: &str) -> &'static str {
    if media_type.contains("json") {
        "json"
    } else if media_type.contains("xml") {
        "xml"
    } else if media_type.contains("html") {
        "html"
    } else {
        "txt"
    }
}

/// The files of a zip: `collection.json`, `env/<name>.json`, `examples/`,
/// `history.json` when given, and `README.md`.
pub fn bundle(
    collection: &Collection,
    environments: &[Environment],
    examples: &[Example],
    history: Option<&str>,
) -> Vec<(String, Vec<u8>)> {
    let mut files = vec![(
        "collection.json".to_string(),
        spagitty(collection).into_bytes(),
    )];
    for env in environments {
        files.push((
            format!("env/{}.json", slug(&env.name)),
            to_text(&env.for_file()).into_bytes(),
        ));
    }
    for example in examples {
        files.push((
            format!(
                "examples/{}.{}",
                slug(&example.call),
                extension(&example.media_type)
            ),
            example.body.clone(),
        ));
    }
    if let Some(history) = history {
        files.push(("history.json".to_string(), history.as_bytes().to_vec()));
    }
    files.push((
        "README.md".to_string(),
        readme(collection, environments).into_bytes(),
    ));
    files
}

/// A zip of [`bundle`].
pub fn zipped(files: &[(String, Vec<u8>)]) -> Result<Vec<u8>> {
    zip::write(files)
}

/// What a shared zip holds, read back: its collection and environments.
pub fn unzip(bytes: &[u8]) -> Result<(Collection, Vec<Environment>)> {
    let files = zip::read(bytes, 64 * 1024 * 1024)?;
    let collection = files
        .iter()
        .find(|(name, _)| name == "collection.json")
        .ok_or_else(|| crate::Error::Calls("the zip has no collection.json".into()))?;
    let collection: Collection = serde_json::from_slice(&collection.1)
        .map_err(|e| crate::Error::Calls(format!("collection.json could not be read: {e}")))?;
    let environments = files
        .iter()
        .filter(|(name, _)| name.starts_with("env/") && name.ends_with(".json"))
        .filter_map(|(_, bytes)| serde_json::from_slice::<Environment>(bytes).ok())
        .collect();
    Ok((collection, environments))
}

/// Commit `files` (paths from the working tree) on the current branch, as
/// the person's own commit with their hooks and signing, and nothing else:
/// anything else already staged would go in too, so it is refused. Never
/// pushes. Answers with the commit's id.
pub fn commit(dir: &std::path::Path, files: &[String], subject: &str) -> Result<String> {
    use crate::{hooks, shell, signing};
    let staged = shell::staged_names(dir)?;
    let others: Vec<&String> = staged.iter().filter(|name| !files.contains(name)).collect();
    if !others.is_empty() {
        return Err(crate::Error::Calls(format!(
            "{} other {} already staged and would go into this commit too; commit or unstage {} first",
            others.len(),
            if others.len() == 1 { "change is" } else { "changes are" },
            if others.len() == 1 { "it" } else { "them" },
        )));
    }
    shell::stage(dir, files)?;
    if shell::staged_names(dir)?.is_empty() {
        return Err(crate::Error::Calls(
            "nothing has changed since the last commit of these files".into(),
        ));
    }
    let repository = crate::repo::open(dir)?;
    let signing = signing::read(&repository);
    let run_hooks = hooks::enabled(dir)?;
    shell::commit_with(
        dir,
        subject,
        "",
        false,
        signing.enabled,
        !run_hooks,
        &mut |_| {},
    )
    .map_err(|error| signing::as_signing_failure(error, &signing.program))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calls::import::postman as reader;
    use crate::calls::model::{
        Auth, Body, Colour, Field, Folder, KeepRule, Pair, WsMode, WsSecurity,
    };

    fn kinds() -> Vec<Variable> {
        vec![
            Variable::plain("baseUrl", "https://plain.example"),
            Variable {
                name: "password".into(),
                value: Some("SECRET-hunter2".into()),
                secret: true,
                ..Variable::default()
            },
            Variable {
                name: "token".into(),
                value: Some("KEPT-ey.J".into()),
                kept: true,
                ..Variable::default()
            },
        ]
    }

    fn collection() -> Collection {
        let login = Call {
            id: "c_1".into(),
            name: "Log in".into(),
            method: Some("POST".into()),
            url: "{{baseUrl}}/auth/login?x=1".into(),
            headers: vec![Pair::new("Accept", "application/json")],
            body: Some(Body {
                kind: BodyType::Json,
                text: Some("{\"email\": \"{{email}}\"}".into()),
                ..Body::default()
            }),
            auth: Some(Auth {
                kind: AuthType::Bearer,
                token: Some("{{token}}".into()),
                ..Auth::default()
            }),
            keep: vec![
                KeepRule {
                    var: "token".into(),
                    from: "$.data.accessToken".into(),
                    every: true,
                    ..KeepRule::default()
                },
                KeepRule {
                    var: "userId".into(),
                    from: "$.data.user['id']".into(),
                    every: true,
                    ..KeepRule::default()
                },
            ],
            ..Call::default()
        };
        let upload = Call {
            id: "c_2".into(),
            name: "Upload".into(),
            method: Some("PUT".into()),
            url: "https://api.example.com/v1/files".into(),
            body: Some(Body {
                kind: BodyType::Multipart,
                fields: vec![Field {
                    name: "doc".into(),
                    value: "a.pdf".into(),
                    file: true,
                    on: true,
                }],
                ..Body::default()
            }),
            ..Call::default()
        };
        let soap = Call {
            id: "c_3".into(),
            name: "GetInvoice".into(),
            kind: Kind::Soap,
            url: "{{baseUrl}}/ws".into(),
            version: Some("1.1".into()),
            action: Some("urn:GetInvoice".into()),
            envelope: Some("<soapenv:Envelope/>".into()),
            ws_security: Some(WsSecurity {
                mode: WsMode::Username,
                ..WsSecurity::default()
            }),
            ..Call::default()
        };
        Collection {
            v: 1,
            name: "orders-api".into(),
            wsdl: None,
            vars: kinds(),
            items: vec![
                Item::Folder(Folder {
                    folder: "Auth".into(),
                    items: vec![Item::Call(Box::new(login))],
                }),
                Item::Call(Box::new(upload)),
                Item::Call(Box::new(soap)),
            ],
        }
    }

    #[test]
    fn no_format_lets_a_secret_or_kept_value_out() {
        let mut env = Environment::new("staging", Colour::Warn);
        env.vars = kinds();
        let files = bundle(&collection(), &[env.clone()], &[], Some("[]"));
        let zip = zipped(&files).unwrap();
        let unpacked = zip::read(&zip, 1 << 20).unwrap();
        let mut texts: Vec<String> = unpacked
            .iter()
            .map(|(_, b)| String::from_utf8_lossy(b).into_owned())
            .collect();
        texts.push(spagitty(&collection()));
        texts.push(postman(&collection()));
        for text in &texts {
            assert!(!text.contains("SECRET-"), "{text}");
            assert!(!text.contains("KEPT-"), "{text}");
        }
        assert!(texts.iter().any(|t| t.contains("https://plain.example")));
        // The README names what to fill in.
        let readme = &texts[unpacked.iter().position(|(n, _)| n == "README.md").unwrap()];
        assert!(readme.contains("- `password` (secret)"));
        assert!(readme.contains("- `token` (kept from a response"));
    }

    #[test]
    fn a_zip_is_read_back() {
        let mut env = Environment::new("staging", Colour::Warn);
        env.vars = vec![Variable::plain("baseUrl", "https://s")];
        let zip = zipped(&bundle(
            &collection(),
            &[env.clone()],
            &[Example {
                call: "Log in".into(),
                media_type: "application/json".into(),
                body: b"{}".to_vec(),
            }],
            None,
        ))
        .unwrap();
        let names: Vec<String> = zip::read(&zip, 1 << 20)
            .unwrap()
            .into_iter()
            .map(|(n, _)| n)
            .collect();
        assert_eq!(
            names,
            [
                "collection.json",
                "env/staging.json",
                "examples/log-in.json",
                "README.md"
            ]
        );
        let (back, envs) = unzip(&zip).unwrap();
        assert_eq!(back.name, "orders-api");
        assert_eq!(super::super::model::calls(&back.items).len(), 3);
        assert_eq!(envs, vec![env]);
    }

    #[test]
    fn postman_round_trips_through_the_importer() {
        let text = postman(&collection());
        let value: Value = serde_json::from_str(&text).unwrap();
        assert!(reader::is_collection(&value));
        let (endpoints, vars, _) = reader::read("x.json", &value);
        assert_eq!(
            vars.iter().map(|v| v.name.as_str()).collect::<Vec<_>>(),
            ["baseUrl", "password", "token"]
        );
        assert_eq!(vars[1].value.as_deref(), Some(""));
        let login = &endpoints[0];
        assert_eq!(login.folder.as_deref(), Some("Auth"));
        assert_eq!(login.call.url, "{{baseUrl}}/auth/login?x=1");
        assert_eq!(login.call.method.as_deref(), Some("POST"));
        assert_eq!(
            login.call.auth.as_ref().unwrap().token.as_deref(),
            Some("{{token}}")
        );
        assert_eq!(
            login.call.body.as_ref().unwrap().text.as_deref(),
            Some("{\"email\": \"{{email}}\"}")
        );
        assert_eq!(
            login
                .call
                .keep
                .iter()
                .map(|r| (r.var.as_str(), r.from.as_str()))
                .collect::<Vec<_>>(),
            [
                ("token", "$.data.accessToken"),
                ("userId", "$.data.user['id']")
            ]
        );
        let upload = &endpoints[1].call;
        assert_eq!(upload.url, "https://api.example.com/v1/files");
        assert!(upload.body.as_ref().unwrap().fields[0].file);
        // SOAP is a raw POST of the envelope with its headers.
        let soap = &endpoints[2].call;
        assert_eq!(soap.method.as_deref(), Some("POST"));
        assert_eq!(
            soap.body.as_ref().unwrap().text.as_deref(),
            Some("<soapenv:Envelope/>")
        );
        assert!(soap
            .headers
            .iter()
            .any(|h| h.name == "SOAPAction" && h.value == "\"urn:GetInvoice\""));
        assert!(soap.notes.as_deref().unwrap().contains("WS-Security"));
    }

    #[test]
    fn a_commit_takes_the_listed_files_and_nothing_else() {
        let fixture = crate::fixture::Fixture::empty();
        fixture.write(".spagitty/calls/orders-api.json", &spagitty(&collection()));
        fixture.write("other.txt", "x");
        let files = vec![".spagitty/calls/orders-api.json".to_string()];
        let id = commit(fixture.path(), &files, "Calls: update orders-api").unwrap();
        assert!(!id.is_empty());
        let shown = fixture.git(&["show", "--name-only", "--format=%s", "HEAD"]);
        assert!(shown.contains("Calls: update orders-api"));
        assert!(shown.contains(".spagitty/calls/orders-api.json"));
        assert!(!shown.contains("other.txt"));
        // Nothing changed: nothing to commit.
        assert!(commit(fixture.path(), &files, "again").is_err());
        // Something else staged would go in too: refused.
        fixture.git(&["add", "other.txt"]);
        fixture.write(".spagitty/calls/orders-api.json", "{}");
        assert!(commit(fixture.path(), &files, "x")
            .unwrap_err()
            .to_string()
            .contains("already staged"));
    }

    #[test]
    fn a_jsonpath_is_a_postman_expression() {
        assert_eq!(
            postman_path("$.a.b[0]['c-d']").as_deref(),
            Some("pm.response.json().a.b[0][\"c-d\"]")
        );
        assert_eq!(postman_path("//inv:total"), None);
    }
}
