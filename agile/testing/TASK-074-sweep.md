<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-074 — Sweep

**Item:** [TASK-074](../items/TASK-074-pull-requests-no-longer-list-gates-5-and-6.md)

| Ticket | Preconditions | Steps | Expected result | Result |
| --- | --- | --- | --- | --- |
| SWEEP-TASK074-01 | A pull request into `main` | Open its checks | Gates 0 to 4 under *checks /*; no *5 · build* or *6 · release* | Not run |
| SWEEP-TASK074-02 | That pull request merged, with a version bump | Open the run on `main` | Gates 0 to 4, then 5 builds four downloads and 6 publishes the release | Not run |
