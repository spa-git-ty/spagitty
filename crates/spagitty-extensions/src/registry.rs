// SPDX-License-Identifier: GPL-3.0-or-later

//! Which extensions exist on this machine, and where.
//!
//! Three sources, one identity each:
//!
//! - **Bundled** — shipped inside Spagitty's own resources, trusted because
//!   the release process put them there. Their identities are reserved: a local
//!   import or a development directory cannot take one, whatever its manifest
//!   claims, and only these carry the official badge. They are updated by
//!   updating Spagitty.
//! - **Local** — imported from a `.spagitty-extension` file, kept in immutable
//!   version directories under `<data>/packages/<id>/<version>/`, with the
//!   installed version recorded as a pointer in the user state.
//! - **Development** — a directory the author attached explicitly. Never
//!   installed, never packaged, and never found by looking in a repository.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::manifest::Manifest;
use crate::package::{self, Validated};
use crate::storage::{Installation, Store};
use crate::{Error, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Provenance {
    Bundled,
    Local,
    Development,
}

#[derive(Debug, Clone)]
pub struct Entry {
    pub manifest: Manifest,
    pub dir: PathBuf,
    pub provenance: Provenance,
}

/// Where things live.
#[derive(Debug, Clone)]
pub struct Paths {
    /// `<application data>/extensions`.
    pub data: PathBuf,
    /// The bundled packages in the application's resources, when there are any.
    pub bundled: Option<PathBuf>,
    /// The directory of the running executable, where a bundled package's
    /// program may have been placed by the platform's packaging instead.
    pub exe_dir: Option<PathBuf>,
}

impl Paths {
    pub fn packages(&self) -> PathBuf {
        self.data.join("packages")
    }

    pub fn package_dir(&self, id: &str, version: &str) -> PathBuf {
        self.packages().join(id).join(version)
    }
}

/// A problem with one entry that keeps it out of the list.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Unreadable {
    pub source: String,
    pub reason: String,
}

/// Everything known, and everything that could not be read.
pub fn discover(paths: &Paths, store: &Store) -> (Vec<Entry>, Vec<Unreadable>) {
    let mut entries: Vec<Entry> = Vec::new();
    let mut problems = Vec::new();

    if let Some(bundled) = &paths.bundled {
        if let Ok(dirs) = std::fs::read_dir(bundled) {
            let mut dirs: Vec<PathBuf> = dirs
                .flatten()
                .map(|d| d.path())
                .filter(|p| p.is_dir())
                .collect();
            dirs.sort();
            for dir in dirs {
                match package::read_directory(&dir) {
                    Ok(manifest) => entries.push(Entry {
                        manifest,
                        dir,
                        provenance: Provenance::Bundled,
                    }),
                    Err(error) => problems.push(Unreadable {
                        source: dir.display().to_string(),
                        reason: error.to_string(),
                    }),
                }
            }
        }
    }

    let (installed, development) = store.read(|s| (s.installed.clone(), s.development.clone()));
    for (id, installation) in installed {
        let dir = paths.package_dir(&id, &installation.current);
        match package::read_directory(&dir) {
            Ok(manifest) if manifest.id == id => {
                if entries.iter().any(|e| e.manifest.id == id) {
                    problems.push(Unreadable {
                        source: id.clone(),
                        reason: "its identity belongs to a bundled extension".into(),
                    });
                } else {
                    entries.push(Entry {
                        manifest,
                        dir,
                        provenance: Provenance::Local,
                    });
                }
            }
            Ok(_) => problems.push(Unreadable {
                source: id.clone(),
                reason: "its installed files name a different extension".into(),
            }),
            Err(error) => problems.push(Unreadable {
                source: id.clone(),
                reason: error.to_string(),
            }),
        }
    }

    for (id, dir) in development {
        match package::read_directory(&dir) {
            Ok(manifest) if manifest.id == id => {
                if entries.iter().any(|e| e.manifest.id == id) {
                    problems.push(Unreadable {
                        source: dir.display().to_string(),
                        reason: format!("{id} is already installed; remove it before attaching a development copy"),
                    });
                } else {
                    entries.push(Entry {
                        manifest,
                        dir,
                        provenance: Provenance::Development,
                    });
                }
            }
            Ok(manifest) => problems.push(Unreadable {
                source: dir.display().to_string(),
                reason: format!(
                    "its manifest now says {}, not {id}; attach it again",
                    manifest.id
                ),
            }),
            Err(error) => problems.push(Unreadable {
                source: dir.display().to_string(),
                reason: error.to_string(),
            }),
        }
    }

    (entries, problems)
}

/// The program to start for `entry` on `target`.
///
/// A bundled package's program may have been moved next to Spagitty's own
/// executable by the platform's packaging, which is where code signing covers
/// it; that location is looked at for bundled packages only.
pub fn program(entry: &Entry, target: &str, paths: &Paths) -> Option<PathBuf> {
    let relative = entry.manifest.entrypoint(target)?;
    let inside = entry
        .dir
        .join(relative.replace('/', std::path::MAIN_SEPARATOR_STR));
    if inside.is_file() {
        return Some(inside);
    }
    if entry.provenance == Provenance::Bundled {
        let name = Path::new(relative).file_name()?;
        let beside = paths.exe_dir.as_ref()?.join(name);
        if beside.is_file() {
            return Some(beside);
        }
    }
    None
}

/// What an install or update did.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Installed {
    pub id: String,
    pub version: String,
    /// The version it replaced, when it was an update.
    pub previous: Option<String>,
}

/// Install a validated package, or update to it.
///
/// The new version is extracted into a staging directory and moved into
/// place, and only then does the pointer move. The previous version stays on
/// disk until the next successful update so that rollback is a pointer change.
/// A failure at any step leaves the previous installation exactly as it was.
pub fn install(
    paths: &Paths,
    store: &Store,
    package: &Validated,
    reserved: &[String],
) -> Result<Installed> {
    let id = package.manifest.id.clone();
    let version = package.manifest.version.clone();
    if reserved.contains(&id) {
        return Err(Error::Conflict(format!(
            "{id} is part of Spagitty and is updated with Spagitty; a file cannot replace it."
        )));
    }
    if store.read(|s| s.development.contains_key(&id)) {
        return Err(Error::Conflict(format!(
            "{id} is attached for development; detach it first."
        )));
    }
    let current = store.read(|s| s.installed.get(&id).cloned());
    if let Some(current) = &current {
        if current.current == version {
            if current.digests.get(&version) == Some(&package.digest) {
                return Err(Error::Refused(format!(
                    "{id} {version} is already installed."
                )));
            }
            return Err(Error::Refused(format!(
                "A different {id} {version} is already installed. A changed package needs a new version number."
            )));
        }
    }

    let target = paths.package_dir(&id, &version);
    let staging = paths
        .packages()
        .join(format!(".staging-{}-{}", std::process::id(), now_ms()));
    package::extract(package, &staging)?;
    if target.exists() {
        // Left over from a version that was installed and rolled back past;
        // replaced only after the new copy is safely extracted.
        remove_package_dir(paths, &target)?;
    }
    std::fs::create_dir_all(target.parent().expect("version directory has a parent"))?;
    if let Err(error) = std::fs::rename(&staging, &target) {
        let _ = std::fs::remove_dir_all(&staging);
        return Err(Error::Io(format!(
            "could not move the new version into place: {error}"
        )));
    }

    let previous = current.as_ref().map(|c| c.current.clone());
    let asked: std::collections::BTreeSet<_> = package.manifest.capabilities.requested().collect();
    let saved = store.update(|s| {
        let entry = s
            .installed
            .entry(id.clone())
            .or_insert_with(Installation::default);
        let superseded = entry.previous.take();
        entry.previous = previous.clone();
        entry.current = version.clone();
        entry
            .digests
            .insert(version.clone(), package.digest.clone());
        if let Some(from) = &previous {
            s.carry_grants(&id, from, &version, &asked);
        }
        superseded
    });
    match saved {
        Ok(superseded) => {
            // The version two updates ago is no longer reachable by rollback.
            if let Some(old) =
                superseded.filter(|old| Some(old) != previous.as_ref() && old != &version)
            {
                let _ = remove_package_dir(paths, &paths.package_dir(&id, &old));
                let _ = store.update(|s| {
                    if let Some(entry) = s.installed.get_mut(&id) {
                        entry.digests.remove(&old);
                    }
                });
            }
            Ok(Installed {
                id,
                version,
                previous,
            })
        }
        Err(error) => {
            // The pointer did not move; take the files back out.
            if current.is_none() {
                let _ = remove_package_dir(paths, &paths.packages().join(&id));
            } else {
                let _ = remove_package_dir(paths, &target);
            }
            Err(error)
        }
    }
}

/// Point an extension back at the version before its last update.
pub fn rollback(paths: &Paths, store: &Store, id: &str) -> Result<Installed> {
    let installation = store
        .read(|s| s.installed.get(id).cloned())
        .ok_or_else(|| Error::NotInstalled(id.into()))?;
    let previous = installation
        .previous
        .clone()
        .ok_or_else(|| Error::Refused(format!("{id} has no earlier version to go back to.")))?;
    if package::read_directory(&paths.package_dir(id, &previous)).is_err() {
        return Err(Error::Refused(format!(
            "{id} {previous} is no longer on disk."
        )));
    }
    store.update(|s| {
        if let Some(entry) = s.installed.get_mut(id) {
            entry.previous = Some(entry.current.clone());
            entry.current = previous.clone();
        }
    })?;
    Ok(Installed {
        id: id.into(),
        version: previous,
        previous: Some(installation.current),
    })
}

/// Remove an installed extension's files and every decision about it.
pub fn uninstall(paths: &Paths, store: &Store, id: &str) -> Result<()> {
    if !store.read(|s| s.installed.contains_key(id)) {
        return Err(Error::NotInstalled(id.into()));
    }
    remove_package_dir(paths, &paths.packages().join(id))?;
    store.update(|s| s.forget(id))?;
    Ok(())
}

/// Remove a directory, but only one that is really inside the packages
/// directory and is not a link — the one rule that keeps an uninstall from
/// deleting something it does not own.
fn remove_package_dir(paths: &Paths, dir: &Path) -> Result<()> {
    let root = std::fs::canonicalize(paths.packages())?;
    let meta = match std::fs::symlink_metadata(dir) {
        Ok(meta) => meta,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    if meta.file_type().is_symlink() || !meta.is_dir() {
        return Err(Error::Refused(format!(
            "{} is not an extension directory; it was left alone.",
            dir.display()
        )));
    }
    let canonical = std::fs::canonicalize(dir)?;
    if !canonical.starts_with(&root) || canonical == root {
        return Err(Error::Refused(format!(
            "{} is outside the extensions directory; it was left alone.",
            dir.display()
        )));
    }
    std::fs::remove_dir_all(canonical)?;
    Ok(())
}

/// Attach a development directory.
pub fn attach(store: &Store, dir: &Path, reserved: &[String]) -> Result<Manifest> {
    let dir = std::fs::canonicalize(dir)
        .map_err(|_| Error::Refused(format!("{} does not exist.", dir.display())))?;
    let manifest = package::read_directory(&dir)?;
    if reserved.contains(&manifest.id) {
        return Err(Error::Conflict(format!(
            "{} belongs to an extension that ships with Spagitty.",
            manifest.id
        )));
    }
    if store.read(|s| s.installed.contains_key(&manifest.id)) {
        return Err(Error::Conflict(format!(
            "{} is installed; remove it before attaching a development copy.",
            manifest.id
        )));
    }
    let id = manifest.id.clone();
    store.update(|s| {
        s.development.insert(id, dir);
    })?;
    Ok(manifest)
}

pub fn detach(store: &Store, id: &str) -> Result<()> {
    if !store.read(|s| s.development.contains_key(id)) {
        return Err(Error::NotInstalled(id.into()));
    }
    store.update(|s| {
        s.development.remove(id);
        let installed = s.installed.contains_key(id);
        if !installed {
            s.forget(id);
        }
    })?;
    Ok(())
}

fn now_ms() -> u128 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::capabilities::Capability;

    pub fn manifest(id: &str, version: &str, capabilities: &[&str]) -> String {
        serde_json::json!({
            "manifestVersion": 1, "id": id, "name": "T", "version": version, "publisher": "P", "license": "MIT",
            "engines": {"spagitty": ">=0.1.0", "extensionApi": "^1.0.0"},
            "runtime": {"kind": "native-process", "entrypoints": {crate::version::current_target(): "bin/w"}},
            "capabilities": {"required": capabilities}
        })
        .to_string()
    }

    fn package_bytes(id: &str, version: &str, capabilities: &[&str], payload: &[u8]) -> Vec<u8> {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("extension.json"),
            manifest(id, version, capabilities),
        )
        .unwrap();
        std::fs::create_dir_all(dir.path().join("bin")).unwrap();
        std::fs::write(dir.path().join("bin").join("w"), payload).unwrap();
        package::pack_directory(dir.path()).unwrap()
    }

    fn setup() -> (tempfile::TempDir, Paths, Store) {
        let dir = tempfile::tempdir().unwrap();
        let paths = Paths {
            data: dir.path().join("extensions"),
            bundled: None,
            exe_dir: None,
        };
        std::fs::create_dir_all(&paths.data).unwrap();
        let store = Store::open(&paths.data);
        (dir, paths, store)
    }

    #[test]
    fn install_update_rollback_and_uninstall() {
        let (_dir, paths, store) = setup();
        let v1 = package::validate(package_bytes(
            "com.example.t",
            "1.0.0",
            &["repository.read"],
            b"\x7fELF one",
        ))
        .unwrap();
        install(&paths, &store, &v1, &[]).unwrap();
        store
            .update(|s| s.grant("com.example.t", "1.0.0", "repo", Capability::RepositoryRead))
            .unwrap();

        let v2 = package::validate(package_bytes(
            "com.example.t",
            "2.0.0",
            &["repository.read", "tools.execute"],
            b"\x7fELF two",
        ))
        .unwrap();
        let installed = install(&paths, &store, &v2, &[]).unwrap();
        assert_eq!(installed.previous.as_deref(), Some("1.0.0"));
        let granted = store.read(|s| s.granted("com.example.t", "2.0.0", "repo"));
        assert!(granted.contains(&Capability::RepositoryRead));
        assert!(
            !granted.contains(&Capability::ToolsExecute),
            "the added capability needs a person"
        );

        let (entries, problems) = discover(&paths, &store);
        assert!(problems.is_empty(), "{problems:?}");
        assert_eq!(entries[0].manifest.version, "2.0.0");

        rollback(&paths, &store, "com.example.t").unwrap();
        let (entries, _) = discover(&paths, &store);
        assert_eq!(entries[0].manifest.version, "1.0.0");

        uninstall(&paths, &store, "com.example.t").unwrap();
        assert!(!paths.packages().join("com.example.t").exists());
        assert!(store.read(|s| s.grants.is_empty()));
        assert!(discover(&paths, &store).0.is_empty());
    }

    #[test]
    fn the_same_version_cannot_be_installed_twice_or_changed_in_place() {
        let (_dir, paths, store) = setup();
        let v1 = package::validate(package_bytes("com.example.t", "1.0.0", &[], b"\x7fELF one"))
            .unwrap();
        install(&paths, &store, &v1, &[]).unwrap();
        let again = package::validate(package_bytes("com.example.t", "1.0.0", &[], b"\x7fELF one"))
            .unwrap();
        assert!(install(&paths, &store, &again, &[]).is_err());
        let changed = package::validate(package_bytes(
            "com.example.t",
            "1.0.0",
            &[],
            b"\x7fELF changed",
        ))
        .unwrap();
        assert!(install(&paths, &store, &changed, &[])
            .unwrap_err()
            .to_string()
            .contains("new version"));
    }

    #[test]
    fn a_bundled_identity_cannot_be_taken_by_an_import_or_a_development_copy() {
        let (_dir, paths, store) = setup();
        let reserved = vec!["spagitty.coderabbit".to_string()];
        let impostor = package::validate(package_bytes(
            "spagitty.coderabbit",
            "9.0.0",
            &[],
            b"\x7fELF",
        ))
        .unwrap();
        assert!(matches!(
            install(&paths, &store, &impostor, &reserved),
            Err(Error::Conflict(_))
        ));

        let dev = tempfile::tempdir().unwrap();
        std::fs::write(
            dev.path().join("extension.json"),
            manifest("spagitty.coderabbit", "1.0.0", &[]),
        )
        .unwrap();
        assert!(matches!(
            attach(&store, dev.path(), &reserved),
            Err(Error::Conflict(_))
        ));
    }

    #[test]
    fn a_development_directory_is_attached_and_detached_explicitly() {
        let (_dir, paths, store) = setup();
        let dev = tempfile::tempdir().unwrap();
        std::fs::write(
            dev.path().join("extension.json"),
            manifest("com.example.dev", "0.1.0", &[]),
        )
        .unwrap();
        attach(&store, dev.path(), &[]).unwrap();
        let (entries, _) = discover(&paths, &store);
        assert_eq!(entries[0].provenance, Provenance::Development);
        let installed =
            package::validate(package_bytes("com.example.dev", "1.0.0", &[], b"\x7fELF")).unwrap();
        assert!(
            install(&paths, &store, &installed, &[]).is_err(),
            "one owner per identity"
        );
        detach(&store, "com.example.dev").unwrap();
        assert!(discover(&paths, &store).0.is_empty());
    }

    #[test]
    fn a_bundled_package_is_found_in_resources_and_its_program_beside_the_executable() {
        let (_dir, mut paths, store) = setup();
        let resources = tempfile::tempdir().unwrap();
        let pkg = resources.path().join("coderabbit");
        std::fs::create_dir_all(&pkg).unwrap();
        std::fs::write(
            pkg.join("extension.json"),
            manifest("spagitty.coderabbit", "1.0.0", &[]),
        )
        .unwrap();
        let exe_dir = tempfile::tempdir().unwrap();
        std::fs::write(exe_dir.path().join("w"), b"program").unwrap();
        paths.bundled = Some(resources.path().to_path_buf());
        paths.exe_dir = Some(exe_dir.path().to_path_buf());
        let (entries, _) = discover(&paths, &store);
        assert_eq!(entries[0].provenance, Provenance::Bundled);
        let found = program(&entries[0], crate::version::current_target(), &paths).unwrap();
        assert_eq!(found, exe_dir.path().join("w"));
        assert!(program(&entries[0], "aarch64-apple-darwin-not", &paths).is_none());
    }

    #[test]
    fn uninstall_never_follows_a_link_out_of_the_packages_directory() {
        let (_dir, paths, store) = setup();
        let outside = tempfile::tempdir().unwrap();
        std::fs::write(outside.path().join("keep"), b"mine").unwrap();
        std::fs::create_dir_all(paths.packages()).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(outside.path(), paths.packages().join("com.example.t")).unwrap();
        #[cfg(windows)]
        if std::os::windows::fs::symlink_dir(outside.path(), paths.packages().join("com.example.t"))
            .is_err()
        {
            return; // creating links needs a privilege the runner may not have
        }
        store
            .update(|s| {
                s.installed.insert(
                    "com.example.t".into(),
                    Installation {
                        current: "1.0.0".into(),
                        ..Default::default()
                    },
                );
            })
            .unwrap();
        assert!(uninstall(&paths, &store, "com.example.t").is_err());
        assert!(outside.path().join("keep").exists());
    }
}
