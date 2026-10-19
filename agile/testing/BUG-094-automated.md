<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-094 — Automated test record

**Item:** [BUG-094](../items/BUG-094-a-resumed-agent-skips-files-that-changed-since.md)

- `src-tauri/src/agents.rs`, `a_run_is_carried_on_only_at_the_revisions_it_worked_on`: a review at the same head is the same; one pushed to, or another pull request, is not; a merge whose second branch moved is not.

## Results — 2026-10-19

- `cargo test -p spagitty --lib` in WSL: 198 passed, one existing ignored test.
- `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check` in WSL: passed.
