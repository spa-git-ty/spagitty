<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-093 — Automated test record

**Item:** [`agile/items/FEAT-093-threads-done-properly.md`](../items/FEAT-093-threads-done-properly.md)

## What was tested

| Suite | Cases |
| --- | --- |
| `crates/spagitty-core/src/forge/github.rs` | Review threads read with their node ids, resolved state and comments' `databaseId`s, the next page's cursor, none after the last page, and a refused query as an error; resolving and reopening are each their own mutation on the thread's id; the threads query only reads. |
| `crates/spagitty-core/src/forge/gitlab.rs` | The room reads notes on the whole merge request too, each with its discussion's id and resolved state, system notes left out; the Pull requests screen's read still has the line comments alone. |
| `crates/spagitty-core/src/forge/review.rs` | Comments on the whole pull request have no path and no line; an answer that is not a list is an error. |
| `src/lib/review/drafts.test.ts` | A line's place by both counters, the other side's being the next line there; a comment on one line named by its side and number; a range by both ends, across the sides, whichever end was picked first, and the draft the backend gets; pending comments placed as threads are; a pending comment through the record whole, and a bad place costing only itself. |
| `src/lib/review/record.test.ts`, `rows.test.ts`, `inbox.test.ts`, `fixes.test.ts` | The record keeps places and the old path; threads carry their id and whether they are on the whole pull request; a conflict fix and the author's change are cut into two cards back to back, however close. |
| `src/routes/review/room.test.ts` | A comment written from a line's `+`, kept with its side, number and place, not sent, counted on Finish review, listed in the card, and deleted; a range by shift-click across a removed and an added line, tinted, kept with both ends; pending comments back after a restart, older ones kept aside; Finish review sends the verdict, the words for the whole and the comments, then clears what was sent and reads the threads again; changes asked for with nothing written cannot be sent; resolving shows at once and is put back when the host refuses; a reply sent at once and shown; a comment on the whole pull request listed. |
| `src/routes/review/page.test.ts` | The room's comment read answered. |

## Test command and output

On Windows 11:

```
$ bun run check
0 ERRORS 0 WARNINGS
$ bun run test
Tests  1 failed | 3153 passed (3154)
$ cargo test -p spagitty-core
test result: ok. 586 passed; 0 failed
$ cargo clippy -p spagitty-core --all-targets
(no warnings)
```

The one failure is `tools/record.test.ts` on BUG-043, which reached `main`
without its plan and testing documents before these branches were cut.

In WSL (Arch):

```
$ cargo test -p spagitty --lib
test result: ok. 106 passed; 0 failed
```

## What is not covered automatically

The GraphQL thread query and the resolve mutations against GitHub itself, the
discussion `PUT` and a `line_range` against the author's work GitLab, and a
restart of the real application. See the sweep.
