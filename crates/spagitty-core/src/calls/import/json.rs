// SPDX-License-Identifier: GPL-3.0-or-later

//! JSON that keeps its key order (FEAT-162).
//!
//! `serde_json` here sorts an object's keys, which is right for the files
//! Spagitty writes and wrong for a body a person reads: an example built
//! from `CreateOrderRequest` should list its fields in the order the class
//! declares them, an OpenAPI schema's properties in the order the file has
//! them, and merging a changed body must leave the person's order alone.
//! This is the small value that does that: read, built, merged and written
//! with two spaces.

use yaml_rust2::Yaml;

#[derive(Debug, Clone, PartialEq)]
pub enum J {
    Null,
    Bool(bool),
    /// As written, so no digit is lost.
    Num(String),
    Str(String),
    Arr(Vec<J>),
    Obj(Vec<(String, J)>),
}

impl J {
    pub fn get(&self, key: &str) -> Option<&J> {
        match self {
            J::Obj(entries) => entries.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    pub fn str(&self) -> Option<&str> {
        match self {
            J::Str(text) => Some(text),
            _ => None,
        }
    }

    pub fn arr(&self) -> &[J] {
        match self {
            J::Arr(items) => items,
            _ => &[],
        }
    }

    pub fn obj(&self) -> &[(String, J)] {
        match self {
            J::Obj(entries) => entries,
            _ => &[],
        }
    }

    /// A string, or a number or boolean as text.
    pub fn text(&self) -> Option<String> {
        match self {
            J::Str(text) | J::Num(text) => Some(text.clone()),
            J::Bool(b) => Some(b.to_string()),
            _ => None,
        }
    }

    /// Written with two spaces, as a person would.
    pub fn pretty(&self) -> String {
        let mut out = String::new();
        write(self, 0, &mut out);
        out
    }

    /// Every key path in it, `a.b`, with `[]` for an array's items: what
    /// change detection compares.
    pub fn paths(&self, prefix: &str, out: &mut Vec<String>) {
        match self {
            J::Obj(entries) => {
                for (key, value) in entries {
                    let path = if prefix.is_empty() {
                        key.clone()
                    } else {
                        format!("{prefix}.{key}")
                    };
                    out.push(path.clone());
                    value.paths(&path, out);
                }
            }
            J::Arr(items) => {
                if let Some(first) = items.first() {
                    first.paths(&format!("{prefix}[]"), out);
                }
            }
            _ => {}
        }
    }
}

fn quote(text: &str, out: &mut String) {
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
}

fn write(value: &J, depth: usize, out: &mut String) {
    let pad = |depth: usize, out: &mut String| out.push_str(&"  ".repeat(depth));
    match value {
        J::Null => out.push_str("null"),
        J::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        J::Num(n) => out.push_str(n),
        J::Str(s) => quote(s, out),
        J::Arr(items) if items.is_empty() => out.push_str("[]"),
        J::Obj(entries) if entries.is_empty() => out.push_str("{}"),
        J::Arr(items) => {
            out.push_str("[\n");
            for (i, item) in items.iter().enumerate() {
                pad(depth + 1, out);
                write(item, depth + 1, out);
                if i + 1 < items.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            pad(depth, out);
            out.push(']');
        }
        J::Obj(entries) => {
            out.push_str("{\n");
            for (i, (key, item)) in entries.iter().enumerate() {
                pad(depth + 1, out);
                quote(key, out);
                out.push_str(": ");
                write(item, depth + 1, out);
                if i + 1 < entries.len() {
                    out.push(',');
                }
                out.push('\n');
            }
            pad(depth, out);
            out.push('}');
        }
    }
}

/// Read JSON, keeping key order; `None` when it is not JSON.
pub fn parse(text: &str) -> Option<J> {
    let mut reader = Reader {
        bytes: text.as_bytes(),
        text,
        at: 0,
    };
    let value = reader.value(0)?;
    reader.space();
    (reader.at == text.len()).then_some(value)
}

struct Reader<'a> {
    bytes: &'a [u8],
    text: &'a str,
    at: usize,
}

impl Reader<'_> {
    fn space(&mut self) {
        while self.at < self.bytes.len() && self.bytes[self.at].is_ascii_whitespace() {
            self.at += 1;
        }
    }

    fn eat(&mut self, c: u8) -> Option<()> {
        self.space();
        (self.bytes.get(self.at) == Some(&c)).then(|| self.at += 1)
    }

    fn value(&mut self, depth: usize) -> Option<J> {
        if depth > 256 {
            return None;
        }
        self.space();
        match *self.bytes.get(self.at)? {
            b'{' => {
                self.at += 1;
                let mut entries = Vec::new();
                if self.eat(b'}').is_some() {
                    return Some(J::Obj(entries));
                }
                loop {
                    self.space();
                    let key = self.string()?;
                    self.eat(b':')?;
                    entries.push((key, self.value(depth + 1)?));
                    if self.eat(b',').is_some() {
                        continue;
                    }
                    self.eat(b'}')?;
                    return Some(J::Obj(entries));
                }
            }
            b'[' => {
                self.at += 1;
                let mut items = Vec::new();
                if self.eat(b']').is_some() {
                    return Some(J::Arr(items));
                }
                loop {
                    items.push(self.value(depth + 1)?);
                    if self.eat(b',').is_some() {
                        continue;
                    }
                    self.eat(b']')?;
                    return Some(J::Arr(items));
                }
            }
            b'"' => self.string().map(J::Str),
            b't' if self.text[self.at..].starts_with("true") => {
                self.at += 4;
                Some(J::Bool(true))
            }
            b'f' if self.text[self.at..].starts_with("false") => {
                self.at += 5;
                Some(J::Bool(false))
            }
            b'n' if self.text[self.at..].starts_with("null") => {
                self.at += 4;
                Some(J::Null)
            }
            _ => {
                let start = self.at;
                while self.at < self.bytes.len()
                    && matches!(
                        self.bytes[self.at],
                        b'-' | b'+' | b'.' | b'e' | b'E' | b'0'..=b'9'
                    )
                {
                    self.at += 1;
                }
                let raw = &self.text[start..self.at];
                raw.parse::<f64>().ok()?;
                Some(J::Num(raw.to_string()))
            }
        }
    }

    fn string(&mut self) -> Option<String> {
        if self.bytes.get(self.at) != Some(&b'"') {
            return None;
        }
        let start = self.at;
        self.at += 1;
        while self.at < self.bytes.len() {
            match self.bytes[self.at] {
                b'\\' => self.at += 2,
                b'"' => {
                    self.at += 1;
                    return serde_json::from_str(&self.text[start..self.at]).ok();
                }
                _ => self.at += 1,
            }
        }
        None
    }
}

/// YAML as the same value; yaml-rust2 keeps a mapping's order.
pub fn from_yaml(yaml: &Yaml) -> J {
    match yaml {
        Yaml::Hash(hash) => J::Obj(
            hash.iter()
                .map(|(key, value)| {
                    let key = match key {
                        Yaml::String(s) => s.clone(),
                        Yaml::Integer(i) => i.to_string(),
                        Yaml::Real(r) => r.clone(),
                        Yaml::Boolean(b) => b.to_string(),
                        _ => String::new(),
                    };
                    (key, from_yaml(value))
                })
                .collect(),
        ),
        Yaml::Array(items) => J::Arr(items.iter().map(from_yaml).collect()),
        Yaml::String(s) => J::Str(s.clone()),
        Yaml::Integer(i) => J::Num(i.to_string()),
        Yaml::Real(r) => J::Num(r.clone()),
        Yaml::Boolean(b) => J::Bool(*b),
        _ => J::Null,
    }
}

/// JSON or YAML, whichever `text` is.
pub fn parse_any(text: &str) -> Option<J> {
    if let Some(value) = parse(text) {
        return Some(value);
    }
    let docs = yaml_rust2::YamlLoader::load_from_str(text).ok()?;
    docs.first().map(from_yaml)
}

/// `new` with the values `old` already has: a key in both keeps `old`'s
/// value (merged again when both are objects), a key only in `new` comes
/// in with its sample, and a key only in `old` goes. `new`'s order wins for
/// new keys; `old`'s keys keep their places.
pub fn merge(new: &J, old: &J) -> J {
    match (new, old) {
        (J::Obj(fresh), J::Obj(kept)) => {
            let mut out: Vec<(String, J)> = kept
                .iter()
                .filter_map(|(key, value)| {
                    fresh
                        .iter()
                        .find(|(k, _)| k == key)
                        .map(|(_, sample)| (key.clone(), merge(sample, value)))
                })
                .collect();
            for (key, sample) in fresh {
                if !out.iter().any(|(k, _)| k == key) {
                    out.push((key.clone(), sample.clone()));
                }
            }
            J::Obj(out)
        }
        (J::Arr(fresh), J::Arr(kept)) => match (fresh.first(), kept.is_empty()) {
            (Some(sample), false) => J::Arr(kept.iter().map(|item| merge(sample, item)).collect()),
            _ => old.clone(),
        },
        _ => old.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_reads_and_writes_in_the_order_given() {
        let text = r#"{"b":1,"a":[true,null,{"z":"x\"y"}],"n":12345678901234567890,"e":{}}"#;
        let value = parse(text).unwrap();
        assert_eq!(
            value.pretty(),
            "{\n  \"b\": 1,\n  \"a\": [\n    true,\n    null,\n    {\n      \"z\": \"x\\\"y\"\n    }\n  ],\n  \"n\": 12345678901234567890,\n  \"e\": {}\n}"
        );
        for bad in ["", "{", "[1,]", "{\"a\" 1}", "nope", "1 2"] {
            assert_eq!(parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn yaml_keeps_its_order_too() {
        let value = parse_any("openapi: 3.0.0\npaths:\n  /b: {}\n  /a: {}\n").unwrap();
        let paths: Vec<&str> = value
            .get("paths")
            .unwrap()
            .obj()
            .iter()
            .map(|(k, _)| k.as_str())
            .collect();
        assert_eq!(paths, ["/b", "/a"]);
    }

    #[test]
    fn merging_keeps_the_persons_values_and_order() {
        let new =
            parse(r#"{"sku":"string","qty":0,"couponCode":"string","lines":[{"a":0,"b":0}]}"#)
                .unwrap();
        let old = parse(r#"{"qty":3,"sku":"AB-1","note":"x","lines":[{"a":5}]}"#).unwrap();
        assert_eq!(
            merge(&new, &old),
            parse(r#"{"qty":3,"sku":"AB-1","lines":[{"a":5,"b":0}],"couponCode":"string"}"#)
                .unwrap()
        );
        let mut paths = Vec::new();
        new.paths("", &mut paths);
        assert_eq!(
            paths,
            [
                "sku",
                "qty",
                "couponCode",
                "lines",
                "lines[].a",
                "lines[].b"
            ]
        );
    }
}
