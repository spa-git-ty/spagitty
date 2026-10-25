// SPDX-License-Identifier: GPL-3.0-or-later

//! Endpoints declared in Java (FEAT-162, proposal §8.2): Spring MVC and
//! WebFlux controllers, and JAX-RS resources.
//!
//! Parsed with the tree-sitter Java grammar Files already compiles in, and
//! read the way a person reads a controller: the class's mapping, each
//! method's mapping and its annotated parameters. A request body's example
//! is made from its class or record wherever it is in the scanned files,
//! field by field to a depth of four, with Bean Validation annotations
//! shaping the samples (`@Email` is an address, `@Min(1)` is 1).
//!
//! A constant used as a path (`@GetMapping(Paths.ORDERS)`) is looked up
//! among the `static final String`s of the scanned files; what cannot be
//! resolved is left as written, so the path still says where it came from.

use std::collections::HashMap;

use tree_sitter::{Node, Parser};

use super::json::J;
use super::{
    build, key_of, words_of, Choices, Declared, Endpoint, Param, Source, SourceKind, Status,
};

/// How deep an example body goes.
const DEPTH: usize = 4;

/// An annotation: its simple name and its arguments, each a key (absent
/// for the single unnamed value) and its values, strings resolved.
#[derive(Debug, Clone, Default)]
struct Ann {
    name: String,
    args: Vec<(Option<String>, Vec<String>)>,
}

impl Ann {
    /// The values of `key`, or of the unnamed value when `key` is `value`.
    fn values(&self, keys: &[&str]) -> Vec<String> {
        for (key, values) in &self.args {
            match key {
                None if keys.contains(&"value") => return values.clone(),
                Some(k) if keys.contains(&k.as_str()) => return values.clone(),
                _ => {}
            }
        }
        Vec::new()
    }

    fn first(&self, keys: &[&str]) -> Option<String> {
        self.values(keys).into_iter().next()
    }
}

#[derive(Debug, Clone)]
struct FieldInfo {
    name: String,
    ty: String,
    anns: Vec<Ann>,
}

#[derive(Debug, Clone, Default)]
struct TypeInfo {
    fields: Vec<FieldInfo>,
    extends: Option<String>,
    /// An enum's constants.
    constants: Vec<String>,
}

/// Every type and string constant the scanned files declare.
#[derive(Default)]
struct World {
    types: HashMap<String, TypeInfo>,
    /// `NAME` and `Class.NAME` to the string it holds.
    strings: HashMap<String, String>,
    /// JAX-RS `@ApplicationPath`.
    application_path: Option<String>,
}

fn text<'a>(node: Node, source: &'a str) -> &'a str {
    node.utf8_text(source.as_bytes()).unwrap_or("")
}

fn children<'t>(node: Node<'t>) -> Vec<Node<'t>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}

fn last_segment(name: &str) -> &str {
    name.rsplit('.').next().unwrap_or(name)
}

/// A string literal's contents, `"a" + "b"` joined, and a constant looked
/// up; anything else as written.
fn value_of(node: Node, source: &str, world: &World, class: &str) -> Vec<String> {
    match node.kind() {
        "string_literal" => {
            let raw = text(node, source);
            let inner = raw
                .strip_prefix("\"\"\"")
                .and_then(|r| r.strip_suffix("\"\"\""))
                .or_else(|| raw.strip_prefix('"').and_then(|r| r.strip_suffix('"')))
                .unwrap_or(raw);
            vec![inner.replace("\\\"", "\"").replace("\\\\", "\\")]
        }
        "element_value_array_initializer" | "array_initializer" => children(node)
            .into_iter()
            .flat_map(|child| value_of(child, source, world, class))
            .collect(),
        "binary_expression" => {
            let parts: Vec<String> = children(node)
                .into_iter()
                .map(|child| value_of(child, source, world, class).join(""))
                .collect();
            vec![parts.join("")]
        }
        "parenthesized_expression" => children(node)
            .first()
            .map(|child| value_of(*child, source, world, class))
            .unwrap_or_default(),
        "identifier" | "field_access" | "scoped_identifier" => {
            let raw = text(node, source).to_string();
            let found = world
                .strings
                .get(&raw)
                .or_else(|| world.strings.get(&format!("{class}.{raw}")))
                .or_else(|| world.strings.get(last_segment(&raw)));
            vec![found.cloned().unwrap_or(raw)]
        }
        _ => vec![text(node, source).to_string()],
    }
}

fn annotation(node: Node, source: &str, world: &World, class: &str) -> Option<Ann> {
    let name = node.child_by_field_name("name")?;
    let mut ann = Ann {
        name: last_segment(text(name, source)).to_string(),
        args: Vec::new(),
    };
    if let Some(args) = node.child_by_field_name("arguments") {
        for arg in children(args) {
            if arg.kind() == "element_value_pair" {
                let key = arg
                    .child_by_field_name("key")
                    .map(|k| text(k, source).to_string());
                let values = arg
                    .child_by_field_name("value")
                    .map(|v| value_of(v, source, world, class))
                    .unwrap_or_default();
                ann.args.push((key, values));
            } else {
                ann.args.push((None, value_of(arg, source, world, class)));
            }
        }
    }
    Some(ann)
}

/// The annotations in a declaration's `modifiers`.
fn annotations(node: Node, source: &str, world: &World, class: &str) -> Vec<Ann> {
    children(node)
        .into_iter()
        .filter(|child| child.kind() == "modifiers")
        .flat_map(children)
        .filter(|child| matches!(child.kind(), "annotation" | "marker_annotation"))
        .filter_map(|child| annotation(child, source, world, class))
        .collect()
}

fn has(anns: &[Ann], name: &str) -> bool {
    anns.iter().any(|ann| ann.name == name)
}

fn find<'a>(anns: &'a [Ann], name: &str) -> Option<&'a Ann> {
    anns.iter().find(|ann| ann.name == name)
}

fn is_static(node: Node, source: &str) -> bool {
    children(node)
        .into_iter()
        .filter(|child| child.kind() == "modifiers")
        .any(|modifiers| {
            text(modifiers, source)
                .split_whitespace()
                .any(|w| w == "static")
        })
}

/// A type as written, without its annotations: `@Valid List<Line>` is
/// `List<Line>`.
fn clean_type(ty: &str) -> String {
    let mut out = String::new();
    let mut words = ty.split_whitespace().peekable();
    while let Some(word) = words.next() {
        if word.starts_with('@') {
            // `@Size(max = 3)` may carry spaces: skip to its closing paren.
            if word.contains('(') && !word.contains(')') {
                for rest in words.by_ref() {
                    if rest.contains(')') {
                        break;
                    }
                }
            }
            continue;
        }
        if word == "final" {
            continue;
        }
        out.push_str(word);
    }
    out
}

/// `Map<String, List<Line>>` as `Map` and its arguments.
fn generic(ty: &str) -> (String, Vec<String>) {
    let ty = clean_type(ty);
    let Some(open) = ty.find('<') else {
        return (ty, Vec::new());
    };
    let base = ty[..open].to_string();
    let inner = &ty[open + 1..ty.rfind('>').unwrap_or(ty.len())];
    let mut args = Vec::new();
    let mut depth = 0;
    let mut start = 0;
    for (i, c) in inner.char_indices() {
        match c {
            '<' => depth += 1,
            '>' => depth -= 1,
            ',' if depth == 0 => {
                args.push(inner[start..i].trim().to_string());
                start = i + 1;
            }
            _ => {}
        }
    }
    args.push(inner[start..].trim().to_string());
    (base, args)
}

/// The type a body really is: `Mono<ResponseEntity<Order>>` is `Order`.
fn unwrap(ty: &str) -> String {
    let (base, args) = generic(ty);
    match last_segment(&base) {
        "Mono" | "Optional" | "ResponseEntity" | "HttpEntity" | "RequestEntity"
        | "JsonNullable"
            if !args.is_empty() =>
        {
            unwrap(&args[0])
        }
        _ => clean_type(ty),
    }
}

fn number(anns: &[Ann], fallback: &str) -> J {
    for name in ["Min", "DecimalMin"] {
        if let Some(value) = find(anns, name).and_then(|ann| ann.first(&["value"])) {
            let value = value.trim_matches('"').trim_end_matches(['L', 'l']);
            if value.parse::<f64>().is_ok() {
                return J::Num(value.to_string());
            }
        }
    }
    if has(anns, "Positive") {
        return J::Num("1".into());
    }
    J::Num(fallback.into())
}

fn string(field: &str, anns: &[Ann]) -> J {
    let lower = field.to_ascii_lowercase();
    if has(anns, "Email") || lower.contains("email") {
        return J::Str("mira@example.com".into());
    }
    if lower.ends_with("url") || lower.ends_with("uri") {
        return J::Str("https://example.com".into());
    }
    if lower.contains("phone") {
        return J::Str("+15550100".into());
    }
    let min = find(anns, "Size")
        .and_then(|ann| ann.first(&["min"]))
        .and_then(|v| v.parse::<usize>().ok())
        .unwrap_or(0);
    let sample = if field.is_empty() {
        "string".to_string()
    } else {
        field.to_string()
    };
    if sample.len() < min {
        return J::Str(format!("{sample}{}", "x".repeat(min - sample.len())));
    }
    J::Str(sample)
}

/// A sample of type `ty` for the field `field`.
fn sample(
    ty: &str,
    field: &str,
    anns: &[Ann],
    world: &World,
    depth: usize,
    seen: &mut Vec<String>,
) -> J {
    let ty = clean_type(ty);
    if let Some(element) = ty.strip_suffix("[]") {
        if element == "byte" {
            return J::Str("base64".into());
        }
        return J::Arr(vec![sample(element, field, anns, world, depth, seen)]);
    }
    let (base, args) = generic(&ty);
    let arg = |at: usize| args.get(at).cloned().unwrap_or_else(|| "Object".into());
    match last_segment(&base) {
        "String" | "CharSequence" => string(field, anns),
        "char" | "Character" => J::Str("a".into()),
        "int" | "Integer" | "long" | "Long" | "short" | "Short" | "byte" | "Byte"
        | "BigInteger" => number(anns, "0"),
        "double" | "Double" | "float" | "Float" | "BigDecimal" | "Number" => number(anns, "0.0"),
        "boolean" | "Boolean" => J::Bool(false),
        "UUID" => J::Str("3fa85f64-5717-4562-b3fc-2c963f66afa6".into()),
        "LocalDate" => J::Str("2026-01-31".into()),
        "LocalDateTime" => J::Str("2026-01-31T09:30:00".into()),
        "Instant" | "OffsetDateTime" | "ZonedDateTime" | "Date" | "Timestamp" => {
            J::Str("2026-01-31T09:30:00Z".into())
        }
        "LocalTime" => J::Str("09:30:00".into()),
        "Duration" => J::Str("PT30M".into()),
        "List" | "Set" | "Collection" | "Iterable" | "ArrayList" | "LinkedList" | "HashSet"
        | "SortedSet" | "Flux" => J::Arr(vec![sample(&arg(0), field, &[], world, depth, seen)]),
        "Map" | "HashMap" | "LinkedHashMap" | "TreeMap" => J::Obj(vec![(
            "key".into(),
            sample(&arg(1), "", &[], world, depth, seen),
        )]),
        "Optional" | "Mono" | "JsonNullable" => sample(&arg(0), field, anns, world, depth, seen),
        "Object" | "JsonNode" | "ObjectNode" => J::Obj(Vec::new()),
        name => match world.types.get(name) {
            Some(info) if !info.constants.is_empty() => J::Str(info.constants[0].clone()),
            Some(_) if depth >= DEPTH || seen.iter().any(|s| s == name) => J::Null,
            Some(_) => {
                seen.push(name.to_string());
                let out = object(name, world, depth + 1, seen);
                seen.pop();
                out
            }
            None => J::Null,
        },
    }
}

/// A class or record's fields as an object, its parents' first.
fn object(name: &str, world: &World, depth: usize, seen: &mut Vec<String>) -> J {
    let mut fields = Vec::new();
    let mut chain = Vec::new();
    let mut at = Some(name.to_string());
    while let Some(current) = at {
        if chain.contains(&current) {
            break;
        }
        let Some(info) = world.types.get(&current) else {
            break;
        };
        chain.push(current);
        at = info.extends.clone();
    }
    for class in chain.iter().rev() {
        for field in &world.types[class].fields {
            if has(&field.anns, "JsonIgnore") {
                continue;
            }
            let key = find(&field.anns, "JsonProperty")
                .and_then(|ann| ann.first(&["value"]))
                .unwrap_or_else(|| field.name.clone());
            let value = sample(&field.ty, &field.name, &field.anns, world, depth, seen);
            fields.push((key, value));
        }
    }
    J::Obj(fields)
}

struct Parsed {
    path: String,
    text: String,
    tree: tree_sitter::Tree,
}

fn type_declarations<'t>(node: Node<'t>, out: &mut Vec<Node<'t>>) {
    for child in children(node) {
        match child.kind() {
            "class_declaration"
            | "record_declaration"
            | "interface_declaration"
            | "enum_declaration" => {
                out.push(child);
                if let Some(body) = child.child_by_field_name("body") {
                    type_declarations(body, out);
                }
            }
            _ => {}
        }
    }
}

/// What every file declares: types, string constants, `@ApplicationPath`.
fn index(files: &[Parsed]) -> World {
    let mut world = World::default();
    // Constants first, in two passes, so one constant may name another.
    for _ in 0..2 {
        for file in files {
            let mut decls = Vec::new();
            type_declarations(file.tree.root_node(), &mut decls);
            for decl in decls {
                let class = decl
                    .child_by_field_name("name")
                    .map(|n| text(n, &file.text).to_string())
                    .unwrap_or_default();
                let Some(body) = decl.child_by_field_name("body") else {
                    continue;
                };
                for member in children(body) {
                    if member.kind() != "field_declaration"
                        && member.kind() != "constant_declaration"
                    {
                        continue;
                    }
                    let ty = member
                        .child_by_field_name("type")
                        .map(|t| text(t, &file.text))
                        .unwrap_or("");
                    let constant =
                        is_static(member, &file.text) || decl.kind() == "interface_declaration";
                    if ty != "String" || !constant {
                        continue;
                    }
                    let mut cursor = member.walk();
                    for declarator in member.children_by_field_name("declarator", &mut cursor) {
                        let (Some(name), Some(value)) = (
                            declarator.child_by_field_name("name"),
                            declarator.child_by_field_name("value"),
                        ) else {
                            continue;
                        };
                        let name = text(name, &file.text).to_string();
                        let resolved = value_of(value, &file.text, &world, &class).join("");
                        world
                            .strings
                            .insert(format!("{class}.{name}"), resolved.clone());
                        world.strings.entry(name).or_insert(resolved);
                    }
                }
            }
        }
    }
    for file in files {
        let mut decls = Vec::new();
        type_declarations(file.tree.root_node(), &mut decls);
        for decl in decls {
            let Some(name) = decl
                .child_by_field_name("name")
                .map(|n| text(n, &file.text).to_string())
            else {
                continue;
            };
            let anns = annotations(decl, &file.text, &world, &name);
            if let Some(path) = find(&anns, "ApplicationPath").and_then(|a| a.first(&["value"])) {
                world.application_path = Some(path);
            }
            let mut info = TypeInfo::default();
            match decl.kind() {
                "record_declaration" => {
                    if let Some(params) = decl.child_by_field_name("parameters") {
                        for param in children(params) {
                            if let (Some(ty), Some(field)) = (
                                param.child_by_field_name("type"),
                                param.child_by_field_name("name"),
                            ) {
                                info.fields.push(FieldInfo {
                                    name: text(field, &file.text).to_string(),
                                    ty: text(ty, &file.text).to_string(),
                                    anns: annotations(param, &file.text, &world, &name),
                                });
                            }
                        }
                    }
                }
                "enum_declaration" => {
                    if let Some(body) = decl.child_by_field_name("body") {
                        info.constants = children(body)
                            .into_iter()
                            .filter(|c| c.kind() == "enum_constant")
                            .filter_map(|c| c.child_by_field_name("name"))
                            .map(|n| text(n, &file.text).to_string())
                            .collect();
                    }
                }
                "class_declaration" => {
                    info.extends = decl
                        .child_by_field_name("superclass")
                        .and_then(|s| children(s).into_iter().next())
                        .map(|t| generic(text(t, &file.text)).0)
                        .map(|t| last_segment(&t).to_string());
                    if let Some(body) = decl.child_by_field_name("body") {
                        for member in children(body) {
                            if member.kind() != "field_declaration" || is_static(member, &file.text)
                            {
                                continue;
                            }
                            if children(member).into_iter().any(|c| {
                                c.kind() == "modifiers" && text(c, &file.text).contains("transient")
                            }) {
                                continue;
                            }
                            let ty = member
                                .child_by_field_name("type")
                                .map(|t| text(t, &file.text).to_string())
                                .unwrap_or_default();
                            let anns = annotations(member, &file.text, &world, &name);
                            let mut cursor = member.walk();
                            for declarator in
                                member.children_by_field_name("declarator", &mut cursor)
                            {
                                if let Some(field) = declarator.child_by_field_name("name") {
                                    info.fields.push(FieldInfo {
                                        name: text(field, &file.text).to_string(),
                                        ty: ty.clone(),
                                        anns: anns.clone(),
                                    });
                                }
                            }
                        }
                    }
                }
                _ => continue,
            }
            world.types.entry(name).or_insert(info);
        }
    }
    world
}

/// `/api` and `orders/` as `/api/orders`.
fn join(parts: &[&str]) -> String {
    let mut out = String::new();
    for part in parts {
        let part = part.trim().trim_matches('/');
        if !part.is_empty() {
            out.push('/');
            out.push_str(part);
        }
    }
    if out.is_empty() {
        "/".into()
    } else {
        out
    }
}

const SPRING_MAPPINGS: [(&str, &str); 5] = [
    ("GetMapping", "GET"),
    ("PostMapping", "POST"),
    ("PutMapping", "PUT"),
    ("PatchMapping", "PATCH"),
    ("DeleteMapping", "DELETE"),
];
const JAXRS_METHODS: [&str; 7] = ["GET", "POST", "PUT", "PATCH", "DELETE", "HEAD", "OPTIONS"];

/// The Spring mapping on a method: its method and paths.
fn spring_mapping(anns: &[Ann]) -> Option<(String, Vec<String>, &Ann)> {
    for (name, method) in SPRING_MAPPINGS {
        if let Some(ann) = find(anns, name) {
            return Some((method.to_string(), ann.values(&["value", "path"]), ann));
        }
    }
    let ann = find(anns, "RequestMapping")?;
    let method = ann
        .first(&["method"])
        .map(|m| last_segment(&m).to_ascii_uppercase())
        .unwrap_or_else(|| "GET".into());
    Some((method, ann.values(&["value", "path"]), ann))
}

struct Endpoints<'a> {
    world: &'a World,
    frameworks: &'a [String],
    out: Vec<Endpoint>,
    sources: Vec<Source>,
}

impl Endpoints<'_> {
    fn file(&mut self, file: &Parsed) {
        let mut decls = Vec::new();
        type_declarations(file.tree.root_node(), &mut decls);
        let reactive = file.text.contains("reactor.core.publisher");
        let start = self.out.len();
        let mut kind = SourceKind::Spring;
        for decl in decls {
            if !matches!(decl.kind(), "class_declaration" | "interface_declaration") {
                continue;
            }
            let Some(class) = decl
                .child_by_field_name("name")
                .map(|n| text(n, &file.text).to_string())
            else {
                continue;
            };
            let anns = annotations(decl, &file.text, self.world, &class);
            let methods: Vec<Node> = decl
                .child_by_field_name("body")
                .map(children)
                .unwrap_or_default()
                .into_iter()
                .filter(|m| m.kind() == "method_declaration")
                .collect();
            let method_anns: Vec<Vec<Ann>> = methods
                .iter()
                .map(|m| annotations(*m, &file.text, self.world, &class))
                .collect();

            let spring_class = has(&anns, "RestController")
                || (has(&anns, "Controller")
                    && (has(&anns, "ResponseBody")
                        || method_anns.iter().any(|a| has(a, "ResponseBody"))))
                || (decl.kind() == "interface_declaration"
                    && method_anns.iter().any(|a| spring_mapping(a).is_some()));
            let jaxrs_class = !spring_class
                && (has(&anns, "Path")
                    || method_anns
                        .iter()
                        .any(|a| JAXRS_METHODS.iter().any(|m| has(a, m))));
            let frameworks = self.frameworks;
            let wanted = |name: &str| frameworks.iter().any(|f| f == name);

            if spring_class
                && (wanted(if reactive {
                    "springWebflux"
                } else {
                    "springMvc"
                }))
            {
                let base = find(&anns, "RequestMapping")
                    .and_then(|a| a.values(&["value", "path"]).into_iter().next())
                    .unwrap_or_default();
                let class_consumes =
                    find(&anns, "RequestMapping").and_then(|a| a.first(&["consumes"]));
                for (method, anns) in methods.iter().zip(&method_anns) {
                    let Some((verb, paths, mapping)) = spring_mapping(anns) else {
                        continue;
                    };
                    let path = join(&[&base, paths.first().map(String::as_str).unwrap_or("")]);
                    let consumes = mapping
                        .first(&["consumes"])
                        .or_else(|| class_consumes.clone());
                    self.method(file, &class, *method, verb, path, consumes, false);
                }
            } else if jaxrs_class && wanted("jaxRs") {
                kind = SourceKind::JaxRs;
                let base = find(&anns, "Path")
                    .and_then(|a| a.first(&["value"]))
                    .unwrap_or_default();
                let app = self.world.application_path.clone().unwrap_or_default();
                let class_consumes = find(&anns, "Consumes").and_then(|a| a.first(&["value"]));
                for (method, anns) in methods.iter().zip(&method_anns) {
                    let Some(verb) = JAXRS_METHODS.iter().find(|m| has(anns, m)) else {
                        continue;
                    };
                    let own = find(anns, "Path")
                        .and_then(|a| a.first(&["value"]))
                        .unwrap_or_default();
                    let path = join(&[&app, &base, &own]);
                    let consumes = find(anns, "Consumes")
                        .and_then(|a| a.first(&["value"]))
                        .or_else(|| class_consumes.clone())
                        .map(|c| {
                            last_segment(&c)
                                .replace("APPLICATION_JSON", "application/json")
                                .replace(
                                    "APPLICATION_FORM_URLENCODED",
                                    "application/x-www-form-urlencoded",
                                )
                                .replace("_TYPE", "")
                        });
                    self.method(
                        file,
                        &class,
                        *method,
                        verb.to_string(),
                        path,
                        consumes,
                        true,
                    );
                }
            }
        }
        let count = self.out.len() - start;
        if count > 0 {
            let label = file
                .path
                .rsplit('/')
                .next()
                .unwrap_or(&file.path)
                .to_string();
            self.sources.push(Source {
                id: format!("code:{}", file.path),
                kind,
                label,
                file: Some(file.path.clone()),
                count,
                error: None,
                in_repository: true,
            });
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn method(
        &mut self,
        file: &Parsed,
        class: &str,
        method: Node,
        verb: String,
        path: String,
        consumes: Option<String>,
        jaxrs: bool,
    ) {
        let source = &file.text;
        let name = method
            .child_by_field_name("name")
            .map(|n| text(n, source).to_string())
            .unwrap_or_default();
        let mut declared = Declared {
            method: verb.clone(),
            path: path.clone(),
            consumes: consumes.clone(),
            ..Declared::default()
        };
        let mut form: Vec<(String, J)> = Vec::new();
        let mut multipart = false;
        if let Some(params) = method.child_by_field_name("parameters") {
            for param in children(params) {
                if param.kind() != "formal_parameter" {
                    continue;
                }
                let anns = annotations(param, source, self.world, class);
                let ty = param
                    .child_by_field_name("type")
                    .map(|t| text(t, source))
                    .unwrap_or("");
                let pname = param
                    .child_by_field_name("name")
                    .map(|n| text(n, source).to_string())
                    .unwrap_or_default();
                let named = |ann: &Ann| {
                    ann.first(&["value", "name"])
                        .filter(|n| !n.is_empty())
                        .unwrap_or_else(|| pname.clone())
                };
                if jaxrs {
                    let default = find(&anns, "DefaultValue")
                        .and_then(|a| a.first(&["value"]))
                        .unwrap_or_default();
                    if let Some(ann) = find(&anns, "QueryParam") {
                        declared.query.push(Param {
                            name: named(ann),
                            sample: default,
                            required: false,
                        });
                    } else if let Some(ann) = find(&anns, "HeaderParam") {
                        declared.headers.push(Param {
                            name: named(ann),
                            sample: default,
                            required: false,
                        });
                    } else if let Some(ann) = find(&anns, "FormParam") {
                        form.push((named(ann), J::Str(default)));
                    } else if anns.is_empty() || has(&anns, "Valid") || has(&anns, "NotNull") {
                        let body = unwrap(ty);
                        declared.body_type = Some(last_segment(&generic(&body).0).to_string());
                        declared.body =
                            Some(sample(&body, "", &[], self.world, 0, &mut Vec::new()));
                    }
                    continue;
                }
                if let Some(ann) = find(&anns, "RequestParam") {
                    if ty.contains("MultipartFile") {
                        multipart = true;
                        form.push((named(ann), J::Str(String::new())));
                    } else if !generic(ty).0.ends_with("Map") {
                        let required = ann
                            .first(&["required"])
                            .map(|r| r != "false")
                            .unwrap_or(true);
                        declared.query.push(Param {
                            name: named(ann),
                            sample: ann.first(&["defaultValue"]).unwrap_or_default(),
                            required,
                        });
                    }
                } else if let Some(ann) = find(&anns, "RequestHeader") {
                    declared.headers.push(Param {
                        name: named(ann),
                        sample: ann.first(&["defaultValue"]).unwrap_or_default(),
                        required: true,
                    });
                } else if let Some(ann) = find(&anns, "RequestPart") {
                    multipart = true;
                    form.push((named(ann), J::Str(String::new())));
                } else if has(&anns, "RequestBody") {
                    let body = unwrap(ty);
                    declared.body_type = Some(last_segment(&generic(&body).0).to_string());
                    declared.body = Some(sample(&body, "", &[], self.world, 0, &mut Vec::new()));
                }
            }
        }
        if declared.body.is_none() && !form.is_empty() {
            declared.body = Some(J::Obj(form));
            if declared.consumes.is_none() || multipart {
                declared.consumes = Some(if multipart {
                    "multipart/form-data".into()
                } else {
                    "application/x-www-form-urlencoded".into()
                });
            }
        }

        let mut hint: Vec<String> = Vec::new();
        if let Some(body) = &declared.body_type {
            hint.push(body.clone());
        }
        let names: Vec<&str> = declared.query.iter().map(|p| p.name.as_str()).collect();
        if !names.is_empty() {
            hint.push(names.join(", "));
        }
        if let Some(c) = declared
            .consumes
            .as_ref()
            .filter(|c| c.as_str() != "application/json")
        {
            hint.push(c.clone());
        }
        let call = build(&declared, &words_of(&name), Choices::default());
        self.out.push(Endpoint {
            id: format!("{}#{}", file.path, self.out.len()),
            key: key_of(&call),
            source: format!("code:{}", file.path),
            group: class.to_string(),
            file: Some(file.path.clone()),
            line: Some(method.start_position().row as u32 + 1),
            method: verb,
            path,
            hint: hint.join(" · "),
            call,
            status: Status::New,
            declared: Some(declared),
            folder: None,
        });
    }
}

/// Every endpoint the Java files declare, for the frameworks named
/// (`springMvc`, `springWebflux`, `jaxRs`), with a source for each file
/// that has any.
pub fn read(files: &[(String, String)], frameworks: &[String]) -> (Vec<Endpoint>, Vec<Source>) {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_java::LANGUAGE.into())
        .is_err()
    {
        return (Vec::new(), Vec::new());
    }
    let parsed: Vec<Parsed> = files
        .iter()
        .filter_map(|(path, text)| {
            parser.parse(text, None).map(|tree| Parsed {
                path: path.clone(),
                text: text.clone(),
                tree,
            })
        })
        .collect();
    let world = index(&parsed);
    let mut endpoints = Endpoints {
        world: &world,
        frameworks,
        out: Vec::new(),
        sources: Vec::new(),
    };
    for file in &parsed {
        endpoints.file(file);
    }
    (endpoints.out, endpoints.sources)
}

/// Whether a Java file is worth parsing: it mentions a mapping, a JAX-RS
/// annotation, or declares a type a body could be.
pub fn interesting(text: &str) -> bool {
    text.contains("class ")
        || text.contains("record ")
        || text.contains("enum ")
        || text.contains("interface ")
}

/// `${PORT:8081}` as `8081`; a plain value as it is.
fn placeholder(value: &str) -> String {
    let value = value.trim().trim_matches('"').trim_matches('\'');
    if let Some(inner) = value.strip_prefix("${").and_then(|v| v.strip_suffix('}')) {
        return inner
            .split_once(':')
            .map(|(_, d)| d.to_string())
            .unwrap_or_default();
    }
    value.to_string()
}

/// `http://localhost:<port><context>` from a Spring Boot `application.yml`
/// or `application.properties` (§8.2), the default profile's.
pub fn base_url(files: &[(String, String)]) -> Option<String> {
    let mut candidates: Vec<&(String, String)> = files
        .iter()
        .filter(|(path, _)| {
            let name = path.rsplit('/').next().unwrap_or(path);
            matches!(
                name,
                "application.yml" | "application.yaml" | "application.properties"
            )
        })
        .collect();
    candidates.sort_by_key(|(path, _)| (path.matches('/').count(), path.clone()));
    let (path, text) = candidates.first()?;
    let mut settings: HashMap<String, String> = HashMap::new();
    if path.ends_with(".properties") {
        for line in text.lines() {
            let line = line.trim();
            if line.starts_with('#') || line.starts_with('!') {
                continue;
            }
            if let Some((key, value)) = line.split_once(['=', ':']) {
                settings.insert(key.trim().to_string(), placeholder(value));
            }
        }
    } else {
        let docs = yaml_rust2::YamlLoader::load_from_str(text).ok()?;
        let doc = docs
            .iter()
            .find(|doc| {
                doc["spring"]["config"]["activate"]["on-profile"].is_badvalue()
                    && doc["spring"]["profiles"].is_badvalue()
            })
            .or(docs.first())?;
        let get = |parts: &[&str]| {
            let mut at = doc;
            for part in parts {
                at = &at[*part];
            }
            match at {
                yaml_rust2::Yaml::String(s) => Some(placeholder(s)),
                yaml_rust2::Yaml::Integer(i) => Some(i.to_string()),
                _ => None,
            }
        };
        for (key, parts) in [
            ("server.port", &["server", "port"][..]),
            (
                "server.servlet.context-path",
                &["server", "servlet", "context-path"][..],
            ),
            ("server.context-path", &["server", "context-path"][..]),
            (
                "spring.webflux.base-path",
                &["spring", "webflux", "base-path"][..],
            ),
        ] {
            if let Some(value) = get(parts) {
                settings.insert(key.to_string(), value);
            }
        }
    }
    let port = settings
        .get("server.port")
        .filter(|p| !p.is_empty() && *p != "0")
        .cloned()
        .unwrap_or_else(|| "8080".into());
    let context = [
        "server.servlet.context-path",
        "spring.webflux.base-path",
        "server.context-path",
    ]
    .iter()
    .find_map(|key| settings.get(*key).filter(|v| !v.is_empty()).cloned())
    .unwrap_or_default();
    let context = context.trim_end_matches('/');
    let context = if context.is_empty() || context.starts_with('/') {
        context.to_string()
    } else {
        format!("/{context}")
    };
    Some(format!("http://localhost:{port}{context}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn files(list: &[(&str, &str)]) -> Vec<(String, String)> {
        list.iter()
            .map(|(p, t)| (p.to_string(), t.to_string()))
            .collect()
    }

    const ALL: [&str; 3] = ["springMvc", "springWebflux", "jaxRs"];

    fn all() -> Vec<String> {
        ALL.map(String::from).to_vec()
    }

    const ORDERS: &str = r#"
package shop;

import org.springframework.web.bind.annotation.*;

@RestController
@RequestMapping(Paths.ORDERS)
public class OrderController {

    @GetMapping("/{id}")
    public Order get(@PathVariable("id") String id, @RequestHeader("X-Tenant") String tenant) { return null; }

    @GetMapping
    public List<Order> list(@RequestParam(defaultValue = "1") int page, @RequestParam(name = "q", required = false) String query) { return null; }

    @PostMapping(consumes = "application/json")
    public ResponseEntity<Order> create(@Valid @RequestBody CreateOrderRequest request) { return null; }

    @RequestMapping(value = "/{id}/cancel", method = RequestMethod.POST)
    public void cancel(@PathVariable String id) {}

    @PostMapping(value = "/{id}/receipt", consumes = "multipart/form-data")
    public void receipt(@PathVariable String id, @RequestParam("file") MultipartFile file) {}

    private void helper() {}
}
"#;

    const MODEL: &str = r#"
package shop;

public final class Paths {
    public static final String API = "/api";
    public static final String ORDERS = API + "/orders";
}

public class Base {
    private UUID id;
}

public class CreateOrderRequest extends Base {
    @NotNull @Size(min = 2) private String sku;
    @Min(1) private int quantity;
    @Email private String contact;
    private List<Line> lines;
    private Status status;
    @JsonProperty("coupon_code") private String couponCode;
    @JsonIgnore private String internal;
    private static final long serialVersionUID = 1L;
    private Map<String, BigDecimal> prices;
    private LocalDate due;
    private CreateOrderRequest parent;
}

public record Line(String sku, @Positive Integer qty, Money price) {}
public record Money(BigDecimal amount, String currency) {}
public enum Status { NEW, PAID }
"#;

    #[test]
    fn spring_controllers_give_their_endpoints() {
        let (endpoints, sources) = read(
            &files(&[
                ("src/OrderController.java", ORDERS),
                ("src/Model.java", MODEL),
            ]),
            &all(),
        );
        let got: Vec<(String, String, String)> = endpoints
            .iter()
            .map(|e| (e.method.clone(), e.path.clone(), e.call.url.clone()))
            .collect();
        assert_eq!(
            got,
            vec![
                (
                    "GET".into(),
                    "/api/orders/{id}".into(),
                    "{{baseUrl}}/api/orders/{{id}}".into()
                ),
                (
                    "GET".into(),
                    "/api/orders".into(),
                    "{{baseUrl}}/api/orders?page=1&q".into()
                ),
                (
                    "POST".into(),
                    "/api/orders".into(),
                    "{{baseUrl}}/api/orders".into()
                ),
                (
                    "POST".into(),
                    "/api/orders/{id}/cancel".into(),
                    "{{baseUrl}}/api/orders/{{id}}/cancel".into()
                ),
                (
                    "POST".into(),
                    "/api/orders/{id}/receipt".into(),
                    "{{baseUrl}}/api/orders/{{id}}/receipt".into()
                ),
            ]
        );
        assert_eq!(endpoints[0].call.headers[0].name, "X-Tenant");
        assert_eq!(endpoints[0].group, "OrderController");
        assert_eq!(endpoints[0].call.name, "Get");
        assert_eq!(endpoints[1].hint, "page, q");
        assert_eq!(endpoints[2].hint, "CreateOrderRequest");
        assert_eq!(
            endpoints[4].call.body.as_ref().unwrap().kind,
            crate::calls::model::BodyType::Multipart
        );
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].count, 5);
        assert_eq!(sources[0].label, "OrderController.java");
    }

    #[test]
    fn a_request_body_is_an_example_of_its_class() {
        let (endpoints, _) = read(
            &files(&[
                ("src/OrderController.java", ORDERS),
                ("src/Model.java", MODEL),
            ]),
            &all(),
        );
        let body = endpoints[2]
            .call
            .body
            .as_ref()
            .unwrap()
            .text
            .clone()
            .unwrap();
        let expected = r#"{
  "id": "3fa85f64-5717-4562-b3fc-2c963f66afa6",
  "sku": "sku",
  "quantity": 1,
  "contact": "mira@example.com",
  "lines": [
    {
      "sku": "sku",
      "qty": 1,
      "price": {
        "amount": 0.0,
        "currency": "currency"
      }
    }
  ],
  "status": "NEW",
  "coupon_code": "couponCode",
  "prices": {
    "key": 0.0
  },
  "due": "2026-01-31",
  "parent": null
}"#;
        assert_eq!(body, expected);
    }

    #[test]
    fn webflux_and_jaxrs_are_read_and_can_be_turned_off() {
        let flux = r#"
import reactor.core.publisher.Mono;
@RestController
@RequestMapping("/v2/users")
class UserHandler {
    @PutMapping("/{userId}")
    Mono<User> replace(@PathVariable String userId, @RequestBody Mono<User> user) { return null; }
}
record User(String name) {}
"#;
        let jaxrs = r#"
@ApplicationPath("api")
public class App extends Application {}

@Path("/invoices")
@Consumes(MediaType.APPLICATION_JSON)
public interface InvoiceResource {
    @GET @Path("{id: [0-9]+}")
    Invoice get(@PathParam("id") long id, @QueryParam("lang") @DefaultValue("en") String lang);

    @POST
    Response create(Invoice invoice);

    @POST @Path("/login") @Consumes(MediaType.APPLICATION_FORM_URLENCODED)
    Response login(@FormParam("user") String user, @FormParam("password") String password);
}
record Invoice(String number) {}
"#;
        let list = files(&[("a/UserHandler.java", flux), ("b/Invoices.java", jaxrs)]);
        let (endpoints, sources) = read(&list, &all());
        let got: Vec<String> = endpoints
            .iter()
            .map(|e| format!("{} {}", e.method, e.call.url))
            .collect();
        assert_eq!(
            got,
            [
                "PUT {{baseUrl}}/v2/users/{{userId}}",
                "GET {{baseUrl}}/api/invoices/{{id}}?lang=en",
                "POST {{baseUrl}}/api/invoices",
                "POST {{baseUrl}}/api/invoices/login",
            ]
        );
        assert_eq!(
            endpoints[0].call.body.as_ref().unwrap().text.as_deref(),
            Some("{\n  \"name\": \"name\"\n}")
        );
        assert_eq!(endpoints[3].call.body.as_ref().unwrap().fields.len(), 2);
        assert_eq!(
            sources.iter().map(|s| s.kind).collect::<Vec<_>>(),
            [SourceKind::Spring, SourceKind::JaxRs]
        );

        let (only_mvc, _) = read(&list, &["springMvc".to_string()]);
        assert!(only_mvc.is_empty());
    }

    #[test]
    fn the_server_address_comes_from_application_yml() {
        let yml = "spring:\n  application:\n    name: orders\nserver:\n  port: ${PORT:8081}\n  servlet:\n    context-path: /shop\n---\nspring:\n  config:\n    activate:\n      on-profile: prod\nserver:\n  port: 80\n";
        assert_eq!(
            base_url(&files(&[("svc/src/main/resources/application.yml", yml)])).as_deref(),
            Some("http://localhost:8081/shop")
        );
        assert_eq!(
            base_url(&files(&[(
                "src/main/resources/application.properties",
                "server.port = 9000\n"
            )]))
            .as_deref(),
            Some("http://localhost:9000")
        );
        assert_eq!(
            base_url(&files(&[(
                "src/main/resources/application.yml",
                "spring:\n  webflux:\n    base-path: api\n"
            )]))
            .as_deref(),
            Some("http://localhost:8080/api")
        );
        assert_eq!(base_url(&files(&[("pom.xml", "")])), None);
    }
}
