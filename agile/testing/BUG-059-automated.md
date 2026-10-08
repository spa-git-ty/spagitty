<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-059 — Automated test record

**Item:** [BUG-059](../items/BUG-059-windows-watcher-never-sees-refs.md)

| Suite | Cases |
| --- | --- |
| `src-tauri/src/watch.rs` | A verbatim prefix and a UNC one are taken off, and a plain path is left alone. A verbatim git directory owns a plain ref event, and a plain one owns a verbatim event: both are ref moves and neither is a working-tree candidate. A write to `.git/objects` under a verbatim git directory changes nothing. A working-tree path is still a candidate, without its prefix. |

## Results — 2026-10-08

- `cargo test -p spagitty watch`: 19 passed.
- `cargo clippy -p spagitty --all-targets -- -D warnings` and `cargo fmt --check`: clean.
- Run on Linux, where a path never carries the verbatim prefix; the tests build the Windows forms as strings. Not run natively on Windows.
