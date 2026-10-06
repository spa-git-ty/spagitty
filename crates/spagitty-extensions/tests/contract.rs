// SPDX-License-Identifier: GPL-3.0-or-later

//! The public contract's fixtures, read by the host.
//!
//! `schemas/extensions/fixtures/` is shared with the TypeScript kit's tests
//! (`packages/extension-sdk`) and the frontend's: every side reads the same
//! files, so a field renamed on one side fails here or there rather than in a
//! user's hands.

use std::path::PathBuf;

use serde_json::Value;
use spagitty_extensions::history::ReviewRecord;
use spagitty_extensions::manifest::Manifest;
use spagitty_extensions::Error;

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../schemas/extensions/fixtures")
}

fn json_files(dir: PathBuf) -> Vec<(String, String)> {
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .unwrap_or_else(|e| panic!("{}: {e}", dir.display()))
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("json"))
        .map(|p| {
            (
                p.file_name().unwrap().to_string_lossy().into_owned(),
                std::fs::read_to_string(&p).unwrap(),
            )
        })
        .collect();
    files.sort();
    files
}

#[test]
fn every_valid_fixture_parses() {
    let valid = json_files(fixtures().join("manifests/valid"));
    assert!(valid.len() >= 2);
    for (name, text) in valid {
        Manifest::parse(&text).unwrap_or_else(|e| panic!("{name}: {e}"));
    }
}

#[test]
fn every_invalid_fixture_is_refused_naming_the_field() {
    let invalid = json_files(fixtures().join("manifests/invalid"));
    assert!(invalid.len() >= 10);
    for (name, text) in invalid {
        let case: Value = serde_json::from_str(&text).unwrap();
        let expect = case["expect"].as_str().unwrap();
        match Manifest::parse(&case["manifest"].to_string()) {
            Err(Error::Manifest(errors)) => assert!(
                errors.iter().any(|e| e.starts_with(expect)),
                "{name}: expected an error at {expect}, got {errors:?}"
            ),
            other => panic!("{name}: expected a manifest error, got {other:?}"),
        }
    }
}

#[test]
fn the_review_record_fixture_round_trips_unchanged() {
    let text = std::fs::read_to_string(fixtures().join("review-record.json")).unwrap();
    let record: ReviewRecord = serde_json::from_str(&text).unwrap();
    let written = serde_json::to_value(&record).unwrap();
    let original: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(
        written, original,
        "the host writes exactly the shape the frontend reads"
    );
}

#[test]
fn the_published_schema_names_every_capability_the_host_knows() {
    let schema: Value = serde_json::from_str(
        &std::fs::read_to_string(fixtures().join("../manifest.v1.schema.json")).unwrap(),
    )
    .unwrap();
    let listed: Vec<&str> = schema["$defs"]["capability"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    let known: Vec<&str> = spagitty_extensions::capabilities::Capability::ALL
        .iter()
        .map(|c| c.as_str())
        .collect();
    assert_eq!(listed, known);
    let targets: Vec<&str> = schema["$defs"]["target"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap())
        .collect();
    assert_eq!(targets, spagitty_extensions::manifest::TARGETS);
}
