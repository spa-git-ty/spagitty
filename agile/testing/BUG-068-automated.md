<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-068 — Automated test record

**Item:** [BUG-068](../items/BUG-068-ordinary-commits-marked-as-conflict-fixes.md)

| Suite | Cases |
| --- | --- |
| `crates/spagitty-core/src/remerge.rs` | A line a merge adds away from any conflict is not a fix; a conflict resolved merging a branch other than the target is not a fix and the merge is not listed; a clean merge of the target and a later ordinary commit marks nothing; the existing resolution is still found and followed to the head. |

## Results — 2026-10-11

- `cargo test -p spagitty-core remerge`: 8 passed. The two new fixtures fail with the fix removed.
