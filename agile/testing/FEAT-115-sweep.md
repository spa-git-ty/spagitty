<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-115 — Visual and native sweep

**Item:** [FEAT-115](../items/FEAT-115-agents-in-review-and-merge.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-FEAT115-01 | No agent installed, no API agent | Open Review, a pull request's room, Merger's plan and a merge with conflicts | Nothing mentions agents: no button, tab, mark or legend entry. Matches 1.3. | P1 | Pass in the mounted screen tests; not run natively. |
| SWEEP-FEAT115-02 | Claude Code installed | Settings › Agents | Claude Code with its version and path; the others as found; Test answers | P1 | Not run. Needs the author's machine. |
| SWEEP-FEAT115-03 | An Anthropic key | Add an API agent, Test, then Add | The key lands in the keychain; the row shows its last four; Test says the model and the time | P1 | Not run. Needs a key. |
| SWEEP-FEAT115-04 | A pull request, Claude Code | Assign at Step by step | Waits at the plan and each file; findings sit under their lines; Accept makes them pending; Finish review carries *Drafted with Claude Code* | P1 | Not run. Needs the author's machine. |
| SWEEP-FEAT115-05 | A merge with conflicts, Codex | Assign at Sign off, resolve and land | Choices applied with the hexagon badge; checks run; the commit dialog says who chose each; the commit carries the trailer | P1 | Not run. Needs the author's machine. |
| SWEEP-FEAT115-06 | A remote agent, a repository never sent | Assign | Asked once what leaves the machine; each step lists the files sent | P1 | Not run. Needs a key. |
| SWEEP-FEAT115-07 | An Ollama server on this machine | Add it as OpenAI-compatible and assign | Says everything stays on this machine and asks nothing | P2 | Not run. |
| SWEEP-FEAT115-08 | macOS, Windows and Linux | Run 02 to 05 | Same, natively; on macOS the keychain asks once for an ad-hoc build | P1 | Not run. |

The engine was exercised end to end on Linux with a real repository, a real scratch worktree and a script standing in for a command-line agent (`assign::tests::a_command_line_agent_*`). No installed agent CLI, provider key or native window was involved.
