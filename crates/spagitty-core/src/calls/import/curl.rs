// SPDX-License-Identifier: GPL-3.0-or-later

//! A cURL command as a call (FEAT-162 *Paste cURL*, FEAT-166).
//!
//! Read as a shell would split it: single quotes, double quotes with their
//! escapes, `$'…'`, and lines continued with `\` or, from Windows' `cmd`, `^`.
//! The options that say what is sent are read: `-X`, `-H`, `-d` and its
//! kinds, `-F`, `-u`, `-A`, `-b`, `-G`, `-I`, `--url`. Options about how
//! curl behaves (`-s`, `-L`, `-k`, `--compressed`) are skipped.

use super::super::model::{new_id, Auth, AuthType, Body, BodyType, Call, Field, Pair};
use super::{name_for, path_of};

/// `text` split into words as a shell would.
pub fn words(text: &str) -> Result<Vec<String>, String> {
    let mut out = Vec::new();
    let mut word = String::new();
    let mut started = false;
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' => match chars.next() {
                Some('\n') => {}
                Some('\r') => {
                    chars.next_if_eq(&'\n');
                }
                Some(next) => {
                    word.push(next);
                    started = true;
                }
                None => {}
            },
            '^' if matches!(chars.peek(), Some('\n') | Some('\r')) => {
                chars.next_if_eq(&'\r');
                chars.next_if_eq(&'\n');
            }
            '\'' => {
                started = true;
                loop {
                    match chars.next() {
                        Some('\'') => break,
                        Some(inner) => word.push(inner),
                        None => return Err("a quote is not closed".into()),
                    }
                }
            }
            '$' if chars.peek() == Some(&'\'') => {
                chars.next();
                started = true;
                loop {
                    match chars.next() {
                        Some('\'') => break,
                        Some('\\') => match chars.next() {
                            Some('n') => word.push('\n'),
                            Some('t') => word.push('\t'),
                            Some(other) => word.push(other),
                            None => return Err("a quote is not closed".into()),
                        },
                        Some(inner) => word.push(inner),
                        None => return Err("a quote is not closed".into()),
                    }
                }
            }
            '"' => {
                started = true;
                loop {
                    match chars.next() {
                        Some('"') => break,
                        Some('\\') => match chars.next() {
                            Some(e @ ('"' | '\\' | '$' | '`')) => word.push(e),
                            Some('\n') => {}
                            Some(other) => {
                                word.push('\\');
                                word.push(other);
                            }
                            None => return Err("a quote is not closed".into()),
                        },
                        Some(inner) => word.push(inner),
                        None => return Err("a quote is not closed".into()),
                    }
                }
            }
            c if c.is_whitespace() => {
                if started {
                    out.push(std::mem::take(&mut word));
                    started = false;
                }
            }
            c => {
                word.push(c);
                started = true;
            }
        }
    }
    if started {
        out.push(word);
    }
    Ok(out)
}

/// A cURL command as a call.
pub fn parse(text: &str) -> Result<Call, String> {
    let words = words(text.trim())?;
    let mut rest = words.iter().map(String::as_str).peekable();
    match rest.peek() {
        Some(first) if first.ends_with("curl") || first.ends_with("curl.exe") => {
            rest.next();
        }
        _ => return Err("this is not a cURL command: it should start with curl".into()),
    }
    let mut method: Option<String> = None;
    let mut url: Option<String> = None;
    let mut headers: Vec<Pair> = Vec::new();
    let mut data: Vec<String> = Vec::new();
    let mut form: Vec<Field> = Vec::new();
    let mut auth: Option<Auth> = None;
    let mut get = false;
    let mut head = false;
    while let Some(word) = rest.next() {
        // `--data=x` and `-dx` as well as `--data x`.
        let (flag, inline) = if let Some(long) = word.strip_prefix("--") {
            match long.split_once('=') {
                Some((f, v)) => (format!("--{f}"), Some(v.to_string())),
                None => (word.to_string(), None),
            }
        } else if word.len() > 2 && word.starts_with('-') && !word.starts_with("--") {
            let flag: String = word.chars().take(2).collect();
            if matches!(
                flag.as_str(),
                "-X" | "-H" | "-d" | "-F" | "-u" | "-A" | "-b" | "-e"
            ) {
                (flag, Some(word[2..].to_string()))
            } else {
                (word.to_string(), None)
            }
        } else {
            (word.to_string(), None)
        };
        let mut value = || {
            inline
                .clone()
                .or_else(|| rest.next().map(str::to_string))
                .unwrap_or_default()
        };
        match flag.as_str() {
            "-X" | "--request" => method = Some(value().to_ascii_uppercase()),
            "-H" | "--header" => {
                let header = value();
                if let Some((name, val)) = header.split_once(':') {
                    headers.push(Pair::new(name.trim(), val.trim()));
                }
            }
            "-d" | "--data" | "--data-raw" | "--data-binary" | "--data-ascii"
            | "--data-urlencode" | "--json" => {
                if flag == "--json" {
                    headers.push(Pair::new("Content-Type", "application/json"));
                }
                data.push(value());
            }
            "-F" | "--form" | "--form-string" => {
                let part = value();
                let (name, val) = part.split_once('=').unwrap_or((&part, ""));
                let file = val.starts_with('@') && flag != "--form-string";
                form.push(Field {
                    name: name.to_string(),
                    value: if file {
                        val[1..].split(';').next().unwrap_or("").to_string()
                    } else {
                        val.to_string()
                    },
                    file,
                    on: true,
                });
            }
            "-u" | "--user" => {
                let creds = value();
                let (user, password) = creds.split_once(':').unwrap_or((&creds, ""));
                auth = Some(Auth {
                    kind: AuthType::Basic,
                    user: Some(user.to_string()),
                    password: Some(password.to_string()),
                    ..Auth::default()
                });
            }
            "-A" | "--user-agent" => headers.push(Pair::new("User-Agent", value())),
            "-b" | "--cookie" => headers.push(Pair::new("Cookie", value())),
            "-e" | "--referer" => headers.push(Pair::new("Referer", value())),
            "--url" => url = Some(value()),
            "-G" | "--get" => get = true,
            "-I" | "--head" => head = true,
            // Options that take a value but do not change what is sent.
            "-o" | "--output" | "-m" | "--max-time" | "--connect-timeout" | "-x" | "--proxy"
            | "--retry" | "-w" | "--write-out" | "--cacert" | "--cert" | "--key" | "-E" => {
                value();
            }
            f if f.starts_with('-') => {}
            _ => {
                if url.is_none() {
                    url = Some(word.to_string());
                }
            }
        }
    }
    let mut url = url.ok_or("the command has no URL")?;
    // A bearer token in a header is the Bearer auth it means.
    if let Some(at) = headers.iter().position(|h| {
        h.name.eq_ignore_ascii_case("authorization")
            && h.value.to_ascii_lowercase().starts_with("bearer ")
    }) {
        let header = headers.remove(at);
        auth = Some(Auth {
            kind: AuthType::Bearer,
            token: Some(header.value[7..].trim().to_string()),
            ..Auth::default()
        });
    }
    let content_type = headers
        .iter()
        .find(|h| h.name.eq_ignore_ascii_case("content-type"))
        .map(|h| h.value.to_ascii_lowercase());
    let joined = data.join("&");
    let mut body = None;
    if get && !data.is_empty() {
        url.push(if url.contains('?') { '&' } else { '?' });
        url.push_str(&joined);
    } else if !form.is_empty() {
        body = Some(Body {
            kind: BodyType::Multipart,
            fields: form,
            ..Body::default()
        });
    } else if !data.is_empty() {
        let json = serde_json::from_str::<serde_json::Value>(&joined).is_ok();
        let kind = match content_type.as_deref() {
            Some(ct) if ct.contains("json") => BodyType::Json,
            Some(ct) if ct.contains("xml") => BodyType::Xml,
            Some(ct) if ct.contains("x-www-form-urlencoded") => BodyType::Form,
            Some(_) => BodyType::Raw,
            None if json => BodyType::Json,
            None => BodyType::Form,
        };
        body = Some(match kind {
            BodyType::Form => Body {
                kind,
                fields: joined
                    .split('&')
                    .filter(|p| !p.is_empty())
                    .map(|p| {
                        let (name, value) = p.split_once('=').unwrap_or((p, ""));
                        Field {
                            name: name.to_string(),
                            value: value.to_string(),
                            file: false,
                            on: true,
                        }
                    })
                    .collect(),
                ..Body::default()
            },
            BodyType::Raw => Body {
                kind,
                text: Some(joined),
                content_type: content_type.clone(),
                ..Body::default()
            },
            _ => Body {
                kind,
                text: Some(joined),
                ..Body::default()
            },
        });
    }
    // What the body type already says is not a header too.
    if matches!(
        body.as_ref().map(|b| b.kind),
        Some(BodyType::Json | BodyType::Form | BodyType::Multipart | BodyType::Raw)
    ) {
        headers.retain(|h| !h.name.eq_ignore_ascii_case("content-type"));
    }
    let method = method.unwrap_or_else(|| {
        if head {
            "HEAD".into()
        } else if body.is_some() {
            "POST".into()
        } else {
            "GET".into()
        }
    });
    Ok(Call {
        id: new_id(),
        name: name_for(&method, &path_of(&url), None),
        method: Some(method),
        url,
        headers,
        body,
        auth,
        ..Call::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shell_splits_words_as_curl_sees_them() {
        assert_eq!(
            words("curl -H 'A: b c' \"x\\\"y\" $'l\\n2' \\\n  --url=z").unwrap(),
            ["curl", "-H", "A: b c", "x\"y", "l\n2", "--url=z"]
        );
        assert_eq!(words("curl ^\r\n -X POST").unwrap(), ["curl", "-X", "POST"]);
        assert!(words("curl 'open").is_err());
    }

    #[test]
    fn a_json_post_is_a_json_call_with_bearer_auth() {
        let call = parse(
            "curl 'https://api.shop.test/v1/orders' \\\n  -H 'Authorization: Bearer ey.J' \\\n  -H 'Content-Type: application/json' \\\n  --data-raw '{\"sku\":\"A\"}' --compressed -s",
        )
        .unwrap();
        assert_eq!(call.method.as_deref(), Some("POST"));
        assert_eq!(call.url, "https://api.shop.test/v1/orders");
        assert!(call.headers.is_empty());
        assert_eq!(call.auth.as_ref().unwrap().token.as_deref(), Some("ey.J"));
        let body = call.body.unwrap();
        assert_eq!(body.kind, BodyType::Json);
        assert_eq!(body.text.as_deref(), Some("{\"sku\":\"A\"}"));
    }

    #[test]
    fn form_data_get_and_user_are_read() {
        let form = parse("curl -X PUT http://h/x -d a=1 -d b=2 -u me:pw").unwrap();
        assert_eq!(form.method.as_deref(), Some("PUT"));
        assert_eq!(form.body.as_ref().unwrap().fields.len(), 2);
        assert_eq!(form.auth.as_ref().unwrap().user.as_deref(), Some("me"));

        let get = parse("curl -G http://h/search -d q=x -d page=2").unwrap();
        assert_eq!(get.url, "http://h/search?q=x&page=2");
        assert!(get.body.is_none());

        let multipart =
            parse("curl http://h/up -F 'doc=@a.pdf;type=application/pdf' -F note=hi").unwrap();
        let body = multipart.body.unwrap();
        assert_eq!(body.kind, BodyType::Multipart);
        assert!(body.fields[0].file && body.fields[0].value == "a.pdf");

        assert!(parse("wget http://h").is_err());
        assert!(parse("curl -s").is_err());
    }
}
