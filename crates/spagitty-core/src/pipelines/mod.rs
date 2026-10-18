// SPDX-License-Identifier: GPL-3.0-or-later

//! A local preview never executes a command or retrieves an include.
mod expressions;
mod yaml;

use crate::{Error, Result};
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::{Arc, OnceLock},
};
use yaml::{Data, Node};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Simulation {
    pub event: String,
    pub reference: String,
    pub base: String,
    pub default_branch: String,
    pub changed: Vec<String>,
    pub changes_known: bool,
}
impl Simulation {
    fn github_event(&self) -> &str {
        match self.event.as_str() {
            "manual" => "workflow_dispatch",
            "tag" => "push",
            e => e,
        }
    }
    fn gitlab_event(&self) -> &str {
        match self.event.as_str() {
            "manual" => "web",
            "pull_request" => "merge_request_event",
            "tag" => "push",
            e => e,
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Problem {
    pub file: String,
    pub line: usize,
    pub message: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Step {
    pub name: String,
    pub command: String,
    pub condition: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Job {
    pub id: String,
    pub name: String,
    pub stage: String,
    pub needs: Vec<String>,
    pub condition: String,
    pub matrix: Vec<Value>,
    pub environment: String,
    pub services: Vec<String>,
    pub steps: Vec<Step>,
    pub runs_on: Vec<String>,
    pub timeout: String,
    pub secrets: Vec<String>,
    pub file: String,
    pub line: usize,
    pub state: String,
    pub reason: String,
    pub depth: usize,
    /// GitLab: `needs` stands for the stage order, not a `needs:` key. The
    /// graph draws stage columns for these rather than every edge.
    #[serde(default)]
    pub implicit: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Workflow {
    pub file: String,
    pub name: String,
    pub kind: String,
    pub triggers: Vec<String>,
    pub stages: Vec<String>,
    pub jobs: Vec<Job>,
    pub problems: Vec<Problem>,
    pub source_files: Vec<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub workflows: Vec<Workflow>,
    pub changed: Vec<String>,
    pub changes_warning: Option<String>,
    pub simulation: Simulation,
}
fn text(n: &Node, key: &str) -> String {
    n.get(key).map(Node::text).unwrap_or_default()
}
fn strings(n: &Node, key: &str) -> Vec<String> {
    n.get(key).map(Node::strings).unwrap_or_default()
}
fn config(message: impl Into<String>) -> Error {
    Error::Workflow(message.into())
}
fn problem(n: &Node, message: impl Into<String>) -> Problem {
    Problem {
        file: n.file.clone(),
        line: n.line,
        message: message.into(),
    }
}

pub fn source(root: &Path, file: &str) -> Result<String> {
    let path = root.join(file);
    let root = root.canonicalize()?;
    let path = path.canonicalize()?;
    if !path.starts_with(&root) || !path.is_file() {
        return Err(config(
            "Workflow includes must stay inside this repository.",
        ));
    }
    if std::fs::metadata(&path)?.len() > 1024 * 1024 {
        return Err(config("Workflow files must be smaller than 1 MiB."));
    }
    Ok(std::fs::read_to_string(path)?)
}
fn read(root: &Path, file: &str) -> Result<yaml::Parsed> {
    type Cache = BTreeMap<std::path::PathBuf, (u64, yaml::Parsed)>;
    static CACHE: OnceLock<std::sync::Mutex<Cache>> = OnceLock::new();
    let source = source(root, file)?;
    let hash = crate::hooks::generate::hash(source.as_bytes());
    let path = root.join(file);
    let mut cache = CACHE
        .get_or_init(Default::default)
        .lock()
        .map_err(|_| config("The workflow cache could not be read."))?;
    if let Some((old, parsed)) = cache.get(&path) {
        if *old == hash {
            return Ok(parsed.clone());
        }
    }
    let parsed = yaml::parse_all(&source, file).map_err(config)?;
    if cache.len() >= 100 {
        cache.clear();
    }
    cache.insert(path, (hash, parsed.clone()));
    Ok(parsed)
}
type ReadSource<'a> = &'a dyn Fn(&str) -> Result<Node>;
type ReadParsed<'a> = &'a dyn Fn(&str) -> Result<yaml::Parsed>;
fn include(
    read_source: ReadSource<'_>,
    file: &str,
    stack: &mut Vec<String>,
    files: &mut BTreeSet<String>,
    opaque: &mut bool,
) -> Result<Node> {
    if stack.len() > 20 || files.len() > 100 {
        return Err(config("Local includes exceed the preview limit."));
    }
    if stack.iter().any(|p| p == file) {
        return Err(config(format!("Local include cycle at {file}.")));
    }
    stack.push(file.into());
    files.insert(file.into());
    let local = read_source(file)?;
    let mut merged = Node {
        data: Arc::new(Data::Map(BTreeMap::new())),
        ..local.clone()
    };
    if let Some(includes) = local.get("include") {
        let entries = if matches!(&*includes.data, Data::List(_)) {
            includes.list().to_vec()
        } else {
            vec![includes.clone()]
        };
        for entry in entries {
            let name = if entry.map().is_some() {
                text(&entry, "local")
            } else {
                entry.text()
            };
            if name.is_empty()
                || name.starts_with("https:")
                || name.contains('*')
                || entry.get("rules").is_some()
            {
                *opaque = true;
                continue;
            }
            let name = name.trim_start_matches('/');
            let imported = include(read_source, name, stack, files, opaque)?;
            merged = yaml::merge(&merged, &imported);
        }
    }
    stack.pop();
    Ok(yaml::merge(&merged, &local))
}
fn inherited(root: &Node, id: &str, stack: &mut Vec<String>) -> std::result::Result<Node, String> {
    if stack.len() > 20 || stack.iter().any(|key| key == id) {
        return Err(format!("Inheritance cycle at {id}."));
    }
    let child = root
        .get(id)
        .ok_or_else(|| format!("Missing inherited job {id}."))?;
    stack.push(id.into());
    let mut merged = Node {
        data: Arc::new(Data::Map(BTreeMap::new())),
        ..child.clone()
    };
    for parent in strings(child, "extends") {
        let parent = inherited(root, &parent, stack)?;
        merged = yaml::merge(&merged, &parent);
    }
    stack.pop();
    merged = yaml::merge(&merged, child);
    if text(child, "inherit") != "false"
        && child
            .get("inherit")
            .map_or(true, |v| text(v, "default") != "false")
    {
        if let (Some(default), Some(map)) = (root.get("default").and_then(Node::map), merged.map())
        {
            let mut values = map.clone();
            let allowed = child
                .get("inherit")
                .and_then(|v| v.get("default"))
                .filter(|v| matches!(&*v.data, Data::List(_)))
                .map(Node::strings);
            for (k, v) in default {
                if allowed.as_ref().map_or(true, |a| a.contains(k)) {
                    values.entry(k.clone()).or_insert_with(|| v.clone());
                }
            }
            merged.data = Arc::new(Data::Map(values));
        }
    }
    Ok(merged)
}
fn schema(github: bool) -> std::result::Result<&'static jsonschema::JSONSchema, String> {
    static GH: OnceLock<std::result::Result<jsonschema::JSONSchema, String>> = OnceLock::new();
    static GL: OnceLock<std::result::Result<jsonschema::JSONSchema, String>> = OnceLock::new();
    let cell = if github { &GH } else { &GL };
    cell.get_or_init(|| {
        let raw = if github {
            include_str!("schemas/github-workflow.json")
        } else {
            include_str!("schemas/gitlab-ci.json")
        };
        let value: Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
        jsonschema::JSONSchema::options()
            .with_draft(jsonschema::Draft::Draft7)
            .compile(&value)
            .map_err(|e| e.to_string())
    })
    .as_ref()
    .map_err(Clone::clone)
}
fn validation(root: &Node, github: bool) -> Vec<Problem> {
    let mut problems = Vec::new();
    match (root.value(), schema(github)) {
        (Ok(value), Ok(schema)) => {
            if let Err(errors) = schema.validate(&value) {
                for error in errors.take(50) {
                    let mut at = root;
                    for part in error.instance_path.to_string().split('/').skip(1) {
                        let key = part.replace("~1", "/").replace("~0", "~");
                        if let Some(next) = at
                            .get(&key)
                            .or_else(|| key.parse::<usize>().ok().and_then(|i| at.list().get(i)))
                        {
                            at = next;
                        } else {
                            break;
                        }
                    }
                    // Validator messages can contain repository values. Keep only the rule and pointer.
                    problems.push(problem(
                        at,
                        format!(
                            "Schema problem at {} ({})",
                            error.instance_path, error.schema_path
                        ),
                    ));
                }
            }
        }
        (Err(e), _) | (_, Err(e)) => problems.push(problem(root, e)),
    }
    problems
}
fn distance(a: &str, b: &str) -> usize {
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, x) in a.bytes().enumerate() {
        let mut previous = row[0];
        row[0] = i + 1;
        for (j, y) in b.bytes().enumerate() {
            let old = row[j + 1];
            row[j + 1] = (row[j] + 1)
                .min(old + 1)
                .min(previous + usize::from(x != y));
            previous = old;
        }
    }
    row[b.len()]
}
fn key_hints(n: &Node, github: bool) -> Vec<Problem> {
    let known = if github {
        "name runs-on needs if strategy steps environment services timeout-minutes uses with secrets permissions env defaults concurrency outputs container continue-on-error"
    } else {
        "stage needs rules only except extends script before_script after_script image services tags timeout environment parallel when variables allow_failure artifacts cache dependencies retry interruptible trigger resource_group coverage inherit id_tokens hooks release pages secrets manual_confirmation start_in identity run"
    };
    n.map()
        .into_iter()
        .flatten()
        .filter_map(|(key, value)| {
            if known.split_whitespace().any(|k| k == key) {
                None
            } else {
                let hint = known
                    .split_whitespace()
                    .min_by_key(|k| distance(key, k))
                    .filter(|k| distance(key, k) <= 2);
                Some(problem(
                    value,
                    format!(
                        "Unknown key {key}{}",
                        hint.map(|h| format!(" — did you mean {h}? "))
                            .unwrap_or_default()
                    ),
                ))
            }
        })
        .collect()
}
fn product(axes: &Map<String, Value>) -> std::result::Result<Vec<Value>, String> {
    let mut rows = vec![Map::new()];
    for (key, values) in axes {
        let Some(values) = values.as_array() else {
            return Err("Matrix axes must be arrays; expressions are decided on the host.".into());
        };
        if rows.len().saturating_mul(values.len()) > 4096 {
            return Err("Matrix expansion exceeds 4096 jobs.".into());
        }
        rows = rows
            .into_iter()
            .flat_map(|row| {
                values.iter().map(move |v| {
                    let mut row = row.clone();
                    row.insert(key.clone(), v.clone());
                    row
                })
            })
            .collect();
    }
    Ok(rows.into_iter().map(Value::Object).collect())
}
fn matrix(n: &Node, github: bool) -> std::result::Result<Vec<Value>, String> {
    let raw = if github {
        n.get("strategy").and_then(|s| s.get("matrix"))
    } else {
        n.get("parallel").and_then(|s| s.get("matrix"))
    };
    let Some(raw) = raw else {
        return Ok(vec![]);
    };
    let v = raw.value()?;
    if !github {
        let mut rows = Vec::new();
        for group in v.as_array().ok_or("Parallel matrix must be a list.")? {
            let mut axes = group
                .as_object()
                .ok_or("Matrix rows must be mappings.")?
                .clone();
            for value in axes.values_mut() {
                if !value.is_array() {
                    *value = Value::Array(vec![value.clone()]);
                }
            }
            rows.extend(product(&axes)?);
            if rows.len() > 4096 {
                return Err("Matrix expansion exceeds 4096 jobs.".into());
            }
        }
        return Ok(rows);
    }
    let mut axes = v
        .as_object()
        .ok_or("Matrix expressions are decided on the host.")?
        .clone();
    let includes = axes.remove("include").unwrap_or(Value::Null);
    let excludes = axes.remove("exclude").unwrap_or(Value::Null);
    let keys: Vec<_> = axes.keys().cloned().collect();
    let mut rows = if axes.is_empty() {
        vec![]
    } else {
        product(&axes)?
    };
    if let Some(excludes) = excludes.as_array() {
        rows.retain(|row| {
            !excludes.iter().any(|ex| {
                ex.as_object()
                    .is_some_and(|ex| ex.iter().all(|(k, v)| row[k] == *v))
            })
        });
    }
    let originals = rows.clone();
    for added in includes.as_array().into_iter().flatten() {
        let Some(added) = added.as_object() else {
            return Err("Matrix include must contain mappings.".into());
        };
        let mut matched = false;
        for (i, base) in originals.iter().enumerate() {
            if keys
                .iter()
                .all(|k| !added.contains_key(k) || base[k] == added[k])
            {
                if let Some(row) = rows[i].as_object_mut() {
                    row.extend(added.clone());
                    matched = true;
                }
            }
        }
        if !matched {
            rows.push(Value::Object(added.clone()));
        }
        if rows.len() > 4096 {
            return Err("Matrix expansion exceeds 4096 jobs.".into());
        }
    }
    Ok(rows)
}
fn all(values: impl IntoIterator<Item = Option<bool>>) -> Option<bool> {
    let mut unknown = false;
    for value in values {
        match value {
            Some(false) => return Some(false),
            None => unknown = true,
            _ => {}
        }
    }
    if unknown {
        None
    } else {
        Some(true)
    }
}
fn any(values: impl IntoIterator<Item = Option<bool>>) -> Option<bool> {
    let mut unknown = false;
    for value in values {
        match value {
            Some(true) => return Some(true),
            None => unknown = true,
            _ => {}
        }
    }
    if unknown {
        None
    } else {
        Some(false)
    }
}
fn paths(n: &Node, c: &Simulation, github: bool) -> Option<bool> {
    if !c.changes_known {
        return None;
    }
    let patterns = if n.map().is_some() {
        strings(n, "paths")
    } else {
        n.strings()
    };
    any(c
        .changed
        .iter()
        .map(|file| expressions::patterns(&patterns, file, github)))
}
fn gh_trigger(root: &Node, c: &Simulation) -> Option<bool> {
    let on = root.get("on")?;
    let event = c.github_event();
    if on.map().is_none() {
        return Some(on.strings().iter().any(|s| s == event));
    }
    let Some(filter) = on.get(event) else {
        return Some(false);
    };
    let mut decisions = Vec::new();
    if filter.get("types").is_some() {
        decisions.push(None);
    }
    let tag = c.event == "tag";
    let key = if tag { "tags" } else { "branches" };
    let ignored = if tag {
        "tags-ignore"
    } else {
        "branches-ignore"
    };
    let refname = if c.event == "pull_request" {
        &c.base
    } else {
        &c.reference
    };
    if let Some(list) = filter.get(key) {
        decisions.push(if refname.is_empty() {
            None
        } else {
            expressions::patterns(&list.strings(), refname, true)
        });
    } else if event == "push"
        && filter.get(if tag { "branches" } else { "tags" }).is_some()
        && filter.get(ignored).is_none()
    {
        decisions.push(Some(false));
    }
    if let Some(list) = filter.get(ignored) {
        decisions.push(expressions::patterns(&list.strings(), refname, true).map(|v| !v));
    }
    if !tag {
        if let Some(list) = filter.get("paths") {
            decisions.push(paths(list, c, true));
        }
        if let Some(list) = filter.get("paths-ignore") {
            decisions.push(if c.changes_known {
                if c.changed.is_empty() {
                    Some(false)
                } else {
                    any(c
                        .changed
                        .iter()
                        .map(|p| expressions::patterns(&list.strings(), p, true).map(|v| !v)))
                }
            } else {
                None
            });
        }
    }
    if event == "schedule" {
        decisions.push(if c.default_branch.is_empty() {
            None
        } else {
            Some(c.reference == c.default_branch)
        });
    }
    all(decisions)
}
fn rules(n: &Node, c: &Simulation) -> (Option<bool>, bool) {
    for rule in n.list() {
        let mut tests = Vec::new();
        if let Some(condition) = rule.get("if") {
            tests.push(expressions::evaluate(&condition.text(), c));
        }
        if let Some(changes) = rule.get("changes") {
            tests.push(if c.event == "push" || c.event == "pull_request" {
                if changes.get("compare_to").is_some() {
                    None
                } else {
                    paths(changes, c, false)
                }
            } else {
                Some(true)
            });
        }
        if rule.get("exists").is_some() {
            tests.push(None);
        }
        if rule.map().is_some_and(|map| {
            map.keys().any(|key| {
                !matches!(
                    key.as_str(),
                    "if" | "changes" | "when" | "allow_failure" | "exists"
                )
            })
        }) || matches!(text(rule, "when").as_str(), "on_failure" | "delayed")
        {
            tests.push(None);
        }
        match all(tests) {
            Some(false) => continue,
            None => return (None, false),
            Some(true) => {
                return (
                    Some(text(rule, "when") != "never"),
                    text(rule, "when") == "manual",
                )
            }
        }
    }
    (Some(false), false)
}
fn legacy(n: &Node, c: &Simulation) -> Option<bool> {
    if let Some(map) = n.map() {
        let mut decisions = Vec::new();
        if let Some(refs) = map.get("refs") {
            decisions.push(legacy(refs, c));
        }
        if let Some(changes) = map.get("changes") {
            decisions.push(paths(changes, c, false));
        }
        if let Some(variables) = map.get("variables") {
            decisions.push(any(variables
                .strings()
                .iter()
                .map(|s| expressions::evaluate(s, c))));
        }
        if map
            .keys()
            .any(|k| !matches!(k.as_str(), "refs" | "changes" | "variables"))
        {
            decisions.push(None);
        }
        return all(decisions);
    }
    any(n.strings().iter().map(|value| match value.as_str() {
        "branches" => Some(c.event != "tag" && c.event != "pull_request"),
        "tags" => Some(c.event == "tag"),
        "merge_requests" => Some(c.event == "pull_request"),
        "schedules" => Some(c.event == "schedule"),
        "pushes" => Some(c.event == "push" || c.event == "tag"),
        "web" => Some(c.event == "manual"),
        v if v.starts_with('/') => expressions::evaluate(&format!("$CI_COMMIT_BRANCH =~ {v}"), c),
        v => Some(v == c.reference),
    }))
}
fn decision(n: &Node, root: &Node, c: &Simulation, github: bool, opaque: bool) -> (String, String) {
    let trigger = if github {
        gh_trigger(root, c)
    } else {
        root.get("workflow")
            .and_then(|v| v.get("rules"))
            .map(|r| rules(r, c).0)
            .unwrap_or(Some(true))
    };
    let (mut chosen, mut manual) = if github {
        (expressions::evaluate(&text(n, "if"), c), false)
    } else if let Some(r) = n.get("rules") {
        rules(r, c)
    } else {
        let mut tests = vec![];
        if let Some(only) = n.get("only") {
            tests.push(legacy(only, c));
        }
        if let Some(except) = n.get("except") {
            tests.push(legacy(except, c).map(|v| !v));
        }
        if n.get("only").is_none() && n.get("except").is_none() {
            tests.push(Some(c.event != "pull_request"));
        }
        (all(tests), text(n, "when") == "manual")
    };
    chosen = all([trigger, chosen]);
    if opaque || !text(n, "uses").is_empty() || text(n, "when") == "delayed" {
        chosen = all([chosen, None]);
    }
    manual |= github && n.get("environment").is_some();
    match chosen {
        Some(false) if trigger == Some(false) => (
            "skipped".into(),
            if github {
                "The workflow's on: does not start a run for this event and ref."
            } else {
                "The workflow: rules create no pipeline for this event and ref."
            }
            .into(),
        ),
        Some(false) => (
            "skipped".into(),
            "Excluded by the event, ref, paths or condition.".into(),
        ),
        None => (
            "host".into(),
            "An include, expression or host context cannot be decided locally.".into(),
        ),
        Some(true) if manual => (
            "approval".into(),
            "Waits for environment or manual approval on the host.".into(),
        ),
        Some(true) => (
            "runs".into(),
            "The supported trigger and condition match; dependencies are assumed to pass.".into(),
        ),
    }
}
fn steps(n: &Node, github: bool) -> Vec<Step> {
    if github {
        n.get("steps")
            .map(Node::list)
            .unwrap_or(&[])
            .iter()
            .map(|n| Step {
                name: if text(n, "name").is_empty() {
                    text(n, "id")
                } else {
                    text(n, "name")
                },
                command: if text(n, "run").is_empty() {
                    text(n, "uses")
                } else {
                    text(n, "run")
                },
                condition: text(n, "if"),
            })
            .collect()
    } else {
        fn commands(n: &Node, out: &mut Vec<Step>) {
            if !n.list().is_empty() {
                for n in n.list() {
                    commands(n, out);
                }
            } else if matches!(&*n.data, Data::Scalar(_)) {
                let command = n.text();
                out.push(Step {
                    name: command.lines().next().unwrap_or("").into(),
                    command,
                    condition: String::new(),
                });
            }
        }
        let mut out = vec![];
        for key in ["before_script", "script", "after_script"] {
            if let Some(n) = n.get(key) {
                commands(n, &mut out);
            }
        }
        out
    }
}
fn job(
    id: &str,
    n: &Node,
    root: &Node,
    c: &Simulation,
    github: bool,
    opaque: bool,
    problems: &mut Vec<Problem>,
) -> Job {
    problems.extend(key_hints(n, github));
    let mut decision = decision(n, root, c, github, opaque);
    let matrix = match matrix(n, github) {
        Ok(v) => v,
        Err(e) => {
            problems.push(problem(n, e));
            decision = (
                "host".into(),
                "The matrix cannot be expanded locally.".into(),
            );
            vec![]
        }
    };
    let steps = steps(n, github);
    let secret_pattern = regex::Regex::new(r"secrets\.([A-Za-z_][A-Za-z0-9_]*)").ok();
    let raw = n.value().map(|v| v.to_string()).unwrap_or_default();
    let mut secrets: BTreeSet<String> = secret_pattern
        .iter()
        .flat_map(|r| {
            r.captures_iter(&raw)
                .filter_map(|cap| cap.get(1).map(|m| m.as_str().to_string()))
        })
        .collect();
    if let Some(map) = n.get("secrets").and_then(Node::map) {
        secrets.extend(map.keys().cloned());
    }
    let needs = n
        .get("needs")
        .map(|list| {
            if !github {
                list.list()
                    .iter()
                    .map(|n| {
                        if n.map().is_some() {
                            text(n, "job")
                        } else {
                            n.text()
                        }
                    })
                    .collect()
            } else {
                list.strings()
            }
        })
        .unwrap_or_default();
    let environment = n
        .get("environment")
        .map(|n| {
            if n.map().is_some() {
                text(n, "name")
            } else {
                n.text()
            }
        })
        .unwrap_or_default();
    Job {
        id: id.into(),
        name: if text(n, "name").is_empty() {
            id.into()
        } else {
            text(n, "name")
        },
        stage: if github {
            "".into()
        } else if text(n, "stage").is_empty() {
            "test".into()
        } else {
            text(n, "stage")
        },
        needs,
        condition: if github {
            text(n, "if")
        } else {
            n.get("rules")
                .map(|r| {
                    r.list()
                        .iter()
                        .map(|n| text(n, "if"))
                        .filter(|s| !s.is_empty())
                        .collect::<Vec<_>>()
                        .join(" · ")
                })
                .unwrap_or_default()
        },
        matrix,
        environment,
        services: n
            .get("services")
            .map(|v| {
                if v.map().is_some() {
                    v.map()
                        .into_iter()
                        .flatten()
                        .map(|(k, _)| k.clone())
                        .collect()
                } else {
                    v.list()
                        .iter()
                        .map(|n| {
                            if n.map().is_some() {
                                text(n, "name")
                            } else {
                                n.text()
                            }
                        })
                        .collect()
                }
            })
            .unwrap_or_default(),
        steps,
        runs_on: strings(n, if github { "runs-on" } else { "tags" }),
        timeout: text(n, if github { "timeout-minutes" } else { "timeout" }),
        secrets: std::mem::take(&mut secrets).into_iter().collect(),
        file: n.file.clone(),
        line: n.line,
        state: decision.0,
        reason: decision.1,
        depth: 0,
        implicit: false,
    }
}
fn graph(workflow: &mut Workflow, implicit: &BTreeSet<String>) {
    let ids: BTreeMap<_, _> = workflow
        .jobs
        .iter()
        .enumerate()
        .map(|(i, j)| (j.id.clone(), i))
        .collect();
    fn depth(
        i: usize,
        jobs: &[Job],
        ids: &BTreeMap<String, usize>,
        active: &mut BTreeSet<usize>,
        known: &mut BTreeMap<usize, usize>,
    ) -> Option<usize> {
        if let Some(d) = known.get(&i) {
            return Some(*d);
        }
        if active.len() > 128 || !active.insert(i) {
            return None;
        }
        let mut d = 0;
        for name in &jobs[i].needs {
            let next = *ids.get(name)?;
            d = d.max(depth(next, jobs, ids, active, known)? + 1);
        }
        active.remove(&i);
        known.insert(i, d);
        Some(d)
    }
    let mut known = BTreeMap::new();
    for i in 0..workflow.jobs.len() {
        let computed = depth(i, &workflow.jobs, &ids, &mut BTreeSet::new(), &mut known);
        if let Some(d) = computed {
            if !implicit.contains(&workflow.jobs[i].id) {
                workflow.jobs[i].depth = d;
            }
        } else {
            let j = &mut workflow.jobs[i];
            j.state = "host".into();
            j.reason = "A dependency is missing or cyclic.".into();
            workflow.problems.push(Problem {
                file: j.file.clone(),
                line: j.line,
                message: j.reason.clone(),
            });
        }
    }
    for _ in 0..workflow.jobs.len() {
        let states: Vec<_> = workflow.jobs.iter().map(|j| j.state.clone()).collect();
        let mut changed = false;
        for j in &mut workflow.jobs {
            if j.state != "runs" && j.state != "approval" {
                continue;
            }
            if j.condition.contains("always()") || implicit.contains(&j.id) {
                continue;
            }
            let state = if j
                .needs
                .iter()
                .any(|n| ids.get(n).is_some_and(|i| states[*i] == "skipped"))
            {
                Some("skipped")
            } else if j
                .needs
                .iter()
                .any(|n| ids.get(n).is_some_and(|i| states[*i] == "host"))
            {
                Some("host")
            } else {
                None
            };
            if let Some(state) = state {
                j.state = state.into();
                j.reason = "A dependency is skipped or decided on the host.".into();
                changed = true;
            }
        }
        if !changed {
            break;
        }
    }
}
fn workflow(root_path: &Path, file: &str, c: &Simulation) -> Result<Workflow> {
    workflow_from(file, c, &|file| read(root_path, file))
}
fn workflow_from(file: &str, c: &Simulation, read_source: ReadParsed<'_>) -> Result<Workflow> {
    let bytes = std::cell::Cell::new(0usize);
    let github = file != ".gitlab-ci.yml";
    let duplicates = std::cell::RefCell::new(Vec::new());
    let bounded = |file: &str| {
        let parsed = read_source(file)?;
        duplicates
            .borrow_mut()
            .extend(parsed.duplicates.iter().map(|d| Problem {
                file: file.into(),
                line: d.line,
                message: format!(
                    "{} is written twice (also on line {}). {}",
                    d.key,
                    d.first,
                    if github {
                        "GitHub refuses a workflow with a repeated key."
                    } else {
                        "GitLab keeps this later one, and so does the preview."
                    }
                ),
            }));
        let node = parsed.root;
        let size = node.value().map_err(config)?.to_string().len();
        bytes.set(bytes.get().saturating_add(size));
        if bytes.get() > 8 * 1024 * 1024 {
            return Err(config("Included workflow data exceeds 8 MiB."));
        }
        Ok(node)
    };
    let read_source: ReadSource<'_> = &bounded;
    let mut files = BTreeSet::new();
    let mut opaque = false;
    let root = if github {
        files.insert(file.into());
        read_source(file)?
    } else {
        yaml::references(&include(
            read_source,
            file,
            &mut vec![],
            &mut files,
            &mut opaque,
        )?)
        .map_err(config)?
    };
    let mut w = Workflow {
        file: file.into(),
        name: if text(&root, "name").is_empty() {
            file.into()
        } else {
            text(&root, "name")
        },
        kind: if github { "gitHub" } else { "gitLab" }.into(),
        triggers: if github {
            root.get("on")
                .map(|n| {
                    n.map()
                        .map(|v| v.keys().cloned().collect())
                        .unwrap_or_else(|| n.strings())
                })
                .unwrap_or_default()
        } else {
            vec!["rules".into()]
        },
        stages: if github {
            vec![]
        } else if root.get("stages").is_some() {
            strings(&root, "stages")
        } else {
            vec![
                ".pre".into(),
                "build".into(),
                "test".into(),
                "deploy".into(),
                ".post".into(),
            ]
        },
        jobs: vec![],
        problems: validation(&root, github),
        source_files: vec![],
    };
    let mut unresolved = BTreeSet::new();
    let mut explicit = BTreeSet::new();
    let jobs = if github {
        root.get("jobs").and_then(Node::map)
    } else {
        root.map()
    };
    for (id, n) in jobs.into_iter().flatten() {
        if !github
            && (id.starts_with('.')
                || matches!(
                    id.as_str(),
                    "stages"
                        | "include"
                        | "default"
                        | "workflow"
                        | "variables"
                        | "image"
                        | "services"
                        | "before_script"
                        | "after_script"
                        | "cache"
                        | "spec"
                        | "types"
                ))
        {
            continue;
        }
        let node = if github {
            n.clone()
        } else {
            match inherited(&root, id, &mut vec![]) {
                Ok(n) => n,
                Err(e) => {
                    w.problems.push(problem(n, e));
                    unresolved.insert(id.clone());
                    n.clone()
                }
            }
        };
        // Local keys win over included ones, so an unread include can change
        // a job's outcome only through what the file leaves unset: the
        // workflow's rules, or the job's own conditions.
        let decided_here = root.get("workflow").is_some()
            && ["rules", "only", "except"]
                .iter()
                .any(|key| node.get(key).is_some());
        let mut j = job(
            id,
            &node,
            &root,
            c,
            github,
            opaque && !decided_here,
            &mut w.problems,
        );
        if node.get("needs").is_some() {
            explicit.insert(id.clone());
        }
        if unresolved.contains(id) {
            j.state = "host".into();
            j.reason = "Inheritance could not be resolved.".into();
        }
        if github {
            let uses = text(n, "uses");
            if let Some(local) = uses.strip_prefix("./") {
                if local.starts_with(".github/workflows/") {
                    files.insert(local.into());
                    match read_source(local) {
                        Ok(child) => {
                            w.problems.extend(validation(&child, true));
                            let mut caller = n.clone();
                            if let Some(map) = n.map() {
                                let mut map = map.clone();
                                map.remove("uses");
                                caller.data = Arc::new(Data::Map(map));
                            }
                            let caller_state = decision(&caller, &root, c, true, false);
                            let mut children = vec![];
                            for (cid, cn) in
                                child.get("jobs").and_then(Node::map).into_iter().flatten()
                            {
                                let mut cj = job(
                                    &format!("{id}/{cid}"),
                                    cn,
                                    &root,
                                    c,
                                    true,
                                    false,
                                    &mut w.problems,
                                );
                                cj.needs = cj.needs.iter().map(|n| format!("{id}/{n}")).collect();
                                if cj.needs.is_empty() {
                                    cj.needs = j.needs.clone();
                                }
                                if caller_state.0 != "runs" {
                                    cj.state = caller_state.0.clone();
                                    cj.reason = caller_state.1.clone();
                                }
                                children.push(cj);
                            }
                            let consumed: BTreeSet<_> =
                                children.iter().flat_map(|j| j.needs.clone()).collect();
                            j.needs = children
                                .iter()
                                .filter(|j| !consumed.contains(&j.id))
                                .map(|j| j.id.clone())
                                .collect();
                            j.state = caller_state.0;
                            j.reason = "Local reusable workflow expanded one level.".into();
                            w.jobs.extend(children);
                        }
                        Err(e) => w.problems.push(problem(n, e.to_string())),
                    }
                }
            }
        }
        w.jobs.push(j);
    }
    // Host stage ordering treats skipped jobs as successful. These edges are
    // for drawing barriers, not GitHub's implicit success() condition.
    let mut implicit = BTreeSet::new();
    if !github {
        for j in &mut w.jobs {
            if !explicit.contains(&j.id) {
                j.implicit = true;
                implicit.insert(j.id.clone());
                if let Some(stage) = w.stages.iter().position(|s| s == &j.stage) {
                    j.depth = stage;
                }
            }
        }
        let stage_jobs: Vec<_> = w
            .jobs
            .iter()
            .map(|j| (j.id.clone(), j.stage.clone()))
            .collect();
        for j in &mut w.jobs {
            if !explicit.contains(&j.id) {
                if let Some(index) = w.stages.iter().position(|s| s == &j.stage) {
                    j.needs = stage_jobs
                        .iter()
                        .filter(|(_, s)| {
                            let previous = w.stages[..index]
                                .iter()
                                .rfind(|s| stage_jobs.iter().any(|(_, stage)| stage == *s));
                            previous == Some(s)
                        })
                        .map(|(id, _)| id.clone())
                        .collect();
                }
            }
        }
    }
    if opaque {
        w.jobs.push(Job {
            id: "remote-includes".into(),
            name: "Remote includes".into(),
            stage: String::new(),
            needs: vec![],
            condition: String::new(),
            matrix: vec![],
            environment: String::new(),
            services: vec![],
            steps: vec![],
            runs_on: vec![],
            timeout: String::new(),
            secrets: vec![],
            file: file.into(),
            line: 1,
            state: "host".into(),
            reason: "Remote, conditional or wildcard includes are not fetched.".into(),
            depth: 0,
            implicit: false,
        });
    }
    w.source_files = files.into_iter().collect();
    graph(&mut w, &implicit);
    let mut repeated = duplicates.into_inner();
    repeated.dedup_by(|a, b| a.file == b.file && a.line == b.line);
    w.problems.splice(0..0, repeated);
    Ok(w)
}

/// Read the configuration at the run's actual commit. Working-copy edits must
/// never change the dependency graph of a run that is already on the host.
pub fn hosted(repo: &crate::forge::Repo, token: &str, id: u64) -> Result<Workflow> {
    use crate::forge::{self, http, issues::root, Kind};
    if id == 0 {
        return Err(config("Choose a pipeline first."));
    }
    let github = repo.kind == Kind::GitHub;
    let endpoint = if github {
        format!("actions/runs/{id}")
    } else {
        format!("pipelines/{id}")
    };
    let response = http::get_json(&format!("{}/{endpoint}", root(repo)?), token, &repo.host)?;
    if !(200..300).contains(&response.status) {
        return Err(forge::status_error(
            &repo.host,
            response.status,
            &response.body,
            response.retry_after.as_deref(),
        ));
    }
    let run: Value = serde_json::from_str(&response.body)
        .map_err(|_| config("The host sent an unreadable run."))?;
    let commit = run[if github { "head_sha" } else { "sha" }]
        .as_str()
        .filter(|s| s.len() >= 40 && s.bytes().all(|c| c.is_ascii_hexdigit()))
        .ok_or_else(|| config("The run has no commit to read."))?;
    let file = if github {
        run["path"]
            .as_str()
            .map(|s| s.split('@').next().unwrap_or(s))
            .ok_or_else(|| config("The host did not name this run's workflow file."))?
    } else {
        ".gitlab-ci.yml"
    };
    let simulation = Simulation {
        event: run[if github { "event" } else { "source" }]
            .as_str()
            .unwrap_or("push")
            .into(),
        reference: run[if github { "head_branch" } else { "ref" }]
            .as_str()
            .unwrap_or("")
            .into(),
        ..Default::default()
    };
    workflow_from(file, &simulation, &|file| {
        if file.contains('\\')
            || file.starts_with('/')
            || file.split('/').any(|s| matches!(s, ".." | "." | ""))
        {
            return Err(config(
                "A workflow source path must stay inside its repository.",
            ));
        }
        let endpoint = if github {
            format!(
                "contents/{}?ref={}",
                forge::encode_segment(file),
                forge::encode_segment(commit)
            )
        } else {
            format!(
                "repository/files/{}/raw?ref={}",
                forge::encode_segment(file),
                forge::encode_segment(commit)
            )
        };
        yaml::parse_all(
            &http::get_repository_text(&format!("{}/{endpoint}", root(repo)?), token, &repo.host)?,
            file,
        )
        .map_err(config)
    })
}
pub fn preview(
    root: &Path,
    mut simulation: Simulation,
    mode: &str,
    from: &str,
    to: &str,
) -> Result<Preview> {
    let repo = crate::repo::open(root)?;
    if simulation.reference.is_empty() {
        simulation.reference = repo
            .head()
            .ok()
            .and_then(|h| h.referent_name().map(|n| n.shorten().to_string()))
            .unwrap_or_default();
    }
    if simulation.default_branch.is_empty() {
        simulation.default_branch = repo
            .find_reference("refs/remotes/origin/HEAD")
            .ok()
            .and_then(|r| r.target().try_name().map(|n| n.as_bstr().to_string()))
            .and_then(|name| {
                name.strip_prefix("refs/remotes/origin/")
                    .map(str::to_string)
            })
            .unwrap_or_default();
    }
    if simulation.base.is_empty() {
        simulation.base = simulation.default_branch.clone();
    }
    let changes = if mode == "range" {
        let resolved = crate::files::resolve(&repo, from)
            .and_then(|a| crate::files::resolve(&repo, to).map(|b| (a.id, b.id)));
        resolved
            .and_then(|(a, b)| crate::diff::changes_between(&repo, &a, &b))
            .map(|v| {
                v.into_iter()
                    .flat_map(|f| [Some(f.path), f.old_path])
                    .flatten()
                    .collect::<Vec<_>>()
            })
    } else {
        // One ref, not every branch's drift: listing a repository with a
        // thousand branches took half a minute before anything was drawn.
        if let Some(upstream) = crate::branches::current_upstream(&repo) {
            working_paths(&repo, &upstream)
        } else {
            Err(config(
                "This branch has no upstream. Choose a commit range for path conditions.",
            ))
        }
    };
    let warning = match changes {
        Ok(mut changed) => {
            changed.sort();
            changed.dedup();
            simulation.changed = changed;
            simulation.changes_known = true;
            None
        }
        Err(e) => {
            simulation.changed = vec![];
            simulation.changes_known = false;
            Some(e.to_string())
        }
    };
    let mut paths = vec![];
    if root.join(".gitlab-ci.yml").exists() {
        paths.push(".gitlab-ci.yml".into());
    }
    if let Ok(entries) = std::fs::read_dir(root.join(".github/workflows")) {
        for entry in entries.flatten() {
            if entry
                .path()
                .extension()
                .is_some_and(|s| s == "yml" || s == "yaml")
            {
                paths.push(format!(
                    ".github/workflows/{}",
                    entry.file_name().to_string_lossy()
                ));
            }
        }
    }
    paths.sort();
    if paths.len() > 100 {
        return Err(config(
            "More than 100 workflow files exceed the preview limit.",
        ));
    }
    let workflows = paths
        .iter()
        .map(|file| {
            workflow(root, file, &simulation).unwrap_or_else(|e| Workflow {
                file: file.clone(),
                name: file.clone(),
                kind: if file == ".gitlab-ci.yml" {
                    "gitLab"
                } else {
                    "gitHub"
                }
                .into(),
                triggers: vec![],
                stages: vec![],
                jobs: vec![],
                problems: vec![Problem {
                    file: file.clone(),
                    line: 1,
                    message: e.to_string(),
                }],
                source_files: vec![file.clone()],
            })
        })
        .collect();
    Ok(Preview {
        workflows,
        changed: simulation.changed.clone(),
        changes_warning: warning,
        simulation,
    })
}

fn working_paths(repo: &gix::Repository, upstream: &str) -> Result<Vec<String>> {
    use gix::status::{
        index_worktree::Item as Work, plumbing::index_as_worktree::EntryStatus, Item,
    };
    let tree = repo
        .rev_parse_single(upstream)
        .map_err(|e| config(e.to_string()))?
        .object()
        .map_err(|e| config(e.to_string()))?
        .peel_to_tree()
        .map_err(|e| config(e.to_string()))?
        .id;
    // This index exists only in memory. Compare final worktree contents, so a
    // committed change reverted locally does not falsely match a path filter.
    let index = repo
        .index_from_tree(&tree)
        .map_err(|e| config(e.to_string()))?;
    let items = repo
        .status(gix::progress::Discard)
        .map_err(|e| config(e.to_string()))?
        .head_tree(tree)
        .index(gix::worktree::IndexPersistedOrInMemory::InMemory(index))
        .untracked_files(gix::status::UntrackedFiles::Files)
        .into_iter(None)
        .map_err(|e| config(e.to_string()))?;
    let mut paths = vec![];
    for item in items {
        match item.map_err(|e| config(e.to_string()))? {
            Item::IndexWorktree(Work::Modification {
                rela_path,
                status: EntryStatus::Change(_) | EntryStatus::Conflict { .. },
                ..
            }) => paths.push(rela_path.to_string()),
            Item::IndexWorktree(Work::DirectoryContents { entry, .. })
                if entry.status == gix::dir::entry::Status::Untracked =>
            {
                paths.push(entry.rela_path.to_string())
            }
            Item::IndexWorktree(Work::Rewrite { dirwalk_entry, .. }) => {
                paths.push(dirwalk_entry.rela_path.to_string())
            }
            _ => {}
        }
    }
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::Fixture;
    fn context() -> Simulation {
        Simulation {
            event: "push".into(),
            reference: "main".into(),
            default_branch: "main".into(),
            changed: vec!["src/main.rs".into()],
            changes_known: true,
            ..Default::default()
        }
    }
    #[test]
    fn github_matrix_paths_conditions_and_missing_dependencies_are_visible() {
        let f = Fixture::linear(1);
        f.write(".github/workflows/ci.yml","name: CI\non:\n  push:\n    branches: [main]\n    paths: ['src/**']\njobs:\n  build:\n    runs-on: ubuntu-latest\n    strategy:\n      matrix:\n        os: [linux, windows]\n        version: [20, 22]\n        exclude: [{os: windows, version: 20}]\n        include: [{os: linux, tool: clang}]\n    steps: [{run: echo ok}]\n  deploy:\n    runs-on: ubuntu-latest\n    needs: build\n    environment: production\n    steps: [{run: echo deploy}]\n  skip:\n    runs-on: ubuntu-latest\n    if: github.ref_name != 'main'\n    steps: [{run: echo skip}]\n  unknown:\n    runs-on: ubuntu-latest\n    needs: missing\n    need: build\n    steps: [{run: echo hi}]\n");
        let w = workflow(f.path(), ".github/workflows/ci.yml", &context()).unwrap();
        assert_eq!(
            w.jobs
                .iter()
                .find(|j| j.id == "build")
                .unwrap()
                .matrix
                .len(),
            3
        );
        assert_eq!(
            w.jobs.iter().find(|j| j.id == "deploy").unwrap().state,
            "approval"
        );
        assert_eq!(
            w.jobs.iter().find(|j| j.id == "skip").unwrap().state,
            "skipped"
        );
        assert!(w
            .problems
            .iter()
            .any(|p| p.message.contains("did you mean needs")));
        assert_eq!(
            w.jobs.iter().find(|j| j.id == "unknown").unwrap().state,
            "host"
        );
    }
    #[test]
    fn gitlab_includes_inheritance_and_references_keep_their_source() {
        let f = Fixture::linear(1);
        f.write(
            "local.yml",
            ".base:\n  script: &script [echo test]\n  tags: [linux]\n",
        );
        f.write(".gitlab-ci.yml","include: {local: local.yml}\nstages: [test, deploy]\ndefault:\n  timeout: 10 minutes\ncheck:\n  extends: .base\n  script: !reference [.base, script]\n  rules:\n    - if: '$CI_COMMIT_BRANCH == $CI_DEFAULT_BRANCH'\nship:\n  stage: deploy\n  when: manual\n  script: echo ship\n");
        let w = workflow(f.path(), ".gitlab-ci.yml", &context()).unwrap();
        let check = w.jobs.iter().find(|j| j.id == "check").unwrap();
        assert_eq!(check.steps[0].command, "echo test");
        assert_eq!(check.runs_on, vec!["linux"]);
        assert_eq!(check.timeout, "10 minutes");
        let ship = w.jobs.iter().find(|j| j.id == "ship").unwrap();
        assert_eq!(ship.needs, vec!["check"]);
        assert_eq!(ship.state, "approval");
        assert!(w.source_files.contains(&"local.yml".into()));
    }
    #[test]
    fn remote_data_is_opaque_and_local_escape_and_cycles_are_refused() {
        let f = Fixture::linear(1);
        f.write(
            ".gitlab-ci.yml",
            "include: {remote: 'https://example.com/ci.yml'}\ncheck:\n  script: echo test\n",
        );
        let w = workflow(f.path(), ".gitlab-ci.yml", &context()).unwrap();
        assert!(w.jobs.iter().all(|j| j.state == "host"));
        // A job with its own rules, under the file's own workflow rules, is
        // decided here: an include cannot override keys the file sets.
        f.write(
            ".gitlab-ci.yml",
            "workflow: {rules: [{when: always}]}\ninclude: {remote: 'https://example.com/ci.yml'}\ncheck:\n  script: echo test\n  rules: [{when: never}]\n",
        );
        let w = workflow(f.path(), ".gitlab-ci.yml", &context()).unwrap();
        assert_eq!(
            w.jobs.iter().find(|j| j.id == "check").unwrap().state,
            "skipped"
        );
        assert!(source(f.path(), "../outside.yml").is_err());
        f.write(".gitlab-ci.yml", "include: {local: .gitlab-ci.yml}");
        assert!(workflow(f.path(), ".gitlab-ci.yml", &context()).is_err());
    }
    #[test]
    fn path_rules_never_guess_when_upstream_is_missing() {
        let f = Fixture::linear(1);
        f.write(".github/workflows/ci.yml","on:\n  push:\n    paths: ['src/**']\njobs:\n  test:\n    runs-on: ubuntu-latest\n    steps: [{run: echo ok}]\n");
        let p = preview(f.path(), context(), "working", "", "").unwrap();
        assert!(p.changes_warning.is_some());
        assert_eq!(p.workflows[0].jobs[0].state, "host");
        let p = preview(f.path(), context(), "range", "HEAD", "HEAD").unwrap();
        assert!(p.changes_warning.is_none());
        assert_eq!(p.workflows[0].jobs[0].state, "skipped");
    }
    #[test]
    fn working_paths_compare_final_contents_without_changing_the_real_index() {
        let f = Fixture::linear(1);
        f.write("source.txt", "before\n");
        f.git(&["add", "-A"]);
        f.commit_all("before");
        let upstream = crate::files::resolve(&crate::repo::open(f.path()).unwrap(), "HEAD")
            .unwrap()
            .id;
        f.write("source.txt", "after\n");
        f.commit_all("after");
        f.write("source.txt", "before\n");
        f.write("new.txt", "new\n");
        let index = std::fs::read(f.path().join(".git/index")).unwrap();
        let paths = working_paths(&crate::repo::open(f.path()).unwrap(), &upstream).unwrap();
        assert!(!paths.contains(&"source.txt".into()));
        assert!(paths.contains(&"new.txt".into()));
        assert_eq!(std::fs::read(f.path().join(".git/index")).unwrap(), index);
    }
    #[test]
    fn local_reusable_workflows_keep_caller_conditions_and_expand_only_one_level() {
        let f = Fixture::linear(1);
        f.write(".github/workflows/called.yml","on: workflow_call\njobs:\n  child:\n    runs-on: ubuntu-latest\n    steps: [{run: echo ok}]\n  nested:\n    uses: ./.github/workflows/other.yml\n");
        f.write(".github/workflows/ci.yml","on: push\njobs:\n  call:\n    if: github.ref_name == 'main'\n    uses: ./.github/workflows/called.yml\n  dependent:\n    runs-on: ubuntu-latest\n    needs: call\n    steps: [{run: echo next}]\n");
        let w = workflow(f.path(), ".github/workflows/ci.yml", &context()).unwrap();
        assert_eq!(
            w.jobs.iter().find(|j| j.id == "call/child").unwrap().state,
            "runs"
        );
        assert_eq!(
            w.jobs.iter().find(|j| j.id == "call/nested").unwrap().state,
            "host"
        );
        let mut c = context();
        c.reference = "other".into();
        let w = workflow(f.path(), ".github/workflows/ci.yml", &c).unwrap();
        assert!(w.jobs.iter().all(|j| j.state == "skipped"));
    }
    #[test]
    fn gitlab_stage_order_does_not_skip_later_jobs_and_inheritance_cycles_are_unknown() {
        let f = Fixture::linear(1);
        f.write(".gitlab-ci.yml","stages: [test, deploy]\nskip:\n  script: echo skip\n  rules: [{if: '$CI_COMMIT_BRANCH == \"other\"'}]\nship:\n  stage: deploy\n  script: echo ship\ncycle:\n  extends: cycle\n  script: echo no\n");
        let w = workflow(f.path(), ".gitlab-ci.yml", &context()).unwrap();
        assert_eq!(
            w.jobs.iter().find(|j| j.id == "ship").unwrap().state,
            "runs"
        );
        assert_eq!(
            w.jobs.iter().find(|j| j.id == "cycle").unwrap().state,
            "host"
        );
    }
    #[test]
    fn triggers_keep_tag_and_path_semantics_and_unknown_rule_order() {
        let n = yaml::parse(
            "on:\n  push:\n    branches: [main]\n    tags: ['v*']\n    paths-ignore: ['docs/**']\n",
            "ci",
        )
        .unwrap();
        let mut c = context();
        assert_eq!(gh_trigger(&n, &c), Some(true));
        c.changed = vec!["docs/a.md".into()];
        assert_eq!(gh_trigger(&n, &c), Some(false));
        c.event = "tag".into();
        c.reference = "v3".into();
        assert_eq!(gh_trigger(&n, &c), Some(true));
        let n = yaml::parse(
            "rules: [{if: '$UNKNOWN', when: never}, {when: always}]",
            "ci",
        )
        .unwrap();
        assert_eq!(rules(n.get("rules").unwrap(), &c).0, None);
        let n = yaml::parse("on: {schedule: [{cron: '0 0 * * *'}]}", "ci").unwrap();
        c.event = "schedule".into();
        c.default_branch.clear();
        assert_eq!(gh_trigger(&n, &c), None);
    }
    #[test]
    fn matrix_limits_and_secret_names_do_not_read_secret_values() {
        let n=yaml::parse("strategy: {matrix: {os: [linux, windows], include: [{os: macos}]}}\nsteps: [{run: 'echo ${{ secrets.DB_PASSWORD }}'}]\n","ci").unwrap();
        assert_eq!(matrix(&n, true).unwrap().len(), 3);
        let mut p = vec![];
        let j = job(
            "test",
            &n,
            &yaml::parse("on: push", "ci").unwrap(),
            &context(),
            true,
            false,
            &mut p,
        );
        assert_eq!(j.secrets, vec!["DB_PASSWORD"]);
        let axis = (0..65).map(|i| i.to_string()).collect::<Vec<_>>().join(",");
        let n = yaml::parse(
            &format!("strategy: {{matrix: {{a: [{axis}], b: [{axis}]}}}}"),
            "ci",
        )
        .unwrap();
        assert!(matrix(&n, true).is_err());
        let mut c = context();
        c.reference = "Main".into();
        assert_eq!(
            expressions::evaluate("$CI_COMMIT_BRANCH == 'main'", &c),
            Some(false)
        );
    }
}
