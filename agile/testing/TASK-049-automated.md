<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-049 — Automated test record

**Item:** [`agile/items/TASK-049-the-record-catches-up-with-what-merged.md`](../items/TASK-049-the-record-catches-up-with-what-merged.md)

## What was tested

`tools/record.test.ts`: every item's status word matches its index row, and
every cited identifier resolves.

The evidence for the status changes, run 2026-09-30:

```
# WSL (Arch Linux)
$ cargo test --workspace --no-fail-fast
test result: ok. 331 passed; 0 failed     (spagitty-farm, unit)
test result: ok. 53 passed; 0 failed      (spagitty-farm, pipeline)

# Windows 11, natively — the modules TASK-033's Windows job runs
$ cargo test -p spagitty-farm --lib execution::tree::tests
test result: ok. 2 passed; 0 failed
$ cargo test -p spagitty-farm --lib verification::command::tests
test result: ok. 16 passed; 0 failed

$ bun run test
Tests  3012 passed (3012)
```
