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
            // `cmd`'s `^` and PowerShell's backtick continue a line.
            '^' | '`' if matches!(chars.peek(), Some('\n') | Some('\r')) => {
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

/// A pasted command: the call, and which of its flags were read and which
/// were not (FEAT-166).
#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct Pasted {
    pub call: Call,
    pub read: Vec<String>,
    pub ignored: Vec<String>,
}

/// A cURL command as a call.
pub fn parse(text: &str) -> Result<Call, String> {
    parse_noting(text).map(|pasted| pasted.call)
}

/// A cURL command as a call, saying which flags it read.
pub fn parse_noting(text: &str) -> Result<Pasted, String> {
    let mut read: Vec<String> = Vec::new();
    let mut ignored: Vec<String> = Vec::new();
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
        let known = matches!(
            flag.as_str(),
            "-X" | "--request"
                | "-H"
                | "--header"
                | "-d"
                | "--data"
                | "--data-raw"
                | "--data-binary"
                | "--data-ascii"
                | "--data-urlencode"
                | "--json"
                | "-F"
                | "--form"
                | "--form-string"
                | "-u"
                | "--user"
                | "-A"
                | "--user-agent"
                | "-b"
                | "--cookie"
                | "-e"
                | "--referer"
                | "--url"
                | "-G"
                | "--get"
                | "-I"
                | "--head"
                | "-L"
                | "--location"
                | "-k"
                | "--insecure"
                | "--compressed"
        );
        if flag.starts_with('-') {
            let list = if known { &mut read } else { &mut ignored };
            if !list.contains(&flag) {
                list.push(flag.clone());
            }
        }
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
    let call_method = method.unwrap_or_else(|| {
        if head {
            "HEAD".into()
        } else if body.is_some() {
            "POST".into()
        } else {
            "GET".into()
        }
    });
    Ok(Pasted {
        call: Call {
            id: new_id(),
            name: name_for(&call_method, &path_of(&url), None),
            method: Some(call_method),
            url,
            headers,
            body,
            auth,
            ..Call::default()
        },
        read,
        ignored,
    })
}

/// How a copied command is quoted.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Quoting {
    /// Single quotes, `\` at line ends: Linux and macOS.
    Posix,
    /// Double quotes, `^` at line ends: Windows' `cmd`.
    Cmd,
}

/// A body as a command sends it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CopyBody {
    None,
    /// `--data-raw`.
    Raw(String),
    /// `--data-urlencode`, each.
    Form(Vec<(String, String)>),
    /// `--form`, each; a file as `@path`.
    Multipart(Vec<(String, String, bool)>),
}

/// What a copy says besides the request.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CopyOptions {
    pub quoting: Quoting,
    /// `-L`: Settings follow redirects.
    pub follow: bool,
    /// `-k`: TLS is not checked for this environment.
    pub insecure: bool,
}

/// `word` quoted for `quoting`, or as it is when nothing in it needs it.
pub fn quote(word: &str, quoting: Quoting) -> String {
    let plain = !word.is_empty()
        && word
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_./:=@+,".contains(c));
    if plain {
        return word.to_string();
    }
    match quoting {
        Quoting::Posix => format!("'{}'", word.replace('\'', "'\\''")),
        Quoting::Cmd => format!("\"{}\"", word.replace('"', "\\\"").replace('\n', "\\n")),
    }
}

/// A request as one `curl` command (FEAT-166). The method is written only
/// when it is not what the rest implies; `basic` is `-u` in place of its
/// header.
pub fn to_curl(
    method: &str,
    url: &str,
    headers: &[(String, String)],
    body: &CopyBody,
    basic: Option<(String, String)>,
    options: CopyOptions,
) -> String {
    let q = |w: &str| quote(w, options.quoting);
    let mut parts: Vec<String> = vec!["curl".into()];
    let method = method.to_ascii_uppercase();
    let has_body = !matches!(body, CopyBody::None);
    match (method.as_str(), has_body) {
        ("GET", false) | ("POST", true) => {}
        ("HEAD", false) => parts.push("-I".into()),
        (other, _) => parts.push(format!("-X {other}")),
    }
    parts.push(q(url));
    for (name, value) in headers {
        if name.eq_ignore_ascii_case("content-length")
            || (basic.is_some() && name.eq_ignore_ascii_case("authorization"))
        {
            continue;
        }
        parts.push(format!("-H {}", q(&format!("{name}: {value}"))));
    }
    if let Some((user, password)) = &basic {
        parts.push(format!("-u {}", q(&format!("{user}:{password}"))));
    }
    match body {
        CopyBody::None => {}
        CopyBody::Raw(text) => parts.push(format!("--data-raw {}", q(text))),
        CopyBody::Form(fields) => {
            for (name, value) in fields {
                parts.push(format!(
                    "--data-urlencode {}",
                    q(&format!("{name}={value}"))
                ));
            }
        }
        CopyBody::Multipart(fields) => {
            for (name, value, file) in fields {
                let part = if *file {
                    format!("{name}=@{value}")
                } else {
                    format!("{name}={value}")
                };
                parts.push(format!("--form {}", q(&part)));
            }
        }
    }
    if options.follow {
        parts.push("-L".into());
    }
    if options.insecure {
        parts.push("-k".into());
    }
    parts.push("--compressed".into());
    let joiner = match options.quoting {
        Quoting::Posix => " \\\n  ",
        Quoting::Cmd => " ^\n  ",
    };
    parts.join(joiner)
}

/// A call as one `curl` command, resolved in `scope` (FEAT-166). Secrets
/// and kept values stay `{{name}}` unless `secrets` is set. `envelope` is a
/// SOAP call's envelope with its WS-Security header, when it has one.
pub fn copy(
    call: &Call,
    scope: &super::super::vars::Scope,
    every: &[(String, String)],
    root: &std::path::Path,
    envelope: Option<&str>,
    secrets: bool,
    options: CopyOptions,
) -> Result<String, String> {
    use super::super::model::Kind;
    let fill = |text: &str| -> Result<String, String> {
        if secrets {
            scope.resolve(text).map_err(|e| e.to_string())
        } else {
            scope.redacted(text).map_err(|e| e.to_string())
        }
    };
    let built = super::super::build::build(call, scope, every, root, envelope)
        .map_err(|e| e.to_string())?;
    let (url, headers, body_text) = if secrets {
        (
            built.out.url.clone(),
            built.out.headers.clone(),
            built
                .out
                .body
                .as_ref()
                .map(|b| String::from_utf8_lossy(b).into_owned()),
        )
    } else {
        (
            built.shown.url.clone(),
            built.shown.headers.clone(),
            built.shown.body.clone(),
        )
    };
    // A form's or a multipart's parts are written as curl's own options,
    // and curl sets their Content-Type, with its boundary.
    let fields = |multipart: bool| -> Result<CopyBody, String> {
        let list: Vec<(String, String, bool)> = call
            .body
            .iter()
            .flat_map(|b| b.fields.iter().filter(|f| f.on))
            .map(|f| Ok((fill(&f.name)?, fill(&f.value)?, f.file)))
            .collect::<Result<_, String>>()?;
        Ok(if multipart {
            CopyBody::Multipart(list)
        } else {
            CopyBody::Form(list.into_iter().map(|(n, v, _)| (n, v)).collect())
        })
    };
    let body = match call.body.as_ref().map(|b| b.kind) {
        _ if call.kind == Kind::Soap => body_text.map(CopyBody::Raw).unwrap_or(CopyBody::None),
        Some(BodyType::Form) => fields(false)?,
        Some(BodyType::Multipart) => fields(true)?,
        _ => body_text.map(CopyBody::Raw).unwrap_or(CopyBody::None),
    };
    let headers: Vec<(String, String)> = headers
        .into_iter()
        .filter(|(name, _)| {
            !(matches!(body, CopyBody::Form(_) | CopyBody::Multipart(_))
                && name.eq_ignore_ascii_case("content-type"))
        })
        .collect();
    let basic = match &call.auth {
        Some(auth) if auth.kind == AuthType::Basic => Some((
            fill(auth.user.as_deref().unwrap_or(""))?,
            fill(auth.password.as_deref().unwrap_or(""))?,
        )),
        _ => None,
    };
    Ok(to_curl(
        &built.out.method,
        &url,
        &headers,
        &body,
        basic,
        options,
    ))
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

    #[test]
    fn a_paste_says_what_it_read_and_what_it_did_not() {
        let pasted =
            parse_noting("curl -L -k -s --retry 3 -X PUT http://h/x `\n -H 'A: b' --compressed")
                .unwrap();
        assert_eq!(pasted.read, ["-L", "-k", "-X", "-H", "--compressed"]);
        assert_eq!(pasted.ignored, ["-s", "--retry"]);
        assert_eq!(pasted.call.method.as_deref(), Some("PUT"));
        assert_eq!(pasted.call.headers, vec![Pair::new("A", "b")]);
    }

    fn options(quoting: Quoting) -> CopyOptions {
        CopyOptions {
            quoting,
            follow: true,
            insecure: false,
        }
    }

    #[test]
    fn a_copy_is_quoted_for_its_shell_and_reads_back_the_same() {
        let headers = vec![
            ("Accept".to_string(), "application/json".to_string()),
            ("X-Why".to_string(), "it's \"ok\"".to_string()),
        ];
        let body = CopyBody::Raw("{\"note\": \"it's\"}".into());
        let posix = to_curl(
            "POST",
            "https://h/orders?a=1&b=2",
            &headers,
            &body,
            None,
            options(Quoting::Posix),
        );
        assert_eq!(
            posix,
            "curl \\\n  'https://h/orders?a=1&b=2' \\\n  -H 'Accept: application/json' \\\n  -H 'X-Why: it'\\''s \"ok\"' \\\n  --data-raw '{\"note\": \"it'\\''s\"}' \\\n  -L \\\n  --compressed"
        );
        let cmd = to_curl(
            "POST",
            "https://h/orders?a=1&b=2",
            &headers,
            &body,
            None,
            options(Quoting::Cmd),
        );
        assert!(cmd.contains(" ^\n  -H \"X-Why: it's \\\"ok\\\"\""));
        for text in [posix, cmd] {
            let back = parse(&text).unwrap();
            assert_eq!(back.method.as_deref(), Some("POST"));
            assert_eq!(back.url, "https://h/orders?a=1&b=2");
            assert_eq!(back.headers[1].value, "it's \"ok\"");
            assert_eq!(
                back.body.unwrap().text.as_deref(),
                Some("{\"note\": \"it's\"}")
            );
        }
    }

    #[test]
    fn a_copy_keeps_secrets_as_names_unless_asked() {
        use crate::calls::model::{Auth, Body, Colour, Environment, Kind, Variable};
        use std::collections::HashMap;
        let mut env = Environment::new("staging", Colour::Warn);
        env.vars = vec![
            Variable::plain("baseUrl", "https://staging"),
            Variable {
                name: "password".into(),
                secret: true,
                ..Variable::default()
            },
        ];
        let private = HashMap::from([("password".to_string(), "SECRET-hunter2".to_string())]);
        let shared = crate::calls::model::Shared::default();
        let scope =
            crate::calls::vars::scope_of(Some((&env, &private)), (&shared, &HashMap::new()), &[]);
        let call = Call {
            id: "c".into(),
            name: "Log in".into(),
            method: Some("POST".into()),
            url: "{{baseUrl}}/login".into(),
            body: Some(Body {
                kind: BodyType::Json,
                text: Some("{\"password\": \"{{password}}\"}".into()),
                ..Body::default()
            }),
            auth: Some(Auth {
                kind: AuthType::Basic,
                user: Some("mira".into()),
                password: Some("{{password}}".into()),
                ..Auth::default()
            }),
            ..Call::default()
        };
        let root = std::path::Path::new(".");
        let shown = copy(
            &call,
            &scope,
            &[],
            root,
            None,
            false,
            options(Quoting::Posix),
        )
        .unwrap();
        assert!(!shown.contains("SECRET-"), "{shown}");
        assert!(shown.contains("https://staging/login"));
        assert!(shown.contains("-u 'mira:{{password}}'"));
        assert!(shown.contains("{{password}}"));
        assert!(!shown.contains("Authorization"));
        let filled = copy(
            &call,
            &scope,
            &[],
            root,
            None,
            true,
            options(Quoting::Posix),
        )
        .unwrap();
        assert!(filled.contains("SECRET-hunter2"));

        // A SOAP call is a POST of its envelope with its SOAP headers.
        let soap = Call {
            id: "s".into(),
            name: "Get".into(),
            kind: Kind::Soap,
            url: "{{baseUrl}}/ws".into(),
            version: Some("1.1".into()),
            action: Some("urn:Get".into()),
            envelope: Some("<e/>".into()),
            ..Call::default()
        };
        let text = copy(
            &soap,
            &scope,
            &[],
            root,
            None,
            false,
            options(Quoting::Posix),
        )
        .unwrap();
        assert!(text.contains("-H 'SOAPAction: \"urn:Get\"'"));
        assert!(text.contains("--data-raw '<e/>'"));
        assert!(!text.contains("-X"));
    }

    #[test]
    fn every_body_kind_round_trips() {
        let form = CopyBody::Form(vec![("a".into(), "1".into()), ("b".into(), "x y".into())]);
        let back = parse(&to_curl(
            "PUT",
            "http://h/f",
            &[],
            &form,
            Some(("me".into(), "pw".into())),
            options(Quoting::Posix),
        ))
        .unwrap();
        assert_eq!(back.method.as_deref(), Some("PUT"));
        assert_eq!(back.body.as_ref().unwrap().fields.len(), 2);
        assert_eq!(back.auth.as_ref().unwrap().user.as_deref(), Some("me"));

        let parts = CopyBody::Multipart(vec![
            ("doc".into(), "a.pdf".into(), true),
            ("note".into(), "hi".into(), false),
        ]);
        let back = parse(&to_curl(
            "POST",
            "http://h/up",
            &[],
            &parts,
            None,
            options(Quoting::Cmd),
        ))
        .unwrap();
        let body = back.body.unwrap();
        assert_eq!(body.kind, BodyType::Multipart);
        assert!(body.fields[0].file);

        let get = to_curl(
            "GET",
            "http://h/x",
            &[],
            &CopyBody::None,
            None,
            options(Quoting::Posix),
        );
        assert!(!get.contains("-X"));
        let delete = to_curl(
            "DELETE",
            "http://h/x",
            &[],
            &CopyBody::None,
            None,
            options(Quoting::Posix),
        );
        assert!(delete.contains("-X DELETE"));
        assert_eq!(parse(&delete).unwrap().method.as_deref(), Some("DELETE"));
    }
}
