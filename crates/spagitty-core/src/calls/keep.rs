// SPDX-License-Identifier: GPL-3.0-or-later

//! Values kept from responses (FEAT-161, proposal §6.2).
//!
//! A keep rule names a variable and where its value is in a response: a
//! JSONPath for JSON (`$.data.accessToken`, `$.items[0]`, `$['a-b']`) or an
//! XPath for XML and SOAP (`/r/x/@k`, `//inv:GetInvoiceResponse/inv:total`,
//! `/r/line[2]`). Only these forms are read: they are the ones the screen
//! writes when a value is clicked, and what a person writes by hand is the
//! same few steps. There are no filters, wildcards or functions.
//!
//! After every send each *every call* rule is read again. A path that finds
//! nothing leaves the old value and marks the variable stale, with the call
//! that last filled it; that record lives in application data
//! ([`KeptLog`]), and the value in the keychain.

use std::collections::BTreeMap;
use std::path::Path;

use quick_xml::events::Event;
use quick_xml::Reader;
use serde::{Deserialize, Serialize};
use serde_json::Value as Json;

use super::body::Shape;
use super::model::{KeepRule, Variable};
use crate::Result;

/// The value `rule` finds in a response body of `shape`, or `None`.
pub fn read(shape: Shape, body: &str, rule: &KeepRule) -> Option<String> {
    let from = rule.from.trim();
    if from.starts_with('$') && shape != Shape::Xml {
        json_path(body, from)
    } else if from.starts_with('/') {
        xpath(body, from, &rule.ns)
    } else {
        None
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Step {
    Key(String),
    Index(usize),
}

fn json_steps(path: &str) -> Option<Vec<Step>> {
    let mut rest = path.strip_prefix('$')?;
    let mut steps = Vec::new();
    while !rest.is_empty() {
        if let Some(after) = rest.strip_prefix('.') {
            let end = after.find(['.', '[']).unwrap_or(after.len());
            if end == 0 {
                return None;
            }
            steps.push(Step::Key(after[..end].to_string()));
            rest = &after[end..];
        } else {
            let after = rest.strip_prefix('[')?;
            let quote = after.chars().next()?;
            if quote == '\'' || quote == '"' {
                let mut key = String::new();
                let mut chars = after[1..].char_indices();
                let mut closed = None;
                while let Some((at, c)) = chars.next() {
                    if c == '\\' {
                        key.push(chars.next()?.1);
                    } else if c == quote {
                        closed = Some(at + 1);
                        break;
                    } else {
                        key.push(c);
                    }
                }
                let after = after.get(closed? + 1..)?.strip_prefix(']')?;
                steps.push(Step::Key(key));
                rest = after;
            } else {
                let end = after.find(']')?;
                steps.push(Step::Index(after[..end].trim().parse().ok()?));
                rest = &after[end + 1..];
            }
        }
    }
    Some(steps)
}

/// A JSONPath's value: a string without its quotes, anything else as JSON.
pub fn json_path(body: &str, path: &str) -> Option<String> {
    let steps = json_steps(path)?;
    let root: Json = serde_json::from_str(body).ok()?;
    let mut at = &root;
    for step in &steps {
        at = match step {
            Step::Key(key) => at.as_object()?.get(key)?,
            Step::Index(index) => at.as_array()?.get(*index)?,
        };
    }
    Some(match at {
        Json::String(text) => text.clone(),
        other => other.to_string(),
    })
}

/// An element as the XPath reader needs it.
#[derive(Debug, Default)]
struct Element {
    /// As written, with its prefix.
    name: String,
    /// The namespace its prefix, or the default, stands for here.
    uri: Option<String>,
    attrs: Vec<(String, String)>,
    children: Vec<Element>,
    text: String,
}

fn local(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

fn prefix(name: &str) -> Option<&str> {
    name.split_once(':').map(|(prefix, _)| prefix)
}

const PREDEFINED: [(&str, &str); 5] = [
    ("amp", "&"),
    ("lt", "<"),
    ("gt", ">"),
    ("quot", "\""),
    ("apos", "'"),
];

fn parse_xml(body: &str) -> Option<Element> {
    let mut reader = Reader::from_str(body);
    // Each open element, with the prefixes it declared.
    let mut stack: Vec<(Element, BTreeMap<String, String>)> = Vec::new();
    let mut root: Option<Element> = None;
    let lookup =
        |stack: &[(Element, BTreeMap<String, String>)], own: &BTreeMap<String, String>, p: &str| {
            own.get(p).cloned().or_else(|| {
                stack
                    .iter()
                    .rev()
                    .find_map(|(_, declared)| declared.get(p).cloned())
            })
        };
    let open = |start: &quick_xml::events::BytesStart,
                stack: &[(Element, BTreeMap<String, String>)]| {
        let name = String::from_utf8_lossy(start.name().as_ref()).into_owned();
        let mut declared = BTreeMap::new();
        let mut attrs = Vec::new();
        for attr in start.attributes().flatten() {
            let key = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
            let raw = String::from_utf8_lossy(&attr.value).into_owned();
            let value = quick_xml::escape::unescape(&raw)
                .map(|v| v.into_owned())
                .unwrap_or(raw);
            if key == "xmlns" {
                declared.insert(String::new(), value);
            } else if let Some(p) = key.strip_prefix("xmlns:") {
                declared.insert(p.to_string(), value);
            } else {
                attrs.push((key, value));
            }
        }
        let uri = lookup(stack, &declared, prefix(&name).unwrap_or(""));
        (
            Element {
                name,
                uri,
                attrs,
                ..Element::default()
            },
            declared,
        )
    };
    let close = |element: Element,
                 stack: &mut Vec<(Element, BTreeMap<String, String>)>,
                 root: &mut Option<Element>| {
        match stack.last_mut() {
            Some((parent, _)) => parent.children.push(element),
            None => *root = Some(element),
        }
    };
    loop {
        match reader.read_event().ok()? {
            Event::Start(start) => {
                let opened = open(&start, &stack);
                stack.push(opened);
            }
            Event::Empty(start) => {
                let (element, _) = open(&start, &stack);
                close(element, &mut stack, &mut root);
            }
            Event::End(_) => {
                let (element, _) = stack.pop()?;
                close(element, &mut stack, &mut root);
            }
            Event::Text(text) => {
                if let Some((element, _)) = stack.last_mut() {
                    element.text.push_str(&text.decode().ok()?);
                }
            }
            Event::CData(data) => {
                if let Some((element, _)) = stack.last_mut() {
                    element.text.push_str(&data.decode().ok()?);
                }
            }
            Event::GeneralRef(reference) => {
                if let Some((element, _)) = stack.last_mut() {
                    if let Ok(Some(c)) = reference.resolve_char_ref() {
                        element.text.push(c);
                    } else {
                        let name = reference.decode().ok()?;
                        let found = PREDEFINED.iter().find(|(entity, _)| *entity == name)?;
                        element.text.push_str(found.1);
                    }
                }
            }
            Event::Eof => break,
            _ => {}
        }
    }
    if !stack.is_empty() {
        return None;
    }
    root
}

#[derive(Debug)]
struct XStep {
    /// `//` before it: any depth below.
    deep: bool,
    name: String,
    /// 1-based, as XPath counts.
    position: Option<usize>,
}

fn xsteps(path: &str) -> Option<(Vec<XStep>, Option<String>)> {
    let mut steps = Vec::new();
    let mut attribute = None;
    let mut rest = path;
    while !rest.is_empty() {
        let deep = rest.starts_with("//");
        rest = rest.strip_prefix("//").or_else(|| rest.strip_prefix('/'))?;
        let end = rest.find('/').unwrap_or(rest.len());
        let part = &rest[..end];
        rest = &rest[end..];
        if let Some(name) = part.strip_prefix('@') {
            if !rest.is_empty() || name.is_empty() {
                return None;
            }
            attribute = Some(name.to_string());
            break;
        }
        let (name, position) = match part.split_once('[') {
            Some((name, index)) => (name, Some(index.strip_suffix(']')?.trim().parse().ok()?)),
            None => (part, None),
        };
        if name.is_empty() {
            return None;
        }
        steps.push(XStep {
            deep,
            name: name.to_string(),
            position,
        });
    }
    Some((steps, attribute))
}

/// Whether `element` is what `step` names. A prefix the rule recorded is
/// matched by its namespace, so a response that uses another prefix for the
/// same namespace is still read; one it did not record is matched as written.
fn names(element: &Element, step: &XStep, ns: &BTreeMap<String, String>) -> bool {
    if local(&element.name) != local(&step.name) {
        return false;
    }
    match prefix(&step.name) {
        Some(p) => match ns.get(p) {
            Some(uri) => element.uri.as_deref() == Some(uri.as_str()),
            None => prefix(&element.name) == Some(p),
        },
        None => prefix(&element.name).is_none(),
    }
}

fn descendants<'a>(element: &'a Element, out: &mut Vec<&'a Element>) {
    for child in &element.children {
        out.push(child);
        descendants(child, out);
    }
}

/// An XPath's value: an element's text, trimmed, or an attribute's value.
pub fn xpath(body: &str, path: &str, ns: &BTreeMap<String, String>) -> Option<String> {
    let root = parse_xml(body)?;
    let (steps, attribute) = xsteps(path)?;
    // A document node above the root, so the first step is a child step.
    let document = Element {
        children: vec![root],
        ..Element::default()
    };
    let mut at: Vec<&Element> = vec![&document];
    for step in &steps {
        let mut next = Vec::new();
        for element in &at {
            let mut pool = Vec::new();
            if step.deep {
                descendants(element, &mut pool);
            } else {
                pool.extend(element.children.iter());
            }
            let matched: Vec<&Element> = pool.into_iter().filter(|e| names(e, step, ns)).collect();
            match step.position {
                Some(position) => {
                    if let Some(found) = matched.get(position.checked_sub(1)?) {
                        next.push(*found);
                    }
                }
                None => next.extend(matched),
            }
        }
        at = next;
    }
    let element = at.first()?;
    if std::ptr::eq(*element, &document) {
        return None;
    }
    match attribute {
        Some(name) => element
            .attrs
            .iter()
            .find(|(key, _)| *key == name || local(key) == name)
            .map(|(_, value)| value.clone()),
        None => Some(element.text.trim().to_string()),
    }
}

/// What a send did to one kept variable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Applied {
    pub var: String,
    /// The new value; `None` when the path found nothing and the variable is
    /// now stale.
    pub value: Option<String>,
    pub from: String,
}

/// Read every *every call* rule from a response. `frozen` says which
/// variables are switched off in this environment: they are left alone.
pub fn apply(
    rules: &[KeepRule],
    shape: Shape,
    body: &str,
    frozen: impl Fn(&str) -> bool,
) -> Vec<Applied> {
    rules
        .iter()
        .filter(|rule| rule.every && !rule.var.trim().is_empty() && !frozen(&rule.var))
        .map(|rule| Applied {
            var: rule.var.clone(),
            value: read(shape, body, rule),
            from: rule.from.clone(),
        })
        .collect()
}

/// Make sure `vars` declares `name` as kept, so its value goes to the
/// keychain and its name to the file. A plain variable of that name becomes
/// kept, and loses the value it had in the file; a secret one is left as it
/// is. True when `vars` changed and its file must be written.
pub fn declare(vars: &mut Vec<Variable>, name: &str) -> bool {
    match vars.iter_mut().find(|var| var.name == name) {
        Some(var) if var.kept || var.secret => false,
        Some(var) => {
            var.kept = true;
            var.value = None;
            true
        }
        None => {
            vars.push(Variable {
                name: name.to_string(),
                kept: true,
                ..Variable::default()
            });
            true
        }
    }
}

/// Where a kept value came from, and whether the last send found it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Kept {
    /// Milliseconds since 1970, when it was last filled.
    pub at: i64,
    pub call_id: String,
    pub call: String,
    pub method: String,
    pub from: String,
    /// The last send's path found nothing; the value is the one before.
    pub stale: bool,
}

/// Every kept value's record in one repository, by environment stem (`*` for
/// the shared variables) and then by name. In application data; no value.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeptLog(pub BTreeMap<String, BTreeMap<String, Kept>>);

impl KeptLog {
    pub fn read(path: &Path) -> KeptLog {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn write(&self, path: &Path) -> Result<()> {
        super::store::write_text(path, &super::model::to_text(&self.0))
    }

    pub fn environment(&self, env: &str) -> BTreeMap<String, Kept> {
        self.0.get(env).cloned().unwrap_or_default()
    }

    pub fn set(&mut self, env: &str, var: &str, kept: Kept) {
        self.0
            .entry(env.to_string())
            .or_default()
            .insert(var.to_string(), kept);
    }

    /// Mark `var` stale, keeping where it came from.
    pub fn stale(&mut self, env: &str, var: &str) {
        let entry = self
            .0
            .entry(env.to_string())
            .or_default()
            .entry(var.to_string())
            .or_default();
        entry.stale = true;
    }

    pub fn forget(&mut self, env: &str, var: &str) {
        if let Some(vars) = self.0.get_mut(env) {
            vars.remove(var);
        }
    }

    pub fn rename_environment(&mut self, from: &str, to: &str) {
        if let Some(vars) = self.0.remove(from) {
            self.0.insert(to.to_string(), vars);
        }
    }

    pub fn drop_environment(&mut self, env: &str) {
        self.0.remove(env);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOGIN: &str = r#"{"data":{"accessToken":"ey.J","user":{"id":"usr_4821","roles":["a","b"]},"n":3600,"ok":true},"a-b":null,"it's":"q"}"#;

    fn rule(from: &str) -> KeepRule {
        KeepRule {
            var: "v".into(),
            from: from.into(),
            every: true,
            ns: BTreeMap::new(),
        }
    }

    #[test]
    fn a_jsonpath_reads_what_the_screen_writes() {
        assert_eq!(
            json_path(LOGIN, "$.data.accessToken").as_deref(),
            Some("ey.J")
        );
        assert_eq!(
            json_path(LOGIN, "$.data.user.id").as_deref(),
            Some("usr_4821")
        );
        assert_eq!(
            json_path(LOGIN, "$.data.user.roles[1]").as_deref(),
            Some("b")
        );
        assert_eq!(json_path(LOGIN, "$.data.n").as_deref(), Some("3600"));
        assert_eq!(json_path(LOGIN, "$.data.ok").as_deref(), Some("true"));
        assert_eq!(json_path(LOGIN, "$['a-b']").as_deref(), Some("null"));
        assert_eq!(json_path(LOGIN, r"$['it\'s']").as_deref(), Some("q"));
        assert_eq!(
            json_path(LOGIN, r#"$["data"]["n"]"#).as_deref(),
            Some("3600")
        );
        assert_eq!(
            json_path(LOGIN, "$.data.user.roles").as_deref(),
            Some(r#"["a","b"]"#)
        );
    }

    #[test]
    fn a_jsonpath_that_finds_nothing_is_none() {
        for path in [
            "$.nope",
            "$.data.user.roles[9]",
            "$.data[0]",
            "data",
            "$.",
            "$[x]",
            "$['open",
        ] {
            assert_eq!(json_path(LOGIN, path), None, "{path}");
        }
        assert_eq!(json_path("not json", "$.a"), None);
    }

    const SOAP: &str = r#"<?xml version="1.0"?>
<soap:Envelope xmlns:soap="http://schemas.xmlsoap.org/soap/envelope/" xmlns:inv="urn:inv">
  <soap:Body>
    <inv:GetInvoiceResponse>
      <inv:invoice id="7">
        <inv:total> 12.50 </inv:total>
        <inv:line>a</inv:line>
        <inv:line>b &amp; c&#33;</inv:line>
      </inv:invoice>
    </inv:GetInvoiceResponse>
  </soap:Body>
</soap:Envelope>"#;

    fn ns() -> BTreeMap<String, String> {
        BTreeMap::from([("inv".to_string(), "urn:inv".to_string())])
    }

    #[test]
    fn an_xpath_reads_text_attributes_and_positions() {
        let total = "//inv:GetInvoiceResponse/inv:invoice/inv:total";
        assert_eq!(xpath(SOAP, total, &ns()).as_deref(), Some("12.50"));
        assert_eq!(
            xpath(SOAP, "//inv:GetInvoiceResponse/inv:invoice/@id", &ns()).as_deref(),
            Some("7")
        );
        assert_eq!(
            xpath(SOAP, "//inv:invoice/inv:line[2]", &ns()).as_deref(),
            Some("b & c!")
        );
        assert_eq!(
            xpath(
                SOAP,
                "/soap:Envelope/soap:Body/inv:GetInvoiceResponse/inv:invoice/inv:line[1]",
                &BTreeMap::new()
            )
            .as_deref(),
            Some("a")
        );
        assert_eq!(
            xpath("<r><x k=\"v\">1</x></r>", "/r/x/@k", &BTreeMap::new()).as_deref(),
            Some("v")
        );
        assert_eq!(
            xpath("<r><![CDATA[<b>]]></r>", "/r", &BTreeMap::new()).as_deref(),
            Some("<b>")
        );
    }

    #[test]
    fn a_recorded_prefix_is_matched_by_its_namespace() {
        // The server answered with another prefix for the same namespace.
        let other = SOAP.replace("inv:", "i:").replace("xmlns:inv", "xmlns:i");
        let total = "//inv:GetInvoiceResponse/inv:invoice/inv:total";
        assert_eq!(xpath(&other, total, &ns()).as_deref(), Some("12.50"));
        // Without the namespace recorded, the prefix must be as written.
        assert_eq!(xpath(&other, total, &BTreeMap::new()), None);
    }

    #[test]
    fn an_xpath_that_finds_nothing_is_none() {
        for path in [
            "//inv:nope",
            "//inv:invoice/inv:line[3]",
            "/inv:invoice",
            "",
            "/",
            "//inv:invoice/@",
        ] {
            assert_eq!(xpath(SOAP, path, &ns()), None, "{path}");
        }
        assert_eq!(xpath("<a><b></a>", "/a", &BTreeMap::new()), None);
    }

    #[test]
    fn a_send_fills_every_call_rules_and_marks_a_missed_one() {
        let rules = vec![
            KeepRule {
                var: "token".into(),
                ..rule("$.data.accessToken")
            },
            KeepRule {
                var: "gone".into(),
                ..rule("$.data.refresh")
            },
            KeepRule {
                var: "once".into(),
                every: false,
                ..rule("$.data.n")
            },
            KeepRule {
                var: "userId".into(),
                ..rule("$.data.user.id")
            },
        ];
        let applied = apply(&rules, Shape::Json, LOGIN, |var| var == "userId");
        assert_eq!(
            applied,
            vec![
                Applied {
                    var: "token".into(),
                    value: Some("ey.J".into()),
                    from: "$.data.accessToken".into()
                },
                Applied {
                    var: "gone".into(),
                    value: None,
                    from: "$.data.refresh".into()
                },
            ]
        );
        // An XPath on a JSON body, and a JSONPath on XML, find nothing.
        assert_eq!(read(Shape::Json, LOGIN, &rule("/data")), None);
        assert_eq!(read(Shape::Xml, SOAP, &rule("$.data")), None);
    }

    #[test]
    fn keeping_declares_the_variable_once() {
        let mut vars = vec![
            Variable::plain("userId", "1"),
            Variable {
                name: "password".into(),
                secret: true,
                ..Variable::default()
            },
        ];
        assert!(declare(&mut vars, "token"));
        assert!(!declare(&mut vars, "token"));
        assert!(declare(&mut vars, "userId"));
        assert!(vars[0].kept && vars[0].value.is_none());
        assert!(!declare(&mut vars, "password"));
        assert!(!vars[1].kept);
        assert_eq!(vars.len(), 3);
    }

    #[test]
    fn the_log_keeps_where_a_value_came_from() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("kept.json");
        let mut log = KeptLog::read(&path);
        log.set(
            "staging",
            "token",
            Kept {
                at: 1,
                call_id: "c_1".into(),
                call: "Log in".into(),
                method: "POST".into(),
                from: "$.data.accessToken".into(),
                stale: false,
            },
        );
        log.stale("staging", "token");
        log.write(&path).unwrap();
        let back = KeptLog::read(&path);
        assert!(back.environment("staging")["token"].stale);
        assert_eq!(back.environment("staging")["token"].call, "Log in");
        let mut moved = back.clone();
        moved.rename_environment("staging", "stage");
        assert!(moved.environment("staging").is_empty());
        assert_eq!(moved.environment("stage").len(), 1);
        moved.forget("stage", "token");
        assert!(moved.environment("stage").is_empty());
    }
}
