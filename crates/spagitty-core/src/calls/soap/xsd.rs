// SPDX-License-Identifier: GPL-3.0-or-later

//! XML Schema, as far as an envelope needs it (FEAT-163, proposal §7.1).
//!
//! The schemas a WSDL holds or imports are kept as their XML, and read when
//! a sample is made: global elements and named types by namespace and name,
//! `sequence`, `all` and `choice` (its first branch), `complexContent`
//! extensions and restrictions, `simpleContent`, attributes, and simple
//! types' enumerations. Every element in the sample carries a comment with
//! its type and whether it is required.

use std::collections::{BTreeMap, HashMap};

use super::super::xml::{escape, Element};

pub const XS: &str = "http://www.w3.org/2001/XMLSchema";
const DEPTH: usize = 12;

/// One schema's own facts, for the declarations in it.
#[derive(Debug, Clone, Default)]
struct Context {
    target: Option<String>,
    qualified: bool,
}

#[derive(Debug, Clone, Default)]
pub struct Schemas {
    elements: HashMap<(Option<String>, String), (Element, usize)>,
    types: HashMap<(Option<String>, String), (Element, usize)>,
    contexts: Vec<Context>,
}

impl Schemas {
    /// Take a `<xs:schema>`'s declarations.
    pub fn add(&mut self, schema: &Element) {
        let target = schema.attr("targetNamespace").map(str::to_string);
        let qualified = schema.attr("elementFormDefault") == Some("qualified");
        let at = self.contexts.len();
        self.contexts.push(Context {
            target: target.clone(),
            qualified,
        });
        for child in &schema.children {
            let Some(name) = child.attr("name") else {
                continue;
            };
            let key = (target.clone(), name.to_string());
            match child.local() {
                "element" => {
                    self.elements.entry(key).or_insert((child.clone(), at));
                }
                "complexType" | "simpleType" => {
                    self.types.entry(key).or_insert((child.clone(), at));
                }
                _ => {}
            }
        }
    }

    pub fn has_element(&self, ns: &Option<String>, name: &str) -> bool {
        self.elements.contains_key(&(ns.clone(), name.to_string()))
    }
}

/// A sample of a built-in type.
pub fn builtin(name: &str, field: &str) -> String {
    match name {
        "int" | "integer" | "long" | "short" | "byte" | "nonNegativeInteger"
        | "positiveInteger" | "unsignedInt" | "unsignedLong" | "unsignedShort" | "unsignedByte"
        | "negativeInteger" | "nonPositiveInteger" => "0".into(),
        "decimal" | "double" | "float" => "0.0".into(),
        "boolean" => "false".into(),
        "date" => "2026-01-31".into(),
        "dateTime" => "2026-01-31T09:30:00Z".into(),
        "time" => "09:30:00".into(),
        "duration" => "PT30M".into(),
        "base64Binary" | "hexBinary" => String::new(),
        "anyURI" => "https://example.com".into(),
        _ if field.is_empty() => "?".into(),
        _ => field.to_string(),
    }
}

/// Writes a sample, giving each namespace a prefix.
pub struct Writer<'a> {
    pub schemas: &'a Schemas,
    /// Namespace to prefix, in the order first used.
    pub prefixes: Vec<(String, String)>,
    /// The prefixes the WSDL itself uses, to use the same ones.
    pub preferred: BTreeMap<String, String>,
    /// Values typed in an envelope being rebuilt, by their path of local
    /// names from the body's first element.
    pub values: HashMap<String, String>,
    pub out: String,
    seen: Vec<String>,
}

impl<'a> Writer<'a> {
    pub fn new(
        schemas: &'a Schemas,
        preferred: &BTreeMap<String, String>,
        values: HashMap<String, String>,
    ) -> Self {
        // The WSDL's own prefix for each namespace, but not the ones that
        // only name WSDL, SOAP or XML Schema.
        let preferred = preferred
            .iter()
            .filter(|(prefix, ns)| {
                !prefix.is_empty()
                    && !ns.contains("schemas.xmlsoap.org")
                    && !ns.contains("www.w3.org")
            })
            .map(|(prefix, ns)| (ns.clone(), prefix.clone()))
            .collect();
        Writer {
            schemas,
            prefixes: Vec::new(),
            preferred,
            values,
            out: String::new(),
            seen: Vec::new(),
        }
    }

    /// The prefix for `ns`, made when first needed.
    pub fn prefix(&mut self, ns: &str) -> String {
        if let Some((_, p)) = self.prefixes.iter().find(|(n, _)| n == ns) {
            return p.clone();
        }
        let taken = |p: &str, list: &[(String, String)]| list.iter().any(|(_, q)| q == p);
        let mut p = self.preferred.get(ns).cloned().unwrap_or_default();
        if p.is_empty() || taken(&p, &self.prefixes) {
            let mut n = self.prefixes.len() + 1;
            loop {
                p = format!("ns{n}");
                if !taken(&p, &self.prefixes) {
                    break;
                }
                n += 1;
            }
        }
        self.prefixes.push((ns.to_string(), p.clone()));
        p
    }

    fn pad(depth: usize) -> String {
        "  ".repeat(depth)
    }

    /// A global element, by namespace and name.
    pub fn global(&mut self, ns: &Option<String>, name: &str, depth: usize, path: &str) {
        let Some((decl, ctx)) = self
            .schemas
            .elements
            .get(&(ns.clone(), name.to_string()))
            .cloned()
        else {
            let p = self.qualify(ns);
            let tag = format!("{p}{name}");
            self.out
                .push_str(&format!("{}<{tag}>?</{tag}>\n", Self::pad(depth)));
            return;
        };
        self.element(&decl, ctx, true, 1, false, depth, path);
    }

    fn qualify(&mut self, ns: &Option<String>) -> String {
        match ns {
            Some(ns) => format!("{}:", self.prefix(ns)),
            None => String::new(),
        }
    }

    /// An element named `name` of the type `ty` (a part with `type=`).
    pub fn typed(&mut self, name: &str, ty: &(Option<String>, String), depth: usize, path: &str) {
        let tag = name.to_string();
        let path = join(path, name);
        if ty.0.as_deref() == Some(XS) {
            let value = self.value(&path, &builtin(&ty.1, name));
            self.out
                .push_str(&format!("{}<!-- xs:{} -->\n", Self::pad(depth), ty.1));
            self.out.push_str(&format!(
                "{}<{tag}>{}</{tag}>\n",
                Self::pad(depth),
                escape(&value)
            ));
            return;
        }
        match self.schemas.types.get(ty).cloned() {
            Some((decl, ctx)) => {
                self.out
                    .push_str(&format!("{}<!-- {} -->\n", Self::pad(depth), ty.1));
                self.complex_or_simple(&tag, &decl, ctx, depth, &path, &ty.1);
            }
            None => self
                .out
                .push_str(&format!("{}<{tag}>?</{tag}>\n", Self::pad(depth))),
        }
    }

    fn value(&self, path: &str, sample: &str) -> String {
        self.values
            .get(path)
            .cloned()
            .unwrap_or_else(|| sample.to_string())
    }

    #[allow(clippy::too_many_arguments)]
    fn element(
        &mut self,
        decl: &Element,
        ctx: usize,
        global: bool,
        min: u32,
        many: bool,
        depth: usize,
        path: &str,
    ) {
        if depth > DEPTH {
            return;
        }
        if let Some(reference) = decl.attr("ref") {
            let (ns, name) = decl.qname(reference);
            let min = decl
                .attr("minOccurs")
                .and_then(|m| m.parse().ok())
                .unwrap_or(min);
            let many = many
                || decl
                    .attr("maxOccurs")
                    .is_some_and(|m| m == "unbounded" || m.parse::<u32>().is_ok_and(|n| n > 1));
            if let Some((target, at)) = self
                .schemas
                .elements
                .get(&(ns.clone(), name.clone()))
                .cloned()
            {
                self.element(&target, at, true, min, many, depth, path);
            }
            return;
        }
        let Some(name) = decl.attr("name") else {
            return;
        };
        let context = self.schemas.contexts[ctx].clone();
        let qualified = global
            || decl
                .attr("form")
                .map(|f| f == "qualified")
                .unwrap_or(context.qualified);
        let ns = if qualified {
            context.target.clone()
        } else {
            None
        };
        let p = self.qualify(&ns);
        let tag = format!("{p}{name}");
        let min = decl
            .attr("minOccurs")
            .and_then(|m| m.parse().ok())
            .unwrap_or(min);
        let many = many
            || decl
                .attr("maxOccurs")
                .is_some_and(|m| m == "unbounded" || m.parse::<u32>().is_ok_and(|n| n > 1));
        let path = join(path, name);

        let (label, body): (String, Option<(Element, usize)>) = if let Some(ty) = decl.attr("type")
        {
            let (tns, tname) = decl.qname(ty);
            if tns.as_deref() == Some(XS) {
                (format!("xs:{tname}"), None)
            } else {
                (
                    tname.clone(),
                    self.schemas.types.get(&(tns, tname)).cloned(),
                )
            }
        } else if let Some(inline) = decl
            .first("complexType")
            .or_else(|| decl.first("simpleType"))
        {
            (String::new(), Some((inline.clone(), ctx)))
        } else {
            ("xs:anyType".into(), None)
        };
        let mut note = vec![];
        if !label.is_empty() {
            note.push(label.clone());
        }
        note.push(if min == 0 {
            "optional".to_string()
        } else {
            "required".to_string()
        });
        if many {
            note.push("repeats".to_string());
        }
        self.out.push_str(&format!(
            "{}<!-- {} -->\n",
            Self::pad(depth),
            note.join(", ")
        ));
        match body {
            None => {
                let sample = label
                    .strip_prefix("xs:")
                    .map(|t| builtin(t, name))
                    .unwrap_or_else(|| "?".into());
                let value = self.value(&path, &sample);
                self.out.push_str(&format!(
                    "{}<{tag}>{}</{tag}>\n",
                    Self::pad(depth),
                    escape(&value)
                ));
            }
            Some((decl, at)) => {
                if !label.is_empty() && self.seen.iter().filter(|s| **s == label).count() >= 2 {
                    self.out
                        .push_str(&format!("{}<{tag}/>\n", Self::pad(depth)));
                    return;
                }
                if !label.is_empty() {
                    self.seen.push(label.clone());
                }
                self.complex_or_simple(&tag, &decl, at, depth, &path, name);
                if !label.is_empty() {
                    self.seen.pop();
                }
            }
        }
    }

    /// The element `tag` with a complex or simple type's content.
    fn complex_or_simple(
        &mut self,
        tag: &str,
        decl: &Element,
        ctx: usize,
        depth: usize,
        path: &str,
        field: &str,
    ) {
        if decl.local() == "simpleType" {
            let sample = self.simple_sample(decl, field);
            let value = self.value(path, &sample);
            self.out.push_str(&format!(
                "{}<{tag}>{}</{tag}>\n",
                Self::pad(depth),
                escape(&value)
            ));
            return;
        }
        let mut attrs: Vec<(String, String)> = Vec::new();
        let mut inner = std::mem::take(&mut self.out);
        let mut text: Option<String> = None;
        self.content(decl, ctx, depth + 1, path, &mut attrs, &mut text, field);
        let children = std::mem::take(&mut self.out);
        inner.push_str(&Self::pad(depth));
        inner.push('<');
        inner.push_str(tag);
        for (name, value) in &attrs {
            inner.push_str(&format!(" {name}=\"{}\"", escape(value)));
        }
        if let Some(text) = text {
            let value = self.value(path, &text);
            inner.push_str(&format!(">{}</{tag}>\n", escape(&value)));
        } else if children.is_empty() {
            inner.push_str("/>\n");
        } else {
            inner.push_str(">\n");
            inner.push_str(&children);
            inner.push_str(&format!("{}</{tag}>\n", Self::pad(depth)));
        }
        self.out = inner;
    }

    fn simple_sample(&self, decl: &Element, field: &str) -> String {
        if let Some(restriction) = decl.first("restriction") {
            if let Some(first) = restriction
                .first("enumeration")
                .and_then(|e| e.attr("value"))
            {
                return first.to_string();
            }
            if let Some(base) = restriction.attr("base") {
                let (ns, name) = restriction.qname(base);
                if ns.as_deref() == Some(XS) {
                    return builtin(&name, field);
                }
                if let Some((inner, _)) = self.schemas.types.get(&(ns, name)) {
                    return self.simple_sample(inner, field);
                }
            }
        }
        builtin("string", field)
    }

    /// A complex type's content: its particles, attributes, and text.
    #[allow(clippy::too_many_arguments)]
    fn content(
        &mut self,
        decl: &Element,
        ctx: usize,
        depth: usize,
        path: &str,
        attrs: &mut Vec<(String, String)>,
        text: &mut Option<String>,
        field: &str,
    ) {
        for child in &decl.children {
            match child.local() {
                "sequence" | "all" => self.particles(child, ctx, depth, path),
                "choice" => {
                    if let Some(first) = child
                        .children
                        .iter()
                        .find(|c| matches!(c.local(), "element" | "sequence" | "choice"))
                    {
                        self.out.push_str(&format!(
                            "{}<!-- one of {} choices -->\n",
                            Self::pad(depth),
                            child.children.len()
                        ));
                        match first.local() {
                            "element" => self.element(first, ctx, false, 1, false, depth, path),
                            _ => self.particles(first, ctx, depth, path),
                        }
                    }
                }
                "attribute" => {
                    if let Some(name) = child.attr("name") {
                        let sample = child
                            .attr("type")
                            .map(|t| child.qname(t))
                            .filter(|(ns, _)| ns.as_deref() == Some(XS))
                            .map(|(_, t)| builtin(&t, name))
                            .unwrap_or_else(|| name.to_string());
                        attrs.push((name.to_string(), sample));
                    }
                }
                "complexContent" => {
                    for how in child
                        .children
                        .iter()
                        .filter(|c| matches!(c.local(), "extension" | "restriction"))
                    {
                        if how.local() == "extension" {
                            if let Some(base) = how.attr("base") {
                                let key = how.qname(base);
                                if let Some((base_decl, at)) = self.schemas.types.get(&key).cloned()
                                {
                                    if !self.seen.contains(&key.1) {
                                        self.seen.push(key.1.clone());
                                        self.content(
                                            &base_decl, at, depth, path, attrs, text, field,
                                        );
                                        self.seen.pop();
                                    }
                                }
                            }
                        }
                        self.content(how, ctx, depth, path, attrs, text, field);
                    }
                }
                "simpleContent" => {
                    for how in &child.children {
                        let base = how.attr("base").map(|b| how.qname(b));
                        *text = Some(match base {
                            Some((Some(ns), name)) if ns == XS => builtin(&name, field),
                            _ => builtin("string", field),
                        });
                        self.content(how, ctx, depth, path, attrs, text, field);
                    }
                }
                _ => {}
            }
        }
    }

    fn particles(&mut self, group: &Element, ctx: usize, depth: usize, path: &str) {
        for child in &group.children {
            match child.local() {
                "element" => self.element(child, ctx, false, 1, false, depth, path),
                "sequence" | "all" => self.particles(child, ctx, depth, path),
                "choice" => {
                    if let Some(first) = child.first("element") {
                        self.out.push_str(&format!(
                            "{}<!-- one of {} choices -->\n",
                            Self::pad(depth),
                            child.children.len()
                        ));
                        self.element(first, ctx, false, 1, false, depth, path);
                    }
                }
                _ => {}
            }
        }
    }
}

fn join(path: &str, name: &str) -> String {
    if path.is_empty() {
        name.to_string()
    } else {
        format!("{path}/{name}")
    }
}

/// The values a person typed in an envelope, by the path of local names
/// from the body's first element: what *Rebuild from WSDL* keeps.
pub fn typed_values(envelope: &str) -> HashMap<String, String> {
    let mut out = HashMap::new();
    let Some(root) = super::super::xml::parse(envelope) else {
        return out;
    };
    let Some(body) = root.first("Body") else {
        return out;
    };
    fn walk(at: &Element, path: &str, out: &mut HashMap<String, String>) {
        let here = join(path, at.local());
        let elements: Vec<&Element> = at.children.iter().collect();
        if elements.is_empty() {
            let text = at.text.trim();
            if !text.is_empty() && text != "?" {
                out.insert(here, text.to_string());
            }
        } else {
            for child in elements {
                walk(child, &here, out);
            }
        }
    }
    for child in &body.children {
        walk(child, "", &mut out);
    }
    out
}
