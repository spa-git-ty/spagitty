<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-039 — Automated test record

**Item:** [`agile/items/BUG-039-a-graph-test-that-reads-the-clock.md`](../items/BUG-039-a-graph-test-that-reads-the-clock.md)

## What was tested

| Suite | Cases |
| --- | --- |
| `crates/spagitty-core/src/graph.rs` | Unchanged: date order interleaves the parallel histories — now by the fixture's dates. |
| `crates/spagitty-core/src/rebase.rs` | The todo leaves out merges, lists the commits git would replay, and lists each after its parent. |

With the fixture dated and the todo test unchanged, the todo test failed on
every run (its order is not git's); that is what showed the second tie.

## Test command and output

On Windows 11, three consecutive runs:

```
$ cargo test -p spagitty-core --no-fail-fast
test result: ok. 536 passed; 0 failed      # run 1
test result: ok. 536 passed; 0 failed      # run 2
test result: ok. 536 passed; 0 failed      # run 3
```

In WSL (Arch Linux): `cargo fmt --check`, `cargo clippy -D warnings` and
`cargo test --workspace` pass.

`cargo test -p spagitty-farm` on native Windows fails 16 unit tests and some
integration tests: they start `/bin/sh` as a stand-in agent and check process
containment. Not touched here; it is TASK-033's.

## What is not covered automatically

Nothing: the change is to test support and tests.
