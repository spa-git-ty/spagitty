<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-137 — Sweep

**Item:** [FEAT-137](../items/FEAT-137-hooks.md)

None of these rows has been run in the application. Automated cases are named in the automated record; they do not replace the manual inspection.

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT137-01 | A repository open | Open Hooks, then Hooks inside the screen | The new group is above Settings with a separator. The builder has stages, editor and seven catalogue blocks. The existing shell is unchanged. | P1 | Not run. Covered by mounted tests. |
| SWEEP-FEAT137-02 | A staged file in a module | Add a tests block, set its paths and command, then Dry run on staged | Confirm first; Runs shows the matching files, current step and its output without making a commit. | P1 | Not run. Covered by fixture, command and mounted tests. |
| SWEEP-FEAT137-03 | Message rules added | Try a valid message, then an invalid one | The generated script accepts or refuses it and says why. | P1 | Not run. Covered by command and mounted tests. |
| SWEEP-FEAT137-04 | A reviewed definition | Save & install, then commit from a terminal | Confirm first. The same blocks run, and Runs shows their record and output. | P1 | Not run. Covered by fixture tests. |
| SWEEP-FEAT137-05 | A file staged in part | Run a formatter with restage on | The partially staged file is left alone and the block reports the reason. No unstaged content enters the index. | P1 | Not run. Covered by fixture tests. |
| SWEEP-FEAT137-06 | A fully staged file formatted | Undo restage; then stage another edit and retry | The first undo restores the old index entry and leaves the working file. The later staging decision is refused. | P1 | Not run. Covered by fixture tests. |
| SWEEP-FEAT137-07 | A manager owns the hooks | Open Hooks, then Import | Runs remains available. The builder is read-only until explicit import. The original manager files stay in place. | P1 | Not run. Covered by fixture and mounted tests. |
| SWEEP-FEAT137-08 | A slow command | Run it, then Stop; separately let a short timeout expire | Each run ends and records cancellation or timeout. | P1 | Not run. Covered by fixture tests. |
| SWEEP-FEAT137-09 | Personal definition exists | Share with repo; update the shared file outside the application and reopen | The team's current definition is shown. Switching back to personal does not change the shared file. | P2 | Not run. Covered by fixture tests. |
| SWEEP-FEAT137-10 | More than one block in a stage | Drag and use the move buttons | Order changes inside the stage. Changing views retains the draft. | P2 | Not run. Ordering covered by store tests. Dragging not driven. |
| SWEEP-FEAT137-11 | An unmarked hand-written hook | Try Save & install | The hook is refused and its content stays as it was. | P1 | Not run. Covered by fixture tests. |
| SWEEP-FEAT137-12 | A linked worktree | Make a terminal commit, open its run from either checkout | Both read the common run record. | P2 | Not run. Common storage covered by a fixture test. |
| SWEEP-FEAT137-13 | A light theme and a narrow window | Read both tabs | Controls, steps and logs remain legible and usable. | P2 | Not run. Token rules covered by structural tests. |
