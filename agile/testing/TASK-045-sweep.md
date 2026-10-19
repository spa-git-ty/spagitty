<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-045 — Manual sweep

> **Refreshed 2026-10-19 (TASK-073).** TASK-056 reversed part of this item: the rail rows it took away are back. The tickets about the rail's rows are obsolete; Conflicts and the personality tickets still apply.

**Item:** [`agile/items/TASK-045-a-rail-of-five.md`](../items/TASK-045-a-rail-of-five.md)

## Sweep tickets

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-TASK045-01 | A clean repository open | 1. Look at the rail | Farm, Graph, Working copy, Branches, Pull requests, Settings | P1 | Obsolete, 2026-10-19 — TASK-056 put Stash, Tags, Reflog, Rebase, Log and All repositories back on the rail, and the rail has changed since. |
| SWEEP-TASK045-02 | A merge stopped on a conflict | 1. Look at the rail 2. Resolve and continue | Conflicts appears after Working copy, then goes | P1 | |
| SWEEP-TASK045-03 | Any repository | 1. Ctrl+F 2. Toolbar Rebase 3. Tab strip `+` → All repositories | Each screen's row appears, active, and leaves when you navigate away | P1 | Obsolete, 2026-10-19 — TASK-056 made Rebase, Log and All repositories always shown on the rail. |
| SWEEP-TASK045-04 | Any repository | 1. Open Branches 2. Click Tags, Stash, Reflog in the header | Each opens; Branches stays active in the rail; counts match | P1 | Obsolete, 2026-10-19 — TASK-056: Tags, Stash and Reflog are rail rows again, and Branches no longer stands for them; the header control stays. |
| SWEEP-TASK045-05 | A fresh configuration directory | 1. Commit 2. Look for a card, notice or sound 3. Settings | Nothing appears; Personality shows Off; no God mode chip | P1 | |
| SWEEP-TASK045-06 | After -05 | 1. Personality → Balanced 2. Palette → Badges | Badges shows the commit's badge; God mode chip is back | P2 | |
