<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-136 — Sweep

**Item:** [FEAT-136](../items/FEAT-136-issues.md)

None of these rows was run in the application. Automated cases are named in the automated record.

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT136-01 | Connected GitHub or GitLab remote | Open Issues, select a remote and an issue | Virtual list, thread and details use the selected host. New group sits above Settings. | P1 | Not run. Covered by mounted tests. |
| SWEEP-FEAT136-02 | Labels and milestones exist | Use state, Mine, label, milestone, sort and text filters | Host list reflects the chosen filters; counts remain visible. | P1 | Not run. Filter mapping covered by core tests. |
| SWEEP-FEAT136-03 | Saved page exists | Disconnect network and refresh | Saved issues remain and the read error is visible. | P1 | Not run. Covered by state tests. |
| SWEEP-FEAT136-04 | Writable account | Create, comment and close an issue | Each action asks for its own confirmation; cancelling leaves the draft. | P1 | Not run. Covered by core and mounted tests. |
| SWEEP-FEAT136-05 | Default tracking branch exists | Edit Start branch suggestion and create | Confirm, create and check out from the default tracking tip. Untracked work remains. | P1 | Not run. Covered by command fixture. |
| SWEEP-FEAT136-06 | An issue has comments | Hand to agent | Full brief opens in Farm for review and agent selection, initially unready. | P1 | Not run. Draft covered by mounted test. |
| SWEEP-FEAT136-07 | Many issues and comments | Page the list and the thread | More rows append without duplicate ids; comments and events have dates. | P2 | Not run. Covered by state tests. |
| SWEEP-FEAT136-08 | Read-only account or uncertain delivery | Attempt a write | Permission refusal hides writes; failed delivery is never retried automatically. | P1 | Not run. Covered by core and state tests. |
| SWEEP-FEAT136-09 | Two repositories or remotes | Switch while a read is pending | Old answers do not replace the current repository. | P1 | Not run. Covered by state tests. |
| SWEEP-FEAT136-10 | Light theme and narrow window | Read list, comments and details | Labels and text remain legible; panels remain usable. | P2 | Not run. |
