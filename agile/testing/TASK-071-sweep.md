<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-071 — Sweep

**Item:** [TASK-071](../items/TASK-071-remove-agy.md)

None of these has been run. The change was checked by its tests and by the build, not in the running application.

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-TASK071-01 | A machine with agy installed | Open Settings › Agents | agy is not listed, found or missing. The other agents are as they were | P1 | Not run. |
| SWEEP-TASK071-02 | An `agents.json` saved by 2.2.1 with **Auto-approve tools** chosen | Start the application and open Settings › Agents | The section opens. Codex's access choice, the remote agents and the repository's rules are as they were saved | P1 | Not run. |
| SWEEP-TASK071-03 | A repository whose Farm was opened by 2.2.1 on a machine with agy, with a custom agent added | Open its Farm | The custom agent and the capabilities chosen for the others are still there. agy is not | P1 | Not run. |
| SWEEP-TASK071-04 | A pull request open in Review, and two branches in Merger | Open **Assign** in each | agy is not among the agents offered | P2 | Not run. |
| SWEEP-TASK071-05 | Codex installed | In Settings › Agents choose **Full Access**, then press **Test** at once | **Test** waits for the choice to be saved, and the previous result is cleared | P2 | Not run. |
