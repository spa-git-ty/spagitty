<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-138 — Sweep

**Item:** [FEAT-138](../items/FEAT-138-pipelines.md)

None of these rows has been run in the built application.

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT138-01 | Workflows in working copy | Open Pipelines, change event/ref/range, select jobs and expand a matrix | Conditions and file problems update; unknown context says decided on the host. | P1 | Not run. Core and mounted tests cover calculations and controls. |
| SWEEP-FEAT138-02 | GitLab local includes and reusable GitHub workflows | Select the workflow and inspect dependencies and source links | Sources retain lines; local data expands; remote data remains opaque. | P1 | Not run. Core fixtures cover normalization. |
| SWEEP-FEAT138-03 | More than 200 jobs | Open graph, expand stages, switch Needs/Stages and scroll | Stage summary first; visible cards and edges only; controls remain reachable. | P1 | Not run. Mounted tests cover collapse and matrix expansion. |
| SWEEP-FEAT138-04 | Connected GitHub account | Open Live, select a run and active job | Steps refresh; the log appears after completion. Run graph uses the run's commit. | P1 | Not run. API shapes and store behavior covered by fixtures. |
| SWEEP-FEAT138-05 | Connected GitLab account | Select a running job, then Cancel; cancel the confirmation and repeat | Partial trace refreshes. Each host write asks once; failures remain visible. | P1 | Not run. Confirmation and trace state covered by tests. |
| SWEEP-FEAT138-06 | Failed completed run | Re-run failed; switch branches while reads are pending | One confirmed host attempt; stale responses cannot replace the selected run. | P1 | Not run. Store tests cover confirmation and stale reads. |
| SWEEP-FEAT138-07 | GitLab CI file | Check with GitLab lint | Current root file is sent only after a separate confirmation. | P1 | Not run. No real lint submission performed. |
| SWEEP-FEAT138-08 | Connected repository | Watch a branch, restart, hide the window, then return | Personal watches persist; default and own request branches remain; hidden-window polling stops. | P1 | Not run. Persistence/shared-clock behavior tested. |
| SWEEP-FEAT138-09 | Light theme and narrow window | Read both views, job details and a long log | Text and controls remain legible and reachable. | P2 | Not run. |
