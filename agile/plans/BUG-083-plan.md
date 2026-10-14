<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-083 — Plan

**Item:** [BUG-083](../items/BUG-083-hiding-the-blame-in-files-breaks-every-line.md)

1. Find why the lines break with the blame hidden: the grid keeps the blame's column at no width, and the number and text flow into the wrong columns. Place them in their own columns.
2. Make the blame a panel of its own in `panels.svelte.ts`, and put the shared `Splitter` on its edge, over the rows.
3. Give every file row on every screen `data-file-path` and the revision that screen shows, and add one window-level menu that opens such a row in Files. Put the entry in the Diff and Stash menu that already exists.
4. Test the columns, the edge, the address, the revisions and the menu. Run the type check and the whole suite.
5. Release it as 3.0.1, as the author asked.
