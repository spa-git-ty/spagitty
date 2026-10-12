<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-118 — Sweep

**Item:** [FEAT-118](../items/FEAT-118-an-agents-work-shown-properly.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT118-01 | An agent reviewing a pull request | Open its card; do not press Details | Only the step list and the current sentence show; none of what was sent or said | P1 | Not run. |
| SWEEP-FEAT118-02 | As above | Press a finished file step, read Details | What Spagitty sent, then the agent's reply as formatted text, the commands as code, the files it read as links; Raw output is below | P1 | Not run. |
| SWEEP-FEAT118-03 | A Codex and a remote agent | The same | Each part is labelled and typed | P2 | Not run. |
