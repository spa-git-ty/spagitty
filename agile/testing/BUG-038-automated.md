<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-038 — Automated test record

**Item:** [`agile/items/BUG-038-interactive-rebase-cannot-start-on-windows.md`](../items/BUG-038-interactive-rebase-cannot-start-on-windows.md)

## What was tested

| Suite | Cases |
| --- | --- |
| `crates/spagitty-core/src/shell.rs` | The scripts written are `sh` on every platform, the plan's path in the sequence editor has no backslash, and the editors git is given are the `.sh` files. A Windows path reaches `sh` with `/`. |
| `crates/spagitty-core/src/ops.rs` | Unchanged: a rebase that conflicts stops with its state on disk; aborting puts the branch back. These failed on Windows before this change and pass after it. |

The new tests run on Linux as well, so the pipeline, which has no Windows
runner for the Rust suite, still catches a return to a batch file.

## Test command and output

On Windows 11:

```
$ cargo test -p spagitty-core ops::tests::a_rebase_that       # without the fix
test result: FAILED. 0 passed; 1 failed

$ cargo test -p spagitty-core --no-fail-fast                    # with it
test result: 535 passed; 1 failed
    graph::walk_tests::date_order_interleaves_parallel_histories
```

The one failure is unrelated and intermittent — it depends on the wall clock —
and has an item of its own. With both fixed, the crate passes on Windows, 536
of 536.

In WSL (Arch Linux): `cargo fmt --check`, `cargo clippy -D warnings` and
`cargo test --workspace` pass.

## What is not covered automatically

A rebase driven from the Rebase screen in the Windows build.
