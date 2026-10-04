<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-091 — Automated test record

**Item:** [`agile/items/FEAT-091-the-review-room.md`](../items/FEAT-091-the-review-room.md)

## What was tested

| Suite | Cases |
| --- | --- |
| `crates/spagitty-core/src/pull.rs` | The changed-file list of a fetched pull request carries the same blob ids as the whole file, and none on a side where the file does not exist. |
| `src/lib/review/rows.test.ts` | Three lines kept either side of a change and the rest folded; an expanded fold shown, nothing folded in the whole file; a run too short to fold shown; two changes joined when the gap between them is too short to fold; a file whose text did not change folded whole, and nothing cut from no lines; a part's header by its numbers and the line above it at the margin, and a side with no lines numbered from the line before; the host's hunks laid end to end; a file's header, folds and card rows with the last row marked; a thread drawn under its line in that line's card, one on a folded line left out; a note for a binary file and one being read. |
| `src/lib/review/threads.test.ts` | Replies chained to replies and replies to a first note both lead to one thread, oldest first; a reply whose parent is missing makes its own thread and a loop settles on one; threads placed by file, side and line, ones with no line left out; where a thread is, by name and line. |
| `src/lib/ui/VirtualRows.test.ts` | Only the rows near the viewport drawn, the rest stood in for by their height; the window follows the scroll; a row above the top edge growing moves the scroll by as much, one below does not; a row brought into view on asking. |
| `src/routes/review/room.test.ts` | The head fetched and diffed from the merge base, and the room landing on the first file not viewed; a tick kept against an older blob dropped and the head and file count kept; the changed part with its folds, and a fold opened; only changed words marked; the whole file with nothing folded and back; *Viewed, next* ticks and goes on, and a tick in the list counts; All shows every file with its header; threads under their lines and in the Conversation card, and a click on a resolved one opens its fold and puts the ruler on its line; the ruler on the first change, `j` and `k`, and off; `Aa` sets code as it was; the host's patch read when the head cannot be fetched, with Whole file off; the reason when neither can be read. |
| `src/routes/review/page.test.ts` | Unchanged, with the room's comment read answered. |
| `src/lib/ui/flat.test.ts` | The room's components read only defined tokens (`--read-size` is published in `app.css`). |

## Test command and output

On Windows 11:

```
$ bun run check
0 ERRORS 0 WARNINGS
$ bun run test
Tests  1 failed | 3128 passed (3129)
$ cargo test -p spagitty-core
test result: ok. 576 passed; 0 failed
$ cargo clippy -p spagitty-core --all-targets
(no warnings)
```

The one failure is `tools/record.test.ts` on BUG-043, which reached `main`
without its plan and testing documents before these branches were cut; it is
not this item's.

In WSL (Arch), because the Tauri crate's test binary does not load on Windows:

```
$ cargo test -p spagitty --lib
test result: ok. 106 passed; 0 failed
```

## What is not covered automatically

Scrolling a real pull request of hundreds of files in WebKitGTK, where the
row measurements and scroll corrections run against real layout; and the room
against the author's work GitLab. See the sweep.
