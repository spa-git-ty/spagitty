<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-064 — Refuse agent attribution in commits and descriptions, in CI

**Status:** Done — the check is in gate 1; its sweep is owed until it has run on a real pull request (2026-10-12).
**Screens:** —.
**Raised by:** TASK-063: `AGENTS.md` forbids agent, tool and session attribution, and 127 trailer lines and five pull request descriptions carry it anyway.

## Acceptance criteria

- A pull request whose commits carry a `Co-Authored-By:` naming an agent or a tool, a `Claude-Session:` trailer, or a *Generated with* line fails gate 1.
- So does a pull request whose description carries one.
- A person named as a co-author, and prose that merely mentions an agent, pass.
- Nothing before the pull request is read: history is the author's to decide (TASK-063).

## Fix

`tools/attribution.mjs` reads the pull request's own commits (`base..head`) and its description, handed in through the environment so the text never reaches a shell. Gate 1 runs it on `pull_request` events; `docs/ci.md` names it. Unit tests are in `tools/attribution.test.ts`.
