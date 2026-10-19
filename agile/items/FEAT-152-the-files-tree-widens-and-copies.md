<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-152 — The Files tree widens by dragging, copies from a right-click, and a long selection holds

**Status:** Done — on `feat/FEAT-152-files-tree`; unmerged. Covered by component tests; not looked at in the running application (the author is checking it).
**Branch:** `feat/FEAT-152-files-tree`.
**Screens:** 1U (Files).
**Raised by:** the author, 2026-10-19: "only add in files view , expand files hirarchy ( click and drag ) , right click ( copy file name , copy file relative project path )", then "also when i try to select from file the select buffer loses its parts while i scroll ! i think only the visiable part is selected , fix it + you can add for the right click "copy file content"".

## Change

- **The tree is as wide as it is dragged.** A divider between the tree and the file, like the blame's: drag it, arrow keys move it, a double-click resets it, and the width is kept. 180 px to 640 px, 240 px at first. *Text* search still opens at 340 px at least.
- **Right-click a row in the tree, or a file the search found:** *Copy file name*, *Copy relative path* (from the top of the repository) and *Copy file content* (the file's text at the revision Files shows, read from git, never the working copy). A folder offers its name and path. Content that is not text says so and copies nothing.
- **A selection longer than the screen keeps all of it.** The file is a virtual list, and the rows a selection ran through were dropped as they scrolled away, so the browser lost the selection's far end and only what was on screen was copied. The rows between a selection's two ends now stay drawn while it lasts.

## Acceptance criteria

- Dragging the divider widens and narrows the tree, and the width survives a restart.
- The three copies put the name, the repository-relative path and the text on the clipboard.
- Selecting from line 1 and scrolling far down, then copying, copies every line in between.
