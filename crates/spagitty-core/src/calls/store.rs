// SPDX-License-Identifier: GPL-3.0-or-later

//! The calls folder on disk: `.spagitty/calls/` unless Settings says
//! otherwise (proposal §5.5).
//!
//! ```text
//! <folder>/
//!   orders-api.json      a collection
//!   env/staging.json     an environment's plain values
//!   shared.json          variables every environment shares
//!   settings.json        this repository's overrides (FEAT-165)
//!   examples/            saved responses (FEAT-164)
//! ```
//!
//! A file that does not parse, or was written by a newer Spagitty, is listed
//! with what is wrong with it and is never written over: the person's work
//! is in it.

use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::calls::model::{to_text, Collection, Environment, Shared, VERSION};
use crate::{Error, Result};

/// The folder's default, relative to the working tree.
pub const DEFAULT_FOLDER: &str = ".spagitty/calls";
/// Files in the folder that are not collections.
const NOT_COLLECTIONS: [&str; 2] = ["shared.json", "settings.json"];

/// A file read from the folder: what it holds, or why it could not be read.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Stored<T> {
    /// The file's name without `.json`: what it is written back as.
    pub stem: String,
    /// The path from the working tree, with `/`, for the screen to show.
    pub file: String,
    pub value: Option<T>,
    pub error: Option<String>,
}

/// The calls folder of a working tree.
#[derive(Debug, Clone)]
pub struct Folder {
    pub workdir: PathBuf,
    /// Relative to `workdir`.
    pub relative: String,
}

impl Folder {
    /// `relative` is cleaned: no leading `/`, no `..`. An empty one is the
    /// default.
    pub fn new(workdir: &Path, relative: &str) -> Result<Self> {
        let cleaned = relative.trim().trim_matches('/').replace('\\', "/");
        let cleaned = if cleaned.is_empty() {
            DEFAULT_FOLDER.to_string()
        } else {
            cleaned
        };
        if cleaned
            .split('/')
            .any(|part| part == ".." || part.is_empty())
            || Path::new(&cleaned).is_absolute()
        {
            return Err(Error::Calls(format!(
                "{cleaned} is not a folder inside the repository"
            )));
        }
        Ok(Folder {
            workdir: workdir.to_path_buf(),
            relative: cleaned,
        })
    }

    pub fn path(&self) -> PathBuf {
        self.workdir.join(&self.relative)
    }

    fn shown(&self, rest: &str) -> String {
        format!("{}/{rest}", self.relative)
    }

    pub fn collections(&self) -> Vec<Stored<Collection>> {
        let mut out: Vec<Stored<Collection>> = json_files(&self.path())
            .into_iter()
            .filter(|name| !NOT_COLLECTIONS.contains(&name.as_str()))
            .map(|name| read_stored(&self.path().join(&name), &name, self.shown(&name)))
            .collect();
        out.sort_by_key(|stored| stored.stem.to_lowercase());
        out
    }

    pub fn collection(&self, stem: &str) -> Result<Collection> {
        let name = format!("{}.json", checked(stem)?);
        let stored: Stored<Collection> =
            read_stored(&self.path().join(&name), &name, self.shown(&name));
        match (stored.value, stored.error) {
            (Some(value), _) => Ok(value),
            (None, error) => Err(Error::Calls(
                error.unwrap_or_else(|| format!("{name} was not found")),
            )),
        }
    }

    /// Write a collection under `stem`, moving it from `was` when it was
    /// renamed. Returns the stem it was written under.
    pub fn write_collection(
        &self,
        collection: &Collection,
        stem: Option<&str>,
        was: Option<&str>,
    ) -> Result<String> {
        let stem = match stem {
            Some(stem) => checked(stem)?.to_string(),
            None => slug(&collection.name),
        };
        let file = self.path().join(format!("{stem}.json"));
        refuse_unreadable::<Collection>(&file)?;
        if was != Some(stem.as_str()) && file.exists() {
            return Err(Error::Calls(format!(
                "{} already exists",
                self.shown(&format!("{stem}.json"))
            )));
        }
        let mut collection = collection.clone();
        collection.v = VERSION;
        write_text(&file, &to_text(&collection))?;
        if let Some(old) = was.filter(|old| *old != stem) {
            let _ = std::fs::remove_file(self.path().join(format!("{}.json", checked(old)?)));
        }
        Ok(stem)
    }

    pub fn delete_collection(&self, stem: &str) -> Result<()> {
        remove(&self.path().join(format!("{}.json", checked(stem)?)))
    }

    pub fn environments(&self) -> Vec<Stored<Environment>> {
        let dir = self.path().join("env");
        let mut out: Vec<Stored<Environment>> = json_files(&dir)
            .into_iter()
            .map(|name| read_stored(&dir.join(&name), &name, self.shown(&format!("env/{name}"))))
            .collect();
        out.sort_by_key(|stored| stored.stem.to_lowercase());
        out
    }

    /// Write an environment, emptied of secret and kept values, under its
    /// name; `was` is its name before a rename.
    pub fn write_environment(&self, environment: &Environment, was: Option<&str>) -> Result<()> {
        let stem = checked(&environment.name)?;
        let file = self.path().join("env").join(format!("{stem}.json"));
        refuse_unreadable::<Environment>(&file)?;
        if was != Some(stem) && file.exists() {
            return Err(Error::Calls(format!(
                "an environment named {stem} already exists"
            )));
        }
        let mut written = environment.for_file();
        written.v = VERSION;
        write_text(&file, &to_text(&written))?;
        if let Some(old) = was.filter(|old| *old != stem) {
            let _ = std::fs::remove_file(
                self.path()
                    .join("env")
                    .join(format!("{}.json", checked(old)?)),
            );
        }
        Ok(())
    }

    pub fn delete_environment(&self, name: &str) -> Result<()> {
        remove(
            &self
                .path()
                .join("env")
                .join(format!("{}.json", checked(name)?)),
        )
    }

    /// `shared.json`, or nothing shared when there is none.
    pub fn shared(&self) -> Result<Shared> {
        let file = self.path().join("shared.json");
        if !file.exists() {
            return Ok(Shared {
                v: VERSION,
                vars: Vec::new(),
            });
        }
        read_json(&file)
    }

    pub fn write_shared(&self, shared: &Shared) -> Result<()> {
        let file = self.path().join("shared.json");
        refuse_unreadable::<Shared>(&file)?;
        let mut written = shared.for_file();
        written.v = VERSION;
        write_text(&file, &to_text(&written))
    }

    /// The repository's `settings.json`, as JSON, when there is one.
    pub fn settings(&self) -> Option<serde_json::Value> {
        let text = std::fs::read_to_string(self.path().join("settings.json")).ok()?;
        serde_json::from_str(&text).ok()
    }
}

/// A file name made from what a person called something: lower case, words
/// joined by `-`, nothing that means anything to a shell or a path.
pub fn slug(name: &str) -> String {
    let mut out = String::new();
    let mut dash = false;
    for c in name.trim().chars() {
        if c.is_alphanumeric() || c == '_' || c == '.' {
            if dash && !out.is_empty() {
                out.push('-');
            }
            dash = false;
            out.extend(c.to_lowercase());
        } else {
            dash = true;
        }
    }
    let out = out.trim_matches('.').to_string();
    if out.is_empty() {
        "collection".to_string()
    } else {
        out
    }
}

/// A stem that names a file in the folder and nothing outside it.
fn checked(stem: &str) -> Result<&str> {
    if stem.is_empty()
        || stem.contains(['/', '\\', ':'])
        || stem.starts_with('.')
        || stem.chars().any(|c| c.is_control())
    {
        return Err(Error::Calls(format!("{stem:?} cannot be a file name")));
    }
    Ok(stem)
}

fn json_files(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.file_type().map(|t| t.is_file()).unwrap_or(false))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.ends_with(".json") && !name.starts_with('.'))
        .collect()
}

fn read_stored<T: DeserializeOwned>(path: &Path, name: &str, shown: String) -> Stored<T> {
    let stem = name.trim_end_matches(".json").to_string();
    match read_json::<T>(path) {
        Ok(value) => Stored {
            stem,
            file: shown,
            value: Some(value),
            error: None,
        },
        Err(error) => Stored {
            stem,
            file: shown,
            value: None,
            error: Some(error.to_string()),
        },
    }
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T> {
    let text = std::fs::read_to_string(path)?;
    let value: serde_json::Value = serde_json::from_str(&text)
        .map_err(|e| Error::Calls(format!("{} is not JSON: {e}", file_name(path))))?;
    if let Some(v) = value.get("v").and_then(|v| v.as_u64()) {
        if v > VERSION as u64 {
            return Err(Error::Calls(format!(
                "{} was written by a newer Spagitty (format {v}); it is left as it is",
                file_name(path)
            )));
        }
    }
    serde_json::from_value(value)
        .map_err(|e| Error::Calls(format!("{} could not be read: {e}", file_name(path))))
}

/// Refuse to write over a file that is there and cannot be read.
fn refuse_unreadable<T: DeserializeOwned>(path: &Path) -> Result<()> {
    if path.exists() {
        read_json::<T>(path).map_err(|e| Error::Calls(format!("{e}; nothing was written")))?;
    }
    Ok(())
}

fn file_name(path: &Path) -> String {
    path.file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default()
}

/// Write by renaming a finished file into place, so a crash halfway leaves
/// the old file rather than half of the new one.
pub fn write_text(path: &Path, text: &str) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let partial = path.with_extension("json.partial");
    std::fs::write(&partial, text)?;
    std::fs::rename(&partial, path)?;
    Ok(())
}

fn remove(path: &Path) -> Result<()> {
    match std::fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::calls::model::{Colour, Item, Variable};

    fn folder() -> (tempfile::TempDir, Folder) {
        let dir = tempfile::tempdir().unwrap();
        let folder = Folder::new(dir.path(), "").unwrap();
        (dir, folder)
    }

    fn collection(name: &str) -> Collection {
        Collection {
            v: VERSION,
            name: name.into(),
            wsdl: None,
            vars: Vec::new(),
            items: Vec::<Item>::new(),
        }
    }

    #[test]
    fn a_collection_is_written_under_its_slug_and_listed() {
        let (dir, folder) = folder();
        let stem = folder
            .write_collection(&collection("Orders API"), None, None)
            .unwrap();
        assert_eq!(stem, "orders-api");
        assert!(dir.path().join(".spagitty/calls/orders-api.json").exists());
        let listed = folder.collections();
        assert_eq!(listed.len(), 1);
        assert_eq!(listed[0].file, ".spagitty/calls/orders-api.json");
        assert_eq!(listed[0].value.as_ref().unwrap().name, "Orders API");
        // A second with the same name is refused rather than written over.
        assert!(folder
            .write_collection(&collection("orders api"), None, None)
            .is_err());
    }

    #[test]
    fn a_rename_moves_the_file() {
        let (_dir, folder) = folder();
        folder
            .write_collection(&collection("a"), None, None)
            .unwrap();
        folder
            .write_collection(&collection("b"), Some("b"), Some("a"))
            .unwrap();
        let stems: Vec<_> = folder.collections().into_iter().map(|c| c.stem).collect();
        assert_eq!(stems, vec!["b"]);
    }

    #[test]
    fn a_broken_or_newer_file_is_listed_and_never_written_over() {
        let (dir, folder) = folder();
        let path = dir.path().join(".spagitty/calls");
        std::fs::create_dir_all(&path).unwrap();
        std::fs::write(path.join("broken.json"), "{ not json").unwrap();
        std::fs::write(path.join("newer.json"), "{\"v\": 9, \"name\": \"n\"}").unwrap();
        std::fs::write(path.join("shared.json"), "{\"v\":1,\"vars\":[]}").unwrap();
        let listed = folder.collections();
        assert_eq!(listed.len(), 2);
        assert!(listed[0].error.as_ref().unwrap().contains("not JSON"));
        assert!(listed[1].error.as_ref().unwrap().contains("newer Spagitty"));
        let refused = folder.write_collection(&collection("x"), Some("broken"), Some("broken"));
        assert!(refused.is_err());
        assert_eq!(
            std::fs::read_to_string(path.join("broken.json")).unwrap(),
            "{ not json"
        );
        let refused = folder.write_collection(&collection("x"), Some("newer"), Some("other"));
        assert!(refused.is_err());
        assert!(std::fs::read_to_string(path.join("newer.json"))
            .unwrap()
            .contains("\"v\": 9"));
    }

    #[test]
    fn environments_hold_names_and_never_private_values() {
        let (dir, folder) = folder();
        let mut env = Environment::new("staging", Colour::Warn);
        env.vars.push(Variable::plain("baseUrl", "https://staging"));
        env.vars.push(Variable {
            name: "password".into(),
            value: Some("hunter22".into()),
            secret: true,
            ..Variable::default()
        });
        folder.write_environment(&env, None).unwrap();
        let text =
            std::fs::read_to_string(dir.path().join(".spagitty/calls/env/staging.json")).unwrap();
        assert!(!text.contains("hunter22"));
        let read = folder.environments();
        assert_eq!(read[0].value.as_ref().unwrap().vars[1].value, None);
        assert!(read[0].value.as_ref().unwrap().vars[1].secret);

        let mut shared = Shared::default();
        shared.vars.push(Variable {
            name: "apiKey".into(),
            value: Some("sk-live".into()),
            secret: true,
            ..Variable::default()
        });
        folder.write_shared(&shared).unwrap();
        let text = std::fs::read_to_string(dir.path().join(".spagitty/calls/shared.json")).unwrap();
        assert!(!text.contains("sk-live"));
    }

    #[test]
    fn the_folder_stays_inside_the_repository() {
        let dir = tempfile::tempdir().unwrap();
        assert!(Folder::new(dir.path(), "../outside").is_err());
        assert_eq!(Folder::new(dir.path(), "/http/").unwrap().relative, "http");
        assert!(folder().1.collection("../x").is_err());
    }

    #[test]
    fn slugs_are_plain() {
        assert_eq!(slug("Orders API"), "orders-api");
        assert_eq!(slug("  billing / ws  "), "billing-ws");
        assert_eq!(slug("..."), "collection");
    }
}
