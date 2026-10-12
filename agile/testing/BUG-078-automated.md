<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-078 — Automated test record

**Item:** [BUG-078](../items/BUG-078-the-attribution-check-refuses-a-person-named-like-an-agent.md)

| Suite | Cases |
| --- | --- |
| `tools/attribution.test.ts` | Claude Dupont, a bare Claude at an ordinary address, Devin Smith, Cody Lee and a co-author with no address pass. Claude Opus 5.5 and Claude Code at any address, Claude at Anthropic's, Devin AI as a `[bot]`, Devin at Cognition's, Cursor Agent, aider, Antigravity and Gemini are each refused. TASK-064's cases still pass. |

## Results — 2026-10-12

- `bunx vitest run tools/attribution.test.ts`: 7 passed.
- `node tools/attribution.mjs commits origin/main..HEAD` on this branch: nothing found.
- The messages of the last 120 commits on `main`, piped through the check, are still refused for their trailers.
