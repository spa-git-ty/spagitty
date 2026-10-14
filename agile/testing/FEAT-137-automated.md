<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-137 — Automated test record

**Item:** [FEAT-137](../items/FEAT-137-hooks.md)

| Layer | Suite | Cases |
| --- | --- | --- |
| Rules | `hooks/blocks.rs` | Unique safe ids, folders confined to the repository, message rules confined to commit-msg, and Conventional Commits parsing for later release notes. |
| Scripts and repositories | `hooks/generate.rs`, `hooks/builder.rs` | Stage selection and quoting; standard TOML round trips; **unmarked hooks refused**; terminal commits write events, output and subject; **block versus warn**; pathspec matching; **partial staging preserved**; formatter restage and stale-safe undo; Spotless receives one quoted file-pattern property; timeout and cancellation; secret patterns without printing the secret; indexed file sizes; shared definitions follow team edits; common records across worktrees; incomplete first events and appends; pre-push protection checks the remote destination. |
| Import | `hooks/import.rs` | YAML quotes, anchors and multiline commands keep their meaning and stage. The original manager file is unchanged. |
| Commands | `src-tauri/src/hooks.rs` | No open repository is reported; install/read/dry-run and try-message call the generated scripts on a fixture repository through a mock application. Scope requirements and a ticket in the last message paragraph are checked by that script. |
| Watcher | `src-tauri/src/watch.rs` | Log writes wake Hooks; reads and unrelated files do not. |
| Store and mounted screens | `src/lib/hooks/builder.test.ts` | Drafts survive tab changes; stale repository replies are dropped; stage ordering; installation and dry-run confirmations; explicit manager import; independent errors; all seven blocks added and a command edited through inputs; Try message uses the command; run and step selection; watcher refresh. |
| Navigation and existing behaviour | `src/lib/nav.test.ts`, `src/lib/hooks/hooks.test.ts`, UI, palette and Settings suites | New group before Settings; existing commit confirmation and output remain; shared tokens, copy, records and reachability. |

## Not covered

- The feature has not been looked at in the running application. The Rust commands and the mounted screens have not been run together in the application.
- Windows only. Linux, macOS and WebKit have not been run.
- Real manager installations, user repositories, long runs and very large logs have not been used.
- Native dragging and the final on-screen panel sizes have not been checked by a person.
- External formatter installations have not run. The Spotless test checks its arguments with a stand-in command.
- CI, Rust coverage, cargo-deny licence/advisory checks and history secret scanning have not run on this change.

## Results — 2026-10-14

- Initial baseline: `cargo test -p spagitty-core hooks::tests`: 6 passed. The sandbox prevented fixture Git initialization; the same command outside it passed.
- `cargo test -p spagitty-core hooks::`: 24 passed, 0 failed, before the final records and checks.
- `cargo test -p spagitty hooks::`: 2 passed, 0 failed, before the final importer changes.
- Focused frontend: builder and navigation suites, 2 files, 59 tests passed, before final records and checks.
- `bun run check`: 0 errors, 0 warnings before final records and checks. The configured Node shim had no active version; commands use the bundled Node runtime on PATH.
- `cargo fmt --all -- --check`: clean. `cargo clippy --workspace --all-targets -- -D warnings`: clean.
- `cargo test -p spagitty -p spagitty-core -p spagitty-process`: 168, 901 and 27 passed, 0 failed. Two pre-existing live tests were ignored: authenticated agent settings and attachment to an installed multiplexer.
- `bun audit --audit-level=high`: no vulnerabilities at or above high, 177 packages checked; one advisory below that level.
- Final `bun run check`: 0 errors, 0 warnings.
- Final `bun run coverage`: 212 files, 4742 tests passed. Statements 84.30%, branches 74.57%, functions 81.10%, lines 86.89%; all exceed the configured 70% floor.
- Final `bun run build`: succeeded with the static adapter.
- `bun run test` after the final records: 212 files, 4742 tests passed.

## Baseline differences outside this item

- `make-icons.py --check` reports differences in nine application icon files. `make-brand.py --check` reports differences in sixteen collateral files. The same commands, using the same Python runtime, report the same files on the untouched starting commit `d7fc6a3` in a separate worktree. No brand generator, source mark or derived asset was changed here.
- The default Node shim has no active runtime. Checks use the bundled Node runtime; Git Bash is placed before the Windows Linux launcher for shell-based tests. The initial Mac refusal test failed with the latter; it passes with Git Bash.
- `docs/testing.md` and `docs/ci.md` describe a 65% frontend floor. `vite.config.ts` enforces 70% in all four measures; that configuration is the check used here.

## Dependencies

- `toml` 0.9.12: MIT OR Apache-2.0, Rust 1.76 minimum. Standard TOML definitions; already in the lockfile.
- `yaml-rust2` 0.10.4: MIT OR Apache-2.0, Rust 1.65 minimum. Manager configuration and the marked parser to be reused by Pipelines. Optional encoding support is disabled. Its new transitive crates are `arraydeque`, `hashlink` and `hashbrown` (MIT OR Apache-2.0), and `foldhash` 0.1.5 (Zlib). `deny.toml` permits Zlib; the existing dependency tree already includes `foldhash` 0.2.0 under that licence.
- The author approved these additions on 2026-10-14. No network client was added.
