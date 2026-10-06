// SPDX-License-Identifier: GPL-3.0-or-later

//! A `.spagitty-extension` package: what is in it, whether it is safe, and
//! putting it on disk.
//!
//! A package is a ZIP ([`crate::zip`]) holding:
//!
//! ```text
//! extension.json          the manifest
//! integrity.json          {"version":1,"algorithm":"sha256","files":{path: hex}}
//! bin/<target>/<program>  one executable per target the manifest names
//! LICENSE, NOTICE, README.md and anything else the extension needs
//! ```
//!
//! **Checksums prove integrity, not identity.** `integrity.json` shows the
//! files are the ones the packer hashed. It says nothing about who that was,
//! and nothing here pretends it does: a package's publisher is whatever its
//! manifest claims, shown as a claim.
//!
//! Validation happens in full before anything is written:
//!
//! 1. the container's own rules ([`crate::zip::Archive::open`]);
//! 2. a valid manifest;
//! 3. an integrity index that lists exactly the files in the archive, each
//!    with the hash it actually has;
//! 4. every declared entrypoint present;
//! 5. nothing executable that is not a declared entrypoint — judged by the
//!    archive's mode bits, by extension, and by the file's first bytes, since a
//!    renamed binary is still a binary.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::manifest::Manifest;
use crate::zip::Archive;
use crate::{Error, Result};

pub const EXTENSION: &str = "spagitty-extension";
pub const MANIFEST: &str = "extension.json";
pub const INTEGRITY: &str = "integrity.json";

const EXECUTABLE_SUFFIXES: &[&str] = &[
    ".exe", ".dll", ".so", ".dylib", ".bat", ".cmd", ".ps1", ".sh", ".com", ".msi", ".scr", ".node",
];

/// `integrity.json`.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Integrity {
    pub version: u32,
    pub algorithm: String,
    pub files: BTreeMap<String, String>,
}

/// One file in a package, as the install dialog lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageFile {
    pub path: String,
    pub size: u64,
    pub sha256: String,
    pub executable: bool,
}

/// A package that passed every check.
pub struct Validated {
    pub manifest: Manifest,
    /// SHA-256 of the whole archive, to tell two copies of a version apart.
    pub digest: String,
    pub files: Vec<PackageFile>,
    /// Things worth saying that are not reasons to refuse.
    pub warnings: Vec<String>,
    archive: Archive,
}

impl std::fmt::Debug for Validated {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Validated")
            .field("id", &self.manifest.id)
            .field("version", &self.manifest.version)
            .field("files", &self.files.len())
            .finish()
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

/// Whether `bytes` begin like a program: PE, ELF, Mach-O, or a script.
pub fn looks_executable(bytes: &[u8]) -> bool {
    let head = |n: usize| bytes.get(..n);
    head(2) == Some(b"MZ")
        || head(4) == Some(b"\x7fELF")
        || head(2) == Some(b"#!")
        || matches!(
            head(4),
            Some([0xfe, 0xed, 0xfa, 0xce])
                | Some([0xfe, 0xed, 0xfa, 0xcf])
                | Some([0xce, 0xfa, 0xed, 0xfe])
                | Some([0xcf, 0xfa, 0xed, 0xfe])
                | Some([0xca, 0xfe, 0xba, 0xbe])
        )
}

fn refuse(why: impl Into<String>) -> Error {
    Error::Package(why.into())
}

/// Check a package from its bytes. Nothing is written.
pub fn validate(bytes: Vec<u8>) -> Result<Validated> {
    let digest = sha256_hex(&bytes);
    let archive = Archive::open(bytes).map_err(refuse)?;

    let manifest_entry = archive
        .find(MANIFEST)
        .ok_or_else(|| refuse("it has no extension.json at its root"))?;
    let manifest_text = String::from_utf8(archive.read(manifest_entry).map_err(refuse)?)
        .map_err(|_| refuse("extension.json is not UTF-8"))?;
    let manifest = Manifest::parse(&manifest_text)?;

    let integrity_entry = archive
        .find(INTEGRITY)
        .ok_or_else(|| refuse("it has no integrity.json; repack it with `bun run ext pack`"))?;
    let integrity: Integrity =
        serde_json::from_slice(&archive.read(integrity_entry).map_err(refuse)?)
            .map_err(|e| refuse(format!("integrity.json is not valid: {e}")))?;
    if integrity.version != 1 || integrity.algorithm != "sha256" {
        return Err(refuse(
            "integrity.json must be version 1 with algorithm sha256",
        ));
    }

    let entrypoints: Vec<&str> = manifest
        .runtime
        .entrypoints
        .values()
        .map(String::as_str)
        .collect();
    let mut files = Vec::new();
    let mut listed = integrity.files.clone();
    for entry in archive.entries().iter().filter(|e| !e.is_dir) {
        if entry.name == INTEGRITY {
            continue;
        }
        let bytes = archive.read(entry).map_err(refuse)?;
        let sha256 = sha256_hex(&bytes);
        match listed.remove(&entry.name) {
            Some(expected) if expected.eq_ignore_ascii_case(&sha256) => {}
            Some(_) => {
                return Err(refuse(format!(
                    "{} does not match its checksum in integrity.json — the package is damaged or was changed after packing",
                    entry.name
                )))
            }
            None => return Err(refuse(format!("{} is not listed in integrity.json", entry.name))),
        }
        let lower = entry.name.to_ascii_lowercase();
        let executable = entry.is_executable()
            || EXECUTABLE_SUFFIXES
                .iter()
                .any(|suffix| lower.ends_with(suffix))
            || looks_executable(&bytes);
        let declared = entrypoints.contains(&entry.name.as_str());
        if executable && !declared {
            return Err(refuse(format!(
                "{} is a program the manifest does not declare as an entrypoint",
                entry.name
            )));
        }
        files.push(PackageFile {
            path: entry.name.clone(),
            size: entry.size,
            sha256,
            executable: declared,
        });
    }
    if let Some(missing) = listed.keys().next() {
        return Err(refuse(format!(
            "integrity.json lists {missing}, which is not in the package"
        )));
    }
    for (target, path) in &manifest.runtime.entrypoints {
        if !files.iter().any(|f| &f.path == path) {
            return Err(refuse(format!(
                "the program for {target} ({path}) is missing"
            )));
        }
    }

    let mut warnings = Vec::new();
    if manifest.license.is_none()
        && !files
            .iter()
            .any(|f| f.path.to_ascii_uppercase().starts_with("LICENSE"))
    {
        warnings.push("It states no licence.".into());
    }

    Ok(Validated {
        manifest,
        digest,
        files,
        warnings,
        archive,
    })
}

/// Write a validated package into `dir`, which must not exist.
///
/// Files are created owner-only where the platform supports it, and the
/// entrypoints are made executable. `dir` is written whole or not at all: a
/// failure removes what was written.
pub fn extract(package: &Validated, dir: &Path) -> Result<()> {
    if dir.exists() {
        return Err(Error::Io(format!("{} already exists", dir.display())));
    }
    std::fs::create_dir_all(dir)?;
    restrict_dir(dir);
    let written = (|| -> Result<()> {
        for entry in package.archive.entries() {
            if entry.is_dir {
                continue;
            }
            // Names were checked when the archive was opened; joining one can
            // only produce a path inside `dir`.
            let target = dir.join(entry.name.replace('/', std::path::MAIN_SEPARATOR_STR));
            debug_assert!(target.starts_with(dir));
            if let Some(parent) = target.parent() {
                std::fs::create_dir_all(parent)?;
                restrict_dir(parent);
            }
            let bytes = package.archive.read(entry).map_err(refuse)?;
            std::fs::write(&target, bytes)?;
            let executable = package
                .files
                .iter()
                .any(|f| f.path == entry.name && f.executable);
            restrict_file(&target, executable);
        }
        Ok(())
    })();
    if written.is_err() {
        let _ = std::fs::remove_dir_all(dir);
    }
    written
}

#[cfg(unix)]
fn restrict_dir(path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o700));
}

#[cfg(not(unix))]
fn restrict_dir(_path: &Path) {}

#[cfg(unix)]
fn restrict_file(path: &Path, executable: bool) {
    use std::os::unix::fs::PermissionsExt;
    let mode = if executable { 0o700 } else { 0o600 };
    let _ = std::fs::set_permissions(path, std::fs::Permissions::from_mode(mode));
}

#[cfg(not(unix))]
fn restrict_file(_path: &Path, _executable: bool) {}

/// Read a package from a development directory: the same manifest rules, no
/// archive and no integrity index, because the author is still changing it.
pub fn read_directory(dir: &Path) -> Result<Manifest> {
    let text = std::fs::read_to_string(dir.join(MANIFEST))
        .map_err(|_| Error::Refused(format!("{} has no extension.json", dir.display())))?;
    Manifest::parse(&text)
}

/// Build a package from a directory, the way `ext pack` does: every file
/// under it, an integrity index, entrypoints marked executable. Used by the
/// host's tests and by tools that want a package without the TypeScript kit.
pub fn pack_directory(dir: &Path) -> Result<Vec<u8>> {
    let manifest = read_directory(dir)?;
    let entrypoints: Vec<String> = manifest.runtime.entrypoints.values().cloned().collect();
    let mut files: Vec<(String, PathBuf)> = Vec::new();
    collect(dir, dir, &mut files)?;
    files.sort();
    let mut integrity = Integrity {
        version: 1,
        algorithm: "sha256".into(),
        files: BTreeMap::new(),
    };
    let mut writer = crate::zip::Writer::new();
    for (name, path) in &files {
        if name == INTEGRITY {
            continue;
        }
        let bytes = std::fs::read(path)?;
        integrity.files.insert(name.clone(), sha256_hex(&bytes));
        let mode = if entrypoints.contains(name) {
            0o100755
        } else {
            0o100644
        };
        writer.file(name, &bytes, Some(mode), true);
    }
    let index = serde_json::to_vec_pretty(&integrity).map_err(|e| Error::Io(e.to_string()))?;
    writer.file(INTEGRITY, &index, Some(0o100644), true);
    Ok(writer.finish())
}

fn collect(root: &Path, dir: &Path, out: &mut Vec<(String, PathBuf)>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            return Err(Error::Package(format!(
                "{} is a link; packages cannot contain links",
                path.display()
            )));
        }
        if kind.is_dir() {
            collect(root, &path, out)?;
        } else {
            let name = path
                .strip_prefix(root)
                .map_err(|e| Error::Io(e.to_string()))?
                .components()
                .map(|c| c.as_os_str().to_string_lossy().into_owned())
                .collect::<Vec<_>>()
                .join("/");
            out.push((name, path));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::zip::Writer;

    const TARGET: &str = "x86_64-unknown-linux-gnu";

    fn manifest() -> String {
        serde_json::json!({
            "manifestVersion": 1, "id": "com.example.hello", "name": "Hello", "version": "1.0.0",
            "publisher": "Example", "license": "MIT",
            "engines": {"spagitty": ">=0.8.0", "extensionApi": "^1.0.0"},
            "runtime": {"kind": "native-process", "entrypoints": {TARGET: "bin/hello"}}
        })
        .to_string()
    }

    /// A package with the given files, an honest integrity index unless
    /// `index` overrides it.
    fn package(
        files: &[(&str, &[u8], Option<u32>)],
        index: Option<BTreeMap<String, String>>,
    ) -> Vec<u8> {
        let mut writer = Writer::new();
        let mut honest = BTreeMap::new();
        for (name, bytes, mode) in files {
            honest.insert(name.to_string(), sha256_hex(bytes));
            writer.file(name, bytes, *mode, true);
        }
        let integrity = Integrity {
            version: 1,
            algorithm: "sha256".into(),
            files: index.unwrap_or(honest),
        };
        writer.file(
            INTEGRITY,
            &serde_json::to_vec(&integrity).unwrap(),
            None,
            false,
        );
        writer.finish()
    }

    const PROGRAM: &[u8] = b"\x7fELF fake program";

    fn good() -> Vec<u8> {
        package(
            &[
                (MANIFEST, manifest().as_bytes(), None),
                ("bin/hello", PROGRAM, Some(0o100755)),
                ("README.md", b"# hi", None),
            ],
            None,
        )
    }

    #[test]
    fn a_well_formed_package_validates_and_extracts() {
        let package = validate(good()).unwrap();
        assert_eq!(package.manifest.id, "com.example.hello");
        assert_eq!(package.files.len(), 3);
        assert!(package
            .files
            .iter()
            .any(|f| f.path == "bin/hello" && f.executable));
        assert!(package.warnings.is_empty());

        let dir = tempfile::tempdir().unwrap();
        let into = dir.path().join("1.0.0");
        extract(&package, &into).unwrap();
        assert_eq!(
            std::fs::read(into.join("bin").join("hello")).unwrap(),
            PROGRAM
        );
        assert!(
            extract(&package, &into).is_err(),
            "an existing directory is never overwritten"
        );
    }

    #[test]
    fn a_file_changed_after_packing_is_caught() {
        let mut index = BTreeMap::new();
        index.insert(MANIFEST.to_string(), sha256_hex(manifest().as_bytes()));
        index.insert("bin/hello".to_string(), sha256_hex(b"something else"));
        let bytes = package(
            &[
                (MANIFEST, manifest().as_bytes(), None),
                ("bin/hello", PROGRAM, Some(0o100755)),
            ],
            Some(index),
        );
        assert!(validate(bytes)
            .unwrap_err()
            .to_string()
            .contains("checksum"));
    }

    #[test]
    fn an_unlisted_file_and_a_missing_listed_file_are_both_refused() {
        let mut index = BTreeMap::new();
        index.insert(MANIFEST.to_string(), sha256_hex(manifest().as_bytes()));
        let unlisted = package(
            &[
                (MANIFEST, manifest().as_bytes(), None),
                ("bin/hello", PROGRAM, Some(0o100755)),
            ],
            Some(index.clone()),
        );
        assert!(validate(unlisted)
            .unwrap_err()
            .to_string()
            .contains("not listed"));

        index.insert("bin/hello".into(), sha256_hex(PROGRAM));
        index.insert("ghost".into(), sha256_hex(b""));
        let ghost = package(
            &[
                (MANIFEST, manifest().as_bytes(), None),
                ("bin/hello", PROGRAM, Some(0o100755)),
            ],
            Some(index),
        );
        assert!(validate(ghost).unwrap_err().to_string().contains("ghost"));
    }

    #[test]
    fn a_missing_entrypoint_is_refused() {
        let bytes = package(&[(MANIFEST, manifest().as_bytes(), None)], None);
        assert!(validate(bytes).unwrap_err().to_string().contains("missing"));
    }

    #[test]
    fn an_undeclared_program_is_refused_however_it_is_disguised() {
        for (name, bytes, mode) in [
            ("tools/helper.exe", &b"plain"[..], None),
            ("assets/logo.png", &b"MZ\x90\x00 really a PE"[..], None),
            ("assets/run", &b"#!/bin/sh\necho hi"[..], None),
            ("assets/data", &b"just data"[..], Some(0o100755)),
        ] {
            let bytes = package(
                &[
                    (MANIFEST, manifest().as_bytes(), None),
                    ("bin/hello", PROGRAM, Some(0o100755)),
                    (name, bytes, mode),
                ],
                None,
            );
            let error = validate(bytes).unwrap_err().to_string();
            assert!(error.contains("does not declare"), "{name}: {error}");
        }
    }

    #[test]
    fn no_manifest_or_no_index_is_refused() {
        let mut writer = Writer::new();
        writer.file("bin/hello", PROGRAM, Some(0o100755), false);
        assert!(validate(writer.finish())
            .unwrap_err()
            .to_string()
            .contains("extension.json"));

        let mut writer = Writer::new();
        writer.file(MANIFEST, manifest().as_bytes(), None, false);
        writer.file("bin/hello", PROGRAM, Some(0o100755), false);
        assert!(validate(writer.finish())
            .unwrap_err()
            .to_string()
            .contains("integrity.json"));
    }

    #[test]
    fn a_package_without_a_licence_is_accepted_with_a_warning() {
        let text = manifest().replace("\"license\":\"MIT\",", "");
        let bytes = package(
            &[
                (MANIFEST, text.as_bytes(), None),
                ("bin/hello", PROGRAM, Some(0o100755)),
            ],
            None,
        );
        let package = validate(bytes).unwrap();
        assert_eq!(package.warnings.len(), 1);
    }

    #[test]
    fn a_directory_packs_into_a_package_that_validates() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join(MANIFEST), manifest()).unwrap();
        std::fs::create_dir_all(dir.path().join("bin")).unwrap();
        std::fs::write(dir.path().join("bin").join("hello"), PROGRAM).unwrap();
        let bytes = pack_directory(dir.path()).unwrap();
        let package = validate(bytes).unwrap();
        assert_eq!(package.files.len(), 2);
    }

    #[test]
    fn programs_are_recognised_by_their_first_bytes() {
        assert!(looks_executable(b"MZ"));
        assert!(looks_executable(b"\x7fELF\x02"));
        assert!(looks_executable(&[0xcf, 0xfa, 0xed, 0xfe, 0]));
        assert!(looks_executable(b"#!/usr/bin/env bun"));
        assert!(!looks_executable(b"{\"json\": true}"));
        assert!(!looks_executable(b""));
    }
}
