<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-069 — The two modals that choose a branch use the branch menu

**Status:** Done — on `release/3.4.0`; unmerged. Covered by component tests; not looked at in the running application (the author is checking it).
**Screens:** 1H, 1J.

## Problem

FEAT-122 gave the toolbar, Merger and Rebase one branch menu that can be searched. Two places still choose a branch from a native `<select>`:

- *Create a pull request* (`src/lib/requests/CreatePRModal.svelte`): the base and the head.
- *Add a worktree* (`src/lib/worktrees/AddWorktreeModal.svelte`): the existing branch.

Both list every ref in one run, with no search.

## Why it was left

The menu is `position: fixed`. Both modals have `backdrop-filter`, which makes the modal the containing block for a fixed child, so the menu would be placed against the modal and not the window. It needs either to be moved out of the modal when it opens or to be placed against its containing block, and either is a change to the menu that deserves its own tests.

## Change

- A field that shows the chosen branch and opens `BranchMenu` (`src/lib/branches/BranchMenu.svelte`), for both modals.
- The menu placed correctly inside a modal, above the modal's own layer.

## Acceptance criteria

- Both modals search branches as the toolbar does, with the keyboard and the pointer.
- The menu is drawn where its field is, in a modal, at any window size.
