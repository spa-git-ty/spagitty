// SPDX-License-Identifier: GPL-3.0-or-later

//! Reading the working tree for Import (FEAT-162, proposal §8.2 and §8.3).
//!
//! The files are the ones git sees — tracked, and untracked but not
//! ignored — so what the person is writing now is found and a build folder
//! is not. Settings' skip globs are left out too, and the scan stops at
//! [`CAP`] files. Each file goes to the reader its name and its first bytes
//! say it is for. Nothing is built, started or written.
//!
//! [`read_file`] is the same readers for one file chosen from outside, and
//! [`read_text`] for text pasted in.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::Value;

use super::super::model::{Collection, Environment, Variable};
use super::{
    bruno, curl, har, http_file, insomnia, java, key_of, nest, openapi, path_of, postman,
    wsdl_source, Endpoint, Found, Source, SourceKind, Status,
};
use crate::{shell, Error, Result};
use wsdl_source::SoapChoices;

/// The most files one scan reads.
pub const CAP: usize = 20_000;
/// A file bigger than this is not read: it is not a controller.
const LARGEST: u64 = 4 * 1024 * 1024;

/// What Settings › Calls says to read.
#[derive(Debug, Clone, Default)]
pub struct Options {
    /// `springMvc`, `springWebflux`, `jaxRs`, `nestjs`.
    pub frameworks: Vec<String>,
    /// `postman`, `insomnia`, `bruno`, `openapi`, `http`, `wsdl`, `har`.
    pub look_for: Vec<String>,
    /// Globs, separated by commas.
    pub skip: String,
    /// The calls folder, from the working tree: its collections are not
    /// sources.
    pub calls_folder: String,
    /// What a SOAP call starts with (FEAT-163).
    pub soap: SoapChoices,
}

/// `**/test/**`, `*.json`, `build/` as a test for a path with `/`.
pub fn glob(pattern: &str, path: &str) -> bool {
    let pattern = pattern.trim().trim_start_matches("./");
    if pattern.is_empty() {
        return false;
    }
    let pattern = if pattern.ends_with('/') {
        format!("{pattern}**")
    } else {
        pattern.to_string()
    };
    let mut regex = String::from("^");
    let mut chars = pattern.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '*' if chars.peek() == Some(&'*') => {
                chars.next();
                if chars.peek() == Some(&'/') {
                    chars.next();
                    regex.push_str("(?:.*/)?");
                } else {
                    regex.push_str(".*");
                }
            }
            '*' => regex.push_str("[^/]*"),
            '?' => regex.push_str("[^/]"),
            c => regex.push_str(&regex::escape(&c.to_string())),
        }
    }
    regex.push('$');
    // A pattern with no `/` matches a name anywhere, as .gitignore does.
    let anywhere = !pattern.contains('/');
    regex::Regex::new(&regex)
        .map(|re| {
            re.is_match(path)
                || (anywhere
                    && path
                        .rsplit('/')
                        .next()
                        .is_some_and(|name| re.is_match(name)))
        })
        .unwrap_or(false)
}

fn skipped(path: &str, skip: &str) -> bool {
    skip.split(',').any(|pattern| glob(pattern, path))
}

/// Which reader a file is for, from its name and, for JSON and YAML, the
/// start of it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Java,
    TypeScript,
    SpringConfig,
    Json,
    Yaml,
    Http,
    Har,
    Wsdl,
    Bru,
    BrunoRoot,
}

fn kind_of(path: &str) -> Option<Kind> {
    let name = path.rsplit('/').next().unwrap_or(path).to_ascii_lowercase();
    if name.ends_with(".java") {
        return Some(Kind::Java);
    }
    if name.ends_with(".ts")
        && !name.ends_with(".d.ts")
        && !name.ends_with(".spec.ts")
        && !name.ends_with(".test.ts")
    {
        return Some(Kind::TypeScript);
    }
    if matches!(
        name.as_str(),
        "application.yml" | "application.yaml" | "application.properties"
    ) {
        return Some(Kind::SpringConfig);
    }
    if name == "bruno.json" {
        return Some(Kind::BrunoRoot);
    }
    if name.ends_with(".bru") {
        return Some(Kind::Bru);
    }
    if name.ends_with(".http") || name.ends_with(".rest") {
        return Some(Kind::Http);
    }
    if name.ends_with(".har") {
        return Some(Kind::Har);
    }
    if name.ends_with(".wsdl") {
        return Some(Kind::Wsdl);
    }
    if name.ends_with(".json")
        && !matches!(
            name.as_str(),
            "package.json" | "package-lock.json" | "tsconfig.json"
        )
    {
        return Some(Kind::Json);
    }
    if name.ends_with(".yaml") || name.ends_with(".yml") {
        return Some(Kind::Yaml);
    }
    None
}

/// One file's endpoints, by what it turns out to be: added to `found`.
fn read_document(
    path: &str,
    text: &str,
    look_for: &[String],
    found: &mut Found,
    in_repository: bool,
) -> bool {
    let wants = |name: &str| look_for.iter().any(|l| l == name);
    let add = |kind: SourceKind, endpoints: Vec<Endpoint>, found: &mut Found| {
        found.sources.push(Source {
            id: format!("file:{path}"),
            kind,
            label: path.rsplit('/').next().unwrap_or(path).to_string(),
            file: Some(path.to_string()),
            count: endpoints.len(),
            error: None,
            in_repository,
        });
        found.endpoints.extend(endpoints);
    };
    if wants("openapi") && (text.contains("openapi") || text.contains("swagger")) {
        if let Some((endpoints, base)) = openapi::read(path, text) {
            if found.base_url.is_none() {
                found.base_url = base;
            }
            add(SourceKind::OpenApi, endpoints, found);
            return true;
        }
    }
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return false;
    };
    if wants("postman") && postman::is_collection(&value) {
        let (endpoints, _vars, scripted) = postman::read(path, &value);
        found.scripted += scripted;
        add(SourceKind::Postman, endpoints, found);
        return true;
    }
    if wants("postman") && postman::is_environment(&value) {
        let (env, _secrets) = postman::environment(&value);
        found.environments.push(env);
        return true;
    }
    if wants("insomnia") && insomnia::is_export(&value) {
        let (endpoints, envs) = insomnia::read(path, &value);
        found.environments.extend(envs);
        add(SourceKind::Insomnia, endpoints, found);
        return true;
    }
    if wants("har") && har::is_har(&value) {
        add(SourceKind::Har, har::read(path, &value), found);
        return true;
    }
    false
}

/// Everything in the working tree that Import can read, as `options` says.
/// `progress` is told how many files have been looked at, every hundred.
pub fn scan(
    repo: &Path,
    options: &Options,
    cancel: &AtomicBool,
    mut progress: impl FnMut(usize),
) -> Result<Found> {
    let listed = shell::working_files(repo)?;
    let folder = options.calls_folder.trim_matches('/');
    let mut paths: Vec<&str> = listed
        .split('\0')
        .filter(|p| !p.is_empty())
        .filter(|p| folder.is_empty() || !p.starts_with(&format!("{folder}/")))
        .filter(|p| !skipped(p, &options.skip))
        .collect();
    paths.sort_unstable();
    paths.dedup();
    let mut found = Found::default();
    if paths.len() > CAP {
        paths.truncate(CAP);
        found.capped = true;
    }
    let wants = |name: &str| options.frameworks.iter().any(|f| f == name);
    let java_on = wants("springMvc") || wants("springWebflux") || wants("jaxRs");
    let nest_on = wants("nestjs");

    let mut java_files = Vec::new();
    let mut ts_files = Vec::new();
    let mut config = Vec::new();
    let mut bruno_roots: Vec<String> = Vec::new();
    let mut bru_files: Vec<String> = Vec::new();
    for (count, path) in paths.iter().enumerate() {
        if cancel.load(Ordering::Relaxed) {
            return Err(Error::Calls("the scan was stopped".into()));
        }
        if count % 100 == 0 {
            progress(count);
        }
        let Some(kind) = kind_of(path) else { continue };
        let full = repo.join(path);
        if std::fs::metadata(&full)
            .map(|m| m.len() > LARGEST)
            .unwrap_or(true)
        {
            continue;
        }
        found.files_read += 1;
        match kind {
            Kind::BrunoRoot => {
                if options.look_for.iter().any(|l| l == "bruno") {
                    bruno_roots.push(
                        path.rsplit_once('/')
                            .map(|(d, _)| d.to_string())
                            .unwrap_or_default(),
                    );
                }
                continue;
            }
            Kind::Bru => {
                bru_files.push(path.to_string());
                continue;
            }
            _ => {}
        }
        let Ok(text) = std::fs::read_to_string(&full) else {
            continue;
        };
        match kind {
            Kind::Java if java_on && java::interesting(&text) => {
                java_files.push((path.to_string(), text))
            }
            Kind::TypeScript if nest_on => {
                if text.contains("class ")
                    || text.contains("interface ")
                    || text.contains("enum ")
                    || text.contains("NestFactory")
                {
                    ts_files.push((path.to_string(), text));
                }
            }
            Kind::SpringConfig if java_on => config.push((path.to_string(), text)),
            Kind::Http if options.look_for.iter().any(|l| l == "http") => {
                let (endpoints, _vars) = http_file::read(path, &text);
                if !endpoints.is_empty() {
                    found.sources.push(Source {
                        id: format!("file:{path}"),
                        kind: SourceKind::Http,
                        label: path.rsplit('/').next().unwrap_or(path).to_string(),
                        file: Some(path.to_string()),
                        count: endpoints.len(),
                        error: None,
                        in_repository: true,
                    });
                    found.endpoints.extend(endpoints);
                }
            }
            Kind::Wsdl if options.look_for.iter().any(|l| l == "wsdl") => {
                let load = |file: &str| std::fs::read_to_string(repo.join(file)).ok();
                match wsdl_source::read(path, &text, &load, &options.soap) {
                    Ok((endpoints, server)) => {
                        if found.base_url.is_none() {
                            found.base_url = server;
                        }
                        found.sources.push(Source {
                            id: format!("file:{path}"),
                            kind: SourceKind::Wsdl,
                            label: path.rsplit('/').next().unwrap_or(path).to_string(),
                            file: Some(path.to_string()),
                            count: endpoints.len(),
                            error: None,
                            in_repository: true,
                        });
                        found.endpoints.extend(endpoints);
                    }
                    Err(error) => found.sources.push(Source {
                        id: format!("file:{path}"),
                        kind: SourceKind::Wsdl,
                        label: path.rsplit('/').next().unwrap_or(path).to_string(),
                        file: Some(path.to_string()),
                        count: 0,
                        error: Some(error),
                        in_repository: true,
                    }),
                }
            }
            Kind::Json | Kind::Har | Kind::Yaml => {
                // Only the start is looked at before a JSON file is parsed:
                // most JSON in a repository is not a collection.
                let head: String = text.chars().take(4096).collect();
                let maybe = head.contains("openapi")
                    || head.contains("swagger")
                    || head.contains("postman")
                    || head.contains("_postman")
                    || head.contains("\"_type\"")
                    || head.contains("\"log\"");
                if maybe {
                    read_document(path, &text, &options.look_for, &mut found, true);
                }
            }
            _ => {}
        }
    }

    if !java_files.is_empty() {
        let (endpoints, sources) = java::read(&java_files, &options.frameworks, &options.soap);
        if !endpoints.is_empty() {
            found.base_url = java::base_url(&config).or(found.base_url.take());
        }
        found.endpoints.extend(endpoints);
        found.sources.extend(sources);
    }
    if !ts_files.is_empty() {
        let (endpoints, sources, base) = nest::read(&ts_files);
        if !endpoints.is_empty() && found.base_url.is_none() {
            found.base_url = base;
        }
        found.endpoints.extend(endpoints);
        found.sources.extend(sources);
    }
    for root in bruno_roots {
        let prefix = if root.is_empty() {
            String::new()
        } else {
            format!("{root}/")
        };
        let files: Vec<(String, String)> = bru_files
            .iter()
            .filter(|f| f.starts_with(&prefix))
            .filter_map(|f| {
                std::fs::read_to_string(repo.join(f))
                    .ok()
                    .map(|text| (f[prefix.len()..].to_string(), text))
            })
            .collect();
        let (endpoints, envs) = bruno::read(&root, &files);
        found.environments.extend(envs);
        found.sources.push(Source {
            id: format!("file:{root}/bruno.json"),
            kind: SourceKind::Bruno,
            label: root
                .rsplit('/')
                .next()
                .filter(|r| !r.is_empty())
                .unwrap_or("bruno")
                .to_string(),
            file: Some(format!("{prefix}bruno.json")),
            count: endpoints.len(),
            error: None,
            in_repository: true,
        });
        found.endpoints.extend(endpoints);
    }
    prefer_wsdl(&mut found);
    progress(paths.len());
    Ok(found)
}

/// An operation a WSDL in the repository has is taken from the WSDL, not
/// from the code that implements it (§8.2): the WSDL is what clients see.
fn prefer_wsdl(found: &mut Found) {
    let from_wsdl: std::collections::HashSet<String> = found
        .endpoints
        .iter()
        .filter(|e| e.method == "SOAP" && !e.source.starts_with("code:"))
        .filter_map(|e| e.call.operation.clone())
        .collect();
    if from_wsdl.is_empty() {
        return;
    }
    found.endpoints.retain(|e| {
        !(e.source.starts_with("code:")
            && e.method == "SOAP"
            && e.call
                .operation
                .as_ref()
                .is_some_and(|op| from_wsdl.contains(op)))
    });
    for source in &mut found.sources {
        if source.id.starts_with("code:") {
            source.count = found
                .endpoints
                .iter()
                .filter(|e| e.source == source.id)
                .count();
        }
    }
    found
        .sources
        .retain(|s| s.count > 0 || s.error.is_some() || !s.id.starts_with("code:"));
}

/// A Spagitty collection, as Share writes one to a single file.
fn spagitty(path: &str, value: &Value, found: &mut Found) -> bool {
    if value["v"].as_u64() != Some(1)
        || !value["items"].is_array()
        || value["name"].as_str().is_none()
    {
        return false;
    }
    let Ok(collection) = serde_json::from_value::<Collection>(value.clone()) else {
        return false;
    };
    let mut endpoints = Vec::new();
    for (folders, call) in super::super::model::calls(&collection.items) {
        let folder = (!folders.is_empty()).then(|| folders.join(" / "));
        endpoints.push(Endpoint {
            id: format!("{path}#{}", endpoints.len()),
            key: key_of(call),
            source: format!("file:{path}"),
            group: folder.clone().unwrap_or_else(|| collection.name.clone()),
            file: Some(path.to_string()),
            line: None,
            method: call.method().to_string(),
            path: path_of(&call.url),
            hint: String::new(),
            call: call.clone(),
            status: Status::New,
            declared: None,
            folder,
        });
    }
    found.sources.push(Source {
        id: format!("file:{path}"),
        kind: SourceKind::Spagitty,
        label: collection.name.clone(),
        file: Some(path.to_string()),
        count: endpoints.len(),
        error: None,
        in_repository: false,
    });
    found.endpoints.extend(endpoints);
    true
}

/// One file chosen from outside the repository, whatever it is.
pub fn read_file(path: &Path, soap: &SoapChoices) -> Result<Found> {
    let name = path.to_string_lossy().replace('\\', "/");
    // A zip is not text: it is read before anything else is.
    if name.to_ascii_lowercase().ends_with(".zip") {
        let bytes = std::fs::read(path)
            .map_err(|e| Error::Calls(format!("{name} could not be read: {e}")))?;
        let (collection, environments) = super::super::share::unzip(&bytes)?;
        let value = serde_json::to_value(&collection).map_err(|e| Error::Calls(e.to_string()))?;
        let mut found = Found::default();
        spagitty(&name, &value, &mut found);
        found.environments = environments;
        return Ok(found);
    }
    let text = std::fs::read_to_string(path)
        .map_err(|e| Error::Calls(format!("{name} could not be read: {e}")))?;
    let lower = name.to_ascii_lowercase();
    let mut found = Found::default();
    if lower.ends_with(".http") || lower.ends_with(".rest") {
        let (endpoints, _) = http_file::read(&name, &text);
        found.sources.push(Source {
            id: format!("file:{name}"),
            kind: SourceKind::Http,
            label: name.rsplit('/').next().unwrap_or(&name).to_string(),
            file: Some(name.clone()),
            count: endpoints.len(),
            error: None,
            in_repository: false,
        });
        found.endpoints = endpoints;
        return Ok(found);
    }
    if lower.ends_with(".wsdl") {
        let load = |file: &str| std::fs::read_to_string(file).ok();
        let (endpoints, server) =
            wsdl_source::read(&name, &text, &load, soap).map_err(Error::Calls)?;
        found.sources.push(Source {
            id: format!("file:{name}"),
            kind: SourceKind::Wsdl,
            label: name.rsplit('/').next().unwrap_or(&name).to_string(),
            file: Some(name.clone()),
            count: endpoints.len(),
            error: None,
            in_repository: false,
        });
        found.endpoints = endpoints;
        found.base_url = server;
        return Ok(found);
    }
    if lower.ends_with(".bru") {
        let call = bruno::request(&text)
            .ok_or_else(|| Error::Calls(format!("{name} is not a Bruno request")))?;
        return Ok(single(&name, SourceKind::Bruno, call));
    }
    if let Ok(value) = serde_json::from_str::<Value>(&text) {
        if spagitty(&name, &value, &mut found) {
            return Ok(found);
        }
    }
    let all = ["postman", "insomnia", "openapi", "har"]
        .map(String::from)
        .to_vec();
    if read_document(&name, &text, &all, &mut found, false) {
        for source in &mut found.sources {
            source.in_repository = false;
        }
        return Ok(found);
    }
    if text.trim_start().starts_with("curl") {
        return read_text(&text);
    }
    Err(Error::Calls(format!(
        "{name} is not a collection Calls can read: Postman, Insomnia, Bruno, OpenAPI, Swagger, .http, HAR, cURL or a Spagitty collection"
    )))
}

fn single(name: &str, kind: SourceKind, call: super::super::model::Call) -> Found {
    let endpoint = Endpoint {
        id: format!("{name}#0"),
        key: key_of(&call),
        source: format!("file:{name}"),
        group: "Imported".into(),
        file: None,
        line: None,
        method: call.method().to_string(),
        path: path_of(&call.url),
        hint: String::new(),
        call,
        status: Status::New,
        declared: None,
        folder: None,
    };
    Found {
        sources: vec![Source {
            id: format!("file:{name}"),
            kind,
            label: name.rsplit('/').next().unwrap_or(name).to_string(),
            file: None,
            count: 1,
            error: None,
            in_repository: false,
        }],
        endpoints: vec![endpoint],
        ..Found::default()
    }
}

/// Text pasted in: a cURL command.
pub fn read_text(text: &str) -> Result<Found> {
    let call = curl::parse(text).map_err(Error::Calls)?;
    Ok(single("Pasted cURL", SourceKind::Curl, call))
}

/// An environment, and its secrets' values for the keychain.
pub type Imported = (Environment, Vec<(String, String)>);

/// A Postman environment chosen from outside, with its secrets for the
/// keychain.
pub fn environment_file(path: &Path) -> Result<Option<Imported>> {
    let text = std::fs::read_to_string(path).map_err(|e| Error::Calls(e.to_string()))?;
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return Ok(None);
    };
    Ok(postman::is_environment(&value).then(|| postman::environment(&value)))
}

/// A collection's own variables from a file that has some: Postman's
/// `variable`, the `.http` file's `@name = value` lines.
pub fn variables_of(path: &str, text: &str) -> Vec<Variable> {
    if path.ends_with(".http") || path.ends_with(".rest") {
        return http_file::read(path, text).1;
    }
    serde_json::from_str::<Value>(text)
        .ok()
        .filter(postman::is_collection)
        .map(|value| postman::read(path, &value).1)
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::Fixture;

    #[test]
    fn globs_skip_what_settings_say() {
        assert!(glob("**/test/**", "src/test/java/A.java"));
        assert!(glob("**/generated/**", "build/generated/x.java"));
        assert!(!glob("**/test/**", "src/main/java/Test.java"));
        assert!(glob("*.min.js", "web/app.min.js"));
        assert!(glob("build/", "build/a/b"));
        assert!(!glob("", "x"));
        assert!(skipped("a/test/b.java", "**/test/**, **/generated/**"));
    }

    fn options() -> Options {
        Options {
            frameworks: ["springMvc", "springWebflux", "jaxRs", "nestjs"]
                .map(String::from)
                .to_vec(),
            look_for: ["postman", "insomnia", "bruno", "openapi", "http", "har"]
                .map(String::from)
                .to_vec(),
            skip: "**/test/**".into(),
            calls_folder: ".spagitty/calls".into(),
            soap: SoapChoices::default(),
        }
    }

    #[test]
    fn a_scan_reads_the_working_tree_and_skips_what_it_should() {
        let fixture = Fixture::empty();
        fixture.write(
            "svc/src/main/java/OrderController.java",
            "@RestController @RequestMapping(\"/orders\") class OrderController { @GetMapping(\"/{id}\") Order get(@PathVariable String id) { return null; } }",
        );
        fixture.write(
            "svc/src/main/resources/application.properties",
            "server.port=8085\n",
        );
        fixture.write(
            "svc/src/test/java/FakeController.java",
            "@RestController class FakeController { @GetMapping(\"/fake\") void f() {} }",
        );
        fixture.write("api/orders.http", "GET {{baseUrl}}/health\n");
        fixture.write(
            "postman/shop.postman_collection.json",
            r#"{"info":{"name":"Shop","schema":"https://schema.getpostman.com/json/collection/v2.1.0/collection.json"},"item":[{"name":"Ping","request":{"method":"GET","url":"{{baseUrl}}/ping"}}]}"#,
        );
        fixture.write("package.json", r#"{"name":"postman-like"}"#);
        fixture.write(
            ".spagitty/calls/orders.json",
            r#"{"v":1,"name":"orders","items":[]}"#,
        );
        fixture.write(".gitignore", "build/\n");
        fixture.write(
            "build/Gen.java",
            "@RestController class Gen { @GetMapping(\"/gen\") void g() {} }",
        );

        let found = scan(fixture.path(), &options(), &AtomicBool::new(false), |_| {}).unwrap();
        let mut keys: Vec<&str> = found.endpoints.iter().map(|e| e.key.as_str()).collect();
        keys.sort_unstable();
        assert_eq!(keys, ["GET /health", "GET /orders/{}", "GET /ping"]);
        assert_eq!(found.base_url.as_deref(), Some("http://localhost:8085"));
        let mut kinds: Vec<SourceKind> = found.sources.iter().map(|s| s.kind).collect();
        kinds.sort_by_key(|k| format!("{k:?}"));
        assert_eq!(
            kinds,
            [SourceKind::Http, SourceKind::Postman, SourceKind::Spring]
        );

        let stopped = AtomicBool::new(true);
        assert!(scan(fixture.path(), &options(), &stopped, |_| {}).is_err());
    }

    #[test]
    fn a_wsdl_in_the_repository_is_preferred_to_the_code_behind_it() {
        use crate::calls::soap::wsdl::tests::{COMMON, INVOICES};
        let fixture = Fixture::empty();
        fixture.write("wsdl/invoices.wsdl", INVOICES);
        fixture.write("wsdl/common.xsd", COMMON);
        fixture.write(
            "src/Invoices.java",
            "@WebService(serviceName = \"BillingService\") public class Invoices { public Invoice GetInvoice(String id) { return null; } public void Other() {} }",
        );
        let mut opts = options();
        opts.frameworks.push("jaxWs".into());
        opts.look_for.push("wsdl".into());
        let found = scan(fixture.path(), &opts, &AtomicBool::new(false), |_| {}).unwrap();
        let mut keys: Vec<(&str, &str)> = found
            .endpoints
            .iter()
            .map(|e| (e.key.as_str(), e.source.as_str()))
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                ("SOAP BillingService#GetInvoice", "file:wsdl/invoices.wsdl"),
                ("SOAP BillingService#Other", "code:src/Invoices.java"),
            ]
        );
        // The schema it includes was read: the line element has its fields.
        let envelope = found
            .endpoints
            .iter()
            .find(|e| e.source.starts_with("file:"))
            .unwrap()
            .call
            .envelope
            .clone()
            .unwrap();
        assert!(envelope.contains("<inv:sku>sku</inv:sku>"));
        assert_eq!(
            found.base_url.as_deref(),
            Some("https://billing.example.com")
        );
    }

    #[test]
    fn a_file_from_outside_is_read_by_what_it_is() {
        let dir = tempfile::tempdir().unwrap();
        let write = |name: &str, text: &str| {
            let path = dir.path().join(name);
            std::fs::write(&path, text).unwrap();
            path
        };
        let collection = write(
            "shared.json",
            r#"{"v":1,"name":"orders-api","items":[{"folder":"Auth","items":[{"id":"c_1","name":"Log in","kind":"rest","method":"POST","url":"{{baseUrl}}/auth/login"}]}]}"#,
        );
        let found = read_file(&collection, &SoapChoices::default()).unwrap();
        assert_eq!(found.sources[0].kind, SourceKind::Spagitty);
        assert_eq!(found.endpoints[0].folder.as_deref(), Some("Auth"));

        let curl = write("call.txt", "curl https://h/x -d '{\"a\":1}'");
        assert_eq!(
            read_file(&curl, &SoapChoices::default()).unwrap().endpoints[0].method,
            "POST"
        );

        let mut env = crate::calls::model::Environment::new("staging", Default::default());
        env.vars
            .push(crate::calls::model::Variable::plain("baseUrl", "https://s"));
        let shared: crate::calls::model::Collection =
            serde_json::from_str(&std::fs::read_to_string(&collection).unwrap()).unwrap();
        let zip =
            crate::calls::share::zipped(&crate::calls::share::bundle(&shared, &[env], &[], None))
                .unwrap();
        let zip_path = dir.path().join("orders-api.zip");
        std::fs::write(&zip_path, zip).unwrap();
        let unzipped = read_file(&zip_path, &SoapChoices::default()).unwrap();
        assert_eq!(unzipped.sources[0].kind, SourceKind::Spagitty);
        assert_eq!(unzipped.endpoints.len(), 1);
        assert_eq!(unzipped.environments[0].name, "staging");

        let junk = write("notes.txt", "hello");
        assert!(read_file(&junk, &SoapChoices::default()).is_err());
        assert!(read_text("not curl").is_err());
    }
}
