<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-099 — The Hooks screen's side panel cards do nothing when clicked

**Status:** Fixed — on `feature/FEAT-158-type-to-search`, in release 3.10.0; unmerged. Covered by a mounted test. The diagnosis is from the code, not from the author's repository; the author is checking whether it is the case they hit.
**Branch:** `feature/FEAT-158-type-to-search`.
**Screens:** Hooks (1X).
**Raised by:** the author, 2026-10-25: "hooks screen is non functional btw nothing happen on click any of the side panel cards !"

## Cause

The click handlers work: the existing mounted tests add every block and pick every step. What fails is a repository whose hooks belong to a manager (Husky, lefthook, pre-commit). There the builder is read-only until the hooks are imported, and:

- every card in the *Add* catalogue was `disabled`, with no disabled style, so it looked like a card and did nothing;
- on *Runs*, the manager's hooks were listed as cards (`.step`) that are not buttons, since a manager's run has no steps to pick.

The banner that offers *Import* sits above both, easy to miss.

## Fix

- A catalogue card is never silently dead. While a manager owns the hooks, clicking one offers the import, through its existing confirmation, and then adds the block. Disabled cards (while saving, or at 100 blocks) now look disabled.
- A manager's hooks on *Runs* are drawn as plain lines, not cards.

## If this is not it

If the author's repository has no hook manager, this is not the fault they saw. The next things to check are a screenshot of the screen and the repository's `core.hooksPath`.
