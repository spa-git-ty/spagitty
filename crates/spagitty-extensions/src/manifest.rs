// SPDX-License-Identifier: GPL-3.0-or-later

//! `extension.json`.
//!
//! `schemas/extensions/manifest.v1.schema.json` is the authority. This module
//! enforces every rule it states, plus the cross-references a schema cannot
//! (a command naming a review provider that exists; a setting naming a tool
//! that is declared; a capability the contributions need being asked for).
//!
//! Validation reads the JSON as a value first and collects **every** broken
//! rule with the path of the field that broke it, so an author fixes a
//! manifest in one pass rather than one error at a time. Only a manifest with
//! no errors is turned into the typed [`Manifest`].

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::capabilities::Capability;
use crate::{Error, Result};

/// The targets a manifest may name an entrypoint for.
pub const TARGETS: [&str; 6] = [
    "x86_64-pc-windows-msvc",
    "aarch64-pc-windows-msvc",
    "x86_64-unknown-linux-gnu",
    "aarch64-unknown-linux-gnu",
    "x86_64-apple-darwin",
    "aarch64-apple-darwin",
];

/// The largest manifest read. A real one is a few kilobytes.
pub const MAX_BYTES: usize = 256 * 1024;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Manifest {
    #[serde(default, rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    pub manifest_version: u64,
    pub id: String,
    pub name: String,
    pub version: String,
    pub publisher: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    pub engines: Engines,
    pub runtime: Runtime,
    #[serde(default)]
    pub activation: Vec<Activation>,
    #[serde(default)]
    pub capabilities: Capabilities,
    #[serde(default)]
    pub external_tools: Vec<ExternalTool>,
    #[serde(default)]
    pub contributes: Contributes,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Engines {
    pub spagitty: String,
    pub extension_api: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Runtime {
    pub kind: String,
    pub entrypoints: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Activation {
    OnCommand,
    OnReviewProvider,
    OnPanel,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Capabilities {
    #[serde(default)]
    pub required: Vec<Capability>,
    #[serde(default)]
    pub optional: Vec<Capability>,
}

impl Capabilities {
    pub fn requested(&self) -> impl Iterator<Item = Capability> + '_ {
        self.required.iter().chain(&self.optional).copied()
    }

    pub fn asks_for(&self, capability: Capability) -> bool {
        self.requested().any(|held| held == capability)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExternalTool {
    pub id: String,
    #[serde(default)]
    pub name: Option<String>,
    pub executable_names: Vec<String>,
    #[serde(default)]
    pub version_args: Vec<String>,
    #[serde(default)]
    pub minimum_version: Option<String>,
    #[serde(default)]
    pub install_url: Option<String>,
    pub profiles: Vec<Profile>,
}

impl ExternalTool {
    pub fn profile(&self, id: &str) -> Option<&Profile> {
        self.profiles.iter().find(|profile| profile.id == id)
    }

    pub fn label(&self) -> &str {
        self.name.as_deref().unwrap_or(&self.id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Profile {
    pub id: String,
    pub args: Vec<String>,
    #[serde(default)]
    pub options: BTreeMap<String, ToolOption>,
    #[serde(default)]
    pub workdir: Option<ProfileWorkdir>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ProfileWorkdir {
    Operation,
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "type", deny_unknown_fields)]
pub enum ToolOption {
    Enum {
        values: BTreeMap<String, Vec<String>>,
        #[serde(default)]
        required: bool,
    },
    Revision {
        flag: String,
        #[serde(default)]
        required: bool,
    },
    Commit {
        flag: String,
        #[serde(default)]
        required: bool,
    },
}

impl ToolOption {
    pub fn required(&self) -> bool {
        match self {
            ToolOption::Enum { required, .. }
            | ToolOption::Revision { required, .. }
            | ToolOption::Commit { required, .. } => *required,
        }
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Contributes {
    #[serde(default)]
    pub commands: Vec<CommandContribution>,
    #[serde(default)]
    pub review_providers: Vec<ReviewProvider>,
    #[serde(default)]
    pub panels: Vec<Panel>,
    #[serde(default)]
    pub settings: Vec<Setting>,
}

/// Where a contribution belongs.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Context {
    #[default]
    Global,
    WorkingCopy,
    FarmTask,
    PullRequest,
}

/// The finite set of conditions a contribution can be shown under.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Predicate {
    RepositoryOpen,
    HasWorkingChanges,
    TaskSelected,
    TaskHasCommit,
    PullRequestSelected,
    ForgeConnected,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CommandContribution {
    pub id: String,
    pub title: String,
    #[serde(default)]
    pub context: Context,
    #[serde(default)]
    pub when: Vec<Predicate>,
    #[serde(default)]
    pub menu: Option<bool>,
    #[serde(default)]
    pub palette: Option<bool>,
    #[serde(default)]
    pub review_provider: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReviewTarget {
    WorkingCopy,
    FarmTask,
    PullRequest,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReviewScope {
    Committed,
    Uncommitted,
    Tracked,
    IncludeUntracked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReviewProvider {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    pub targets: Vec<ReviewTarget>,
    #[serde(default)]
    pub scopes: Vec<ReviewScope>,
    #[serde(default)]
    pub configuration_files: Vec<String>,
    #[serde(default)]
    pub sends_code_to: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Renderer {
    ReviewFindings,
    ReviewStatus,
    Summary,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Panel {
    pub id: String,
    pub title: String,
    pub renderer: Renderer,
    #[serde(default)]
    pub location: Option<Context>,
    #[serde(default)]
    pub provider: Option<String>,
}

impl Panel {
    /// Where the panel is drawn when the manifest does not say.
    pub fn placed(&self) -> Context {
        self.location.unwrap_or(match self.renderer {
            Renderer::ReviewStatus => Context::PullRequest,
            Renderer::ReviewFindings => Context::WorkingCopy,
            Renderer::Summary => Context::Global,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SettingType {
    Boolean,
    Enum,
    Text,
    Number,
    Executable,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum SettingScope {
    #[default]
    User,
    Repository,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Setting {
    pub key: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub kind: SettingType,
    #[serde(default)]
    pub scope: SettingScope,
    #[serde(default)]
    pub default: Option<Value>,
    #[serde(default)]
    pub values: Vec<String>,
    #[serde(default)]
    pub labels: BTreeMap<String, String>,
    #[serde(default)]
    pub min: Option<f64>,
    #[serde(default)]
    pub max: Option<f64>,
    #[serde(default)]
    pub max_length: Option<u32>,
    #[serde(default)]
    pub tool: Option<String>,
}

impl Setting {
    /// The value used when nobody has chosen one.
    pub fn default_value(&self) -> Value {
        if let Some(value) = &self.default {
            return value.clone();
        }
        match self.kind {
            SettingType::Boolean => Value::Bool(false),
            SettingType::Enum => self
                .values
                .first()
                .map(|v| Value::String(v.clone()))
                .unwrap_or(Value::Null),
            SettingType::Text => Value::String(String::new()),
            SettingType::Number => serde_json::json!(self.min.unwrap_or(0.0)),
            SettingType::Executable => Value::Null,
        }
    }

    /// Check a value somebody wants to store. `Err` says why not.
    pub fn accept(&self, value: &Value) -> std::result::Result<Value, String> {
        match self.kind {
            SettingType::Boolean => value
                .as_bool()
                .map(Value::Bool)
                .ok_or_else(|| "must be on or off".to_string()),
            SettingType::Enum => match value.as_str() {
                Some(choice) if self.values.iter().any(|v| v == choice) => Ok(value.clone()),
                _ => Err(format!("must be one of {}", self.values.join(", "))),
            },
            SettingType::Text => {
                let text = value.as_str().ok_or("must be text")?;
                let limit = self.max_length.unwrap_or(1000) as usize;
                if text.chars().count() > limit {
                    return Err(format!("must be at most {limit} characters"));
                }
                Ok(value.clone())
            }
            SettingType::Number => {
                let number = value.as_f64().ok_or("must be a number")?;
                if !number.is_finite() {
                    return Err("must be a number".into());
                }
                if self.min.is_some_and(|min| number < min)
                    || self.max.is_some_and(|max| number > max)
                {
                    return Err(format!(
                        "must be between {} and {}",
                        self.min
                            .map(|m| m.to_string())
                            .unwrap_or_else(|| "-∞".into()),
                        self.max
                            .map(|m| m.to_string())
                            .unwrap_or_else(|| "∞".into())
                    ));
                }
                Ok(value.clone())
            }
            // Executables are chosen through their own path, which checks the
            // file; a setting write cannot name one.
            SettingType::Executable => Err("is chosen with Choose…, not typed".into()),
        }
    }
}

impl Manifest {
    /// Read and validate a manifest. Every broken rule is reported at once.
    pub fn parse(text: &str) -> Result<Manifest> {
        if text.len() > MAX_BYTES {
            return Err(Error::Manifest(vec![format!(
                "extension.json is larger than {} KiB",
                MAX_BYTES / 1024
            )]));
        }
        let value: Value = serde_json::from_str(text).map_err(|error| {
            Error::Manifest(vec![format!("extension.json is not JSON: {error}")])
        })?;
        let errors = validate(&value);
        if !errors.is_empty() {
            return Err(Error::Manifest(errors));
        }
        serde_json::from_value(value).map_err(|error| Error::Manifest(vec![error.to_string()]))
    }

    pub fn command(&self, id: &str) -> Option<&CommandContribution> {
        self.contributes.commands.iter().find(|c| c.id == id)
    }

    pub fn review_provider(&self, id: &str) -> Option<&ReviewProvider> {
        self.contributes
            .review_providers
            .iter()
            .find(|p| p.id == id)
    }

    pub fn panel(&self, id: &str) -> Option<&Panel> {
        self.contributes.panels.iter().find(|p| p.id == id)
    }

    pub fn setting(&self, key: &str) -> Option<&Setting> {
        self.contributes.settings.iter().find(|s| s.key == key)
    }

    pub fn tool(&self, id: &str) -> Option<&ExternalTool> {
        self.external_tools.iter().find(|t| t.id == id)
    }

    /// The entrypoint for `target`, relative to the package root.
    pub fn entrypoint(&self, target: &str) -> Option<&str> {
        self.runtime.entrypoints.get(target).map(String::as_str)
    }
}

// ── Validation ───────────────────────────────────────────────────────────

/// Every rule the manifest breaks, each prefixed with the field's path.
pub fn validate(value: &Value) -> Vec<String> {
    let mut v = Validator::default();
    v.manifest(value);
    v.errors
}

#[derive(Default)]
struct Validator {
    errors: Vec<String>,
}

const ROOT_KEYS: &[&str] = &[
    "$schema",
    "manifestVersion",
    "id",
    "name",
    "version",
    "publisher",
    "description",
    "license",
    "homepage",
    "engines",
    "runtime",
    "activation",
    "capabilities",
    "externalTools",
    "contributes",
];

impl Validator {
    fn error(&mut self, path: &str, message: impl AsRef<str>) {
        let path = if path.is_empty() { "manifest" } else { path };
        self.errors.push(format!("{path}: {}", message.as_ref()));
    }

    /// The object at `path`, refusing keys outside `allowed` and requiring
    /// those in `required`.
    fn object<'a>(
        &mut self,
        path: &str,
        value: &'a Value,
        allowed: &[&str],
        required: &[&str],
    ) -> Option<&'a Map<String, Value>> {
        let Some(object) = value.as_object() else {
            self.error(path, "must be an object");
            return None;
        };
        for key in object.keys() {
            if !allowed.contains(&key.as_str()) {
                self.error(
                    &join(path, key),
                    "is not a field this manifest version knows",
                );
            }
        }
        for key in required {
            if !object.contains_key(*key) {
                self.error(&join(path, key), "is required");
            }
        }
        Some(object)
    }

    fn string<'a>(
        &mut self,
        path: &str,
        value: Option<&'a Value>,
        min: usize,
        max: usize,
    ) -> Option<&'a str> {
        let value = value?;
        let Some(text) = value.as_str() else {
            self.error(path, "must be a string");
            return None;
        };
        let length = text.chars().count();
        if length < min {
            self.error(
                path,
                if min == 1 {
                    "must not be empty".to_string()
                } else {
                    format!("must be at least {min} characters")
                },
            );
        } else if length > max {
            self.error(path, format!("must be at most {max} characters"));
        }
        Some(text)
    }

    fn array<'a>(&mut self, path: &str, value: Option<&'a Value>, max: usize) -> &'a [Value] {
        let Some(value) = value else { return &[] };
        let Some(items) = value.as_array() else {
            self.error(path, "must be a list");
            return &[];
        };
        if items.len() > max {
            self.error(path, format!("may have at most {max} entries"));
        }
        items
    }

    fn bool(&mut self, path: &str, value: Option<&Value>) {
        if let Some(value) = value {
            if !value.is_boolean() {
                self.error(path, "must be true or false");
            }
        }
    }

    fn one_of(&mut self, path: &str, value: Option<&Value>, allowed: &[&str]) -> Option<String> {
        let text = self.string(path, value, 1, 64)?;
        if allowed.contains(&text) {
            Some(text.to_string())
        } else {
            self.error(path, format!("must be one of {}", allowed.join(", ")));
            None
        }
    }

    fn local_id(&mut self, path: &str, value: Option<&Value>) -> Option<String> {
        let text = self.string(path, value, 1, 64)?;
        if is_local_id(text) {
            Some(text.to_string())
        } else {
            self.error(
                path,
                "must start with a letter and use only letters, digits, '-' and '_'",
            );
            None
        }
    }

    fn https(&mut self, path: &str, value: Option<&Value>) {
        if let Some(text) = self.string(path, value, 1, 300) {
            if !text.starts_with("https://") || text.chars().any(char::is_whitespace) {
                self.error(path, "must be an https:// address");
            }
        }
    }

    fn unique(&mut self, path: &str, what: &str, ids: &[String]) {
        let mut seen = BTreeSet::new();
        for id in ids {
            if !seen.insert(id.as_str()) {
                self.error(path, format!("declares the {what} '{id}' more than once"));
            }
        }
    }

    fn manifest(&mut self, value: &Value) {
        let Some(root) = self.object(
            "",
            value,
            ROOT_KEYS,
            &[
                "manifestVersion",
                "id",
                "name",
                "version",
                "publisher",
                "engines",
                "runtime",
            ],
        ) else {
            return;
        };

        match root.get("manifestVersion") {
            Some(v) if v.as_u64() == Some(crate::MANIFEST_VERSION) => {}
            Some(v) => self.error(
                "manifestVersion",
                format!(
                    "is {v}; this Spagitty reads manifest version {}",
                    crate::MANIFEST_VERSION
                ),
            ),
            None => {}
        }

        if let Some(id) = self.string("id", root.get("id"), 3, 100) {
            if !is_extension_id(id) {
                self.error(
                    "id",
                    "must be two to five lowercase dot-separated labels, such as com.example.hello",
                );
            }
        }
        self.string("name", root.get("name"), 1, 60);
        self.string("publisher", root.get("publisher"), 1, 60);
        self.string("description", root.get("description"), 0, 300);
        self.string("license", root.get("license"), 1, 64);
        if root.contains_key("homepage") {
            self.https("homepage", root.get("homepage"));
        }
        if let Some(version) = self.string("version", root.get("version"), 5, 64) {
            if semver::Version::parse(version).is_err() {
                self.error("version", "must be a semantic version such as 1.0.0");
            }
        }

        if let Some(engines) = root.get("engines") {
            if let Some(engines) = self.object(
                "engines",
                engines,
                &["spagitty", "extensionApi"],
                &["spagitty", "extensionApi"],
            ) {
                for key in ["spagitty", "extensionApi"] {
                    let path = format!("engines.{key}");
                    if let Some(range) = self.string(&path, engines.get(key), 1, 100) {
                        if crate::version::parse_range(range).is_none() {
                            self.error(
                                &path,
                                "is not a version requirement, such as >=0.9.0, <1.0.0",
                            );
                        }
                    }
                }
            }
        }

        if let Some(runtime) = root.get("runtime") {
            self.runtime(runtime);
        }

        let activation = self.array("activation", root.get("activation"), 3);
        let mut seen = Vec::new();
        for (index, entry) in activation.iter().enumerate() {
            if let Some(kind) = self.one_of(
                &format!("activation[{index}]"),
                Some(entry),
                &["onCommand", "onReviewProvider", "onPanel"],
            ) {
                seen.push(kind);
            }
        }
        self.unique("activation", "activation", &seen);

        let capabilities = root
            .get("capabilities")
            .map(|value| self.capabilities(value))
            .unwrap_or_default();

        let tools = self.array("externalTools", root.get("externalTools"), 8);
        let mut tool_ids = Vec::new();
        for (index, tool) in tools.iter().enumerate() {
            if let Some(id) = self.tool(&format!("externalTools[{index}]"), tool) {
                tool_ids.push(id);
            }
        }
        self.unique("externalTools", "tool", &tool_ids);
        if !tool_ids.is_empty() && !capabilities.contains(&Capability::ToolsExecute) {
            self.error(
                "externalTools",
                "declares tools, so capabilities must ask for tools.execute",
            );
        }

        if let Some(contributes) = root.get("contributes") {
            self.contributes(contributes, &capabilities, &tool_ids);
        }
    }

    fn runtime(&mut self, value: &Value) {
        let Some(runtime) = self.object(
            "runtime",
            value,
            &["kind", "entrypoints"],
            &["kind", "entrypoints"],
        ) else {
            return;
        };
        match runtime.get("kind").and_then(Value::as_str) {
            Some("native-process") | None => {}
            Some(other) => self.error(
                "runtime.kind",
                format!("is '{other}'; only native-process is supported"),
            ),
        }
        let Some(entrypoints) = runtime.get("entrypoints") else {
            return;
        };
        let Some(entrypoints) = entrypoints.as_object() else {
            self.error("runtime.entrypoints", "must be an object of target to path");
            return;
        };
        if entrypoints.is_empty() {
            self.error("runtime.entrypoints", "must name at least one target");
        }
        for (target, path) in entrypoints {
            let at = format!("runtime.entrypoints.{target}");
            if !TARGETS.contains(&target.as_str()) {
                self.error(
                    &at,
                    format!("is not a target; use one of {}", TARGETS.join(", ")),
                );
            }
            if let Some(path) = self.string(&at, Some(path), 1, 200) {
                if !is_package_path(path) {
                    self.error(
                        &at,
                        "must be a relative path inside the package, with forward slashes",
                    );
                }
            }
        }
    }

    fn capabilities(&mut self, value: &Value) -> Vec<Capability> {
        let Some(object) = self.object("capabilities", value, &["required", "optional"], &[])
        else {
            return Vec::new();
        };
        let mut all = Vec::new();
        for key in ["required", "optional"] {
            let path = format!("capabilities.{key}");
            let mut names = Vec::new();
            for (index, entry) in self.array(&path, object.get(key), 5).iter().enumerate() {
                let at = format!("{path}[{index}]");
                match entry.as_str().map(|name| (name, Capability::parse(name))) {
                    Some((_, Some(capability))) => {
                        if all.contains(&capability) {
                            self.error(&at, format!("{capability} is listed more than once"));
                        }
                        all.push(capability);
                        names.push(capability.as_str().to_string());
                    }
                    Some((name, None)) => self.error(
                        &at,
                        format!("'{name}' is not a capability this Spagitty knows"),
                    ),
                    None => self.error(&at, "must be a string"),
                }
            }
        }
        all
    }

    fn tool(&mut self, path: &str, value: &Value) -> Option<String> {
        let tool = self.object(
            path,
            value,
            &[
                "id",
                "name",
                "executableNames",
                "versionArgs",
                "minimumVersion",
                "installUrl",
                "profiles",
            ],
            &["id", "executableNames", "profiles"],
        )?;
        let id = self.local_id(&join(path, "id"), tool.get("id"));
        self.string(&join(path, "name"), tool.get("name"), 1, 60);
        let names = self.array(
            &join(path, "executableNames"),
            tool.get("executableNames"),
            4,
        );
        if tool.contains_key("executableNames") && names.is_empty() {
            self.error(
                &join(path, "executableNames"),
                "must name at least one executable",
            );
        }
        for (index, name) in names.iter().enumerate() {
            let at = format!("{path}.executableNames[{index}]");
            if let Some(name) = self.string(&at, Some(name), 1, 64) {
                if !name
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
                {
                    self.error(&at, "must be a bare file name, never a path");
                }
            }
        }
        for (index, arg) in self
            .array(&join(path, "versionArgs"), tool.get("versionArgs"), 4)
            .iter()
            .enumerate()
        {
            self.literal(&format!("{path}.versionArgs[{index}]"), arg);
        }
        if let Some(version) = self.string(
            &join(path, "minimumVersion"),
            tool.get("minimumVersion"),
            5,
            64,
        ) {
            if semver::Version::parse(version).is_err() {
                self.error(&join(path, "minimumVersion"), "must be a semantic version");
            }
        }
        if tool.contains_key("installUrl") {
            self.https(&join(path, "installUrl"), tool.get("installUrl"));
        }
        let profiles = self.array(&join(path, "profiles"), tool.get("profiles"), 16);
        if tool.contains_key("profiles") && profiles.is_empty() {
            self.error(&join(path, "profiles"), "must declare at least one profile");
        }
        let mut ids = Vec::new();
        for (index, profile) in profiles.iter().enumerate() {
            if let Some(id) = self.profile(&format!("{path}.profiles[{index}]"), profile) {
                ids.push(id);
            }
        }
        self.unique(&join(path, "profiles"), "profile", &ids);
        id
    }

    fn literal(&mut self, path: &str, value: &Value) {
        if let Some(text) = self.string(path, Some(value), 1, 128) {
            if text.chars().any(|c| c.is_whitespace() || c == '\0') {
                self.error(path, "must be one argument with no spaces");
            }
        }
    }

    fn profile(&mut self, path: &str, value: &Value) -> Option<String> {
        let profile = self.object(
            path,
            value,
            &["id", "args", "options", "workdir", "timeoutMs"],
            &["id", "args"],
        )?;
        let id = self.local_id(&join(path, "id"), profile.get("id"));
        for (index, arg) in self
            .array(&join(path, "args"), profile.get("args"), 16)
            .iter()
            .enumerate()
        {
            self.literal(&format!("{path}.args[{index}]"), arg);
        }
        if profile.contains_key("workdir") {
            self.one_of(
                &join(path, "workdir"),
                profile.get("workdir"),
                &["operation", "none"],
            );
        }
        if let Some(timeout) = profile.get("timeoutMs") {
            match timeout.as_u64() {
                Some(ms) if (1000..=7_200_000).contains(&ms) => {}
                _ => self.error(&join(path, "timeoutMs"), "must be between 1000 and 7200000"),
            }
        }
        if let Some(options) = profile.get("options") {
            let at = join(path, "options");
            match options.as_object() {
                Some(options) => {
                    if options.len() > 8 {
                        self.error(&at, "may declare at most 8 options");
                    }
                    for (name, option) in options {
                        let here = join(&at, name);
                        if !is_local_id(name) {
                            self.error(&here, "is not a valid option name");
                        }
                        self.option(&here, option);
                    }
                }
                None => self.error(&at, "must be an object"),
            }
        }
        id
    }

    fn option(&mut self, path: &str, value: &Value) {
        let kind = value.get("type").and_then(Value::as_str);
        match kind {
            Some("enum") => {
                let Some(option) = self.object(
                    path,
                    value,
                    &["type", "values", "required"],
                    &["type", "values"],
                ) else {
                    return;
                };
                self.bool(&join(path, "required"), option.get("required"));
                match option.get("values").and_then(Value::as_object) {
                    Some(values) if !values.is_empty() && values.len() <= 16 => {
                        for (name, args) in values {
                            let at = join(&join(path, "values"), name);
                            if !is_local_id(name) {
                                self.error(&at, "is not a valid value name");
                            }
                            for (index, arg) in self.array(&at, Some(args), 4).iter().enumerate() {
                                self.literal(&format!("{at}[{index}]"), arg);
                            }
                        }
                    }
                    Some(_) => {
                        self.error(&join(path, "values"), "must have between 1 and 16 values")
                    }
                    None if option.contains_key("values") => self.error(
                        &join(path, "values"),
                        "must be an object of value to arguments",
                    ),
                    None => {}
                }
            }
            Some("revision") | Some("commit") => {
                let Some(option) = self.object(
                    path,
                    value,
                    &["type", "flag", "required"],
                    &["type", "flag"],
                ) else {
                    return;
                };
                self.bool(&join(path, "required"), option.get("required"));
                if let Some(flag) = self.string(&join(path, "flag"), option.get("flag"), 2, 43) {
                    if !is_flag(flag) {
                        self.error(&join(path, "flag"), "must be a flag such as --base");
                    }
                }
            }
            _ => self.error(&join(path, "type"), "must be enum, revision or commit"),
        }
    }

    fn contributes(&mut self, value: &Value, capabilities: &[Capability], tools: &[String]) {
        let Some(contributes) = self.object(
            "contributes",
            value,
            &["commands", "reviewProviders", "panels", "settings"],
            &[],
        ) else {
            return;
        };

        // Providers first: commands and panels refer to them.
        let mut providers = Vec::new();
        let list = self.array(
            "contributes.reviewProviders",
            contributes.get("reviewProviders"),
            4,
        );
        for (index, provider) in list.iter().enumerate() {
            let path = format!("contributes.reviewProviders[{index}]");
            let Some(object) = self.object(
                &path,
                provider,
                &[
                    "id",
                    "title",
                    "targets",
                    "scopes",
                    "configurationFiles",
                    "sendsCodeTo",
                ],
                &["id", "targets"],
            ) else {
                continue;
            };
            if let Some(id) = self.local_id(&join(&path, "id"), object.get("id")) {
                providers.push(id);
            }
            self.string(&join(&path, "title"), object.get("title"), 1, 60);
            self.string(
                &join(&path, "sendsCodeTo"),
                object.get("sendsCodeTo"),
                1,
                120,
            );
            let targets = self.array(&join(&path, "targets"), object.get("targets"), 3);
            if object.contains_key("targets") && targets.is_empty() {
                self.error(&join(&path, "targets"), "must name at least one target");
            }
            let mut seen = Vec::new();
            for (i, target) in targets.iter().enumerate() {
                if let Some(t) = self.one_of(
                    &format!("{path}.targets[{i}]"),
                    Some(target),
                    &["workingCopy", "farmTask", "pullRequest"],
                ) {
                    seen.push(t);
                }
            }
            self.unique(&join(&path, "targets"), "target", &seen);
            let mut seen = Vec::new();
            for (i, scope) in self
                .array(&join(&path, "scopes"), object.get("scopes"), 4)
                .iter()
                .enumerate()
            {
                if let Some(s) = self.one_of(
                    &format!("{path}.scopes[{i}]"),
                    Some(scope),
                    &["committed", "uncommitted", "tracked", "includeUntracked"],
                ) {
                    seen.push(s);
                }
            }
            self.unique(&join(&path, "scopes"), "scope", &seen);
            for (i, file) in self
                .array(
                    &join(&path, "configurationFiles"),
                    object.get("configurationFiles"),
                    16,
                )
                .iter()
                .enumerate()
            {
                let at = format!("{path}.configurationFiles[{i}]");
                if let Some(file) = self.string(&at, Some(file), 1, 200) {
                    if !is_package_path(file) {
                        self.error(&at, "must be a relative path inside the repository");
                    }
                }
            }
        }
        self.unique("contributes.reviewProviders", "review provider", &providers);
        if !providers.is_empty() && !capabilities.contains(&Capability::ReviewProvide) {
            self.error(
                "contributes.reviewProviders",
                "needs the review.provide capability",
            );
        }

        let mut commands = Vec::new();
        let list = self.array("contributes.commands", contributes.get("commands"), 50);
        for (index, command) in list.iter().enumerate() {
            let path = format!("contributes.commands[{index}]");
            let Some(object) = self.object(
                &path,
                command,
                &[
                    "id",
                    "title",
                    "context",
                    "when",
                    "menu",
                    "palette",
                    "reviewProvider",
                ],
                &["id", "title"],
            ) else {
                continue;
            };
            if let Some(id) = self.local_id(&join(&path, "id"), object.get("id")) {
                commands.push(id);
            }
            self.string(&join(&path, "title"), object.get("title"), 1, 80);
            if object.contains_key("context") {
                self.one_of(&join(&path, "context"), object.get("context"), CONTEXTS);
            }
            let mut seen = Vec::new();
            for (i, when) in self
                .array(&join(&path, "when"), object.get("when"), 6)
                .iter()
                .enumerate()
            {
                if let Some(w) = self.one_of(&format!("{path}.when[{i}]"), Some(when), PREDICATES) {
                    seen.push(w);
                }
            }
            self.unique(&join(&path, "when"), "condition", &seen);
            self.bool(&join(&path, "menu"), object.get("menu"));
            self.bool(&join(&path, "palette"), object.get("palette"));
            if object.contains_key("reviewProvider") {
                if let Some(provider) =
                    self.local_id(&join(&path, "reviewProvider"), object.get("reviewProvider"))
                {
                    if !providers.contains(&provider) {
                        self.error(
                            &join(&path, "reviewProvider"),
                            format!("names '{provider}', which is not declared"),
                        );
                    }
                }
            }
        }
        self.unique("contributes.commands", "command", &commands);

        let mut panels = Vec::new();
        let list = self.array("contributes.panels", contributes.get("panels"), 10);
        for (index, panel) in list.iter().enumerate() {
            let path = format!("contributes.panels[{index}]");
            let Some(object) = self.object(
                &path,
                panel,
                &["id", "title", "renderer", "location", "provider"],
                &["id", "title", "renderer"],
            ) else {
                continue;
            };
            if let Some(id) = self.local_id(&join(&path, "id"), object.get("id")) {
                panels.push(id);
            }
            self.string(&join(&path, "title"), object.get("title"), 1, 60);
            let renderer = self.one_of(
                &join(&path, "renderer"),
                object.get("renderer"),
                &["reviewFindings", "reviewStatus", "summary"],
            );
            if object.contains_key("location") {
                self.one_of(&join(&path, "location"), object.get("location"), CONTEXTS);
            }
            let provider = if object.contains_key("provider") {
                self.local_id(&join(&path, "provider"), object.get("provider"))
            } else {
                None
            };
            if let Some(provider) = &provider {
                if !providers.contains(provider) {
                    self.error(
                        &join(&path, "provider"),
                        format!("names '{provider}', which is not declared"),
                    );
                }
            }
            if renderer.as_deref() == Some("reviewFindings")
                && !object.contains_key("provider")
                && providers.len() != 1
            {
                self.error(
                    &join(&path, "provider"),
                    "is required: a findings panel shows one review provider's results",
                );
            }
        }
        self.unique("contributes.panels", "panel", &panels);

        let mut keys = Vec::new();
        let list = self.array("contributes.settings", contributes.get("settings"), 50);
        for (index, setting) in list.iter().enumerate() {
            let path = format!("contributes.settings[{index}]");
            if let Some(key) = self.setting(&path, setting, tools) {
                keys.push(key);
            }
        }
        self.unique("contributes.settings", "setting", &keys);
    }

    fn setting(&mut self, path: &str, value: &Value, tools: &[String]) -> Option<String> {
        let setting = self.object(
            path,
            value,
            &[
                "key",
                "title",
                "description",
                "type",
                "scope",
                "default",
                "values",
                "labels",
                "min",
                "max",
                "maxLength",
                "tool",
            ],
            &["key", "type"],
        )?;
        let key = self.local_id(&join(path, "key"), setting.get("key"));
        self.string(&join(path, "title"), setting.get("title"), 1, 60);
        self.string(
            &join(path, "description"),
            setting.get("description"),
            0,
            200,
        );
        if setting.contains_key("scope") {
            self.one_of(
                &join(path, "scope"),
                setting.get("scope"),
                &["user", "repository"],
            );
        }
        let kind = self.one_of(
            &join(path, "type"),
            setting.get("type"),
            &["boolean", "enum", "text", "number", "executable"],
        );
        let default = setting.get("default");
        let mut values = Vec::new();
        for (i, entry) in self
            .array(&join(path, "values"), setting.get("values"), 20)
            .iter()
            .enumerate()
        {
            let at = format!("{path}.values[{i}]");
            if let Some(text) = self.string(&at, Some(entry), 1, 40) {
                if !text
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
                {
                    self.error(&at, "must use only letters, digits, '-' and '_'");
                }
                values.push(text.to_string());
            }
        }
        self.unique(&join(path, "values"), "value", &values);
        if let Some(labels) = setting.get("labels") {
            match labels.as_object() {
                Some(labels) => {
                    for (name, label) in labels {
                        let at = join(&join(path, "labels"), name);
                        if !values.contains(name) {
                            self.error(&at, "labels a value that is not in values");
                        }
                        self.string(&at, Some(label), 1, 60);
                    }
                }
                None => self.error(&join(path, "labels"), "must be an object"),
            }
        }
        let number = |v: Option<&Value>| v.and_then(Value::as_f64);
        for bound in ["min", "max"] {
            if let Some(b) = setting.get(bound) {
                if !b.is_number() {
                    self.error(&join(path, bound), "must be a number");
                }
            }
        }
        if let Some(limit) = setting.get("maxLength") {
            match limit.as_u64() {
                Some(1..=1000) => {}
                _ => self.error(&join(path, "maxLength"), "must be between 1 and 1000"),
            }
        }

        match kind.as_deref() {
            Some("boolean") => {
                if default.is_some_and(|d| !d.is_boolean()) {
                    self.error(&join(path, "default"), "must be true or false");
                }
            }
            Some("enum") => {
                if values.is_empty() {
                    self.error(&join(path, "values"), "is required for an enum setting");
                }
                if let Some(d) = default {
                    if !d.as_str().is_some_and(|d| values.iter().any(|v| v == d)) {
                        self.error(&join(path, "default"), "must be one of values");
                    }
                }
            }
            Some("text") => {
                if let Some(d) = default {
                    match d.as_str() {
                        Some(text) => {
                            let limit = setting
                                .get("maxLength")
                                .and_then(Value::as_u64)
                                .unwrap_or(1000) as usize;
                            if text.chars().count() > limit {
                                self.error(&join(path, "default"), "is longer than maxLength");
                            }
                        }
                        None => self.error(&join(path, "default"), "must be text"),
                    }
                }
            }
            Some("number") => {
                let (min, max) = (number(setting.get("min")), number(setting.get("max")));
                if let (Some(min), Some(max)) = (min, max) {
                    if min > max {
                        self.error(&join(path, "min"), "is greater than max");
                    }
                }
                if let Some(d) = default {
                    match d.as_f64() {
                        Some(n) if min.is_some_and(|m| n < m) || max.is_some_and(|m| n > m) => {
                            self.error(&join(path, "default"), "is outside min and max")
                        }
                        Some(_) => {}
                        None => self.error(&join(path, "default"), "must be a number"),
                    }
                }
            }
            Some("executable") => {
                if default.is_some() {
                    self.error(
                        &join(path, "default"),
                        "is not allowed: an executable is always chosen by the user",
                    );
                }
                match setting.get("tool").and_then(Value::as_str) {
                    Some(tool) if tools.iter().any(|t| t == tool) => {}
                    Some(tool) => self.error(
                        &join(path, "tool"),
                        format!("names '{tool}', which is not declared in externalTools"),
                    ),
                    None => {
                        self.error(&join(path, "tool"), "is required for an executable setting")
                    }
                }
            }
            _ => {}
        }
        if setting.contains_key("tool") && kind.as_deref() != Some("executable") {
            self.error(&join(path, "tool"), "only applies to an executable setting");
        }
        key
    }
}

const CONTEXTS: &[&str] = &["global", "workingCopy", "farmTask", "pullRequest"];
const PREDICATES: &[&str] = &[
    "repositoryOpen",
    "hasWorkingChanges",
    "taskSelected",
    "taskHasCommit",
    "pullRequestSelected",
    "forgeConnected",
];

fn join(path: &str, key: &str) -> String {
    if path.is_empty() {
        key.to_string()
    } else {
        format!("{path}.{key}")
    }
}

/// Two to five dot-separated labels of lowercase letters and digits, hyphens
/// inside a label only.
pub fn is_extension_id(id: &str) -> bool {
    if id.len() > 100 {
        return false;
    }
    let labels: Vec<&str> = id.split('.').collect();
    (2..=5).contains(&labels.len())
        && labels.iter().all(|label| {
            !label.is_empty()
                && !label.starts_with('-')
                && !label.ends_with('-')
                && !label.contains("--")
                && label
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
        })
}

/// A contribution, profile, option or setting name.
pub fn is_local_id(id: &str) -> bool {
    let mut chars = id.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic())
        && id.len() <= 64
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-'))
}

/// A relative path with forward slashes and no `.` or `..` segment.
pub fn is_package_path(path: &str) -> bool {
    !path.is_empty()
        && path.len() <= 200
        && path.split('/').all(|segment| {
            !segment.is_empty()
                && segment != "."
                && segment != ".."
                && segment
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        })
}

fn is_flag(flag: &str) -> bool {
    let body = flag.strip_prefix("--").or_else(|| flag.strip_prefix('-'));
    match body {
        Some(body) => {
            let mut chars = body.chars();
            matches!(chars.next(), Some(c) if c.is_ascii_alphanumeric())
                && body.len() <= 41
                && chars.all(|c| c.is_ascii_alphanumeric() || c == '-')
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn minimal() -> Value {
        serde_json::json!({
            "manifestVersion": 1,
            "id": "com.example.hello",
            "name": "Hello",
            "version": "1.0.0",
            "publisher": "Example",
            "engines": {"spagitty": ">=0.9.0, <1.0.0", "extensionApi": "^1.0.0"},
            "runtime": {"kind": "native-process", "entrypoints": {"x86_64-pc-windows-msvc": "bin/hello.exe"}}
        })
    }

    fn errors(value: Value) -> Vec<String> {
        validate(&value)
    }

    #[test]
    fn the_smallest_manifest_is_valid() {
        assert_eq!(errors(minimal()), Vec::<String>::new());
        let manifest = Manifest::parse(&minimal().to_string()).unwrap();
        assert_eq!(manifest.id, "com.example.hello");
        assert_eq!(
            manifest.entrypoint("x86_64-pc-windows-msvc"),
            Some("bin/hello.exe")
        );
    }

    #[test]
    fn every_broken_rule_is_reported_at_once_with_its_path() {
        let mut value = minimal();
        value["id"] = "Hello".into();
        value["version"] = "one".into();
        value["colour"] = "red".into();
        let found = errors(value);
        assert!(found.iter().any(|e| e.starts_with("id:")), "{found:?}");
        assert!(found.iter().any(|e| e.starts_with("version:")), "{found:?}");
        assert!(found.iter().any(|e| e.starts_with("colour:")), "{found:?}");
    }

    #[test]
    fn extension_ids_are_reverse_dns() {
        for good in [
            "com.example.hello",
            "spagitty.coderabbit",
            "a.b",
            "io.x-y.z9",
        ] {
            assert!(is_extension_id(good), "{good}");
        }
        for bad in [
            "hello",
            "Com.Example",
            "a..b",
            "-a.b",
            "a.b-",
            "a.b.c.d.e.f",
            "a_b.c",
            "a.b/c",
            "",
        ] {
            assert!(!is_extension_id(bad), "{bad}");
        }
    }

    #[test]
    fn package_paths_cannot_climb_or_be_absolute() {
        for good in ["bin/x.exe", "a", "bin/linux-x64/worker"] {
            assert!(is_package_path(good), "{good}");
        }
        for bad in [
            "/bin/x",
            "../x",
            "bin/../x",
            "bin\\x.exe",
            "C:/x",
            "./x",
            "bin//x",
            "bin/ x",
            "",
        ] {
            assert!(!is_package_path(bad), "{bad}");
        }
    }

    #[test]
    fn an_unknown_capability_is_an_error_not_a_hint() {
        let mut value = minimal();
        value["capabilities"] = serde_json::json!({"required": ["filesystem.write"]});
        let found = errors(value);
        assert!(
            found.iter().any(|e| e.contains("filesystem.write")),
            "{found:?}"
        );
    }

    #[test]
    fn a_capability_listed_twice_is_refused() {
        let mut value = minimal();
        value["capabilities"] =
            serde_json::json!({"required": ["repository.read"], "optional": ["repository.read"]});
        assert!(!errors(value).is_empty());
    }

    #[test]
    fn a_target_outside_the_list_is_refused() {
        let mut value = minimal();
        value["runtime"]["entrypoints"] = serde_json::json!({"riscv64-unknown-linux-gnu": "bin/x"});
        assert!(errors(value).iter().any(|e| e.contains("riscv64")));
    }

    #[test]
    fn contributions_must_reference_what_exists() {
        let mut value = minimal();
        value["contributes"] = serde_json::json!({
            "commands": [{"id": "go", "title": "Go", "reviewProvider": "missing"}],
            "panels": [{"id": "p", "title": "P", "renderer": "reviewFindings", "provider": "missing"}],
            "settings": [{"key": "path", "type": "executable", "tool": "nope"}]
        });
        let found = errors(value);
        assert_eq!(
            found.iter().filter(|e| e.contains("not declared")).count(),
            3,
            "{found:?}"
        );
    }

    #[test]
    fn review_providers_and_tools_need_their_capabilities() {
        let mut value = minimal();
        value["externalTools"] = serde_json::json!([
            {"id": "t", "executableNames": ["t"], "profiles": [{"id": "run", "args": ["go"]}]}
        ]);
        value["contributes"] =
            serde_json::json!({"reviewProviders": [{"id": "r", "targets": ["workingCopy"]}]});
        let found = errors(value);
        assert!(
            found.iter().any(|e| e.contains("tools.execute")),
            "{found:?}"
        );
        assert!(
            found.iter().any(|e| e.contains("review.provide")),
            "{found:?}"
        );
    }

    #[test]
    fn duplicate_contribution_ids_are_refused() {
        let mut value = minimal();
        value["contributes"] = serde_json::json!({
            "commands": [{"id": "go", "title": "Go"}, {"id": "go", "title": "Go again"}]
        });
        assert!(errors(value).iter().any(|e| e.contains("more than once")));
    }

    #[test]
    fn a_tool_argument_is_one_word_and_an_executable_is_a_bare_name() {
        let mut value = minimal();
        value["capabilities"] = serde_json::json!({"required": ["tools.execute"]});
        value["externalTools"] = serde_json::json!([{
            "id": "t", "executableNames": ["/usr/bin/t"],
            "profiles": [{"id": "run", "args": ["review; rm -rf /"],
                          "options": {"base": {"type": "revision", "flag": "base"}}}]
        }]);
        let found = errors(value);
        assert!(
            found.iter().any(|e| e.contains("bare file name")),
            "{found:?}"
        );
        assert!(found.iter().any(|e| e.contains("no spaces")), "{found:?}");
        assert!(found.iter().any(|e| e.contains("flag")), "{found:?}");
    }

    #[test]
    fn settings_defaults_must_fit_their_type() {
        let mut value = minimal();
        value["contributes"] = serde_json::json!({"settings": [
            {"key": "region", "type": "enum", "values": ["us", "eu"], "default": "mars"},
            {"key": "limit", "type": "number", "min": 5, "max": 1, "default": 9},
            {"key": "on", "type": "boolean", "default": "yes"}
        ]});
        let found = errors(value);
        assert_eq!(found.len(), 4, "{found:?}");
    }

    #[test]
    fn a_setting_accepts_only_values_its_declaration_allows() {
        let enum_setting = Setting {
            key: "region".into(),
            title: None,
            description: None,
            kind: SettingType::Enum,
            scope: SettingScope::User,
            default: None,
            values: vec!["us".into(), "eu".into()],
            labels: BTreeMap::new(),
            min: None,
            max: None,
            max_length: None,
            tool: None,
        };
        assert!(enum_setting.accept(&"eu".into()).is_ok());
        assert!(enum_setting.accept(&"mars".into()).is_err());
        assert_eq!(enum_setting.default_value(), Value::from("us"));

        let number = Setting {
            kind: SettingType::Number,
            min: Some(1.0),
            max: Some(10.0),
            values: vec![],
            ..enum_setting.clone()
        };
        assert!(number.accept(&serde_json::json!(5)).is_ok());
        assert!(number.accept(&serde_json::json!(11)).is_err());
        assert!(number.accept(&"5".into()).is_err());

        let text = Setting {
            kind: SettingType::Text,
            max_length: Some(3),
            values: vec![],
            ..enum_setting.clone()
        };
        assert!(text.accept(&"abc".into()).is_ok());
        assert!(text.accept(&"abcd".into()).is_err());

        let exe = Setting {
            kind: SettingType::Executable,
            values: vec![],
            ..enum_setting
        };
        assert!(exe.accept(&"/bin/sh".into()).is_err());
        assert_eq!(exe.default_value(), Value::Null);
    }

    #[test]
    fn a_manifest_that_is_not_json_says_so() {
        match Manifest::parse("{not json") {
            Err(Error::Manifest(errors)) => assert!(errors[0].contains("not JSON")),
            other => panic!("{other:?}"),
        }
        assert!(Manifest::parse(&" ".repeat(MAX_BYTES + 1)).is_err());
    }

    #[test]
    fn panels_are_placed_where_their_renderer_belongs_by_default() {
        let panel = Panel {
            id: "p".into(),
            title: "P".into(),
            renderer: Renderer::ReviewStatus,
            location: None,
            provider: None,
        };
        assert_eq!(panel.placed(), Context::PullRequest);
        let panel = Panel {
            renderer: Renderer::Summary,
            ..panel
        };
        assert_eq!(panel.placed(), Context::Global);
        let panel = Panel {
            location: Some(Context::FarmTask),
            ..panel
        };
        assert_eq!(panel.placed(), Context::FarmTask);
    }
}
