<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-046 — Plan

**Item:** [`agile/items/BUG-046-a-console-window-opens-for-git.md`](../items/BUG-046-a-console-window-opens-for-git.md)

## Approach

`shell::program(name)` is `Command::new(name)` plus, on Windows,
`creation_flags(CREATE_NO_WINDOW)`. `shell::command` and `clone_start` build
git through it, which covers every function in `shell`. `signing::on_path`, the
Farm's `detector::run_briefly` and `service::diff_summary` call it too.

A source test walks `spagitty-core/src`, skipping test modules and the
fixtures, and expects exactly one `Command::new(`: the one in `program`.

`launch_difftool` and `launch_mergetool` lose their console as well. Their
stdin and output were already null, so a terminal tool could not have been
used there; a graphical one is unaffected.

## Files

| File | Change |
| --- | --- |
| `crates/spagitty-core/src/shell.rs` | `program`; `command` and `clone_start` use it; the source test. |
| `crates/spagitty-core/src/signing.rs` | The program probe uses `program`. |
| `crates/spagitty-farm/src/agent/detector.rs` | The version probe uses `program`. |
| `crates/spagitty-farm/src/service.rs` | `diff_summary` uses `program`. |
| `CHANGELOG.md` | A *Fixed* entry. |

## Risks and rollback

- A git that needs a console to ask for something gets none. It never had a
  usable one here: `GIT_TERMINAL_PROMPT=0` already turns prompts into errors,
  and credential helpers open their own windows. Rollback is a revert.
