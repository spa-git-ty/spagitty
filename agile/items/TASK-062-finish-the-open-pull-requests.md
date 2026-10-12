<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-062 — Finish the three open pull requests

**Status:** Done — the three were rebased, reviewed and fixed, and merged together as pull request #63 on 2026-10-12; #57, #59 and #60 were closed in its favour.
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

## What was done, 2026-10-11

Nothing is merged and nothing is pushed. The three branches are stacked, because each changes files the one before it changes, so they merge in this order: pull request #62 (the records), then #60, then #59, and the work they carry; #57 is replaced by the rebuilt branch.

| PR | Branch | What was found and done |
| --- | --- | --- |
| #60 | `fix/gemini-thought-signatures` | Rebased. Reviewed: the signature was echoed inside `functionCall` (a field Google does not have) and one signature was copied onto every call; both are corrected, and the other causes of a Gemini 400 are fixed (BUG-071). Records added. |
| #59 | `bugfix/BUG-065-agent-cli-access` | Rebased. Reviewed: it is the first half of BUG-070; the second half (the prompt cannot cross a Windows `.cmd` shim as an argument) is added on top. One CodeRabbit concern stands: Settings *Test* accepts the unpredictable value anywhere in the output, not only in the final answer. Its access choices already use *Full access* (TASK-060). The real-agent sweep is not run. |
| #57 | `bugfix/BUG-063-a-conflicting-pull-request-is-not-marked` | Rebuilt on the stack, since FEAT-117 took merging out of Pull requests: the Review and Pull requests marks are kept, the merge dialog changes are dropped, and the failing *code quality* check (a type error on the *Back with you* path) is fixed. **The pull request's description still carries an attribution line, which `AGENTS.md` forbids; it has to be removed on GitHub by whoever updates it.** |
