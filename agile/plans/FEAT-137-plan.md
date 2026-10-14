<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-137 — Plan

**Item:** [FEAT-137](../items/FEAT-137-hooks.md)

Written on 2026-10-14, before the code.

## The pieces

1. **The blocks** in `crates/spagitty-core/src/hooks/blocks.rs`: `Definition`, `Block`, `Stage`, validation, the closed catalogue and the Conventional Commits parser reused by Releases. Settings use typed values, never an unvalidated script fragment for a filename.
2. **The standalone script** in `hooks/generate.rs` and its bundled `runner.sh`: generate all three stages from blocks, use git pathspecs, write JSON-lines events and per-block logs, enforce timeout and block/warn, preserve partial staging and retain 50 records. The terminal has no dependency on the application.
3. **Storage and installation** in `hooks/builder.rs`: shared TOML or personal TOML, checked parsing, explicit installation, marked scripts only, safe run/log identifiers and bounded reads. Import known manager commands without changing their original files.
4. **Execution** in `shell.rs`: run generated hooks and test commit messages through git’s own hook runner. The existing process boundary and command log remain authoritative.
5. **The commands** in `src-tauri/src/hooks.rs`: load/save/install, import, preview staged matches, dry-run, try-message and list/read runs. Register them in `lib.rs`. Reuse the repository watcher for run-record events, including common git directories in worktrees.
6. **Typed screen state** in `src/lib/hooks/builder-types.ts`, `builder-api.ts`, `builder.svelte.ts`: generation guards when repositories change, separate read/save/run errors, dirty state, stage ordering and selected block/run/step.
7. **The screen** at `/hooks`, with `/hooks/runs` and `/hooks/edit`: a header switch, Runs’ step list/log/restage/staged summaries and the builder’s stage list/editor/catalogue. Use shared controls, tokens and fonts. Add the Settings link.
8. **Navigation and records** in `nav.ts`, `icons.ts`, palette commands, the item, automated record, sweep, screens document and changelog. The new group sits immediately above Settings.

## What is decided, and why

- **Managers retain control by default.** Import is explicit. An import never overwrites an existing manager stub.
- **Shared or personal storage.** Use `.spagitty/hooks.toml` for the shared definition and application data for a personal definition.
- **One implementation of message checks.** Try a message invokes the generated script on a temporary file; no separate frontend regex validator.
- **Standalone run records.** Shell commits and application commits write the same records. The screen reads them without entering the commit’s path.
- **A separate navigation group.** Place these screens above Settings with a separator; the shell’s existing appearance stays.
- **Incremental review.** Checked feature branches have separate pull requests. The initial release is merged, so Hooks targets main; later features target the preceding feature branch.

## Where this differs from the item or the proposal

- The four navigation rows sit after Tools and before Settings.

## Dependencies

Configuration parser dependencies:

- **`toml` 0.9.12, MIT OR Apache-2.0.** Read and write standard TOML definitions, including team edits. Already present in the lockfile; its Rust 1.76 minimum fits the workspace.
- **`yaml-rust2` 0.10.4, MIT OR Apache-2.0.** Read manager YAML, preserving quoting, multiline commands and anchors. Pipelines will reuse its marked parser. Its Rust 1.65 minimum fits the workspace, and encoding support is disabled. Version 0.10.2 failed to compile with encoding disabled; 0.10.4 is the compatible patch release.

## Not in this plan

- Taking over an unmarked hook, installing on clone, or stashing a partially staged file.
- The three host screens, which have their own items.

## What decides it is right

- Fixture tests execute generated hooks through git and show block/warn, path matching, safe quoting, timeout, run records and preserved partially staged content.
- Storage tests round-trip every block setting and refuse malformed definitions, unmarked hooks, unsafe identifiers and invalid stages.
- Command tests use a mock application and real repository and report no open repository clearly.
- Mounted tests exercise adding/editing/reordering/removing blocks, saving/installing, trying messages, dry runs, run selection and manager consent. Store tests cover stale replies and independent failures.
- Navigation, token, copy and record checks pass with types, coverage and the frontend build.
