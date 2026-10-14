<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-132 — Plan

**Item:** [FEAT-132](../items/FEAT-132-investigate-the-life-of-a-file-folder-branch-or-line.md)

Written on 2026-10-13, before the code.

## The core

One new module, `crates/spagitty-core/src/investigate.rs`. A life is a list of **entries**, newest first, read a page at a time. Each of the four targets fills the same shape, so the screen draws one list and one chart.

- **File**: `git log --raw --numstat -M`, with `--follow` when *Follow renames* is on. The raw line gives the status and the two paths; the numstat line gives the lines added and removed.
  - **Kind**: `A` on the oldest entry is *born*. `R` in the same folder is *renamed*, and `R` to another folder is *moved*. `D` is *deleted*. Otherwise the share decides between *rewrite* and *edit*.
  - **Share**: lines removed over the lines the file had before the commit. *Before* is exact for a commit with one parent: the lines after, less the lines added, plus the lines removed. `REWRITE_SHARE = 0.5` is the named constant.
  - **Size**: the lines of the blob at each commit, counted from the object. Not summed from numstat, which drifts across merges.
- **Folder**: the same log on a pathspec, without `--follow`. An entry counts the files added, removed and renamed under the folder. Size is files and lines, summed from the oldest entry up. The **biggest edits** are the twenty with the most lines changed; the entries are marked, and the screen lists the structural ones and those.
- **Branch**: born at its merge base with the default branch (the root commit, for the default branch itself). Entries are the commits since the fork, a merge among them marked as one. Where the branch landed is the first commit on the default branch's first-parent line that contains its tip. A local branch's reflog adds its rebases and resets. Size is commits ahead, and the facts carry ahead and behind today.
- **Line**: `git log -L <n>,<n>:<path>`. Each entry carries the line's text as that commit left it.
- **Agents**: the log's format carries each commit's `Co-authored-by` trailers, judged by `attribution.rs`.
- **Today**, for a file or a line's file: one `blame::compact` at the revision gives who owns the file today, by author, and how many lines each commit still has. It is asked for once per life, not once per event.
- **The selected event's largest hunk** is `diff::hunks` between the two blobs, the hunk with the most changed lines.

Reading is paged (`skip`, `limit`), so the first screen paints from the first page. `LIFE_MAX` is 5,000 entries; past it the screen offers *Load more*.

## The screen

`/investigate`, on the rail in Tools after Rebase, code 1O. `/history` redirects to it with its path. File history's own store and view are removed, because this replaces them.

- **Store**: the target (kind, path or branch, line, revision), *Follow renames*, the range, the pages read so far, the selected entry, and today's blame. A life is kept by target and HEAD, a few of them.
- **Life card**: the facts line, the size chart as one SVG path with a marker on each born, renamed, moved and rewrite, a row of ticks per author, and the months under it. Pure functions lay it out (`chart.ts`), so the geometry is tested without a browser.
- **Events**: newest first, each with kind, author, summary and `+/−`.
- **The selected event**: share, lines alive, ownership bar, largest hunk, and *Open file at <sha>*, *Compare with today*, *Show in Graph*.
- **Choosing a target**: the kind is a segmented control. A file, a folder or a line is typed into a field that searches the paths at the revision (`files::find`). A branch is chosen from the branch menu.

No new dependency.

## What decides it is right

- A file renamed in place, then moved to another folder, has one *renamed* and one *moved* entry, and its life goes back past both to the commit that added it.
- A commit that removes half the file's lines is a *rewrite*, and one line fewer is an *edit*.
- A commit's surviving lines are what today's blame gives it.
- A branch is born at its merge base, with a reflog and without one.
- The first page is on screen before the second is asked for.
