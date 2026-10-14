// SPDX-License-Identifier: GPL-3.0-or-later

//! Keep positions without expanding aliases until the bounded conversion pass.
use serde_json::{Map, Value};
use std::{collections::BTreeMap, sync::Arc};
use yaml_rust2::{
    parser::{Event, MarkedEventReceiver, Parser},
    scanner::{Marker, TScalarStyle},
    Yaml,
};

#[derive(Clone, Debug)]
pub struct Node {
    pub data: Arc<Data>,
    pub line: usize,
    pub file: String,
    pub reference: bool,
}
#[derive(Debug)]
pub enum Data {
    Scalar(Value),
    List(Vec<Node>),
    Map(BTreeMap<String, Node>),
}
impl Node {
    pub fn get(&self, key: &str) -> Option<&Self> {
        self.map()?.get(key)
    }
    pub fn map(&self) -> Option<&BTreeMap<String, Node>> {
        if let Data::Map(v) = &*self.data {
            Some(v)
        } else {
            None
        }
    }
    pub fn list(&self) -> &[Node] {
        if let Data::List(v) = &*self.data {
            v
        } else {
            &[]
        }
    }
    pub fn text(&self) -> String {
        match &*self.data {
            Data::Scalar(Value::String(v)) => v.clone(),
            Data::Scalar(v) => v.to_string(),
            _ => String::new(),
        }
    }
    pub fn strings(&self) -> Vec<String> {
        if matches!(&*self.data, Data::List(_)) {
            self.list().iter().map(Self::text).collect()
        } else {
            vec![self.text()]
        }
    }
    pub fn value(&self) -> Result<Value, String> {
        fn visit(n: &Node, left: &mut usize, depth: usize) -> Result<Value, String> {
            if *left == 0 || depth > 64 {
                return Err("YAML expansion exceeds the preview limit.".into());
            }
            *left -= 1;
            Ok(match &*n.data {
                Data::Scalar(v) => v.clone(),
                Data::List(v) => Value::Array(
                    v.iter()
                        .map(|n| visit(n, left, depth + 1))
                        .collect::<Result<_, _>>()?,
                ),
                Data::Map(v) => Value::Object(
                    v.iter()
                        .map(|(k, n)| Ok((k.clone(), visit(n, left, depth + 1)?)))
                        .collect::<Result<Map<_, _>, String>>()?,
                ),
            })
        }
        visit(self, &mut 100_000, 0)
    }
}
#[derive(Default)]
struct Events(Vec<(Event, Marker)>);
impl MarkedEventReceiver for Events {
    fn on_event(&mut self, event: Event, mark: Marker) {
        self.0.push((event, mark));
    }
}

pub fn parse(text: &str, file: &str) -> Result<Node, String> {
    if text.len() > 1024 * 1024 {
        return Err("Workflow files must be smaller than 1 MiB.".into());
    }
    let mut events = Events::default();
    Parser::new_from_str(text)
        .load(&mut events, true)
        .map_err(|e| e.to_string())?;
    let mut cursor = 0;
    let mut anchors = BTreeMap::new();
    while cursor < events.0.len()
        && matches!(
            events.0[cursor].0,
            Event::StreamStart | Event::DocumentStart
        )
    {
        cursor += 1;
    }
    fn node(
        events: &[(Event, Marker)],
        cursor: &mut usize,
        anchors: &mut BTreeMap<usize, Node>,
        file: &str,
        depth: usize,
    ) -> Result<Node, String> {
        if depth > 64 {
            return Err("YAML nesting exceeds the preview limit.".into());
        }
        let (event, mark) = events.get(*cursor).ok_or("Incomplete YAML.")?;
        *cursor += 1;
        let anchor;
        let mut reference = false;
        let data = match event {
            Event::Alias(id) => {
                let mut n = anchors
                    .get(id)
                    .cloned()
                    .ok_or("Recursive or missing YAML anchor.")?;
                n.line = mark.line();
                return Ok(n);
            }
            Event::Scalar(text, style, id, tag) => {
                anchor = *id;
                let plain = *style == TScalarStyle::Plain
                    && !tag.as_ref().is_some_and(|t| t.suffix == "str");
                Data::Scalar(if plain {
                    match Yaml::from_str(text) {
                        Yaml::Boolean(v) => Value::Bool(v),
                        Yaml::Integer(v) => v.into(),
                        Yaml::Null => Value::Null,
                        _ => Value::String(text.clone()),
                    }
                } else {
                    Value::String(text.clone())
                })
            }
            Event::SequenceStart(id, tag) => {
                anchor = *id;
                reference = tag.as_ref().is_some_and(|t| t.suffix == "reference");
                let mut values = Vec::new();
                while !matches!(events.get(*cursor).map(|x| &x.0), Some(Event::SequenceEnd)) {
                    values.push(node(events, cursor, anchors, file, depth + 1)?);
                }
                *cursor += 1;
                Data::List(values)
            }
            Event::MappingStart(id, _) => {
                anchor = *id;
                let mut values = BTreeMap::new();
                let mut merges = Vec::new();
                while !matches!(events.get(*cursor).map(|x| &x.0), Some(Event::MappingEnd)) {
                    let key = node(events, cursor, anchors, file, depth + 1)?;
                    let name = key.text();
                    if name.is_empty() {
                        return Err("Workflow mapping keys must be text.".into());
                    }
                    let mut value = node(events, cursor, anchors, file, depth + 1)?;
                    value.line = key.line;
                    if name == "<<" {
                        if value.map().is_some() {
                            merges.push(value);
                        } else {
                            merges.extend(value.list().iter().cloned());
                        }
                    } else if values.insert(name.clone(), value).is_some() {
                        return Err(format!("Duplicate YAML key {name}."));
                    }
                }
                *cursor += 1;
                for merged in merges {
                    if let Some(map) = merged.map() {
                        for (key, value) in map {
                            values.entry(key.clone()).or_insert_with(|| value.clone());
                        }
                    } else {
                        return Err("A YAML merge must name a mapping.".into());
                    }
                }
                Data::Map(values)
            }
            _ => return Err("A workflow must contain one YAML document.".into()),
        };
        let n = Node {
            data: Arc::new(data),
            line: mark.line(),
            file: file.into(),
            reference,
        };
        if anchor != 0 {
            anchors.insert(anchor, n.clone());
        }
        Ok(n)
    }
    let result = node(&events.0, &mut cursor, &mut anchors, file, 0)?;
    if events.0[cursor..]
        .iter()
        .any(|(e, _)| matches!(e, Event::DocumentStart))
    {
        return Err("A workflow must contain one YAML document.".into());
    }
    result.value()?;
    Ok(result)
}

pub fn merge(base: &Node, added: &Node) -> Node {
    if let (Some(a), Some(b)) = (base.map(), added.map()) {
        let mut map = a.clone();
        for (key, value) in b {
            let value = map
                .get(key)
                .map(|old| merge(old, value))
                .unwrap_or_else(|| value.clone());
            map.insert(key.clone(), value);
        }
        Node {
            data: Arc::new(Data::Map(map)),
            ..added.clone()
        }
    } else {
        added.clone()
    }
}
pub fn references(root: &Node) -> Result<Node, String> {
    fn walk(n: &Node, root: &Node, depth: usize, left: &mut usize) -> Result<Node, String> {
        if depth > 32 || *left == 0 {
            return Err("YAML references exceed the preview limit.".into());
        }
        *left -= 1;
        if n.reference {
            let mut found = root;
            for key in n.strings() {
                found = found
                    .get(&key)
                    .ok_or_else(|| format!("Missing !reference key {key}."))?;
            }
            return walk(found, root, depth + 1, left);
        }
        Ok(Node {
            data: Arc::new(match &*n.data {
                Data::Scalar(v) => Data::Scalar(v.clone()),
                Data::List(v) => Data::List(
                    v.iter()
                        .map(|v| walk(v, root, depth + 1, left))
                        .collect::<Result<_, _>>()?,
                ),
                Data::Map(v) => Data::Map(
                    v.iter()
                        .map(|(k, v)| Ok((k.clone(), walk(v, root, depth + 1, left)?)))
                        .collect::<Result<_, String>>()?,
                ),
            }),
            ..n.clone()
        })
    }
    walk(root, root, 0, &mut 100_000)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn anchors_references_and_positions_survive_without_boolean_on() {
        let root=parse("on: push\n.base: &base\n  script: [echo ok]\njob:\n  <<: *base\n  after_script: !reference [.base, script]\n","ci.yml").unwrap();
        let root = references(&root).unwrap();
        assert_eq!(root.get("on").unwrap().text(), "push");
        let job = root.get("job").unwrap();
        assert_eq!(job.line, 4);
        assert_eq!(
            job.get("after_script").unwrap().value().unwrap(),
            serde_json::json!(["echo ok"])
        );
    }
    #[test]
    fn recursive_anchors_documents_and_duplicate_keys_are_refused() {
        for text in ["a: &a [*a]", "a: 1\na: 2", "a: 1\n---\nb: 2"] {
            assert!(parse(text, "ci.yml").is_err());
        }
        assert!(references(&parse("a: !reference [a]", "ci.yml").unwrap()).is_err());
    }
}
