<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-069 — Plan

**Item:** [TASK-069](../items/TASK-069-the-two-modals-that-choose-a-branch-use-the-branch-menu.md)

1. A `portal` action (`src/lib/ui/portal.ts`) that moves an element to the end of `<body>` while it is mounted and removes it when it is not.
2. `BranchMenu` takes `layer: 'page' | 'dialog'`. `dialog` portals it and puts it at z-index 1001, above the dialogs' scrim at 1000; `page`, the default, leaves the toolbar, Merger and Rebase menus exactly as they were.
3. `BranchField` (`src/lib/branches/BranchField.svelte`): a field that shows the branch chosen, or a placeholder, and opens the menu below itself with `layer="dialog"`. It is a `<button>`, so the form's `<label for>` still names it.
4. `optionsOf` in `pick.ts` turns the Branches store's rows into the menu's options, each under the group of its kind.
5. *Create a pull request* (base and head) and *Add a worktree* (existing branch) use the field. The text input each falls back to with no branches loaded stays.
6. Tests: the field, the menu's place in `<body>` and its removal, search, pointer and keyboard picks, `optionsOf`.
