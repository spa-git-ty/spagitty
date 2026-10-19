<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-073 — Automated test record

**Item:** [TASK-073](../items/TASK-073-the-sweeps-refreshed-and-run-from-a-portal.md)

`tools/sweep/sweeps.test.ts`:

- `stateOf` reads a result's first words, and takes what only a test or a headless browser saw as not run.
- Each of the five shapes is read, and a result written into each lands in its place: a row's cell (with a pipe in the note escaped), a section's line, a section with none yet, a bulleted result over two lines, a check's `Pass/fail` line, the end of a prose document. Windows line endings are kept; a row with more cells than its header is refused.
- Over the real documents: all 286 read with at least one ticket, no ticket name twice within a document, every row as wide as its header, and `agile/testing/sweeps.md` as it would be written now.

## Results — 2026-10-19

- `bunx vitest run tools`: 1,343 passed, with the record test.
- The server, run for a minute on port 5299 and driven with `curl`: the page and `/api/sweeps` (286 sweeps) answered; a file was kept under `evidence/BUG-095/SWEEP-BUG095-02/` and linked from the result written into its row; `/evidence/..%2F..%2F..%2Fpackage.json` was 404 and a result for `../README.md` was refused. The test result and file were removed afterwards.
- The page itself was not looked at in a browser: the author is checking it.
- One document was malformed and fixed: `TASK-044-sweep.md` SWEEP-002 had six cells under a five-column header, which put its priority in the result column.
