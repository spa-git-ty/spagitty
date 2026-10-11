<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-062 — Finish the three open pull requests

**Status:** Backlog — reported 2026-10-11, not started.
**Screens:** —.
**Raised by:** the author, 2026-10-11: three pull requests are open, none merged, and their validity and quality are not known.

## The pull requests

| PR | What it is | State on 2026-10-11 | What it needs |
| --- | --- | --- | --- |
| #59 | Codex access, OMP and agy launch, agent viewed ticks | 16 commits behind `main`; conflicts in `CHANGELOG.md` and `agile/README.md` | Rebase. Review against `AGENTS.md`. Run its sweep in the real app for every agent. It is the first half of BUG-070. Its access choices take the names from TASK-060. |
| #60 | Gemini thought signatures and `default_api:` tool names | Conflicts in `CHANGELOG.md`; no item, plan or testing records | Rebase. Record it as BUG-071. Test against the real Google API. |
| #57 | Mark pull requests that conflict with their base (BUG-063) | 4 commits behind; *code quality* check failing; same two conflicts; its description carries a tool attribution line, which `AGENTS.md` forbids | Fix the check. Keep the conflict marks. Replace the Pull requests screen's merge path with *Open in Merger*, as FEAT-117 requires. Remove the attribution line. |

## Acceptance criteria

- Each pull request is rebased on `main`, passes every check, and has been reviewed. The review's findings are fixed or recorded as items.
- Each one meets its own item's criteria, tested in the real app, not only by the suite.
- The author merges them. Nothing here is merged without the author.

## For the agent who picks this up

**Who:** unassigned. Any agent working on this repository can start cold.

- Read the CodeRabbit comments already on #57 and #59 first.
- Take #60 and #59 first: they hold the fixes for the two agent bugs.
