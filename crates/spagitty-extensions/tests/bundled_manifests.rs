// SPDX-License-Identifier: GPL-3.0-or-later

//! A manifest Spagitty ships must admit the Spagitty that ships it.
//!
//! 2.0.0 left every bundled extension saying *It needs Spagitty >=0.8.1,
//! <2.0.0* (BUG-066), because nothing compared the manifests with the version
//! of the application. This does.

use std::path::{Path, PathBuf};

use spagitty_extensions::version::satisfies;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn engines(path: &str) -> Option<(String, String)> {
    let text = std::fs::read_to_string(root().join(path)).ok()?;
    let value: serde_json::Value = serde_json::from_str(&text).expect("a manifest is JSON");
    let engines = &value["engines"];
    Some((
        engines["spagitty"]
            .as_str()
            .expect("engines.spagitty")
            .to_string(),
        engines["extensionApi"]
            .as_str()
            .expect("engines.extensionApi")
            .to_string(),
    ))
}

#[test]
fn every_manifest_we_ship_admits_this_version_of_spagitty() {
    let version = env!("CARGO_PKG_VERSION");
    let mut seen = 0;
    for path in [
        "extensions/coderabbit/extension.json",
        // Generated from the one above; absent until `bun run ext bundle`.
        "extensions/coderabbit/bundle/spagitty.coderabbit/extension.json",
        "examples/extensions/hello/extension.json",
        "packages/extension-sdk/templates/basic/extension.json",
    ] {
        let Some((spagitty, api)) = engines(path) else {
            continue;
        };
        seen += 1;
        assert!(
            satisfies(&spagitty, version),
            "{path} needs Spagitty {spagitty}, and this is {version}"
        );
        assert!(
            satisfies(&api, spagitty_extensions::API_VERSION),
            "{path} needs extension API {api}, and this is {}",
            spagitty_extensions::API_VERSION
        );
    }
    assert!(seen >= 3, "the shipped manifests were not found");
}

#[test]
fn the_application_version_is_the_one_in_package_json() {
    let text = std::fs::read_to_string(root().join("package.json")).expect("package.json");
    let value: serde_json::Value = serde_json::from_str(&text).unwrap();
    assert_eq!(value["version"], env!("CARGO_PKG_VERSION"));
}
