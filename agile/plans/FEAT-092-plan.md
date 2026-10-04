<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-092 — Plan

**Item:** [`agile/items/FEAT-092-conflict-fix-origin.md`](../items/FEAT-092-conflict-fix-origin.md)

## Approach

A core module, `remerge.rs`. It walks the two-parent commits between the merge
base and the head, newest first, and for each runs
`git show --remerge-diff --unified=2000` through `shell.rs` — wide context, so
a side kept whole is shown whole. The output is parsed into files and lines
numbered as in the merge, and cut into places: inside a pair of conflict
markers every committed line is the resolution, as are the lines added after
the closing marker up to the next unchanged line; lines added away from any
marker are a place with no sides. The markers' sections are the two parents'
sides; the target's is the parent the target's tip contains.

Each place's lines are moved to the head through a whole-file diff from the
merge to the head, keeping only lines both leave alone, and kept only where
the pull request adds them. A file holds author work when it adds a line that
no fix wrote, or removes lines and adds none.

`review_conflicts` runs it off the main thread with the session let go. The
room asks once the head is fetched, keeps the answer, and saves the files and
merges into the record. `rows.ts` gives a changed part the `conflict` tone when
a fix wrote one of its added lines, marks those lines, and puts a side row
under the header when one is open.

## Files

| File | Change |
| --- | --- |
| `crates/spagitty-core/src/remerge.rs`, `lib.rs`, `shell.rs` | The module; `remerge_diff`. |
| `src-tauri/src/commands.rs`, `lib.rs` | `review_conflicts`. |
| `src/lib/types.ts`, `api.ts` | `ConflictFix`, `FixedFile`, `ConflictFixes`; `reviewConflicts`. |
| `src/lib/review/rows.ts` | The conflict tone, fixed lines, side rows. |
| `src/lib/review/room.svelte.ts` | The fixes, the filter, the sides, the record. |
| `src/lib/review/RoomDiff.svelte`, `RoomFiles.svelte` | The card, the markers, the chips, the legend. |
| `src/app.css` | `--resolve`, `--resolve-soft`. |

## Risks and rollback

- **Git older than 2.36** has no `--remerge-diff`: the room says it could not
  look, and every card is the author's.
- **A merge of a huge tree** takes git a while: it runs off the main thread,
  and the room is readable meanwhile.
- **A conflict fix later rewritten** belongs to the commit that rewrote it,
  which is the author's.
- Rollback is a revert.
