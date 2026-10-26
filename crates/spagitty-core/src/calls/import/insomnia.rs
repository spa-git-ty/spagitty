// SPDX-License-Identifier: GPL-3.0-or-later

//! Insomnia exports, format 4 (FEAT-162, proposal §8.3): requests in their
//! request groups, and environments. Insomnia writes a variable as
//! `{{ _.baseUrl }}`; Calls writes `{{baseUrl}}`.

use serde_json::Value;

use super::super::model::{
    new_id, Auth, AuthType, Body, BodyType, Call, Environment, Field, Pair, Variable,
};
use super::{key_of, path_of, Endpoint, Status};

pub fn is_export(value: &Value) -> bool {
    value["_type"].as_str() == Some("export") && value["resources"].is_array()
}

/// `{{ _.name }}` and `{{_.name}}` as `{{name}}`.
fn vars(text: &str) -> String {
    let re = regex::Regex::new(r"\{\{\s*_\.([\w.-]+)\s*\}\}").expect("a valid pattern");
    re.replace_all(text, "{{$1}}").into_owned()
}

fn s(value: &Value) -> String {
    vars(value.as_str().unwrap_or(""))
}

fn body_of(body: &Value) -> Option<Body> {
    let mime = body["mimeType"].as_str().unwrap_or("");
    if mime.contains("form-urlencoded") || mime.contains("multipart") {
        let fields = body["params"]
            .as_array()
            .map(|list| {
                list.iter()
                    .map(|p| Field {
                        name: s(&p["name"]),
                        value: if p["type"] == "file" {
                            s(&p["fileName"])
                        } else {
                            s(&p["value"])
                        },
                        file: p["type"] == "file",
                        on: p["disabled"].as_bool() != Some(true),
                    })
                    .collect()
            })
            .unwrap_or_default();
        return Some(Body {
            kind: if mime.contains("multipart") {
                BodyType::Multipart
            } else {
                BodyType::Form
            },
            fields,
            ..Body::default()
        });
    }
    let text = body["text"].as_str()?;
    Some(Body {
        kind: if mime.contains("json") {
            BodyType::Json
        } else if mime.contains("xml") {
            BodyType::Xml
        } else {
            BodyType::Raw
        },
        text: Some(vars(text)),
        content_type: (!mime.is_empty() && !mime.contains("json") && !mime.contains("xml"))
            .then(|| mime.to_string()),
        ..Body::default()
    })
}

fn auth_of(auth: &Value) -> Option<Auth> {
    match auth["type"].as_str()? {
        "bearer" => Some(Auth {
            kind: AuthType::Bearer,
            token: Some(s(&auth["token"])),
            ..Auth::default()
        }),
        "basic" => Some(Auth {
            kind: AuthType::Basic,
            user: Some(s(&auth["username"])),
            password: Some(s(&auth["password"])),
            ..Auth::default()
        }),
        _ => None,
    }
}

/// The export's requests and environments. The base environment's values
/// are shared, so they are folded into each sub-environment.
pub fn read(file: &str, value: &Value) -> (Vec<Endpoint>, Vec<Environment>) {
    let resources = value["resources"].as_array().cloned().unwrap_or_default();
    let by_id = |id: &str| resources.iter().find(|r| r["_id"].as_str() == Some(id));
    let folder_of = |parent: &str| -> Option<String> {
        let mut names = Vec::new();
        let mut at = parent.to_string();
        for _ in 0..16 {
            let Some(found) = by_id(&at) else { break };
            if found["_type"] != "request_group" {
                break;
            }
            names.push(found["name"].as_str().unwrap_or("Folder").to_string());
            at = found["parentId"].as_str().unwrap_or("").to_string();
        }
        names.reverse();
        (!names.is_empty()).then(|| names.join(" / "))
    };
    let mut out = Vec::new();
    for resource in resources.iter().filter(|r| r["_type"] == "request") {
        let headers = resource["headers"]
            .as_array()
            .map(|list| {
                list.iter()
                    .map(|h| Pair {
                        name: s(&h["name"]),
                        value: s(&h["value"]),
                        on: h["disabled"].as_bool() != Some(true),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let call = Call {
            id: new_id(),
            name: resource["name"].as_str().unwrap_or("Request").to_string(),
            method: Some(
                resource["method"]
                    .as_str()
                    .unwrap_or("GET")
                    .to_ascii_uppercase(),
            ),
            url: s(&resource["url"]),
            headers,
            body: body_of(&resource["body"]),
            auth: auth_of(&resource["authentication"]),
            ..Call::default()
        };
        let folder = folder_of(resource["parentId"].as_str().unwrap_or(""));
        out.push(Endpoint {
            id: format!("{file}#{}", out.len()),
            key: key_of(&call),
            source: format!("file:{file}"),
            group: folder.clone().unwrap_or_else(|| "Requests".into()),
            file: Some(file.to_string()),
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

    let environments: Vec<&Value> = resources
        .iter()
        .filter(|r| r["_type"] == "environment")
        .collect();
    let base = environments.iter().find(|e| {
        by_id(e["parentId"].as_str().unwrap_or("")).is_some_and(|p| p["_type"] == "workspace")
    });
    let plain = |data: &Value| -> Vec<Variable> {
        data.as_object()
            .map(|map| {
                map.iter()
                    .filter(|(_, v)| !v.is_object() && !v.is_array())
                    .map(|(k, v)| {
                        Variable::plain(
                            k,
                            &vars(
                                &v.as_str()
                                    .map(str::to_string)
                                    .unwrap_or_else(|| v.to_string()),
                            ),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default()
    };
    let shared = base.map(|b| plain(&b["data"])).unwrap_or_default();
    let mut envs = Vec::new();
    for env in &environments {
        if Some(env) == base {
            continue;
        }
        let name = super::super::store::slug(env["name"].as_str().unwrap_or("imported"));
        let mut made = Environment::new(&name, Default::default());
        made.vars = shared.clone();
        for var in plain(&env["data"]) {
            made.vars.retain(|v| v.name != var.name);
            made.vars.push(var);
        }
        envs.push(made);
    }
    (out, envs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_export_gives_requests_in_groups_and_environments() {
        let export = r#"{"_type":"export","__export_format":4,"resources":[
          {"_id":"wrk_1","_type":"workspace","name":"Shop"},
          {"_id":"fld_1","_type":"request_group","parentId":"wrk_1","name":"Orders"},
          {"_id":"fld_2","_type":"request_group","parentId":"fld_1","name":"Admin"},
          {"_id":"req_1","_type":"request","parentId":"fld_2","name":"Cancel","method":"post",
           "url":"{{ _.baseUrl }}/orders/1/cancel","headers":[{"name":"X-Why","value":"{{_.reason}}"}],
           "body":{"mimeType":"application/json","text":"{\"note\":\"{{ _.note }}\"}"},
           "authentication":{"type":"bearer","token":"{{ _.token }}"}},
          {"_id":"env_1","_type":"environment","parentId":"wrk_1","name":"Base Environment","data":{"baseUrl":"http://localhost:3000","tenant":"acme"}},
          {"_id":"env_2","_type":"environment","parentId":"env_1","name":"Staging","data":{"baseUrl":"https://staging"}}
        ]}"#;
        let value: Value = serde_json::from_str(export).unwrap();
        assert!(is_export(&value));
        let (endpoints, envs) = read("insomnia.json", &value);
        let call = &endpoints[0].call;
        assert_eq!(endpoints[0].folder.as_deref(), Some("Orders / Admin"));
        assert_eq!(call.url, "{{baseUrl}}/orders/1/cancel");
        assert_eq!(call.headers[0].value, "{{reason}}");
        assert_eq!(
            call.body.as_ref().unwrap().text.as_deref(),
            Some("{\"note\":\"{{note}}\"}")
        );
        assert_eq!(
            call.auth.as_ref().unwrap().token.as_deref(),
            Some("{{token}}")
        );
        assert_eq!(envs.len(), 1);
        assert_eq!(envs[0].name, "staging");
        let names: Vec<(&str, &str)> = envs[0]
            .vars
            .iter()
            .map(|v| (v.name.as_str(), v.value.as_deref().unwrap_or("")))
            .collect();
        assert_eq!(names, [("tenant", "acme"), ("baseUrl", "https://staging")]);
    }
}
