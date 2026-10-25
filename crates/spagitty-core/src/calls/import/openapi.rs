// SPDX-License-Identifier: GPL-3.0-or-later

//! OpenAPI 3 and Swagger 2, in JSON or YAML (FEAT-162, proposal §8.3).
//!
//! Each operation is an endpoint that *declares* its shape, as a controller
//! does, so it is matched and marked *Changed* the same way. Request bodies
//! are made from the schema: its `example`, `default` or first `enum` when
//! it has one, and otherwise a sample by type and format, to a depth of
//! four. Only references inside the file are followed (`#/components/…`);
//! one to another file or a URL is left as `null` rather than fetched.

use super::json::{parse_any, J};
use super::{build, key_of, name_for, words_of, Choices, Declared, Endpoint, Param, Status};

const DEPTH: usize = 4;
const METHODS: [&str; 8] = [
    "get", "put", "post", "delete", "options", "head", "patch", "trace",
];

/// Whether `text` is an OpenAPI or Swagger document.
pub fn is_openapi(value: &J) -> bool {
    value
        .get("openapi")
        .and_then(J::text)
        .is_some_and(|v| v.starts_with('3'))
        || value
            .get("swagger")
            .and_then(J::text)
            .is_some_and(|v| v.starts_with('2'))
}

/// `#/components/schemas/Order`, followed in `root`.
fn pointer<'a>(root: &'a J, reference: &str) -> Option<&'a J> {
    let path = reference.strip_prefix("#/")?;
    let mut at = root;
    for part in path.split('/') {
        let part = part.replace("~1", "/").replace("~0", "~");
        at = at.get(&part)?;
    }
    Some(at)
}

fn resolve<'a>(root: &'a J, value: &'a J) -> &'a J {
    let mut at = value;
    for _ in 0..8 {
        match at
            .get("$ref")
            .and_then(J::str)
            .and_then(|r| pointer(root, r))
        {
            Some(next) => at = next,
            None => break,
        }
    }
    at
}

fn ref_name(value: &J) -> Option<String> {
    value
        .get("$ref")
        .and_then(J::str)
        .map(|r| r.rsplit('/').next().unwrap_or(r).to_string())
}

/// A sample of `schema`, named `field`.
fn sample(root: &J, schema: &J, field: &str, depth: usize, seen: &mut Vec<String>) -> J {
    if let Some(name) = ref_name(schema) {
        if depth >= DEPTH || seen.contains(&name) {
            return J::Null;
        }
        seen.push(name);
        let out = sample(root, resolve(root, schema), field, depth + 1, seen);
        seen.pop();
        return out;
    }
    if let Some(example) = schema.get("example").or_else(|| schema.get("default")) {
        return example.clone();
    }
    if let Some(first) = schema.get("enum").and_then(|e| e.arr().first()) {
        return first.clone();
    }
    if let Some(all) = schema.get("allOf") {
        let mut entries = Vec::new();
        for part in all.arr() {
            if let J::Obj(fields) = sample(root, part, field, depth, seen) {
                for (key, value) in fields {
                    if !entries.iter().any(|(k, _): &(String, J)| *k == key) {
                        entries.push((key, value));
                    }
                }
            }
        }
        return J::Obj(entries);
    }
    for either in ["oneOf", "anyOf"] {
        if let Some(first) = schema.get(either).and_then(|e| e.arr().first()) {
            return sample(root, first, field, depth, seen);
        }
    }
    let kind =
        schema
            .get("type")
            .and_then(J::str)
            .unwrap_or(if schema.get("properties").is_some() {
                "object"
            } else if schema.get("items").is_some() {
                "array"
            } else {
                ""
            });
    let number = |fallback: &str| {
        schema
            .get("minimum")
            .and_then(J::text)
            .map(J::Num)
            .unwrap_or_else(|| J::Num(fallback.into()))
    };
    match kind {
        "object" => {
            let mut entries: Vec<(String, J)> = schema
                .get("properties")
                .map(|p| p.obj())
                .unwrap_or(&[])
                .iter()
                .filter(|(_, prop)| prop.get("readOnly") != Some(&J::Bool(true)))
                .map(|(key, prop)| (key.clone(), sample(root, prop, key, depth + 1, seen)))
                .collect();
            if entries.is_empty() {
                if let Some(extra @ J::Obj(_)) = schema.get("additionalProperties") {
                    entries.push(("key".into(), sample(root, extra, "", depth + 1, seen)));
                }
            }
            J::Obj(entries)
        }
        "array" => J::Arr(vec![schema
            .get("items")
            .map(|items| sample(root, items, field, depth + 1, seen))
            .unwrap_or(J::Null)]),
        "integer" => number("0"),
        "number" => number("0.0"),
        "boolean" => J::Bool(false),
        "string" => J::Str(match schema.get("format").and_then(J::str).unwrap_or("") {
            "email" => "mira@example.com".into(),
            "date" => "2026-01-31".into(),
            "date-time" => "2026-01-31T09:30:00Z".into(),
            "uuid" => "3fa85f64-5717-4562-b3fc-2c963f66afa6".into(),
            "uri" | "url" => "https://example.com".into(),
            "byte" => "base64".into(),
            "binary" => String::new(),
            _ if field.is_empty() => "string".into(),
            _ => field.to_string(),
        }),
        _ => J::Null,
    }
}

fn param_sample(root: &J, param: &J) -> String {
    let schema = param
        .get("schema")
        .map(|s| resolve(root, s))
        .unwrap_or(param);
    param
        .get("example")
        .or_else(|| schema.get("example"))
        .or_else(|| schema.get("default"))
        .or_else(|| schema.get("enum").and_then(|e| e.arr().first()))
        .and_then(J::text)
        .unwrap_or_default()
}

/// Where calls go: OpenAPI 3's first server, or Swagger 2's scheme, host
/// and base path. A relative one is a prefix for the paths instead.
fn server(root: &J) -> (Option<String>, String) {
    if let Some(url) = root
        .get("servers")
        .and_then(|s| s.arr().first())
        .and_then(|s| s.get("url"))
        .and_then(J::str)
    {
        let url = url.trim_end_matches('/');
        return if url.contains("://") {
            (Some(url.to_string()), String::new())
        } else {
            (None, url.to_string())
        };
    }
    let base_path = root
        .get("basePath")
        .and_then(J::str)
        .unwrap_or("")
        .trim_end_matches('/')
        .to_string();
    match root.get("host").and_then(J::str) {
        Some(host) => {
            let scheme = root
                .get("schemes")
                .and_then(|s| s.arr().first())
                .and_then(J::str)
                .unwrap_or("https");
            (Some(format!("{scheme}://{host}{base_path}")), String::new())
        }
        None => (None, base_path),
    }
}

/// Every operation in an OpenAPI or Swagger file, and its server.
pub fn read(file: &str, text: &str) -> Option<(Vec<Endpoint>, Option<String>)> {
    let root = parse_any(text)?;
    if !is_openapi(&root) {
        return None;
    }
    let (base_url, prefix) = server(&root);
    let mut out = Vec::new();
    for (path, item) in root.get("paths").map(J::obj).unwrap_or(&[]) {
        let item = resolve(&root, item);
        let shared: Vec<&J> = item
            .get("parameters")
            .map(|p| p.arr().iter().collect())
            .unwrap_or_default();
        for method in METHODS {
            let Some(op) = item.get(method) else {
                continue;
            };
            let verb = method.to_ascii_uppercase();
            let mut declared = Declared {
                method: verb.clone(),
                path: format!("{prefix}{path}"),
                ..Declared::default()
            };
            let mut form: Vec<(String, J)> = Vec::new();
            let own: Vec<&J> = op
                .get("parameters")
                .map(|p| p.arr().iter().collect())
                .unwrap_or_default();
            for param in shared.iter().chain(own.iter()) {
                let param = resolve(&root, param);
                let name = param.get("name").and_then(J::str).unwrap_or("").to_string();
                let required = param.get("required") == Some(&J::Bool(true));
                let found = Param {
                    name: name.clone(),
                    sample: param_sample(&root, param),
                    required,
                };
                match param.get("in").and_then(J::str) {
                    Some("query") => declared.query.push(found),
                    Some("header") => declared.headers.push(found),
                    Some("formData") => form.push((name, J::Str(found.sample))),
                    Some("body") => {
                        if let Some(schema) = param.get("schema") {
                            declared.body_type = ref_name(schema);
                            declared.body = Some(sample(&root, schema, "", 0, &mut Vec::new()));
                        }
                    }
                    _ => {}
                }
            }
            if let Some(body) = op.get("requestBody").map(|b| resolve(&root, b)) {
                let content = body.get("content").map(J::obj).unwrap_or(&[]);
                let chosen = content
                    .iter()
                    .find(|(media, _)| media.contains("json"))
                    .or_else(|| content.first());
                if let Some((media, entry)) = chosen {
                    declared.consumes = Some(media.clone());
                    if let Some(example) = entry.get("example") {
                        declared.body = Some(example.clone());
                    } else if let Some(schema) = entry.get("schema") {
                        declared.body_type = ref_name(schema);
                        declared.body = Some(sample(&root, schema, "", 0, &mut Vec::new()));
                    }
                }
            }
            if declared.body.is_none() && !form.is_empty() {
                declared.body = Some(J::Obj(form));
                declared.consumes = Some("application/x-www-form-urlencoded".into());
            }
            let name = op
                .get("summary")
                .and_then(J::str)
                .map(str::to_string)
                .or_else(|| op.get("operationId").and_then(J::str).map(words_of))
                .unwrap_or_else(|| name_for(&verb, path, None));
            let group = op
                .get("tags")
                .and_then(|t| t.arr().first())
                .and_then(J::str)
                .map(str::to_string)
                .unwrap_or_else(|| {
                    path.split('/')
                        .find(|s| !s.is_empty() && !s.starts_with('{'))
                        .unwrap_or("api")
                        .to_string()
                });
            let mut hint: Vec<String> = Vec::new();
            if let Some(body) = &declared.body_type {
                hint.push(body.clone());
            }
            let names: Vec<&str> = declared.query.iter().map(|p| p.name.as_str()).collect();
            if !names.is_empty() {
                hint.push(names.join(", "));
            }
            let call = build(&declared, &name, Choices::default());
            out.push(Endpoint {
                id: format!("{file}#{}", out.len()),
                key: key_of(&call),
                source: format!("file:{file}"),
                group,
                file: Some(file.to_string()),
                line: None,
                method: verb,
                path: declared.path.clone(),
                hint: hint.join(" · "),
                call,
                status: Status::New,
                declared: Some(declared),
                folder: None,
            });
        }
    }
    Some((out, base_url))
}

#[cfg(test)]
mod tests {
    use super::*;

    const V3: &str = r##"
openapi: 3.0.3
info: { title: Orders, version: "1" }
servers:
  - url: https://orders.example.com/v1/
paths:
  /orders:
    get:
      tags: [orders]
      operationId: listOrders
      parameters:
        - $ref: "#/components/parameters/Page"
        - { name: X-Tenant, in: header, schema: { type: string, default: acme } }
    post:
      summary: Place an order
      requestBody:
        content:
          application/json:
            schema: { $ref: "#/components/schemas/NewOrder" }
  /orders/{id}:
    parameters:
      - { name: id, in: path, required: true, schema: { type: string } }
    delete: {}
components:
  parameters:
    Page: { name: page, in: query, schema: { type: integer, default: 1 } }
  schemas:
    NewOrder:
      allOf:
        - { $ref: "#/components/schemas/Base" }
        - type: object
          properties:
            sku: { type: string, example: AB-1 }
            qty: { type: integer, minimum: 1 }
            due: { type: string, format: date }
            lines: { type: array, items: { $ref: "#/components/schemas/NewOrder" } }
            state: { type: string, enum: [new, paid] }
    Base:
      type: object
      properties:
        id: { type: string, format: uuid, readOnly: true }
        contact: { type: string, format: email }
"##;

    #[test]
    fn openapi_3_gives_its_operations_with_example_bodies() {
        let (endpoints, base) = read("api/openapi.yaml", V3).unwrap();
        assert_eq!(base.as_deref(), Some("https://orders.example.com/v1"));
        let got: Vec<String> = endpoints
            .iter()
            .map(|e| format!("{} {} | {}", e.method, e.call.url, e.call.name))
            .collect();
        assert_eq!(
            got,
            [
                "GET {{baseUrl}}/orders?page=1 | List orders",
                "POST {{baseUrl}}/orders | Place an order",
                "DELETE {{baseUrl}}/orders/{{id}} | Delete orders",
            ]
        );
        assert_eq!(endpoints[0].group, "orders");
        assert_eq!(endpoints[0].call.headers[0].value, "acme");
        assert_eq!(endpoints[1].hint, "NewOrder");
        assert_eq!(
            endpoints[1].call.body.as_ref().unwrap().text.as_deref(),
            Some(
                "{\n  \"contact\": \"mira@example.com\",\n  \"sku\": \"AB-1\",\n  \"qty\": 1,\n  \"due\": \"2026-01-31\",\n  \"lines\": [\n    null\n  ],\n  \"state\": \"new\"\n}"
            )
        );
    }

    #[test]
    fn swagger_2_reads_body_and_form_parameters() {
        let v2 = r##"{"swagger":"2.0","host":"api.local:8080","basePath":"/shop","schemes":["http"],
            "paths":{"/login":{"post":{"consumes":["application/x-www-form-urlencoded"],"parameters":[
                {"name":"user","in":"formData","type":"string"},{"name":"password","in":"formData","type":"string"}]}},
            "/items":{"put":{"parameters":[{"name":"body","in":"body","schema":{"$ref":"#/definitions/Item"}}]}}},
            "definitions":{"Item":{"type":"object","properties":{"name":{"type":"string"},"price":{"type":"number"}}}}}"##;
        let (endpoints, base) = read("swagger.json", v2).unwrap();
        assert_eq!(base.as_deref(), Some("http://api.local:8080/shop"));
        assert_eq!(endpoints[0].call.body.as_ref().unwrap().fields.len(), 2);
        assert_eq!(
            endpoints[1].call.body.as_ref().unwrap().text.as_deref(),
            Some("{\n  \"name\": \"name\",\n  \"price\": 0.0\n}")
        );
        assert!(read("x.json", r#"{"name":"not openapi"}"#).is_none());
    }
}
