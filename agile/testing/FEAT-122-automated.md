<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-122 — Automated test record

**Item:** [FEAT-122](../items/FEAT-122-one-branch-menu-that-can-be-searched.md)

| Suite | Cases |
| --- | --- |
| `src/lib/branches/pick.test.ts` | Words are lowered and split; the name itself ranks before a prefix, a part and anywhere, for every separator a branch name uses; a missing word finds nothing; the order given is kept among equals and when nothing is typed; groups come in a fixed order and an empty one is left out; a long group is cut and says how many it found; tags are the newest few until something is typed; a match is marked in every place, in the name's own case, and across the slash; every character of a name is drawn once. |
| `src/lib/branches/BranchMenu.test.ts` | Groups under headings with their counts; a listbox driven from a combobox; every row a tick slot and then the name; the folder part quieter; the reading and empty states; the cut of a long group. The branch held is ticked and not disabled or dimmed; the branch checked out is marked `current` when another is held; one tick only; picking the branch held closes and picks nothing; a branch with a reason is listed and not picked. The focus goes to the field; typing narrows and counts; any part, any case; every group is searched; the match is marked; nothing matching says so. The cursor starts on the branch held; Enter picks the best match; the row under the keys is named for a screen reader; the arrows skip what cannot be picked and wrap; Enter with no row does nothing; Escape closes, picks nothing and gives the focus back; a press on a row does not take the focus; the pointer moves the cursor only onto a row that can be picked. Close comes before pick; a press elsewhere, the focus leaving and a resize close; a press on the control or inside does not. |
| `src/lib/chrome/chrome.test.ts` — *Toolbar* | The branch checked out is first, ticked, marked `current`, not disabled, and is not switched to; a branch is found by typing part of its name and Enter switches to it; the list is read again on a later opening. The earlier cases still hold: a second click closes, a press inside does not, local branches only, a refused checkout is said. |
| `src/routes/merge/page.test.ts`, `src/routes/rebase/page.test.ts` | Picking another branch on a Merger card asks for the forecast again; picking a branch on Rebase plans. Both through the new list. |
| `src/lib/ui/flat.test.ts` | The number of components that blur what is behind them is unchanged. |

## Results — 2026-10-12

- `bunx vitest run src/lib/branches/pick.test.ts src/lib/branches/BranchMenu.test.ts src/lib/chrome src/routes/rebase src/routes/merge/page.test.ts`: 196 passed.
- `bunx vitest run`: 4036 passed in 189 files. `bun run check`: zero errors, zero warnings.
