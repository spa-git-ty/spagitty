<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-083 — Hiding the blame in Files breaks every line

**Status:** Fixed — on `release/3.0.1`; unmerged. Covered by mounted tests. Not looked at in the running application: the author is checking it.
**Screens:** 1U (Files), and every screen that lists a file: 1A, 1C, 1D, 1K, 1Q, 1R, 1S, Stash, Conflicts, Pull requests, Log search.
**Raised by:** the author, 2026-10-14, on 3.0.0, with two pictures of Files: "fixes needed on version 3 for this, when i hide blame pan on this window". In the same session: "also the blame ban should be resizable !!" and "also ANY FILE ,, in ANY FIEW i can right click and open on viewer".

## What happens

With the blame shown, a file reads well. Press **Blame** or `b` to hide it and every line breaks: the line number is drawn over the first letters of the code, and the code is cut after a few characters.

## Cause

Each line is a grid of three columns: blame, number, text. With the blame hidden, its column was still there at no width, but nothing was drawn in it. The number and the text, left to flow, took the first two columns: the number the empty one, the text the number's.

## Acceptance criteria

- With the blame hidden, the number and the text are in their own columns and the code has the rest of the width.
- The blame's width is changed by dragging its edge, or with the arrow keys once the edge has the focus. It is kept with the other panels' widths, and the edge is not there while the blame is hidden.
- A file on any other screen opens in Files from a right-click, at the revision that screen shows. A file Files cannot read is offered with the reason and cannot be chosen.

## Fix

- **`FileView.svelte`**: the number is placed in column 2 and the text in column 3. The blame is `--files-blame-w` wide, a panel of its own (`filesBlame`, 140 to 520, 260 at first), and its edge is the `Splitter` the other panels use. It replaces a width that was a quarter of the pane, between 180 and 290.
- **Open in Files.** A row that names a file says so with `data-file-path`, and the revision it shows with `data-file-rev` (`src/lib/files/open.ts`). One menu on the window (`FileMenu.svelte`, in the layout) offers *Open in Files* there. The Diff and Stash lists have a menu of their own, and it has the entry first. The revision is the one the screen shows:
  - **a commit's file** (Graph, Diff, Stash, Log search, a commit of a pull request): that commit. A file the commit deleted is read at its parent.
  - **Working copy and Conflicts**: the branch that is checked out. Files reads commits, not the working copy, so a new file, staged or not, is offered with *Not in a commit yet* and cannot be chosen.
  - **Merger**: side A's tip.
  - **Review**: the fetched head. A file the pull request deletes is read where the head left the target. Not fetched, it cannot be chosen.
  - **Pull requests**: the head the host gives. Files reads only this repository, so a head that has not been fetched opens to Files' own error.
  - **Farm**: the task's branch.

## Not done

- Files' own tree is not marked, and neither is Investigate, which has an *Open in Files* button.
- The second and third requests add behaviour, which a PATCH release does not by the changelog's rule. The author asked for 3.0.1.
