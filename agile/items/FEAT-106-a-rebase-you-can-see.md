<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-106 — A rebase you can see

**Status:** Done — merged into `main`; the manual sweep is not yet run.
**Branch:** `feature/FEAT-106-a-rebase-you-can-see`
**Screens:** 1E.
**Raised by:** the author, 2026-10-07: the Rebase screen was never designed — a header of fields and two plain lists. "Make it easy and visualized like the merger screen."

## Change

Rebase is laid out as Merger is (FEAT-100), and nothing is written until **Rebase now**:

- **Stage.** The branch being moved (amber, *Moves*) on the left with its commits since the two split; the branch it is replayed onto (blue, *Starting point*) on the right, chosen from a menu of branches, remote branches and tags; arrows into the **Result** between them. Choosing a branch plans at once; the branch's upstream is chosen on arrival.
- **Result.** One sentence saying what happens, three counts (commits after, folded, dropped), and a box that says how many commits may stop on a conflict, with a meter a segment per commit — or that the plan cannot run, and why. *Rebase now* and *Reset plan*.
- **The plan.** One card per commit in replay order, coloured by what happens to it: Pick, Reword, Squash or Drop as a segmented control; drag or Alt+↑ / Alt+↓ to reorder; a squash says it folds into the one above (or that there is nothing above); risky rows say *may conflict*. A **reword takes its new message on the row** — before this, reword had no message field and ran as a pick.
- **History after.** The branch drawn as the plan leaves it: the starting point at the foot, each new commit above it, folded ones with how many they hold, reworded ones ringed, risky ones marked, and what is dropped listed.
- **Running and stopped** show in the Result card and the status pill: progress while git replays; when it stops, *Resolve conflicts*, *Continue*, *Skip this commit*, *Abort*. The plan stays visible and locked.

## Acceptance criteria

- A rebase can be planned, read and run from this screen without typing a revision, and the result is visible before anything is written.
