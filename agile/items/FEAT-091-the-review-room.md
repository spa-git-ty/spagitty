<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-091 — The review room

**Status:** Open — built on `feature/FEAT-091-the-review-room`, not yet merged.
**Branch:** `feature/FEAT-091-the-review-room`
**Screens:** 1R.
**Raised by:** the author, 2026-10-04, as slice 3 of the Review handoff
(`design_handoff_review/Review Room.dc.html`): reading a pull request as a
dyslexic reviewer, without missing the points that matter.

## Problem

Opening a pull request from the Review inbox showed its title and nothing
else. The files were there to be read from disk (FEAT-089) and the reading
settings were there to set them (FEAT-090), but there was no room to read
them in.

## Change

- **Three columns**, as the handoff draws them: the touched files on the left,
  the diff in the middle with the review pill floating over its foot, and the
  Conversation as an inset card on the right. The header says how many files
  are viewed, with a bar.
- **Opening a pull request fetches its head** and lands on the first file not
  yet viewed.
- **Viewed ticks** on each file, in the list and on the file's header. A tick
  is kept against the file's blob at the head: a file the author changes after
  it was ticked comes back unticked, and its stale tick is dropped so the
  count stays true. Kept in the pull request's record, so it survives a
  restart.
- **Changes or Whole file.** Changes shows each changed part with three lines
  either side, and folds what lies between as `N unchanged lines`; each fold
  opens on its own. A run too short to be worth folding is shown. Whole file
  shows every line, the unchanged runs as plain cards between the changed
  ones. Each changed part is headed `@@ -a,b +c,d @@` and the line it sits
  under, as git heads it.
- **One or All.** One reads a file at a time, with previous and next and its
  place among them; *Viewed, next* ticks it and opens the next one not viewed.
  All is every file in one column, each with its header.
- **The review pill**: previous and next file, Changes | Whole file, One |
  All, the focus ruler, `Aa`, and *Viewed, next*. The only blurred surface the
  room adds; no diff row is blurred.
- **Reading aids**, from Settings › Reading:
  - the **focus ruler**, a warm band under the line being read, starting on
    the first change; `j` and `k` move it, and a line number sets it. With
    *Whole chunk* the other chunks fade back;
  - **calm colours** and **changed words**, as on Diff;
  - **`Aa`** switches code and comments between the reading set and code as
    it was set before it.
- **Threads, read-only**: each drawn under its line, inside that line's card,
  with who said what and when. The Conversation card lists them as Open or
  Resolved, and a click goes to the line, opening the fold over it.
- **Only the rows in view are drawn.** The diff is one flat list of rows —
  a file's header, each card's header, lines and threads — each measured once
  drawn; the column stays put under the reader when a row above changes
  height. `src/lib/ui/VirtualRows.svelte` is general, for any long list whose
  rows are not one height.
- **When the head cannot be fetched**, the room reads the host's patch
  instead and says so; Whole file then needs the fetch and is off.
- The changed-file list carries each side's blob id, so the viewed ticks are
  known without reading every file.

## Non-scope

- Conflict fixes — their filter chips, legend and cards: their own item.
- Writing: comments, replies, Resolve and Reopen, the whole-pull-request box
  and Finish review: their own item.

## Acceptance criteria

- From the Review inbox, opening a pull request goes straight to the first
  unviewed file.
- Every file can be read as its changed parts or as the whole file, one at a
  time or all scrolling.
- A ticked file the author changes afterwards reads as unviewed.
- With Calm colours on, no diff row is a solid red or green block.
- A pull request of hundreds of files scrolls without drawing all of them.
