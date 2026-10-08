<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-060 — Merger sits on "Working out the merge…" when it is first opened

**Status:** Backlog — reported 2026-10-08, not started.
**Screens:** 1S.
**Raised by:** Claude (Claude Code), 2026-10-08, while recording the demo videos on Windows 11 with the 1.3.0 and 1.3.2 release builds.

## What happens

- The first time Merger is opened after launch, and often after choosing a different branch B, the result card shows *Working out the merge…* and never resolves. Measured: still spinning after 20 s.
- Calling the backend directly for the same pair answers at once: `merger_forecast` took 442 ms (main ↔ feature/new-heading-color), 654 ms, 1.5 s and 3 s for others.
- Leaving and coming back (Graph, then Merger) usually resolves it in about 110 ms, because the second time nothing new is written.

## Cause (likely, not yet proven)

The forecast is answered, then thrown away.

1. The Merger page re-primes on every change of `repo.info` and `repo.token` (`src/routes/merge/+page.svelte`, the `$effect`).
2. `prime()` ends in `load()`, which bumps `seq`.
3. An answer for an older `seq` is dropped (`store.svelte.ts`, `if (mine !== seq) return`).
4. On Windows, BUG-059 makes the backend emit a worktree change every ~300 ms, and each one runs `repo.refresh()`, which replaces `repo.info`.

A forecast that takes longer than the gap between two refreshes is discarded every time, so it never lands. A forecast that writes merge objects into `.git` also feeds the loop. The second visit is fast enough (objects already exist) to win the race, which matches what was seen. With `repo-changed` events muted in the page, every visit resolved.

## Acceptance criteria

- Opening Merger, and choosing any branch B, shows the forecast within the backend's own time, every time, on Windows and elsewhere.
- A refresh that does not move either branch's tip does not restart a forecast that is in flight.

## For the agent who picks this up

**Who:** unassigned. Any agent working on this repository can start cold.

- **Do BUG-059 first** and re-test: if the loop is gone, this may be gone with it.
- **Still worth hardening:** `prime()` could skip `load()` when the path, `a`, `b` and both tips are unchanged, or re-prime on `repo.token` only. The `$effect` reads `repo.info` (a new object on every refresh), which is broader than the comment ("whenever the refs move") says it means.
- **Where:** `src/lib/merger/store.svelte.ts` (`prime`, `load`, `seq`), `src/routes/merge/+page.svelte`.
- **Test:** a store test where a second `prime()` with identical inputs arrives while the first forecast is pending, and the first answer still lands.
- **Branch:** `bugfix/BUG-060-merger-forecast-lost-on-first-open`, with plan, testing documents and a changelog entry when the work starts.
