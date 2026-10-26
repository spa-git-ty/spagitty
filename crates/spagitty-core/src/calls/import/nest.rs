// SPDX-License-Identifier: GPL-3.0-or-later

//! Endpoints declared by NestJS controllers (FEAT-162; the author asked for
//! NestJS on 2026-10-26).
//!
//! Read from TypeScript with the tree-sitter grammar Files already compiles
//! in, the way the Java reader reads Spring: `@Controller('orders')` on the
//! class, `@Get(':id')` and the others on its methods, and `@Param`,
//! `@Query`, `@Headers` and `@Body` on their parameters. A body's example
//! comes from its DTO class or interface, shaped by class-validator
//! (`@IsEmail`, `@Min(1)`, `@IsEnum(Status)`) and by Swagger's
//! `@ApiProperty({ example })` where it is given. `setGlobalPrefix('api')`
//! in `main.ts` is the paths' prefix, and the port `listen` is given is the
//! `baseUrl`.

use std::collections::HashMap;

use tree_sitter::{Node, Parser};

use super::json::J;
use super::{
    build, key_of, words_of, Choices, Declared, Endpoint, Param, Source, SourceKind, Status,
};

const DEPTH: usize = 4;
const VERBS: [(&str, &str); 7] = [
    ("Get", "GET"),
    ("Post", "POST"),
    ("Put", "PUT"),
    ("Patch", "PATCH"),
    ("Delete", "DELETE"),
    ("Head", "HEAD"),
    ("Options", "OPTIONS"),
];

/// A decorator: its name and its arguments, as the text of each and, for
/// an object argument, its keys.
#[derive(Debug, Clone, Default)]
struct Deco {
    name: String,
    /// String arguments' contents; anything else as written.
    args: Vec<String>,
    /// `{ path: 'x', example: 3 }` as its entries.
    object: Vec<(String, J)>,
}

impl Deco {
    fn first(&self) -> Option<&str> {
        self.args.first().map(String::as_str)
    }
}

#[derive(Debug, Clone)]
struct Prop {
    name: String,
    ty: String,
    decos: Vec<Deco>,
}

#[derive(Default)]
struct World {
    /// Classes and interfaces, by name.
    types: HashMap<String, Vec<Prop>>,
    /// Enums, by name, with their values.
    enums: HashMap<String, Vec<J>>,
}

fn text<'a>(node: Node, source: &'a str) -> &'a str {
    node.utf8_text(source.as_bytes()).unwrap_or("")
}

fn children<'t>(node: Node<'t>) -> Vec<Node<'t>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor).collect()
}

/// A literal as a value: strings without quotes, numbers and booleans as
/// they are, objects and arrays inside out.
fn literal(node: Node, source: &str) -> J {
    match node.kind() {
        "string" | "template_string" => J::Str(
            text(node, source)
                .trim_matches(|c| c == '\'' || c == '"' || c == '`')
                .to_string(),
        ),
        "number" => J::Num(text(node, source).to_string()),
        "true" => J::Bool(true),
        "false" => J::Bool(false),
        "null" | "undefined" => J::Null,
        "array" => J::Arr(
            children(node)
                .into_iter()
                .map(|c| literal(c, source))
                .collect(),
        ),
        "object" => J::Obj(
            children(node)
                .into_iter()
                .filter(|c| c.kind() == "pair")
                .filter_map(|pair| {
                    let key = pair.child_by_field_name("key")?;
                    let value = pair.child_by_field_name("value")?;
                    Some((
                        text(key, source).trim_matches(['\'', '"']).to_string(),
                        literal(value, source),
                    ))
                })
                .collect(),
        ),
        _ => J::Str(text(node, source).to_string()),
    }
}

fn decorator(node: Node, source: &str) -> Option<Deco> {
    let inner = children(node).into_iter().next()?;
    let (callee, args) = match inner.kind() {
        "call_expression" => (
            inner.child_by_field_name("function")?,
            inner.child_by_field_name("arguments"),
        ),
        _ => (inner, None),
    };
    let name = text(callee, source)
        .rsplit('.')
        .next()
        .unwrap_or("")
        .to_string();
    let mut deco = Deco {
        name,
        ..Deco::default()
    };
    for arg in args.map(children).unwrap_or_default() {
        match literal(arg, source) {
            J::Obj(entries) => deco.object = entries,
            J::Str(s) => deco.args.push(s),
            other => deco.args.push(other.text().unwrap_or_default()),
        }
    }
    Some(deco)
}

/// The decorators that are a node's own children.
fn own_decorators(node: Node, source: &str) -> Vec<Deco> {
    children(node)
        .into_iter()
        .filter(|c| c.kind() == "decorator")
        .filter_map(|c| decorator(c, source))
        .collect()
}

fn find<'a>(decos: &'a [Deco], name: &str) -> Option<&'a Deco> {
    decos.iter().find(|d| d.name == name)
}

/// A type annotation's type, as written: `: Order[]` is `Order[]`.
fn type_text(node: Option<Node>, source: &str) -> String {
    node.map(|n| text(n, source).trim_start_matches(':').trim().to_string())
        .unwrap_or_default()
}

/// `{ path: 'x' }` or `'x'`: the path a decorator gives.
fn path_arg(deco: &Deco) -> String {
    deco.first()
        .map(str::to_string)
        .or_else(|| {
            deco.object
                .iter()
                .find(|(k, _)| k == "path")
                .and_then(|(_, v)| v.text())
        })
        .unwrap_or_default()
}

/// `orders/:id` as `orders/{id}`.
fn braces(path: &str) -> String {
    path.split('/')
        .map(|s| match s.strip_prefix(':') {
            Some(name) => format!("{{{}}}", name.trim_end_matches('?')),
            None => s.to_string(),
        })
        .collect::<Vec<_>>()
        .join("/")
}

fn join(parts: &[&str]) -> String {
    let joined: Vec<&str> = parts
        .iter()
        .map(|p| p.trim().trim_matches('/'))
        .filter(|p| !p.is_empty())
        .collect();
    format!("/{}", joined.join("/"))
}

/// A sample of the TypeScript type `ty`.
fn sample(
    ty: &str,
    field: &str,
    decos: &[Deco],
    world: &World,
    depth: usize,
    seen: &mut Vec<String>,
) -> J {
    if let Some(example) = find(decos, "ApiProperty")
        .or_else(|| find(decos, "ApiPropertyOptional"))
        .and_then(|d| d.object.iter().find(|(k, _)| k == "example"))
    {
        return example.1.clone();
    }
    let ty = ty.trim();
    // `string | null`, `'a' | 'b'`: the first that is not nothing.
    if ty.contains('|') {
        let first = ty
            .split('|')
            .map(str::trim)
            .find(|t| !matches!(*t, "null" | "undefined"))
            .unwrap_or("null");
        if first.starts_with(['\'', '"']) {
            return J::Str(first.trim_matches(['\'', '"']).to_string());
        }
        return sample(first, field, decos, world, depth, seen);
    }
    if let Some(element) = ty.strip_suffix("[]") {
        return J::Arr(vec![sample(element, field, decos, world, depth, seen)]);
    }
    let (base, args) = match ty.find('<') {
        Some(at) => (
            &ty[..at],
            ty[at + 1..ty.rfind('>').unwrap_or(ty.len())]
                .split(',')
                .map(str::trim)
                .collect::<Vec<_>>(),
        ),
        None => (ty, Vec::new()),
    };
    let arg = |at: usize| args.get(at).copied().unwrap_or("any");
    let has = |name: &str| decos.iter().any(|d| d.name == name);
    let min = |name: &str| {
        find(decos, name)
            .and_then(|d| d.first())
            .map(str::to_string)
    };
    if let Some(enum_name) = find(decos, "IsEnum").and_then(|d| d.first()) {
        if let Some(first) = world.enums.get(enum_name).and_then(|v| v.first()) {
            return first.clone();
        }
    }
    if let Some(nested) = find(decos, "Type").and_then(|d| d.first()) {
        // `@Type(() => Line)`.
        let name = nested.rsplit("=>").next().unwrap_or(nested).trim();
        if world.types.contains_key(name) && base != name {
            let inner = sample(name, field, &[], world, depth, seen);
            return if base == "Array" || ty.ends_with("[]") {
                J::Arr(vec![inner])
            } else {
                inner
            };
        }
    }
    match base {
        "string" | "String" => {
            let lower = field.to_ascii_lowercase();
            J::Str(if has("IsEmail") || lower.contains("email") {
                "mira@example.com".into()
            } else if has("IsUUID") {
                "3fa85f64-5717-4562-b3fc-2c963f66afa6".into()
            } else if has("IsDateString") || has("IsISO8601") {
                "2026-01-31T09:30:00Z".into()
            } else if has("IsUrl") || lower.ends_with("url") {
                "https://example.com".into()
            } else if let Some(n) = min("MinLength")
                .and_then(|n| n.parse::<usize>().ok())
                .filter(|n| *n > field.len())
            {
                format!("{field}{}", "x".repeat(n - field.len()))
            } else if field.is_empty() {
                "string".into()
            } else {
                field.to_string()
            })
        }
        "number" | "Number" | "bigint" => J::Num(
            min("Min")
                .or_else(|| has("IsPositive").then(|| "1".to_string()))
                .unwrap_or_else(|| "0".into()),
        ),
        "boolean" | "Boolean" => J::Bool(false),
        "Date" => J::Str("2026-01-31T09:30:00Z".into()),
        "Array" | "ReadonlyArray" | "Set" => {
            J::Arr(vec![sample(arg(0), field, &[], world, depth, seen)])
        }
        "Record" | "Map" => J::Obj(vec![(
            "key".into(),
            sample(arg(1), "", &[], world, depth, seen),
        )]),
        "Promise" | "Observable" | "Partial" | "Required" | "Readonly" => {
            sample(arg(0), field, decos, world, depth, seen)
        }
        "any" | "unknown" | "object" | "Object" => J::Obj(Vec::new()),
        name => {
            if let Some(first) = world.enums.get(name).and_then(|v| v.first()) {
                return first.clone();
            }
            match world.types.get(name) {
                Some(_) if depth >= DEPTH || seen.iter().any(|s| s == name) => J::Null,
                Some(props) => {
                    seen.push(name.to_string());
                    let out = J::Obj(
                        props
                            .iter()
                            .map(|p| {
                                (
                                    p.name.clone(),
                                    sample(&p.ty, &p.name, &p.decos, world, depth + 1, seen),
                                )
                            })
                            .collect(),
                    );
                    seen.pop();
                    out
                }
                None => J::Null,
            }
        }
    }
}

struct Parsed {
    path: String,
    text: String,
    tree: tree_sitter::Tree,
}

/// Top-level declarations, each with the decorators written before it
/// (an exported class keeps them on its `export` statement).
fn declarations<'t>(root: Node<'t>, source: &str) -> Vec<(Node<'t>, Vec<Deco>)> {
    let mut out = Vec::new();
    for child in children(root) {
        match child.kind() {
            "export_statement" => {
                let decos = own_decorators(child, source);
                if let Some(decl) = child.child_by_field_name("declaration") {
                    let mut all = decos;
                    all.extend(own_decorators(decl, source));
                    out.push((decl, all));
                }
            }
            _ => {
                let decos = own_decorators(child, source);
                out.push((child, decos));
            }
        }
    }
    out
}

fn index(files: &[Parsed]) -> World {
    let mut world = World::default();
    for file in files {
        let source = &file.text;
        for (decl, _) in declarations(file.tree.root_node(), source) {
            let Some(name) = decl
                .child_by_field_name("name")
                .map(|n| text(n, source).to_string())
            else {
                continue;
            };
            let Some(body) = decl.child_by_field_name("body") else {
                continue;
            };
            match decl.kind() {
                "class_declaration" | "abstract_class_declaration" => {
                    let mut pending: Vec<Deco> = Vec::new();
                    let mut props = Vec::new();
                    for member in children(body) {
                        match member.kind() {
                            "decorator" => pending.extend(decorator(member, source)),
                            "public_field_definition" => {
                                let mut decos = std::mem::take(&mut pending);
                                decos.extend(own_decorators(member, source));
                                if text(member, source).trim_start().starts_with("static")
                                    || decos
                                        .iter()
                                        .any(|d| d.name == "Exclude" || d.name == "ApiHideProperty")
                                {
                                    continue;
                                }
                                if let Some(field) = member.child_by_field_name("name") {
                                    props.push(Prop {
                                        name: text(field, source).to_string(),
                                        ty: type_text(member.child_by_field_name("type"), source),
                                        decos,
                                    });
                                }
                            }
                            _ => pending.clear(),
                        }
                    }
                    world.types.entry(name).or_insert(props);
                }
                "interface_declaration" => {
                    let props = children(body)
                        .into_iter()
                        .filter(|m| m.kind() == "property_signature")
                        .filter_map(|m| {
                            Some(Prop {
                                name: text(m.child_by_field_name("name")?, source).to_string(),
                                ty: type_text(m.child_by_field_name("type"), source),
                                decos: Vec::new(),
                            })
                        })
                        .collect();
                    world.types.entry(name).or_insert(props);
                }
                "enum_declaration" => {
                    let values = children(body)
                        .into_iter()
                        .filter_map(|m| match m.kind() {
                            "enum_assignment" => {
                                m.child_by_field_name("value").map(|v| literal(v, source))
                            }
                            "property_identifier" => Some(J::Str(text(m, source).to_string())),
                            _ => None,
                        })
                        .collect();
                    world.enums.insert(name, values);
                }
                _ => {}
            }
        }
    }
    world
}

/// The global prefix and the port, from the files that start the app.
fn bootstrap(files: &[(String, String)]) -> (String, Option<String>) {
    let prefix =
        regex::Regex::new(r#"setGlobalPrefix\(\s*['"`]([^'"`]*)['"`]"#).expect("a valid pattern");
    let listen = regex::Regex::new(r#"\.listen\(\s*(?:[\w.]+\s*(?:\?\?|\|\|)\s*)?['"]?(\d{2,5})"#)
        .expect("a valid pattern");
    let mut found_prefix = String::new();
    let mut port = None;
    for (_, text) in files {
        if !text.contains("NestFactory") {
            continue;
        }
        if let Some(m) = prefix.captures(text) {
            found_prefix = m[1].to_string();
        }
        if let Some(m) = listen.captures(text) {
            port = Some(m[1].to_string());
        }
    }
    (found_prefix, port.map(|p| format!("http://localhost:{p}")))
}

/// Every endpoint the TypeScript files' Nest controllers declare, a source
/// for each file that has any, and the `baseUrl` from `main.ts`.
pub fn read(files: &[(String, String)]) -> (Vec<Endpoint>, Vec<Source>, Option<String>) {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
        .is_err()
    {
        return (Vec::new(), Vec::new(), None);
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
    let (prefix, base_url) = bootstrap(files);
    let mut out = Vec::new();
    let mut sources = Vec::new();
    for file in &parsed {
        let source = &file.text;
        let start = out.len();
        for (decl, decos) in declarations(file.tree.root_node(), source) {
            if decl.kind() != "class_declaration" {
                continue;
            }
            let Some(controller) = find(&decos, "Controller") else {
                continue;
            };
            let class = decl
                .child_by_field_name("name")
                .map(|n| text(n, source).to_string())
                .unwrap_or_default();
            let base = path_arg(controller);
            let Some(body) = decl.child_by_field_name("body") else {
                continue;
            };
            let mut pending: Vec<Deco> = Vec::new();
            for member in children(body) {
                if member.kind() == "decorator" {
                    pending.extend(decorator(member, source));
                    continue;
                }
                if member.kind() != "method_definition" {
                    pending.clear();
                    continue;
                }
                let mut decos = std::mem::take(&mut pending);
                decos.extend(own_decorators(member, source));
                let Some((verb_deco, verb)) = VERBS
                    .iter()
                    .find_map(|(name, verb)| find(&decos, name).map(|d| (d, *verb)))
                else {
                    continue;
                };
                let path = braces(&join(&[&prefix, &base, &path_arg(verb_deco)]));
                let mut declared = Declared {
                    method: verb.to_string(),
                    path: path.clone(),
                    ..Declared::default()
                };
                let mut fields: Vec<(String, J)> = Vec::new();
                let mut multipart = false;
                for param in member
                    .child_by_field_name("parameters")
                    .map(children)
                    .unwrap_or_default()
                {
                    let pdecos = own_decorators(param, source);
                    let pname = param
                        .child_by_field_name("pattern")
                        .map(|p| text(p, source).to_string())
                        .unwrap_or_default();
                    let ty = type_text(param.child_by_field_name("type"), source);
                    if let Some(d) = find(&pdecos, "Query") {
                        match d.first() {
                            Some(name) => declared.query.push(Param {
                                name: name.to_string(),
                                ..Param::default()
                            }),
                            // `@Query() filter: ListOrdersQuery`: its fields.
                            None => {
                                if let J::Obj(entries) =
                                    sample(&ty, "", &[], &world, 0, &mut Vec::new())
                                {
                                    for (name, value) in entries {
                                        // A string's sample is its own name, which is no
                                        // help in a query: only numbers and flags are kept.
                                        let sample = match value {
                                            J::Str(_) => String::new(),
                                            other => other.text().unwrap_or_default(),
                                        };
                                        declared.query.push(Param {
                                            name,
                                            sample,
                                            required: false,
                                        });
                                    }
                                }
                            }
                        }
                    } else if let Some(d) = find(&pdecos, "Headers") {
                        if let Some(name) = d.first() {
                            declared.headers.push(Param {
                                name: name.to_string(),
                                ..Param::default()
                            });
                        }
                    } else if let Some(d) = find(&pdecos, "Body") {
                        match d.first() {
                            Some(field) => fields.push((
                                field.to_string(),
                                sample(&ty, field, &[], &world, 0, &mut Vec::new()),
                            )),
                            None => {
                                let unwrapped = ty.trim_end_matches("[]");
                                declared.body_type = Some(
                                    unwrapped.split('<').next().unwrap_or(unwrapped).to_string(),
                                );
                                declared.body =
                                    Some(sample(&ty, "", &[], &world, 0, &mut Vec::new()));
                            }
                        }
                    } else if let Some(d) =
                        find(&pdecos, "UploadedFile").or_else(|| find(&pdecos, "UploadedFiles"))
                    {
                        multipart = true;
                        fields.push((
                            d.first().unwrap_or(&pname).to_string(),
                            J::Str(String::new()),
                        ));
                    }
                }
                if multipart {
                    declared.consumes = Some("multipart/form-data".into());
                }
                if declared.body.is_none() && !fields.is_empty() {
                    declared.body = Some(J::Obj(fields));
                }
                let name = member
                    .child_by_field_name("name")
                    .map(|n| words_of(text(n, source)))
                    .unwrap_or_default();
                let mut hint: Vec<String> = Vec::new();
                if let Some(t) = &declared.body_type {
                    hint.push(t.clone());
                }
                let names: Vec<&str> = declared.query.iter().map(|p| p.name.as_str()).collect();
                if !names.is_empty() {
                    hint.push(names.join(", "));
                }
                let call = build(&declared, &name, Choices::default());
                out.push(Endpoint {
                    id: format!("{}#{}", file.path, out.len()),
                    key: key_of(&call),
                    source: format!("code:{}", file.path),
                    group: class.clone(),
                    file: Some(file.path.clone()),
                    line: Some(member.start_position().row as u32 + 1),
                    method: verb.to_string(),
                    path,
                    hint: hint.join(" · "),
                    call,
                    status: Status::New,
                    declared: Some(declared),
                    folder: None,
                });
            }
        }
        let count = out.len() - start;
        if count > 0 {
            sources.push(Source {
                id: format!("code:{}", file.path),
                kind: SourceKind::Nest,
                label: file
                    .path
                    .rsplit('/')
                    .next()
                    .unwrap_or(&file.path)
                    .to_string(),
                file: Some(file.path.clone()),
                count,
                error: None,
                in_repository: true,
            });
        }
    }
    (out, sources, base_url)
}

#[cfg(test)]
mod tests {
    use super::*;

    const CONTROLLER: &str = r#"
import { Body, Controller, Get, Headers, Param, Post, Query, UploadedFile } from '@nestjs/common';

@Controller('orders')
export class OrdersController {
  constructor(private readonly orders: OrdersService) {}

  @Get(':id')
  findOne(@Param('id') id: string, @Headers('x-tenant') tenant: string) {}

  @Get()
  findAll(@Query() query: ListQuery) {}

  @Post()
  @HttpCode(201)
  create(@Body() dto: CreateOrderDto): Promise<Order> { return null; }

  @Post(':id/receipt')
  upload(@Param('id') id: string, @UploadedFile() file: Express.Multer.File) {}

  helper() {}
}
"#;

    const DTO: &str = r#"
export enum Status { New = 'new', Paid = 'paid' }

export class LineDto {
  @IsString() sku: string;
  @IsPositive() qty: number;
}

export class CreateOrderDto {
  @ApiProperty({ example: 'AB-1' })
  @IsString()
  sku: string;

  @Min(2)
  quantity: number;

  @IsEmail()
  contact: string;

  @IsEnum(Status)
  status: Status;

  @ValidateNested({ each: true })
  @Type(() => LineDto)
  lines: LineDto[];

  @IsOptional()
  note?: string | null;

  due: Date;
  meta: Record<string, number>;
  static VERSION = 1;
}

export interface ListQuery {
  page: number;
  q?: string;
}
"#;

    const MAIN: &str = r#"
async function bootstrap() {
  const app = await NestFactory.create(AppModule);
  app.setGlobalPrefix('api/v1');
  await app.listen(process.env.PORT ?? 3001);
}
"#;

    fn files() -> Vec<(String, String)> {
        vec![
            ("src/orders/orders.controller.ts".into(), CONTROLLER.into()),
            ("src/orders/dto.ts".into(), DTO.into()),
            ("src/main.ts".into(), MAIN.into()),
        ]
    }

    #[test]
    fn nest_controllers_give_their_endpoints() {
        let (endpoints, sources, base) = read(&files());
        assert_eq!(base.as_deref(), Some("http://localhost:3001"));
        let got: Vec<String> = endpoints
            .iter()
            .map(|e| format!("{} {} | {}", e.method, e.call.url, e.call.name))
            .collect();
        assert_eq!(
            got,
            [
                "GET {{baseUrl}}/api/v1/orders/{{id}} | Find one",
                "GET {{baseUrl}}/api/v1/orders?page=0&q | Find all",
                "POST {{baseUrl}}/api/v1/orders | Create",
                "POST {{baseUrl}}/api/v1/orders/{{id}}/receipt | Upload",
            ]
        );
        assert_eq!(endpoints[0].call.headers[0].name, "x-tenant");
        assert_eq!(endpoints[0].key, "GET /api/v1/orders/{}");
        assert_eq!(endpoints[2].hint, "CreateOrderDto");
        assert_eq!(
            endpoints[3].call.body.as_ref().unwrap().kind,
            crate::calls::model::BodyType::Multipart
        );
        assert_eq!(sources.len(), 1);
        assert_eq!(sources[0].kind, SourceKind::Nest);
        assert_eq!(sources[0].count, 4);
    }

    #[test]
    fn a_dto_is_an_example_shaped_by_its_decorators() {
        let (endpoints, _, _) = read(&files());
        assert_eq!(
            endpoints[2].call.body.as_ref().unwrap().text.as_deref(),
            Some(
                r#"{
  "sku": "AB-1",
  "quantity": 2,
  "contact": "mira@example.com",
  "status": "new",
  "lines": [
    {
      "sku": "sku",
      "qty": 1
    }
  ],
  "note": "note",
  "due": "2026-01-31T09:30:00Z",
  "meta": {
    "key": 0
  }
}"#
            )
        );
    }
}
