<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-063 — Sweep

**Item:** [TASK-063](../items/TASK-063-audit-the-merged-pull-requests.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-TASK063-01 | TASK-064's check on a pull request | Follow SWEEP-TASK064-01 to 03 | The check fails on attribution and passes a clean change | P1 | Partly, 2026-10-12: a clean change passes (SWEEP-TASK064-03). The two refusals are not run. |
| SWEEP-TASK063-02 | The author | Decide what happens to the trailers and *Claude*-authored commits already on `main`, and to the merged descriptions | The decision is written in the item | P1 | Done, 2026-10-12: nothing is rewritten; the rule is for what comes next and is in `AGENTS.md`. |
