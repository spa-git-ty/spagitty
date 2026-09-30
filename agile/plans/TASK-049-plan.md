<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-049 — Plan

**Item:** [`agile/items/TASK-049-the-record-catches-up-with-what-merged.md`](../items/TASK-049-the-record-catches-up-with-what-merged.md)

## Approach

For each of the nine, three checks before touching its status:

1. **Merged?** `git merge-base --is-ancestor origin/<branch> main`, and the
   merge commit that took it there. All seven pull-request branches have no
   commits `main` lacks.
2. **Released?** `git tag --contains`. All are in 0.5.1 and later.
3. **Still true?** The tests each item's record names, run today: the farm
   crate in WSL (331 unit, 53 pipeline), and on native Windows the two test
   modules TASK-033's Windows job runs.

Then the status word in the item and in the index, with the evidence on the
status line.

FEAT-079 and FEAT-081 get the sweep rows the release build can show, driven
over the webview's debugging port on 2026-09-30, recorded as observed; the
rows that need a packet capture, a 60fps recording, a restart or a particular
repository stay empty, and both items stay `Open` saying so.

The same run filled the rows it showed for BUG-040 and noted what it measured
for TASK-048 without counting it as a row.

## Files

| File | Change |
| --- | --- |
| `agile/items/{BUG-024..027,TASK-032,033,036,FEAT-079,081}-*.md` | Status lines. |
| `agile/README.md` | Seven index rows. |
| `agile/testing/{FEAT-079,FEAT-081,BUG-040,TASK-048}-sweep.md` | Results. |

## Risks and rollback

- None to the application. Rollback is a revert.
