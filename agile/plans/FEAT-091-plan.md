<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-091 — Plan

**Item:** [`agile/items/FEAT-091-the-review-room.md`](../items/FEAT-091-the-review-room.md)

## Approach

The room's state is a store of its own beside the inbox's,
`review/room.svelte.ts`: it fetches the head and lists the files
(`review_checkout`, `review_files`), reads each file whole when it is first
shown (`review_file`), and keeps what lasts — the viewed ticks — through the
inbox store's `saveRecord`. Everything else is how the room is being looked at
and lasts as long as the room. A failed fetch falls back to
`pull_request_files`, the host's patch.

The layout is pure, in `review/rows.ts`: a file's lines are cut into blocks
(changed parts with three lines of context, joined when the gap between them is
under four lines; folds or plain runs between), and the blocks of every shown
file are flattened into rows that each know their card and whether they end
it. `review/threads.ts` groups the host's flat comments into threads and places
them by path, side and line.

The column draws those rows through `ui/VirtualRows.svelte`: offsets from
measured heights or an estimate, a binary search for the window, spacers above
and below, a `ResizeObserver` per drawn row, and the scroll moved by the change
in any row above the top edge. A card is drawn a row at a time — sides on every
row, the top on its header, the bottom on its last row — so it can be cut
anywhere.

The core's `FileChange` gains `old_blob` and `new_blob`, set where it is built.

## Files

| File | Change |
| --- | --- |
| `crates/spagitty-core/src/diff.rs`, `pull.rs` | Blob ids on each changed file; the test. |
| `src/lib/types.ts` | `FileChange.oldBlob` / `newBlob`. |
| `src/lib/ui/VirtualRows.svelte` | The measured virtual list. |
| `src/lib/review/rows.ts`, `threads.ts` | Blocks, rows, threads. |
| `src/lib/review/room.svelte.ts` | The room's state. |
| `src/lib/review/ReviewRoom.svelte`, `RoomFiles.svelte`, `RoomDiff.svelte`, `RoomPill.svelte`, `RoomConversation.svelte` | The room. |
| `src/lib/review/store.svelte.ts` | `recordAt`. |
| `src/lib/ui/icons.ts` | `unfold`. |
| `src/app.css` | `--read-size`. |

## Risks and rollback

- **Estimates far from the truth** make the scrollbar's thumb wander as rows
  are measured; the reading position does not move, because the scroll is
  corrected by every change above it.
- **A file of tens of thousands of lines in Whole file** is one list of that
  many rows, of which a screenful is drawn; reading it is the backend's cost,
  once.
- Rollback is a revert; nothing outside Review reads what this adds.
