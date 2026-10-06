// SPDX-License-Identifier: GPL-3.0-or-later

mod licenses;

use std::path::PathBuf;

fn main() {
    // GPL-3 wants a build to be able to point at the source it came from. The
    // commit is stamped in here and shown in Settings -> Advanced -> About.
    let sha = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()
        .filter(|o| o.status.success())
        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        .unwrap_or_else(|| "unknown".into());
    println!("cargo:rustc-env=SPAGITTY_COMMIT={sha}");

    // The other half of the same obligation: what this binary is made of. The
    // list is generated from `Cargo.lock` and the installed frontend tree
    // rather than typed, and a build that cannot generate it still builds —
    // see `licenses.rs`.
    let manifest_dir = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").expect("a manifest dir"));
    let out_dir = PathBuf::from(std::env::var("OUT_DIR").expect("an out dir"));
    licenses::generate(&manifest_dir, &out_dir);

    for trigger in licenses::rerun_triggers(&manifest_dir) {
        println!("cargo:rerun-if-changed={}", trigger.display());
    }

    // Both the application and Cargo's library test executable link the native
    // dialog API, which needs Common Controls v6. Link one manifest for all
    // targets; omit Tauri's copy to avoid a duplicate manifest in the app.
    let mut attributes = tauri_build::Attributes::new();
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg=/MANIFESTINPUT:{}",
            manifest_dir.join("windows-controls.manifest").display()
        );
        println!("cargo:rerun-if-changed=windows-controls.manifest");
        attributes = attributes
            .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
    }
    tauri_build::try_build(attributes).expect("Tauri build resources");
}
