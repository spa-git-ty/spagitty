<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-033 — Automated checks

Baseline including routes: 58.28% branches (test scaffolding still counted in this
measurement). Product-only configuration excludes src/testing, not maintained
screens.

## What was written

Mounted headless route tests, each driving the real page component and its stores
against a mocked backend API rather than a stand-in implementation:

- `src/routes/farm/page.test.ts`, `rebase/page.test.ts`, `branches/page.test.ts`
  and `references.test.ts` — landed with the first commit of this item.
- `src/routes/badges/page.test.ts` — empty-repository versus empty-record states,
  equipping a title, clipboard success and refusal, confirmation before forgetting.
- `src/routes/changes/page.test.ts` — clean tree, subject required before commit,
  commit failure surfaced, conflicts blocking a commit, read error then recovery.
- `src/routes/conflicts/page.test.ts` — continuation blocked until every file is
  resolved, abort and continue dispatch, read failure then recovery.
- `src/routes/diff/page.test.ts` — deep-linked commit read once, file navigation
  and view switching, keyboard handling inside inputs, missing commit, no repository.
- `src/routes/requests/page.test.ts` — review queue split, host error routed to
  Settings, unsupported remote, creation validation and host rejection.
- `src/routes/settings/page.test.ts` — fragment-driven section selection for every
  section, and `hashchange` handling.
- `src/routes/stash/page.test.ts` — stash contents and file diff, typed keys kept
  out of hunk navigation, push arguments, read error then recovery, no repository.

Typed fixtures for these shapes live in `src/testing/git-fixtures.ts`. Assertions
are on rendered text, disabled state, dispatched arguments and store state; no
test executes a page without asserting on its result.

## Recorded results

Frontend, full suite with coverage (`bun run coverage`), 2026-09-05:

- 2,545 tests passed across 121 files.
- Statements 79.60% (11471/14409), branches 70.95% (3979/5608),
  functions 77.86% (3053/3921), lines 81.43% (7853/9643).
- Thresholds in `vite.config.ts` are 70% on all four, over `src/lib/**` and
  `src/routes/**`. The floor was met by adding tests; no threshold was lowered and
  no product file was excluded.

`bun run check`: zero errors, zero warnings.

Rust: unchanged by this item; the workspace suite and its 70% line floor were last
recorded green under BUG-027.

## Platform process gate

Gate 3 gained a `farm processes` matrix running
`execution::tree::tests` and `verification::command::tests` on ubuntu-latest,
macos-latest and windows-latest. Three Unix shell-specific tests were replaced with
portable fixtures driven by the test binary itself, so the same assertions run on
Windows. Gate 4 depends on both coverage and all three platform jobs.

First run on PR #38 (`33957348890`): macOS passed (1m08s), Windows passed (1m55s,
Job Object containment exercised), Ubuntu failed to compile only because
`libdbus-1-dev` was absent; the job now uses the repository's existing
`linux-deps` action, as gates 2 and 3 already did.

## Not covered here

Manual sweep `COVERAGE-01` is still owed and cannot be satisfied headlessly. No UI
was launched, driven or screenshotted for this item.
