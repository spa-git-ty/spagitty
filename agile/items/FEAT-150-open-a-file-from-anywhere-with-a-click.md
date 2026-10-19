<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-150 — Open a file from anywhere with a click

**Status:** Done — on `release/3.3.0`; unmerged. Covered by component tests; not looked at in the running application (the author is checking it).
**Screens:** every screen that lists files
**Raised by:** the author, 2026-10-19: "on any part of the app if i click on a file I can open it on the file viewer".

## Change

A file row could be opened in Files only from its right-click menu
(`FileMenu.svelte`). Now it opens with the pointer as well, without changing
what a click already did where a click did something.

## Acceptance criteria

- Every row marked as a file (`data-file-path`) opens in Files on a
  double-click, and on a click with Ctrl (⌘ on a Mac), at the revision the row
  shows and, where the row knows one, at its line.
- A row whose click did nothing — a name in a list — opens on a plain click.
  It is marked `data-file-open="click"`. A button inside it keeps its own click.
- A row that selects a diff on click (Changes, Commit, Pull requests, Review)
  keeps doing that; the double-click and Ctrl-click open Files.
- A file Files cannot read (not in a commit yet) does not open; its right-click
  entry still says why. A double-click in a text field is left to the field.
- Newly marked: Home's working-copy paths, the agent card's plan and the paths
  an agent read, CodeRabbit's findings (at their line), search results' files,
  a farm task's changed files, and the resolver's files that merge on their own.

## What was built

- `files/open.ts`: `fileAttrs` takes `{ click, line }`; `targetOf` reads a row.
- `files/FileMenu.svelte` listens for click and double-click on the window, as
  it already did for the right-click.
