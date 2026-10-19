<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-151 — Automated test record

**Item:** [FEAT-151](../items/FEAT-151-search-the-text-of-files.md)

- `crates/spagitty-core/src/files.rs`: text in every file at the revision, with smart case; a regex, a whole word and *Match case*; an escape that is not a capital; a pattern that is not a regex refused, and the same characters searched as text; binary files left out and *more* past the limit; a hit's column, length and marked part in UTF-16 units, on a short line with an accent and on a long line cut around its match. The module's guard that nothing reaches the working tree still passes.
- `src-tauri/src/commands.rs`: `files_search` is classified as a reader by `every_command_is_read_or_write_on_purpose`.
- `src/lib/files/keys.test.ts`: text by default, regex, whole word, case, a bad regex finding nothing, stepping both ways with wrapping, Alt+R, W, C. The old `find` tests pass unchanged.
- `src/lib/files/store.test.ts`: the search sends its options; turning an option re-runs it; a bad regex is said without asking the backend; a found line opens with its match and `n` carries on; regex and whole word in the file on screen; going back to names puts the text search away.

## Results — 2026-10-19

- `cargo test -p spagitty-core --lib files::`: 22 passed (Windows and WSL).
- `cargo test -p spagitty --lib` in WSL: 198 passed, one existing ignored test.
- `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check` in WSL: passed.
- `bunx vitest run src/lib/files`: 155 passed. `bun run check`: 0 errors, 0 warnings.
