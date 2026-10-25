// SPDX-License-Identifier: GPL-3.0-or-later

//! HAR files, as a browser's network panel saves them (FEAT-162). Each
//! entry's request is a call, grouped by host. Headers the browser adds on
//! its own (cookies, `:authority`, `Content-Length`) are left out.

use serde_json::Value;

use super::super::model::{new_id, Body, BodyType, Call, Field, Pair};
use super::{key_of, name_for, path_of, Endpoint, Status};

const DROPPED: [&str; 8] = [
    "cookie",
    "content-length",
    "host",
    "connection",
    "accept-encoding",
    "sec-fetch-mode",
    "sec-fetch-site",
    "sec-fetch-dest",
];

pub fn is_har(value: &Value) -> bool {
    value["log"]["entries"].is_array()
}

pub fn read(file: &str, value: &Value) -> Vec<Endpoint> {
    let mut out = Vec::new();
    for entry in value["log"]["entries"].as_array().into_iter().flatten() {
        let request = &entry["request"];
        let url = request["url"].as_str().unwrap_or("").to_string();
        let method = request["method"]
            .as_str()
            .unwrap_or("GET")
            .to_ascii_uppercase();
        let headers: Vec<Pair> = request["headers"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|h| {
                let name = h["name"].as_str()?;
                let lower = name.to_ascii_lowercase();
                (!name.starts_with(':')
                    && !DROPPED.contains(&lower.as_str())
                    && !lower.starts_with("sec-ch-"))
                .then(|| Pair::new(name, h["value"].as_str().unwrap_or("")))
            })
            .collect();
        let post = &request["postData"];
        let mime = post["mimeType"].as_str().unwrap_or("");
        let body = if let Some(params) = post["params"].as_array().filter(|p| !p.is_empty()) {
            Some(Body {
                kind: if mime.contains("multipart") {
                    BodyType::Multipart
                } else {
                    BodyType::Form
                },
                fields: params
                    .iter()
                    .map(|p| Field {
                        name: p["name"].as_str().unwrap_or("").to_string(),
                        value: p["value"].as_str().unwrap_or("").to_string(),
                        file: p["fileName"].is_string(),
                        on: true,
                    })
                    .collect(),
                ..Body::default()
            })
        } else {
            post["text"]
                .as_str()
                .filter(|t| !t.is_empty())
                .map(|text| Body {
                    kind: if mime.contains("json") {
                        BodyType::Json
                    } else if mime.contains("xml") {
                        BodyType::Xml
                    } else {
                        BodyType::Raw
                    },
                    text: Some(text.to_string()),
                    content_type: (!mime.contains("json")
                        && !mime.contains("xml")
                        && !mime.is_empty())
                    .then(|| mime.to_string()),
                    ..Body::default()
                })
        };
        let path = path_of(&url);
        let host = url
            .split_once("://")
            .map(|(_, r)| r.split('/').next().unwrap_or(""))
            .unwrap_or("")
            .to_string();
        let call = Call {
            id: new_id(),
            name: name_for(&method, &path, None),
            method: Some(method.clone()),
            url,
            headers: headers
                .into_iter()
                .filter(|h| {
                    !(body.is_some()
                        && h.name.eq_ignore_ascii_case("content-type")
                        && mime.contains("json"))
                })
                .collect(),
            body,
            ..Call::default()
        };
        out.push(Endpoint {
            id: format!("{file}#{}", out.len()),
            key: key_of(&call),
            source: format!("file:{file}"),
            group: host.clone(),
            file: Some(file.to_string()),
            line: None,
            method,
            path,
            hint: String::new(),
            call,
            status: Status::New,
            declared: None,
            folder: Some(host),
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_become_calls_by_host_without_the_browsers_headers() {
        let har = r#"{"log":{"entries":[
          {"request":{"method":"POST","url":"https://api.shop.test/v1/orders?x=1",
            "headers":[{"name":":authority","value":"api"},{"name":"Cookie","value":"sid=1"},{"name":"Authorization","value":"Bearer t"},{"name":"Content-Type","value":"application/json"}],
            "postData":{"mimeType":"application/json","text":"{\"sku\":\"A\"}"}}},
          {"request":{"method":"GET","url":"https://cdn.shop.test/logo.png","headers":[]}}
        ]}}"#;
        let value: Value = serde_json::from_str(har).unwrap();
        assert!(is_har(&value));
        let endpoints = read("trace.har", &value);
        assert_eq!(endpoints.len(), 2);
        assert_eq!(endpoints[0].group, "api.shop.test");
        assert_eq!(endpoints[0].key, "POST /v1/orders");
        assert_eq!(
            endpoints[0].call.headers,
            vec![Pair::new("Authorization", "Bearer t")]
        );
        assert_eq!(
            endpoints[0].call.body.as_ref().unwrap().kind,
            BodyType::Json
        );
    }
}
