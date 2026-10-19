<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-069 — Sweep

**Item:** [TASK-069](../items/TASK-069-the-two-modals-that-choose-a-branch-use-the-branch-menu.md)

| Ticket | Preconditions | Steps | Expected result | Result |
| --- | --- | --- | --- | --- |
| SWEEP-TASK069-01 | A repository on GitHub or GitLab with several branches; Pull requests open | **Create pull request**, then click *Base branch* | The branch menu opens right under the field, above the dialog, focused in its search | Not run; the author is checking it |
| SWEEP-TASK069-02 | Same | Type part of a branch name, press the arrows and Enter | The list narrows; Enter fills the field; the dialog stays open | Not run |
| SWEEP-TASK069-03 | Same | Open *Head branch*, press Escape | Only the menu closes, not the dialog | Not run |
| SWEEP-TASK069-04 | Worktrees open | **Add worktree**, *Existing Branch*, click *Branch* | The menu opens under the field; picking fills it | Not run |
| SWEEP-TASK069-05 | Same | Resize the window small and open the field near the bottom | The menu stays inside the window, above the field if there is no room below | Not run |
| SWEEP-TASK069-06 | Light and dark | Open either field | The menu reads as the toolbar's does | Not run |
