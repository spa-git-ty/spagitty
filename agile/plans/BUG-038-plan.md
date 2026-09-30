<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-038 — Plan

**Item:** [`agile/items/BUG-038-interactive-rebase-cannot-start-on-windows.md`](../items/BUG-038-interactive-rebase-cannot-start-on-windows.md)

## Approach

`SequenceScripts::write` drops its `cfg!(windows)` branch and writes the
`sh` pair everywhere: `cat '<plan>' > "$1"` and `exit 0`. `forward` renders the
plan's path with `/`. The scripts are still invoked by their quoted native
path; that part already worked, since git hands the whole editor string to
`sh -c`, which runs a file that starts with `#!` whether or not it has an
execute bit.

Checked by hand first, in a scratch repository on this machine: the batch file
fails with git's editor error, and the `sh` script runs a `drop`/`pick` plan to
the expected conflict.

## Files

| File | Change |
| --- | --- |
| `crates/spagitty-core/src/shell.rs` | `sh` scripts on every platform; `forward`; tests. |

## Risks and rollback

- **A Windows git without `sh`.** Git for Windows always ships it and always
  runs editors through it; there is no supported Windows git that does not.
- Rollback is a revert, which puts the broken batch file back.
