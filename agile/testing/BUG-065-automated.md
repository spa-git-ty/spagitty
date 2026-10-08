<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-065 — Automated checks

- Rust adapter and local-driver tests cover Codex startup, default permissions,
  opt-in Full Access, sandbox errors, OMP print mode and agy print/plan modes.
- Detector tests cover standard install paths and npm's native Bun runtime.
- Machine configuration tests cover safe defaults and persisted Full Access.
- OMP tests cover persisted model/profile arguments in headless commands and
  editing those choices in mounted Settings.
- Mounted Settings tests cover enabling and disabling Codex Full Access.
- Settings Test verifies a temporary-file marker, rejects permission failures,
  and waits for access choices to save while clearing the old result.
- agy tests cover persisted tool approval, plan/print argument ordering, avoiding
  duplicate approval flags and reporting a headless permission denial as failed.
- Review tests cover completed-file event persistence, blob marks, deletions,
  failed and waiting steps, stale heads and manual unticks.
- Run the Rust suites, frontend suite, type checks and production builds.

## Results — Windows, 2026-10-08

- `bun run test`: 177 files, 3,671 tests passed with the final changes.
- Focused Settings, agent sheets, review-agent, review-record and provider-description
  suites: 94 tests passed after adding OMP model/profile configuration. Settings
  and agent sheets passed 32 tests after adding agy auto-approval.
- `bun run check`: zero errors or warnings.
- `bun run build`: production frontend build passed.
- `cargo fmt --all --check`: passed.
- `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- `cargo build -p spagitty`: desktop build passed after staging the extension
  with `bun tools/extensions/bundle.ts --debug`.
- `cargo test -p spagitty --lib agents::tests`: 12 tests passed, 1 live test ignored, including
  configuration persistence, Full Access argument propagation and zero-exit
  configuration errors in Settings Test, OMP's saved model/profile arguments,
  and agy's opt-in tool approval while retaining plan mode. The ignored live
  test was run separately and passed for both providers through the Settings handlers.
- The Windows-compatible farm library run passed 386 tests, with 25 filtered:

  ```text
  cargo test -p spagitty-farm --lib -- --skip execution::process::tests --skip verification::verifier::tests --skip assign::guard::tests::a_path_is_put_back_by_its_name_not_as_a_pattern
  ```

The unfiltered farm suite has 17 existing Windows failures, reproduced with
the same names on unchanged `main`: 13 process tests and 3 verifier tests use
`/bin/sh`, and the guard test creates `*.txt`, an invalid Windows filename.
The filtered run also excludes 8 passing tests from those two modules. Git
Bash is installed, but native Windows Rust subprocesses cannot resolve
`/bin/sh`; the complete suite remains for the existing Linux CI gate.

No dependencies were added. The diff was checked for unintended provider-id,
permission, persistence and viewed-blob changes; no conflicts remain.
