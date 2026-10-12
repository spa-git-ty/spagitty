<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-122 — One branch menu, that can be searched and says where you are

**Status:** Done — on `feature/FEAT-121-say-what-is-running`; unmerged. Seen in the built frontend against fixtures; not yet in the running app.
**Screens:** chrome, 1S, 1E.
**Raised by:** the author, 2026-10-12, on 2.1.0: the branch dropdown in Merger and elsewhere looks scattered, the current branch is badly marked, and branches cannot be searched.

## Problem

A branch was chosen from the right-click menu (`src/lib/ui/Menu.svelte`) with branch names in it: in the toolbar's switcher, on Merger's two cards, and on the Rebase screen. Three things about that were wrong for a list of names.

- **It could not be searched.** This repository has about forty local branches. The one wanted had to be found by reading.
- **Where you are was the faintest row.** The only way that menu can say "this one" is to disable the entry, so the branch checked out was drawn at under half strength with *already on it* beside it.
- **It looked scattered.** Each row was a name against the left edge and a note against the right: an upstream, *checked out*, a reason. A column of them read as two ragged lists, and the menu was narrow enough to cut most names short.

The toolbar's list was also read once and kept, so a branch made on the graph or in Merger was missing from it until the Branches screen was opened.

## Change

One component, `src/lib/branches/BranchMenu.svelte`, used in all three places.

- **A field to type in**, focused on opening. Every word typed has to be in the name, in any order and any case. The best answer is first: the name itself, then a name that starts with it, then a part of a name that does, then anywhere.
- **The match is marked** in each name, and the field says how many of how many were found.
- **Groups under headings** with how many each holds: *Branches*, *Remote branches*, *Tags*. A heading stays in view while its group scrolls.
- **One column.** Every row has a slot for the tick and then the name. The folder part of a name (`feature/`) is quieter than the part that tells it from its neighbours. Nothing is pushed to the far edge except the reason a branch cannot be picked.
- **Where you are, at full strength.** The branch held is ticked, in the accent, and heavier. The branch checked out carries `current`, wherever it is listed. Picking the branch already held closes the menu and does nothing.
- **Keys.** The focus stays in the field. The arrows move over the rows that can be picked, Enter picks, Escape closes and gives the focus back to the control.
- **It keeps its place.** Opened from the toolbar, which is at the bottom of the window, it goes above the control. Its height is held while typing, so the field does not move under the fingers.
- **A long list is cut.** A hundred rows of a group are drawn and the rest are said to be found by typing. Tags are the newest thirty until something is typed, and every tag that matches after.
- **The toolbar reads its list on every opening**, showing what it had meanwhile.

## Acceptance criteria

- Typing narrows the list in the toolbar, on both Merger cards and on Rebase, and Enter picks the best match.
- The branch held is ticked and is not fainter than its neighbours; the branch checked out is marked `current`.
- A branch that cannot be picked is listed with its reason and is not picked by click or by Enter.
- The menu stays inside the window, opens above a control near the bottom, and does not change size while typing.
- A second press on the control closes it (BUG-018), as do Escape, a press elsewhere and the focus leaving.

## What was done

- `src/lib/branches/pick.ts`: the rules, apart from the component so they can be tested: `search`, `sections`, `marks`, `drawn`.
- `BranchMenu.svelte`: the list. It takes the shared `.floating` surface, so the count of components that blur what is behind them does not rise.
- `src/lib/chrome/Toolbar.svelte`, `src/lib/merger/BranchPicker.svelte` (which the Rebase screen also uses) open it in place of `Menu`.
- The menu is placed by its laid-out size. `Menu` reads a rectangle while it is still growing in, a tenth too small; here that put a menu near the right edge past it.

## Not in this change

- *Create a pull request* and *Add a worktree* choose a branch from a native `<select>`. They are recorded as TASK-069.
- The Branches screen keeps its own filter and table.
