<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-152 — Plan

**Item:** [FEAT-152](../items/FEAT-152-the-files-tree-widens-and-copies.md)

1. `FILES_TREE_W` in `metrics.ts` and a `filesTree` panel in `panels.svelte.ts` (180 to 640 px), so the width is clamped, kept and reset like every other.
2. A `Splitter` between `FileTree` and `FileView` on the Files page; the tree takes `--files-tree-w`, and *Text* at least 340 px.
3. `FileTree`: a right-click on a tree row, a found file or a search result's file opens a `Menu` with *Copy file name*, *Copy relative path* and, for a file, *Copy file content*, read with `filesRead` at the resolved revision.
4. `VirtualRows` takes `hold`, rows kept drawn whatever the scroll. `FileView` follows `selectionchange` and holds the rows between the selection's two ends while it is not collapsed.
