<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-075 — Plan

**Item:** [TASK-075](../items/TASK-075-remove-coderabbit.md)

1. Delete `extensions/coderabbit/`, `tools/extensions/bundle.ts` and `bundle.test.ts`; drop the workspace member and refresh `Cargo.lock`, keeping lockfile format 3.
2. `src-tauri/tauri.conf.json`: no bundle step before dev and build, no extension resource. `src-tauri/tauri.macos.conf.json`: no `externalBin`. `.github/workflows/checks.yml`: no staging steps, no `-p coderabbit-extension`. `.gitignore`: no staging paths.
3. `crates/spagitty-extensions/tests/bundled_manifests.rs`: check only the manifests still in the tree.
4. Farm: `supplemental_mode` in `crates/spagitty-farm/src/service.rs`, used at both gates; the application no longer sets a reviewer; remove `src-tauri/src/supplemental.rs` and `farm_review_supplemental`; remove the rule from `RulesEditor.svelte`, the button from `TaskJourney.svelte`, and `reviewSupplemental` from `api.ts`.
5. Docs and records.
