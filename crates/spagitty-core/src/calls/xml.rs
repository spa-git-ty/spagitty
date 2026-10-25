// SPDX-License-Identifier: GPL-3.0-or-later

//! XML as a small tree (FEAT-161, FEAT-163): what keeping by XPath, reading
//! a WSDL and its schemas, and reading a SOAP fault need.
//!
//! Each element knows the namespace its own prefix stands for and every
//! prefix in scope where it is, because a WSDL names types and messages by
//! qualified names in attribute values (`type="tns:Order"`), and those
//! prefixes mean what the element's scope says. Read with `quick-xml`;
//! entities are resolved, and a document that does not close is `None`.

use std::collections::BTreeMap;

use quick_xml::events::{BytesStart, Event};
use quick_xml::Reader;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Element {
    /// As written, with its prefix.
    pub name: String,
    /// The namespace its prefix, or the default, stands for here.
    pub uri: Option<String>,
    /// Attributes other than namespace declarations, in order.
    pub attrs: Vec<(String, String)>,
    pub children: Vec<Element>,
    /// The text directly inside it, CDATA included, untrimmed.
    pub text: String,
    /// Every prefix in scope here (`""` for the default namespace).
    pub ns: BTreeMap<String, String>,
}

pub fn local(name: &str) -> &str {
    name.rsplit(':').next().unwrap_or(name)
}

pub fn prefix(name: &str) -> Option<&str> {
    name.split_once(':').map(|(prefix, _)| prefix)
}

impl Element {
    pub fn local(&self) -> &str {
        local(&self.name)
    }

    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(key, _)| key == name || local(key) == name)
            .map(|(_, value)| value.as_str())
    }

    /// The children with this local name.
    pub fn all<'a>(&'a self, name: &'a str) -> impl Iterator<Item = &'a Element> + 'a {
        self.children
            .iter()
            .filter(move |child| child.local() == name)
    }

    pub fn first(&self, name: &str) -> Option<&Element> {
        self.children.iter().find(|child| child.local() == name)
    }

    /// Every element below this one, depth first.
    pub fn descendants(&self) -> Vec<&Element> {
        let mut out = Vec::new();
        fn walk<'a>(at: &'a Element, out: &mut Vec<&'a Element>) {
            for child in &at.children {
                out.push(child);
                walk(child, out);
            }
        }
        walk(self, &mut out);
        out
    }

    /// The text of this element and every one below it, as XPath's
    /// `string()` gives it.
    pub fn all_text(&self) -> String {
        let mut out = self.text.clone();
        for child in &self.children {
            out.push_str(&child.all_text());
        }
        out
    }

    /// A qualified name in an attribute value, `tns:Order`, as its
    /// namespace and local name in this element's scope.
    pub fn qname(&self, value: &str) -> (Option<String>, String) {
        match value.split_once(':') {
            Some((prefix, name)) => (self.ns.get(prefix).cloned(), name.to_string()),
            None => (self.ns.get("").cloned(), value.to_string()),
        }
    }
}

const PREDEFINED: [(&str, &str); 5] = [
    ("amp", "&"),
    ("lt", "<"),
    ("gt", ">"),
    ("quot", "\""),
    ("apos", "'"),
];

fn open(start: &BytesStart, scope: &BTreeMap<String, String>) -> Element {
    let name = String::from_utf8_lossy(start.name().as_ref()).into_owned();
    let mut ns = scope.clone();
    let mut attrs = Vec::new();
    for attr in start.attributes().flatten() {
        let key = String::from_utf8_lossy(attr.key.as_ref()).into_owned();
        let raw = String::from_utf8_lossy(&attr.value).into_owned();
        let value = quick_xml::escape::unescape(&raw)
            .map(|v| v.into_owned())
            .unwrap_or(raw);
        if key == "xmlns" {
            ns.insert(String::new(), value);
        } else if let Some(p) = key.strip_prefix("xmlns:") {
            ns.insert(p.to_string(), value);
        } else {
            attrs.push((key, value));
        }
    }
    let uri = ns.get(prefix(&name).unwrap_or("")).cloned();
    Element {
        name,
        uri,
        attrs,
        ns,
        ..Element::default()
    }
}

/// The document's root element, or `None` when it is not well-formed.
pub fn parse(text: &str) -> Option<Element> {
    let mut reader = Reader::from_str(text);
    let mut stack: Vec<Element> = Vec::new();
    let mut root: Option<Element> = None;
    let empty = BTreeMap::new();
    let close = |element: Element, stack: &mut Vec<Element>, root: &mut Option<Element>| match stack
        .last_mut()
    {
        Some(parent) => parent.children.push(element),
        None => {
            if root.is_none() {
                *root = Some(element);
            }
        }
    };
    loop {
        match reader.read_event().ok()? {
            Event::Start(start) => {
                let scope = stack.last().map(|e| &e.ns).unwrap_or(&empty);
                let element = open(&start, scope);
                stack.push(element);
            }
            Event::Empty(start) => {
                let scope = stack.last().map(|e| &e.ns).unwrap_or(&empty);
                let element = open(&start, scope);
                close(element, &mut stack, &mut root);
            }
            Event::End(_) => {
                let element = stack.pop()?;
                close(element, &mut stack, &mut root);
            }
            Event::Text(text) => {
                if let Some(element) = stack.last_mut() {
                    element.text.push_str(&text.decode().ok()?);
                }
            }
            Event::CData(data) => {
                if let Some(element) = stack.last_mut() {
                    element.text.push_str(&data.decode().ok()?);
                }
            }
            Event::GeneralRef(reference) => {
                if let Some(element) = stack.last_mut() {
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

/// `text` safe inside an element or a double-quoted attribute.
pub fn escape(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            c => out.push(c),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn elements_know_their_namespaces_and_scope() {
        let root = parse(
            r#"<w:definitions xmlns:w="urn:wsdl" xmlns:tns="urn:shop" targetNamespace="urn:shop"><w:message name="M"><w:part element="tns:Order"/></w:message><x xmlns="urn:default">a &amp; b<![CDATA[<c>]]></x></w:definitions>"#,
        )
        .unwrap();
        assert_eq!(root.local(), "definitions");
        assert_eq!(root.uri.as_deref(), Some("urn:wsdl"));
        assert_eq!(root.attr("targetNamespace"), Some("urn:shop"));
        let part = root.first("message").unwrap().first("part").unwrap();
        assert_eq!(
            part.qname(part.attr("element").unwrap()),
            (Some("urn:shop".into()), "Order".into())
        );
        let x = root.first("x").unwrap();
        assert_eq!(x.uri.as_deref(), Some("urn:default"));
        assert_eq!(x.text, "a & b<c>");
        assert_eq!(root.descendants().len(), 3);
        assert!(parse("<a><b></a>").is_none());
        assert_eq!(escape("a<b & \"c\""), "a&lt;b &amp; &quot;c&quot;");
    }
}
