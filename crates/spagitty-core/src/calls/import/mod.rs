// SPDX-License-Identifier: GPL-3.0-or-later

//! Import (FEAT-162, proposal §8): calls from code and from collection files.
//!
//! Two kinds of reader feed it.
//!
//! - **Code** — Java controllers (`java`) and OpenAPI files (`openapi`) —
//!   says what an endpoint *declares*: a method, a path with its variables,
//!   parameters, headers and the shape of its body. [`build`] makes that a
//!   call with the person's choices from *Bring in*: whether path variables
//!   become `{{variables}}` and whether the body is an example.
//! - **Files** — Postman, Insomnia, Bruno, `.http`, HAR, cURL — already hold
//!   calls, which are taken as they are.
//!
//! Every endpoint is then matched against the repository's collections by
//! method and normalised path ([`key_of`]), and is *New*, *Changed* (its
//! shape differs, and [`Status::Changed`] says how) or *In* a collection.
//! Bringing a changed one in ([`update`]) adds what is new and removes what
//! is gone, and keeps the values the person typed.
//!
//! Nothing here builds, starts or calls the service, and nothing is
//! written but the collection the person brings calls into.

pub mod bruno;
pub mod curl;
pub mod har;
pub mod http_file;
pub mod insomnia;
pub mod java;
pub mod json;
pub mod nest;
pub mod openapi;
pub mod postman;
pub mod scan;
pub mod wsdl_source;

use serde::{Deserialize, Serialize};

use super::model::{
    new_id, Body, BodyType, Call, Collection, Environment, Field, Folder, Item, Kind, Pair,
};
use json::J;

/// Where endpoints came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SourceKind {
    Spring,
    JaxRs,
    /// NestJS controllers (the author's request, 2026-10-26).
    Nest,
    /// JAX-WS `@WebService` classes (FEAT-163).
    JaxWs,
    /// Spring-WS `@Endpoint` classes (FEAT-163).
    SpringWs,
    /// A WSDL file (FEAT-163).
    Wsdl,
    OpenApi,
    Postman,
    Insomnia,
    Bruno,
    Http,
    Har,
    Curl,
    Spagitty,
}

impl SourceKind {
    /// Read from code, rather than a file that already holds calls.
    pub fn is_code(self) -> bool {
        matches!(
            self,
            SourceKind::Spring
                | SourceKind::JaxRs
                | SourceKind::Nest
                | SourceKind::OpenApi
                | SourceKind::JaxWs
                | SourceKind::SpringWs
                | SourceKind::Wsdl
        )
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Source {
    pub id: String,
    pub kind: SourceKind,
    /// What the screen calls it: `OrderController.java`, `orders.postman_collection.json`.
    pub label: String,
    /// From the working tree; absent for a pasted cURL.
    pub file: Option<String>,
    pub count: usize,
    /// Why it could not be read.
    pub error: Option<String>,
    /// A file in the repository, which can be opened in place.
    pub in_repository: bool,
}

/// One endpoint's state against the repository's collections.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    tag = "kind"
)]
pub enum Status {
    New,
    /// In `collection`, as `call_id`, with a different shape: `what` says how.
    Changed {
        collection: String,
        call_id: String,
        what: String,
    },
    In {
        collection: String,
        call_id: String,
    },
}

/// A parameter a piece of code declares.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Param {
    pub name: String,
    /// A default or example value, when the code or the file gives one.
    pub sample: String,
    pub required: bool,
}

/// What a piece of code says an endpoint is.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Declared {
    pub method: String,
    /// With `{name}` for each path variable.
    pub path: String,
    pub query: Vec<Param>,
    pub headers: Vec<Param>,
    /// The body's example, and the class or schema it came from.
    #[serde(skip)]
    pub body: Option<J>,
    pub body_type: Option<String>,
    /// `application/json`, `application/x-www-form-urlencoded`, …
    pub consumes: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Endpoint {
    /// Unique within one scan.
    pub id: String,
    pub key: String,
    pub source: String,
    /// The controller, service, tag or folder it is drawn under.
    pub group: String,
    pub file: Option<String>,
    pub line: Option<u32>,
    pub method: String,
    pub path: String,
    /// The request class, the parameters, the media type.
    pub hint: String,
    /// The call as it would be brought in with the default choices.
    pub call: Call,
    pub status: Status,
    /// Set for code: what it declares, to build again with other choices.
    #[serde(skip)]
    pub declared: Option<Declared>,
    /// The folder it was in, for a file: Postman folders, Bruno directories.
    #[serde(skip)]
    pub folder: Option<String>,
}

/// Everything one scan or one file found.
#[derive(Debug, Clone, Default, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Found {
    pub sources: Vec<Source>,
    pub endpoints: Vec<Endpoint>,
    /// Environments a file brought: Postman, Insomnia and Bruno ones.
    pub environments: Vec<Environment>,
    /// Calls that had Postman script lines Import did not turn into a keep
    /// rule; those lines are in the call's notes.
    pub scripted: usize,
    /// `http://localhost:<port><context>` from `application.yml`.
    pub base_url: Option<String>,
    pub files_read: usize,
    /// The scan stopped at the file cap.
    pub capped: bool,
}

/// What *Bring in* was asked to do.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Folders {
    /// One per controller, service or file folder.
    #[default]
    PerGroup,
    /// By the path's first segment: `/orders/{id}` is in `orders`.
    ByPath,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Choices {
    pub folders: Folders,
    /// *Example bodies from request classes*.
    pub bodies: bool,
    /// *Path variables become variables*.
    pub path_vars: bool,
}

impl Default for Choices {
    fn default() -> Self {
        Choices {
            folders: Folders::PerGroup,
            bodies: true,
            path_vars: true,
        }
    }
}

/// A path with every variable as `{}`, without a trailing slash: `/orders/{id}`,
/// `/orders/{{orderId}}` and `/orders/:id` are the same path.
pub fn normalise(path: &str) -> String {
    let path = path.split(['?', '#']).next().unwrap_or("");
    let segments: Vec<String> = path
        .split('/')
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let variable = (segment.starts_with('{') && segment.ends_with('}'))
                || segment.starts_with(':')
                || segment.starts_with('<') && segment.ends_with('>');
            if variable {
                "{}".to_string()
            } else {
                segment.to_string()
            }
        })
        .collect();
    format!("/{}", segments.join("/"))
}

/// The path of a call's URL, without what stands for the server: a leading
/// `{{variable}}`, or a scheme and host.
pub fn path_of(url: &str) -> String {
    let url = url.trim();
    let rest = if let Some(after) = url.strip_prefix("{{") {
        after.split_once("}}").map(|(_, rest)| rest).unwrap_or("")
    } else if let Some((_, after)) = url.split_once("://") {
        after.find('/').map(|at| &after[at..]).unwrap_or("/")
    } else {
        url
    };
    rest.to_string()
}

/// The matching key: `GET /orders/{}`, or for SOAP its service and operation.
pub fn key_of(call: &Call) -> String {
    match call.kind {
        Kind::Soap => format!(
            "SOAP {}#{}",
            call.service.as_deref().unwrap_or(""),
            call.operation.as_deref().unwrap_or("")
        ),
        Kind::Rest => format!(
            "{} {}",
            call.method().to_ascii_uppercase(),
            normalise(&path_of(&call.url))
        ),
    }
}

/// The names in a URL's query, in order.
fn query_names(url: &str) -> Vec<String> {
    let Some((_, query)) = url.split_once('?') else {
        return Vec::new();
    };
    query
        .split('#')
        .next()
        .unwrap_or("")
        .split('&')
        .filter(|part| !part.is_empty())
        .map(|part| part.split('=').next().unwrap_or("").to_string())
        .collect()
}

/// What change detection compares: the query's names, the form's names and
/// every key path in a JSON body.
pub fn shape(call: &Call) -> Vec<String> {
    let mut out: Vec<String> = query_names(&call.url)
        .into_iter()
        .map(|name| format!("?{name}"))
        .collect();
    if let Some(body) = &call.body {
        match body.kind {
            BodyType::Json => {
                if let Some(value) = body.text.as_deref().and_then(json::parse) {
                    value.paths("", &mut out);
                }
            }
            BodyType::Form | BodyType::Multipart => {
                out.extend(body.fields.iter().map(|field| field.name.clone()))
            }
            _ => {}
        }
    }
    out
}

/// How `fresh` (what the code says now) differs from `kept` (the call in
/// the collection), in words, or `None` when the shape is the same.
pub fn difference(fresh: &Call, kept: &Call, body_type: Option<&str>) -> Option<String> {
    let new = shape(fresh);
    let old = shape(kept);
    let gained: Vec<&str> = new
        .iter()
        .filter(|name| !old.contains(name))
        .map(|name| name.trim_start_matches('?'))
        .collect();
    let lost: Vec<&str> = old
        .iter()
        .filter(|name| !new.contains(name))
        .map(|name| name.trim_start_matches('?'))
        .collect();
    if gained.is_empty() && lost.is_empty() {
        return None;
    }
    let mut parts = Vec::new();
    if !gained.is_empty() {
        parts.push(format!("gained {}", gained.join(", ")));
    }
    if !lost.is_empty() {
        parts.push(format!("lost {}", lost.join(", ")));
    }
    let subject = body_type.unwrap_or("The request");
    Some(format!("{subject} {}", parts.join("; ")))
}

/// Every call in the collections, by its key, with the collection's stem.
fn index(collections: &[(String, Collection)]) -> Vec<(String, String, Call)> {
    let mut out = Vec::new();
    for (stem, collection) in collections {
        for (_, call) in super::model::calls(&collection.items) {
            out.push((key_of(call), stem.clone(), call.clone()));
        }
    }
    out
}

/// Mark each endpoint *New*, *Changed* or *In* against `collections`.
pub fn mark(endpoints: &mut [Endpoint], collections: &[(String, Collection)]) {
    let known = index(collections);
    for endpoint in endpoints {
        endpoint.status = match known.iter().find(|(key, _, _)| *key == endpoint.key) {
            None => Status::New,
            Some((_, stem, call)) => {
                let body_type = endpoint
                    .declared
                    .as_ref()
                    .and_then(|d| d.body_type.as_deref());
                match difference(&endpoint.call, call, body_type) {
                    // Only code can say a call's shape has changed: a file
                    // is someone's copy, not the source of truth.
                    Some(what) if endpoint.declared.is_some() => Status::Changed {
                        collection: stem.clone(),
                        call_id: call.id.clone(),
                        what,
                    },
                    _ => Status::In {
                        collection: stem.clone(),
                        call_id: call.id.clone(),
                    },
                }
            }
        };
    }
}

/// A name for a call from what it does: `GET /orders/{id}` is *Get order*.
pub fn name_for(method: &str, path: &str, given: Option<&str>) -> String {
    if let Some(given) = given.map(str::trim).filter(|g| !g.is_empty()) {
        return given.to_string();
    }
    let words: Vec<&str> = path
        .split('/')
        .filter(|s| !s.is_empty() && !s.starts_with('{') && !s.starts_with(':'))
        .collect();
    let last = words.last().copied().unwrap_or("root");
    let verb = match method {
        "GET" => "Get",
        "POST" => "Create",
        "PUT" => "Replace",
        "PATCH" => "Update",
        "DELETE" => "Delete",
        other => other,
    };
    format!("{verb} {last}")
}

/// Split a camelCase method name into words: `createOrder` is *Create order*.
pub fn words_of(identifier: &str) -> String {
    let mut out = String::new();
    for (i, c) in identifier.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            out.push(' ');
            out.extend(c.to_lowercase());
        } else if i == 0 {
            out.extend(c.to_uppercase());
        } else {
            out.push(c);
        }
    }
    out.replace('_', " ")
}

/// A declared endpoint as a call, with `choices`.
pub fn build(declared: &Declared, name: &str, choices: Choices) -> Call {
    let path = if choices.path_vars {
        let mut out = String::new();
        let mut rest = declared.path.as_str();
        while let Some(open) = rest.find('{') {
            out.push_str(&rest[..open]);
            let Some(close) = rest[open..].find('}') else {
                break;
            };
            let inner = &rest[open + 1..open + close];
            // `{id:[0-9]+}` in JAX-RS: the name is before the colon.
            let var = inner.split(':').next().unwrap_or(inner).trim();
            out.push_str(&format!("{{{{{var}}}}}"));
            rest = &rest[open + close + 1..];
        }
        out.push_str(rest);
        out
    } else {
        declared.path.clone()
    };
    let path = if path.starts_with('/') {
        path
    } else {
        format!("/{path}")
    };
    let query: Vec<String> = declared
        .query
        .iter()
        .map(|param| {
            if param.sample.is_empty() {
                param.name.clone()
            } else {
                format!("{}={}", param.name, param.sample)
            }
        })
        .collect();
    let url = if query.is_empty() {
        format!("{{{{baseUrl}}}}{path}")
    } else {
        format!("{{{{baseUrl}}}}{path}?{}", query.join("&"))
    };
    let mut headers: Vec<Pair> = declared
        .headers
        .iter()
        .map(|param| Pair::new(&param.name, &param.sample))
        .collect();
    let consumes = declared.consumes.as_deref().unwrap_or("application/json");
    let body = match &declared.body {
        _ if matches!(
            declared.method.as_str(),
            "GET" | "HEAD" | "DELETE" | "OPTIONS"
        ) && declared.body.is_none() =>
        {
            None
        }
        None => None,
        Some(example)
            if consumes.contains("x-www-form-urlencoded") || consumes.contains("multipart") =>
        {
            let fields = example
                .obj()
                .iter()
                .map(|(name, value)| Field {
                    name: name.clone(),
                    value: if choices.bodies {
                        value.text().unwrap_or_default()
                    } else {
                        String::new()
                    },
                    file: false,
                    on: true,
                })
                .collect();
            Some(Body {
                kind: if consumes.contains("multipart") {
                    BodyType::Multipart
                } else {
                    BodyType::Form
                },
                fields,
                ..Body::default()
            })
        }
        Some(example) => Some(Body {
            kind: if consumes.contains("xml") {
                BodyType::Xml
            } else {
                BodyType::Json
            },
            text: Some(if choices.bodies {
                example.pretty()
            } else if matches!(example, J::Arr(_)) {
                "[]".to_string()
            } else {
                "{}".to_string()
            }),
            ..Body::default()
        }),
    };
    if body.is_some()
        && !consumes.is_empty()
        && consumes != "application/json"
        && !consumes.contains("form")
    {
        headers.push(Pair::new("Content-Type", consumes));
    }
    Call {
        id: new_id(),
        name: name.to_string(),
        kind: Kind::Rest,
        method: Some(declared.method.clone()),
        url,
        headers,
        body,
        ..Call::default()
    }
}

/// Bring `fresh`'s new shape into `kept`, keeping what the person typed:
/// query parameters and body fields that are still there keep their values,
/// new ones come in with their samples, and ones that are gone go.
pub fn update(kept: &mut Call, fresh: &Call) {
    // The query.
    let old: Vec<(String, String)> = kept
        .url
        .split_once('?')
        .map(|(_, q)| {
            q.split('&')
                .filter(|p| !p.is_empty())
                .map(|p| match p.split_once('=') {
                    Some((n, v)) => (n.to_string(), v.to_string()),
                    None => (p.to_string(), String::new()),
                })
                .collect()
        })
        .unwrap_or_default();
    let new: Vec<(String, String)> = fresh
        .url
        .split_once('?')
        .map(|(_, q)| {
            q.split('&')
                .filter(|p| !p.is_empty())
                .map(|p| match p.split_once('=') {
                    Some((n, v)) => (n.to_string(), v.to_string()),
                    None => (p.to_string(), String::new()),
                })
                .collect()
        })
        .unwrap_or_default();
    let base = kept.url.split('?').next().unwrap_or("").to_string();
    let mut query: Vec<(String, String)> = old
        .iter()
        .filter(|(name, _)| new.iter().any(|(n, _)| n == name))
        .cloned()
        .collect();
    for (name, value) in &new {
        if !query.iter().any(|(n, _)| n == name) {
            query.push((name.clone(), value.clone()));
        }
    }
    kept.url = if query.is_empty() {
        base
    } else {
        let parts: Vec<String> = query
            .iter()
            .map(|(n, v)| {
                if v.is_empty() {
                    n.clone()
                } else {
                    format!("{n}={v}")
                }
            })
            .collect();
        format!("{base}?{}", parts.join("&"))
    };

    // The body.
    match (&mut kept.body, &fresh.body) {
        (Some(old), Some(new)) if old.kind == BodyType::Json && new.kind == BodyType::Json => {
            let parsed = (
                new.text.as_deref().and_then(json::parse),
                old.text.as_deref().and_then(json::parse),
            );
            // A body with a bare `{{variable}}` is not JSON and is left as
            // it is: rewriting what cannot be read would lose it.
            if let (Some(sample), Some(mine)) = parsed {
                old.text = Some(json::merge(&sample, &mine).pretty());
            }
        }
        (Some(old), Some(new))
            if matches!(old.kind, BodyType::Form | BodyType::Multipart) && old.kind == new.kind =>
        {
            let mut fields: Vec<Field> = old
                .fields
                .iter()
                .filter(|field| new.fields.iter().any(|f| f.name == field.name))
                .cloned()
                .collect();
            for field in &new.fields {
                if !fields.iter().any(|f| f.name == field.name) {
                    fields.push(field.clone());
                }
            }
            old.fields = fields;
        }
        (None, Some(new)) => kept.body = Some(new.clone()),
        _ => {}
    }
}

/// The folder an endpoint goes in.
fn folder_for(endpoint: &Endpoint, folders: Folders) -> Option<String> {
    match folders {
        Folders::PerGroup => endpoint
            .folder
            .clone()
            .or_else(|| Some(endpoint.group.clone()))
            .filter(|g| !g.is_empty()),
        Folders::ByPath => endpoint
            .path
            .split('/')
            .find(|s| !s.is_empty() && !s.starts_with('{') && !s.starts_with(':'))
            .map(str::to_string),
    }
}

fn add_to_folder(items: &mut Vec<Item>, folder: Option<&str>, call: Call) {
    let Some(name) = folder else {
        items.push(Item::Call(Box::new(call)));
        return;
    };
    for item in items.iter_mut() {
        if let Item::Folder(found) = item {
            if found.folder == name {
                found.items.push(Item::Call(Box::new(call)));
                return;
            }
        }
    }
    items.push(Item::Folder(Folder {
        folder: name.to_string(),
        items: vec![Item::Call(Box::new(call))],
    }));
}

/// What bringing endpoints in did.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Brought {
    pub added: usize,
    pub changed: usize,
    pub unchanged: usize,
}

/// Bring `endpoints` into `collection`. A changed one updates the call it
/// matched when that call is in this collection; one already in it is left;
/// anything else is added, in its folder.
pub fn bring_in(
    collection: &mut Collection,
    stem: &str,
    endpoints: &[&Endpoint],
    choices: Choices,
) -> Brought {
    let mut brought = Brought::default();
    // A SOAP collection keeps the WSDL in the repository it was made from,
    // for *Rebuild from WSDL* (FEAT-163).
    if collection.wsdl.is_none() {
        collection.wsdl = endpoints
            .iter()
            .filter(|e| e.call.kind == Kind::Soap)
            .filter_map(|e| e.file.clone())
            .find(|file| {
                file.ends_with(".wsdl")
                    && !file.contains("://")
                    && !file.starts_with('/')
                    && !file.contains(':')
            });
    }
    for endpoint in endpoints {
        let fresh = match &endpoint.declared {
            Some(declared) => build(declared, &endpoint.call.name, choices),
            None => endpoint.call.clone(),
        };
        let matched = match &endpoint.status {
            Status::Changed {
                collection: c,
                call_id,
                ..
            }
            | Status::In {
                collection: c,
                call_id,
            } if c == stem => Some(call_id.clone()),
            _ => None,
        };
        match matched {
            Some(id) => {
                let changed = matches!(endpoint.status, Status::Changed { .. });
                if let Some(call) = super::model::call_mut(&mut collection.items, &id) {
                    if changed {
                        update(call, &fresh);
                        brought.changed += 1;
                    } else {
                        brought.unchanged += 1;
                    }
                }
            }
            None => {
                let mut call = fresh;
                call.id = new_id();
                add_to_folder(
                    &mut collection.items,
                    folder_for(endpoint, choices.folders).as_deref(),
                    call,
                );
                brought.added += 1;
            }
        }
    }
    brought
}

#[cfg(test)]
mod tests {
    use super::*;

    fn declared(method: &str, path: &str) -> Declared {
        Declared {
            method: method.into(),
            path: path.into(),
            ..Declared::default()
        }
    }

    #[test]
    fn paths_match_whatever_their_variables_are_called() {
        assert_eq!(normalise("/orders/{id}/"), "/orders/{}");
        assert_eq!(normalise("/orders/{{orderId}}?x=1"), "/orders/{}");
        assert_eq!(normalise("orders/:id"), "/orders/{}");
        assert_eq!(path_of("{{baseUrl}}/orders/1"), "/orders/1");
        assert_eq!(path_of("https://api.example.com/v1/orders"), "/v1/orders");
        assert_eq!(path_of("https://api.example.com"), "/");
        let call = build(&declared("GET", "/orders/{id}"), "Get", Choices::default());
        assert_eq!(key_of(&call), "GET /orders/{}");
    }

    #[test]
    fn a_declared_endpoint_is_built_with_the_choices() {
        let mut d = declared("POST", "/orders/{id:[0-9]+}/lines");
        d.query = vec![Param {
            name: "dryRun".into(),
            sample: "false".into(),
            required: false,
        }];
        d.headers = vec![Param {
            name: "X-Tenant".into(),
            ..Param::default()
        }];
        d.body = json::parse(r#"{"sku":"string","qty":0}"#);
        let call = build(&d, "Add line", Choices::default());
        assert_eq!(call.url, "{{baseUrl}}/orders/{{id}}/lines?dryRun=false");
        assert_eq!(call.headers, vec![Pair::new("X-Tenant", "")]);
        assert_eq!(
            call.body.as_ref().unwrap().text.as_deref(),
            Some("{\n  \"sku\": \"string\",\n  \"qty\": 0\n}")
        );

        let plain = build(
            &d,
            "Add line",
            Choices {
                bodies: false,
                path_vars: false,
                ..Choices::default()
            },
        );
        assert_eq!(
            plain.url,
            "{{baseUrl}}/orders/{id:[0-9]+}/lines?dryRun=false"
        );
        assert_eq!(plain.body.unwrap().text.as_deref(), Some("{}"));

        let mut form = declared("POST", "/login");
        form.consumes = Some("application/x-www-form-urlencoded".into());
        form.body = json::parse(r#"{"user":"string","password":"string"}"#);
        let body = build(&form, "Log in", Choices::default()).body.unwrap();
        assert_eq!(body.kind, BodyType::Form);
        assert_eq!(body.fields.len(), 2);
    }

    fn collection_with(call: Call) -> Collection {
        Collection {
            v: 1,
            name: "orders".into(),
            wsdl: None,
            vars: Vec::new(),
            items: vec![Item::Folder(Folder {
                folder: "Orders".into(),
                items: vec![Item::Call(Box::new(call))],
            })],
        }
    }

    fn endpoint(d: Declared) -> Endpoint {
        let call = build(&d, "Create order", Choices::default());
        Endpoint {
            id: "e1".into(),
            key: key_of(&call),
            source: "s".into(),
            group: "OrderController".into(),
            file: None,
            line: None,
            method: d.method.clone(),
            path: d.path.clone(),
            hint: String::new(),
            call,
            status: Status::New,
            declared: Some(d),
            folder: None,
        }
    }

    #[test]
    fn an_endpoint_is_new_changed_or_in_a_collection() {
        let mut fresh = declared("POST", "/orders");
        fresh.body = json::parse(r#"{"sku":"string","qty":0,"couponCode":"string"}"#);
        fresh.body_type = Some("CreateOrderRequest".into());
        let mut kept = build(
            &declared("POST", "/orders"),
            "Create order",
            Choices::default(),
        );
        kept.id = "c_kept".into();
        kept.body = Some(Body {
            kind: BodyType::Json,
            text: Some(r#"{"qty": 3, "sku": "AB-1", "note": "x"}"#.into()),
            ..Body::default()
        });
        let collections = vec![("orders".to_string(), collection_with(kept.clone()))];

        let mut found = vec![
            endpoint(fresh.clone()),
            endpoint(declared("GET", "/orders/{id}")),
        ];
        mark(&mut found, &collections);
        assert_eq!(
            found[0].status,
            Status::Changed {
                collection: "orders".into(),
                call_id: "c_kept".into(),
                what: "CreateOrderRequest gained couponCode; lost note".into()
            }
        );
        assert_eq!(found[1].status, Status::New);

        // The same shape is In.
        let mut same = declared("POST", "/orders/");
        same.body = json::parse(r#"{"qty":0,"sku":"string","note":"string"}"#);
        let mut again = vec![endpoint(same)];
        mark(&mut again, &collections);
        assert!(matches!(again[0].status, Status::In { .. }));
    }

    #[test]
    fn bringing_in_updates_a_changed_call_and_adds_new_ones_in_folders() {
        let mut fresh = declared("POST", "/orders");
        fresh.body = json::parse(r#"{"sku":"string","qty":0,"couponCode":"string"}"#);
        let mut kept = build(
            &declared("POST", "/orders?old=1&page=2"),
            "Create order",
            Choices::default(),
        );
        kept.url = "{{baseUrl}}/orders?old=1&page=7".into();
        kept.id = "c_kept".into();
        kept.body = Some(Body {
            kind: BodyType::Json,
            text: Some(r#"{"qty": 3, "sku": "AB-1", "note": "x"}"#.into()),
            ..Body::default()
        });
        let mut collection = collection_with(kept);
        fresh.query = vec![
            Param {
                name: "page".into(),
                sample: "1".into(),
                required: false,
            },
            Param {
                name: "size".into(),
                sample: "20".into(),
                required: false,
            },
        ];
        let mut found = vec![endpoint(fresh), endpoint(declared("GET", "/orders/{id}"))];
        mark(&mut found, &[("orders".to_string(), collection.clone())]);
        let refs: Vec<&Endpoint> = found.iter().collect();
        let brought = bring_in(&mut collection, "orders", &refs, Choices::default());
        assert_eq!(
            brought,
            Brought {
                added: 1,
                changed: 1,
                unchanged: 0
            }
        );
        let calls = super::super::model::calls(&collection.items);
        let updated = calls.iter().find(|(_, c)| c.id == "c_kept").unwrap().1;
        assert_eq!(updated.url, "{{baseUrl}}/orders?page=7&size=20");
        assert_eq!(
            updated.body.as_ref().unwrap().text.as_deref(),
            Some("{\n  \"qty\": 3,\n  \"sku\": \"AB-1\",\n  \"couponCode\": \"string\"\n}")
        );
        // The new one is in its controller's folder.
        let (folder, added) = calls.iter().find(|(_, c)| c.id != "c_kept").unwrap();
        assert_eq!(folder, &vec!["OrderController".to_string()]);
        assert_eq!(added.url, "{{baseUrl}}/orders/{{id}}");
    }

    #[test]
    fn names_come_from_what_a_call_does() {
        assert_eq!(name_for("GET", "/orders/{id}", None), "Get orders");
        assert_eq!(
            name_for("POST", "/orders", Some(" Place order ")),
            "Place order"
        );
        assert_eq!(words_of("createOrderLine"), "Create order line");
    }
}
