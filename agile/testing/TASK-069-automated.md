<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-069 — Automated test record

**Item:** [TASK-069](../items/TASK-069-the-two-modals-that-choose-a-branch-use-the-branch-menu.md)

- `src/lib/branches/BranchField.test.ts`: the field shows its branch or its placeholder; it opens the menu as a child of `<body>`, outside the field's own tree, with the dialog layer; typing narrows it, and a click or Enter hands the pick over and closes it; destroying the field leaves nothing in `<body>`; a busy form's field is disabled.
- `src/lib/branches/pick.test.ts`, *optionsOf*: branches, remote branches and tags each land in their group.
- `src/lib/branches/BranchMenu.test.ts`: unchanged, and still passing, for the page layer.

## Results — 2026-10-19

- `bunx vitest run src/lib/branches`: 158 passed.
- `bun run check`: 0 errors, 0 warnings.
- jsdom does not lay anything out, so where the menu is drawn in a real window is owed to the sweep: the author is checking it.
