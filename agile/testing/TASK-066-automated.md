<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-066 — Automated test record

**Item:** [TASK-066](../items/TASK-066-style-notes-from-the-2-0-audit.md)

- `src-tauri/src/watch.rs`, `a_verbatim_prefix_is_taken_off`: a volume path and a `GLOBALROOT` device path keep their prefix; a drive path and a UNC path lose it as before. `a_verbatim_git_dir_still_owns_the_events_under_it` (BUG-059) still passes.
- `tools/release-notes.mjs`: run on Windows for 3.3.0; it printed the section and exited 0.
- BUG-094 has its own record.

## Results — 2026-10-19

- `cargo test -p spagitty --lib` in WSL: 198 passed, one existing ignored test.
- `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check` in WSL: passed.
