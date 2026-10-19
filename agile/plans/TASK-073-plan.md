<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-073 — Plan

**Item:** [TASK-073](../items/TASK-073-the-sweeps-refreshed-and-run-from-a-portal.md)

## The portal

1. `tools/sweep/sweeps.ts` reads a sweep document into tickets and writes one result back. Five shapes are read: ticket tables; `## BUG-006-T1` sections with a step table and a `**Result:**` line; `### SWEEP-1F-01` sections with bulleted fields; `## BASE-01` checks with a numbered list and a `**Pass/fail:**` line; and prose, which is one ticket. Each ticket keeps where its result lives, so writing changes that cell or line and nothing else, line endings included.
2. `tools/sweep/index.ts` writes `agile/testing/sweeps.md` from the documents and the agile index.
3. `tools/sweep/serve.ts` serves `tools/sweep/portal.html` on `127.0.0.1`, lists the sweeps, writes results, and keeps attachments in `agile/testing/evidence/` (ignored by git), refusing any path outside it.
4. The page: sweeps with their progress, search and filters, each ticket with its steps, a result, a note, and evidence by file, drop, paste or screen recording.
5. `bun run sweep` and `bun run sweep:index`; a test keeps the index current and every row writable.

## The refresh

1. Rules for what has been replaced since a sweep was written — the theme families (TASK-051), reviewing in Pull requests (FEAT-116), merging there (FEAT-117), File history (FEAT-132), agy (TASK-071), dropping a branch on another (TASK-056), the title bar (FEAT-043, FEAT-082), the explanatory copy (TASK-050) — run over every ticket not yet passed.
2. Every ticket of the items before the revamp of 2026-09-29 (FEAT-001 to FEAT-081, TASK-001 to TASK-044, BUG-001 to BUG-058), 1,063 of them, read by title, and the doubtful ones in full.
3. What no longer exists is marked *Obsolete*, naming what replaced it, through the portal's own writer. Wording that names an old screen but still describes a real check is brought up to date. The documents most changed get a note under their heading. Nothing is deleted.
