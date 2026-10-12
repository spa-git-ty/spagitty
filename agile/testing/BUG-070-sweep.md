<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-070 — Sweep

**Item:** [BUG-070](../items/BUG-070-local-agents-do-not-get-their-instructions.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG070-01 | Claude Code, Codex, Cursor, Oh My Pi, agy and a custom agent installed (Windows with npm shims) | Settings › Agents, press *Test* on each | Each reports ok after reading the test file | P1 | Not run. |
| SWEEP-BUG070-02 | As above | Assign each to one Review step on a pull request | The step finishes with findings that show the agent had the instructions | P1 | Not run. |
| SWEEP-BUG070-03 | As above | Assign each to one Merger step | The step finishes with a plan | P1 | Not run. |
| SWEEP-BUG070-04 | macOS and Linux | Repeat 01 to 03 | The same | P1 | Not run. |
| SWEEP-BUG070-05 | Codex whose sandbox cannot start | Press *Test* | It fails and names Settings › Agents › Codex | P2 | Not run. |
| SWEEP-BUG070-06 | Windows; a custom agent whose executable is a `.cmd` file and whose arguments hold `{prompt}` | Press *Test*, then assign it to a Review step | Both say the file is a `.cmd`, and to use its `.exe` or take `{prompt}` out; nothing is started | P2 | Not run. |
