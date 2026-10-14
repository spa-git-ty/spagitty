<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-082 — Automated test record

**Item:** [BUG-082](../items/BUG-082-saving-the-git-identity-clears-the-fields.md)

- `src-tauri/src/commands.rs`: save and reread name and email in global and local scope with the repository kept open; compare the response with `git config`.
- `src/lib/settings/store.test.ts`: preserve the other draft while saving each scope, then save it in sequence; keep an edit made during the pending write. Existing tests cover write failures, clearing and overlapping writes.
- `src/lib/settings/sections.test.ts`: mount the real section, type both fields and save each; assert that both values stay visible and the next Save remains available.

## Results — 2026-10-12

Before the fix, the command regression failed on the stale global name and both store regressions lost the edited email.

- The command regression now passes for global and repository name and email, scope separation, clearing a local override and an external configuration edit.
- The store and mounted component regressions pass in both scopes.
- `bunx vitest run`: 190 files, 4155 tests passed.
- `bun run check`: zero errors and zero warnings.
- `cargo fmt --all --check` and `cargo clippy --workspace --all-targets -- -D warnings`: passed.
- The workspace Rust run passed the Tauri suite (129 passed, one existing ignored test) and the core suite (696 unit tests plus integration tests). It stopped at 17 existing Farm failures caused by Unix assumptions on Windows; Linux CI must complete the full workspace check. See [BUG-081's validation](BUG-081-automated.md).
- Frontend and Windows workspace builds succeeded. Diff reviewed; no unresolved conflicts. No dependency added.

The desktop sweep has not run.
