<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-158 — Automated test record

**Item:** [FEAT-158](../items/FEAT-158-type-to-search-any-list.md)

- `src/lib/ui/speedsearch.test.ts`: an empty query matches everything; substrings in any case; word starts in order (`fb`, `cr`, `comro`, `r39`); no letters from the middle of words; every word needed; a wrong first guess backed off. `createSearch` narrows by any of a row's texts. Mounted through `src/testing/SpeedSearchHarness.svelte`: the first letter opens and narrows, the pill shows the query and `n of m`, matches are marked, Backspace and Escape, Enter opens the first match, a field's letters, Ctrl and Cmd, a space, reserved letters, an open menu and a modal are left alone, a field-backed list takes the letter into its field, clicking away ends the search and clicking a match keeps it.
- `src/lib/graph/rows.test.ts`: a key the listbox does not handle is left alone (now `F2`, since a letter starts a search); typing jumps to the first match, dims the rest, keeps every row, and the arrows wrap between matches; Escape undims.
- `src/lib/ui/flat.test.ts`: the pill's key hint is sized in `em`, not pixels.

## Results — 2026-10-25

- `bunx vitest run`: 230 files, 5115 tests passed (the Files selection test that times out under the full run's load passed on its own, as it did for FEAT-156).
- `bun run check`: 0 errors, 0 warnings.
- `bun run build`: succeeds.
