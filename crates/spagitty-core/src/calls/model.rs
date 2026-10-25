// SPDX-License-Identifier: GPL-3.0-or-later

//! What a collection, an environment and the shared variables are, as files.
//!
//! These files are meant to be read in a diff (docs/proposals/calls.md §5.5),
//! so the shape is the reviewer's as much as the program's:
//!
//! - **Stable key order.** Every type is a struct, written field by field in
//!   the order declared here, never through a map whose order could change.
//!   A change to one call is a change to a few lines.
//! - **Nothing written that says nothing.** An empty list, an absent body or
//!   a default switch is left out, so a plain `GET` is four lines.
//! - **Two-space indent and a final newline**, which is what [`to_text`]
//!   writes.
//!
//! An environment names its secrets and kept values and never holds them:
//! [`Environment::for_file`] is what every writer goes through, and it empties
//! them.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

/// The format's version. A file with a higher one is read as far as it can
/// be and never written over.
pub const VERSION: u32 = 1;

fn is_false(value: &bool) -> bool {
    !*value
}

fn is_true(value: &bool) -> bool {
    *value
}

fn yes() -> bool {
    true
}

/// A named set of calls in folders: one file under the calls folder.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    #[serde(default = "version")]
    pub v: u32,
    pub name: String,
    /// The WSDL a SOAP collection was made from, relative to the repository
    /// (FEAT-163).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub wsdl: Option<String>,
    /// Variables of the collection's own, after the environment and the
    /// shared ones. Plain values only.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub vars: Vec<Variable>,
    #[serde(default)]
    pub items: Vec<Item>,
}

fn version() -> u32 {
    VERSION
}

/// A folder or a call. Told apart by the `folder` key, which only a folder
/// has.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Item {
    Folder(Folder),
    Call(Box<Call>),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Folder {
    pub folder: String,
    #[serde(default)]
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    #[default]
    Rest,
    Soap,
}

/// One saved request.
///
/// REST and SOAP share the struct rather than being two: they share the
/// name, the URL, the headers and the keep rules, and a file is easier to
/// read when a call is one shape with a few keys absent than when it is one
/// of two shapes.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Call {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub kind: Kind,
    /// REST only. `GET` when absent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub method: Option<String>,
    #[serde(default)]
    pub url: String,
    /// In order, as `[name, value]`. A header switched off is kept, with a
    /// third element `false`, so turning it back on loses nothing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub headers: Vec<Pair>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<Body>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub auth: Option<Auth>,
    // SOAP (FEAT-163).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub port: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub operation: Option<String>,
    /// `1.1` or `1.2`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// The `SOAPAction` (1.1) or the `action` parameter (1.2), from the WSDL.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub envelope: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ws_security: Option<WsSecurity>,
    /// Values kept from this call's responses (FEAT-161).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub keep: Vec<KeepRule>,
    /// Free text: where a Postman script that was not brought in is kept
    /// (FEAT-162).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub notes: Option<String>,
}

impl Call {
    pub fn method(&self) -> &str {
        match self.kind {
            Kind::Soap => "POST",
            Kind::Rest => self.method.as_deref().unwrap_or("GET"),
        }
    }
}

/// A name and a value, and whether it is used. Written as a two-element
/// array when it is on, which is what a person reading the file expects.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Pair {
    pub name: String,
    pub value: String,
    pub on: bool,
}

impl Pair {
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Pair {
            name: name.into(),
            value: value.into(),
            on: true,
        }
    }
}

impl Serialize for Pair {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeTuple;
        let mut tuple = s.serialize_tuple(if self.on { 2 } else { 3 })?;
        tuple.serialize_element(&self.name)?;
        tuple.serialize_element(&self.value)?;
        if !self.on {
            tuple.serialize_element(&false)?;
        }
        tuple.end()
    }
}

impl<'de> Deserialize<'de> for Pair {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let parts = Vec::<serde_json::Value>::deserialize(d)?;
        let text = |at: usize| match parts.get(at) {
            Some(serde_json::Value::String(text)) => text.clone(),
            Some(serde_json::Value::Null) | None => String::new(),
            Some(other) => other.to_string(),
        };
        Ok(Pair {
            name: text(0),
            value: text(1),
            on: parts.get(2).and_then(|v| v.as_bool()).unwrap_or(true),
        })
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum BodyType {
    Json,
    Xml,
    Form,
    Multipart,
    Raw,
    #[default]
    None,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Body {
    #[serde(rename = "type", default)]
    pub kind: BodyType,
    /// JSON, XML and raw.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    /// Form and multipart, in order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub fields: Vec<Field>,
    /// Raw only: the `Content-Type` to send it with.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
}

/// One part of a form. A multipart part may be a file, by its path relative
/// to the repository.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Field {
    pub name: String,
    #[serde(default)]
    pub value: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub file: bool,
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub on: bool,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum AuthType {
    #[default]
    None,
    Bearer,
    Basic,
    ApiKey,
}

/// Where an API key goes.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum KeyPlace {
    #[default]
    Header,
    Query,
}

/// How a call proves who is calling. Values are usually `{{variables}}`, so
/// the secret itself is in the keychain and this holds its name.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Auth {
    #[serde(rename = "type", default)]
    pub kind: AuthType,
    /// Bearer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub token: Option<String>,
    /// Basic.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// API key.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(rename = "in", default, skip_serializing_if = "Option::is_none")]
    pub place: Option<KeyPlace>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum WsMode {
    #[default]
    None,
    Username,
    /// Drawn and not offered: FEAT-167.
    Signed,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WsSecurity {
    #[serde(default)]
    pub mode: WsMode,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// `PasswordDigest` rather than `PasswordText`.
    #[serde(default, skip_serializing_if = "is_false")]
    pub digest: bool,
}

/// A variable filled from a response (FEAT-161).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct KeepRule {
    pub var: String,
    /// A JSONPath for JSON, an XPath for XML and SOAP.
    pub from: String,
    /// Read again after every send. Off: kept once, when it was clicked.
    #[serde(default, skip_serializing_if = "is_false")]
    pub every: bool,
    /// An XPath's prefixes, as the response declared them.
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub ns: BTreeMap<String, String>,
}

/// The colours an environment can have. Red means production.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Colour {
    #[default]
    Ok,
    Warn,
    Danger,
    Blue,
    Teal,
}

/// A named set of variables with a colour.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Environment {
    #[serde(default = "version")]
    pub v: u32,
    pub name: String,
    #[serde(default)]
    pub color: Colour,
    #[serde(default)]
    pub ask_before_sending: bool,
    /// Off for a self-signed local server. Settings › Calls says to turn it
    /// off here, for one environment, and not for every call.
    #[serde(default = "yes", skip_serializing_if = "is_true")]
    pub check_tls: bool,
    #[serde(default)]
    pub vars: Vec<Variable>,
}

impl Environment {
    /// A new environment. One named `production`, or coloured red, asks
    /// before sending (FEAT-161).
    pub fn new(name: &str, color: Colour) -> Self {
        Environment {
            v: VERSION,
            name: name.to_string(),
            color,
            ask_before_sending: name.eq_ignore_ascii_case("production") || color == Colour::Danger,
            check_tls: true,
            vars: Vec::new(),
        }
    }

    /// What may be written to a file: secret and kept values emptied. Every
    /// writer goes through this.
    pub fn for_file(&self) -> Environment {
        Environment {
            vars: self.vars.iter().map(Variable::for_file).collect(),
            ..self.clone()
        }
    }
}

/// The variables every environment shares: `shared.json`.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Shared {
    #[serde(default = "version")]
    pub v: u32,
    #[serde(default)]
    pub vars: Vec<Variable>,
}

impl Shared {
    pub fn for_file(&self) -> Shared {
        Shared {
            v: self.v,
            vars: self.vars.iter().map(Variable::for_file).collect(),
        }
    }
}

/// One variable. A plain one has its value here; a secret or a kept one has
/// only its name, and its value in the keychain.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Variable {
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub secret: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub kept: bool,
    /// A kept value whose rules are switched off in this environment: it
    /// keeps the value it has.
    #[serde(default, skip_serializing_if = "is_false")]
    pub frozen: bool,
}

impl Variable {
    pub fn plain(name: &str, value: &str) -> Self {
        Variable {
            name: name.to_string(),
            value: Some(value.to_string()),
            ..Variable::default()
        }
    }

    /// Whether its value belongs in the keychain.
    pub fn private(&self) -> bool {
        self.secret || self.kept
    }

    pub(crate) fn for_file(&self) -> Variable {
        Variable {
            value: if self.private() {
                None
            } else {
                self.value.clone()
            },
            ..self.clone()
        }
    }
}

/// `value` as a file: two-space indent, struct order, a final newline.
pub fn to_text<T: Serialize>(value: &T) -> String {
    let mut text = serde_json::to_string_pretty(value).unwrap_or_default();
    text.push('\n');
    text
}

/// A fresh id for a call: `c_` and twelve characters that sort by time.
pub fn new_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    const ALPHABET: &[u8] = b"0123456789abcdefghjkmnpqrstvwxyz";
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let mut out = String::from("c_");
    // Eight characters of time, then four of chance.
    for shift in (0..8).rev() {
        out.push(ALPHABET[((millis >> (shift * 5)) & 31) as usize] as char);
    }
    let noise = random_u64();
    for shift in 0..4 {
        out.push(ALPHABET[((noise >> (shift * 5)) & 31) as usize] as char);
    }
    out
}

/// A random number, from the standard library's hasher seeds: enough for an
/// id and for `{{$randomInt}}`, and no dependency for it.
pub fn random_u64() -> u64 {
    use std::collections::hash_map::RandomState;
    use std::hash::{BuildHasher, Hasher};
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let mut hasher = RandomState::new().build_hasher();
    hasher.write_u64(COUNTER.fetch_add(1, Ordering::Relaxed));
    if let Ok(now) = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH) {
        hasher.write_u128(now.as_nanos());
    }
    hasher.finish()
}

/// Every call in `items`, depth first, with the folders it is in.
pub fn calls(items: &[Item]) -> Vec<(Vec<String>, &Call)> {
    fn walk<'a>(items: &'a [Item], path: &mut Vec<String>, out: &mut Vec<(Vec<String>, &'a Call)>) {
        for item in items {
            match item {
                Item::Call(call) => out.push((path.clone(), call)),
                Item::Folder(folder) => {
                    path.push(folder.folder.clone());
                    walk(&folder.items, path, out);
                    path.pop();
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(items, &mut Vec::new(), &mut out);
    out
}

/// The call with `id`, to change in place.
pub fn call_mut<'a>(items: &'a mut [Item], id: &str) -> Option<&'a mut Call> {
    for item in items {
        match item {
            Item::Call(call) if call.id == id => return Some(call),
            Item::Call(_) => {}
            Item::Folder(folder) => {
                if let Some(found) = call_mut(&mut folder.items, id) {
                    return Some(found);
                }
            }
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const SAMPLE: &str = r#"{
  "v": 1,
  "name": "orders-api",
  "items": [
    {
      "folder": "Auth",
      "items": [
        {
          "id": "c_01H",
          "name": "Log in",
          "kind": "rest",
          "method": "POST",
          "url": "{{baseUrl}}/auth/login",
          "headers": [
            [
              "Accept",
              "application/json"
            ]
          ],
          "body": {
            "type": "json",
            "text": "{\n  \"email\": \"{{email}}\"\n}"
          },
          "keep": [
            {
              "var": "token",
              "from": "$.data.accessToken",
              "every": true
            }
          ]
        }
      ]
    }
  ]
}
"#;

    #[test]
    fn a_collection_reads_and_writes_back_to_the_same_bytes() {
        let collection: Collection = serde_json::from_str(SAMPLE).unwrap();
        assert_eq!(to_text(&collection), SAMPLE);
    }

    #[test]
    fn a_plain_get_is_four_keys() {
        let call = Call {
            id: "c_1".into(),
            name: "Ping".into(),
            url: "http://localhost/ping".into(),
            ..Call::default()
        };
        let text = to_text(&call);
        assert_eq!(
            text,
            "{\n  \"id\": \"c_1\",\n  \"name\": \"Ping\",\n  \"kind\": \"rest\",\n  \"url\": \"http://localhost/ping\"\n}\n"
        );
        assert_eq!(call.method(), "GET");
    }

    #[test]
    fn a_header_switched_off_keeps_its_value() {
        let pair = Pair {
            on: false,
            ..Pair::new("X-Debug", "1")
        };
        let text = serde_json::to_string(&pair).unwrap();
        assert_eq!(text, r#"["X-Debug","1",false]"#);
        assert_eq!(serde_json::from_str::<Pair>(&text).unwrap(), pair);
    }

    #[test]
    fn an_environment_file_never_holds_a_secret_or_kept_value() {
        let mut env = Environment::new("staging", Colour::Warn);
        env.vars.push(Variable::plain("baseUrl", "https://staging"));
        env.vars.push(Variable {
            name: "password".into(),
            value: Some("hunter2".into()),
            secret: true,
            ..Variable::default()
        });
        env.vars.push(Variable {
            name: "token".into(),
            value: Some("eyJ".into()),
            kept: true,
            ..Variable::default()
        });
        let text = to_text(&env.for_file());
        assert!(text.contains("https://staging"));
        assert!(!text.contains("hunter2"));
        assert!(!text.contains("eyJ"));
        assert!(text.contains("\"password\""));
    }

    #[test]
    fn production_and_red_ask_before_sending() {
        assert!(Environment::new("production", Colour::Ok).ask_before_sending);
        assert!(Environment::new("prod-eu", Colour::Danger).ask_before_sending);
        assert!(!Environment::new("staging", Colour::Warn).ask_before_sending);
    }

    #[test]
    fn ids_are_distinct_and_shaped() {
        let a = new_id();
        let b = new_id();
        assert_ne!(a, b);
        assert_eq!(a.len(), 14);
        assert!(a.starts_with("c_"));
    }

    #[test]
    fn calls_are_found_in_folders() {
        let collection: Collection = serde_json::from_str(SAMPLE).unwrap();
        let found = calls(&collection.items);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].0, vec!["Auth".to_string()]);
        assert_eq!(found[0].1.name, "Log in");
    }
}
