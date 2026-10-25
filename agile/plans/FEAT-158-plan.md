<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-158 — Plan

**Item:** [FEAT-158](../items/FEAT-158-type-to-search-any-list.md)

1. `src/lib/ui/speedsearch.svelte.ts`: `match` (substring, else word starts in order, with a memo so a long name cannot go exponential); `createSearch`, one list's query with `filter`, `test` and `ranges`; the `speedSearch` action, a registry of lists, and one capture-phase `keydown` and `pointerdown` listener on the window that decide which list the letters go to.
2. `src/lib/ui/SpeedSearchBox.svelte`: the pill, mounted once in `src/routes/+layout.svelte`, placed over the searched list's top right. It never takes the focus.
3. `src/lib/ui/Hit.svelte`: a row's text with the matched stretches marked.
4. Lists with a field: Branches, Tags, Reflog, Files, Untrack and Notifications pass `field`, and the first letter is typed into it.
5. Lists without one filter their rows through `createSearch`: Stash (`reserve: 'jk'`), Working copy, Repositories, Pull requests, Review, Issues, Releases, Investigate's events, Search's results, Terminal's sessions (`primary: false`), Worktrees, Submodules.
6. Graph: `onstep` and a `hits` list; the rows that do not match are dimmed with the author filter's `dim`, and each letter jumps to the first match from the selection.
