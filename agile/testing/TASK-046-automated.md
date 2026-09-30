<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-046 — Automated test record

**Item:** [`agile/items/TASK-046-lists-you-can-read.md`](../items/TASK-046-lists-you-can-read.md)

## What was tested

| Suite | Cases |
| --- | --- |
| `src/lib/diff/panes.test.ts` | A file is named first and whole, its folder after it, the whole path in the title; a dotfile is `.gitignore` with no hidden mark. |
| `src/lib/graph/CommitDetail.test.ts` | The same for the detail's files; every commit action is a button that does something — Cherry-pick, Revert, Interactive rebase, Copy SHA — none titled "Not built yet". |
| `src/lib/changes/panes.test.ts` | An untracked file carries the `U` badge. Everything the column did before — both sections, per-row stage and unstage, discard on the unstaged side only, busy states — still passes. |
| `src/lib/stash/panes.test.ts` | A stash row leads with what was written: `On main: half of the login form` reads `half of the login form`, and git's `WIP on …`, prefixed or not, reads `Work in progress`; the whole message stays in the title. |
| `src/routes/stash/page.test.ts` | Counts entries by their rows now that the header no longer repeats the count. |
| `src/lib/chrome/chrome.test.ts` | The strip is ready once history is on screen, complete or not. |

### Tests changed, and why

The dotfile tests asserted the left-to-right mark that `direction: rtl` needed;
the direction is gone, so they assert the plain name. The untracked test
asserted `?`; it asserts `U`. The merge-action test asserted a chip that did
nothing; it now asserts that every chip does something.

## Test command and output

On Windows 11:

```
$ bun run check
COMPLETED 1165 FILES 0 ERRORS 0 WARNINGS 0 FILES_WITH_PROBLEMS

$ bun run test
Tests  2955 passed (2955)
```

## What is not covered automatically

How the lists read. They were checked in the release build on this machine,
driven through the webview's debugging port, on a repository with 16 changed
files and a stash.
