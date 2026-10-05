<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-046 — Automated test record

**Item:** [`agile/items/BUG-046-a-console-window-opens-for-git.md`](../items/BUG-046-a-console-window-opens-for-git.md)

## What was tested

`shell::tests::every_process_is_built_by_program`: the only `Command::new(` in
`spagitty-core`'s sources, test modules and fixtures aside, is the one in
`program`. Before the fix it named `shell.rs:53`, `shell.rs:529` and
`signing.rs:334`.

## Test command and output

On Windows 11:

- `cargo test -p spagitty-core --lib` — 587 passed.
- `cargo test -p spagitty-farm --lib agent::detector` — 3 passed.
- `cargo clippy -p spagitty-core -p spagitty-farm --all-targets -- -D warnings`
  — clean.

## What is not covered automatically

Whether a window appears, which needs a GUI process on a Windows desktop. See
the sweep.
