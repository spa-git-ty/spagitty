<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-064 — Automated test record

**Item:** [TASK-064](../items/TASK-064-refuse-agent-attribution-in-ci.md)

| Suite | Cases |
| --- | --- |
| `tools/attribution.test.ts` | A co-author that is an agent or a tool is refused, three spellings; a session trailer and two *Generated with/by* lines are refused; a person as co-author passes; prose that names an agent passes; a message with Windows line endings is read. |

## Results — 2026-10-12

- `vitest run tools/attribution.test.ts`: 5 passed.
- By hand: `node tools/attribution.mjs commits origin/main~40..origin/main` lists the trailers on `main` and exits 1; the same over this branch's commits exits 0; `text -` refuses a description with a *Generated with* line and passes a clean one.
