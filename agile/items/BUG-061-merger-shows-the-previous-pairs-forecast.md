<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-061 — Merger shows the previous pair's forecast under a new branch name

**Status:** Backlog — reported 2026-10-08, not started.
**Screens:** 1S.
**Raised by:** the author, 2026-10-08, while recording the demo videos with the 1.3.2 release build.

## What happens

Seen with A = `main` and B = `feature/new-heading-color` showing *1 conflict in 1 file*:

1. Choose B = `agent/dark-mode`.
2. The B card's title changes to `agent/dark-mode`.
3. The result card, the strategies and *What changes* still show the old pair: "main gets 1 commit from feature/new-heading-color", *1 conflict in 1 file*, *Resolve 1 conflict*, and the B card lists `57477be Blue headings…`.

Nothing on the screen says the result is out of date. With BUG-060 in play, it stayed like that. A *Resolve* or *Merge now* clicked then acts on a plan for a different branch than the one named.

## Cause

`load()` in `src/lib/merger/store.svelte.ts` keeps `forecast` while the next one is fetched (`loading = true`, `forecast` untouched). `MergerPlan.svelte` renders the plan whenever `forecast` is set, and shows the loader only when it is `null`. So during a reload the old pair's plan is drawn under the new pair's names.

## Acceptance criteria

- When A, B or where it lands changes the pair, the plan for the old pair is not shown as the plan for the new one. Either clear it, or keep it visibly marked as out of date and disable *Resolve* and *Merge now*, until the new forecast lands.
- Changing only the strategy, which re-derives from the same forecast (`plan.ts`), does not flash a loader.

## For the agent who picks this up

**Who:** unassigned. Any agent working on this repository can start cold.

- **Where:** `src/lib/merger/store.svelte.ts` (`load`, `pickA`, `pickB`) and `src/lib/merger/MergerPlan.svelte` (the `{#if}` chain around `<Loader label="Working out the merge…" />`). Checking that `forecast.a.name`/`forecast.b.name` match `a`/`b` before drawing is one cheap guard.
- **Independent of BUG-059 and BUG-060:** a slow forecast on any platform shows the stale plan for as long as it takes.
- **Test:** a component test that seeds a forecast, changes B while the next forecast is pending, and expects no *Resolve* button for the old pair.
- **Branch:** `bugfix/BUG-061-merger-shows-the-previous-pairs-forecast`, with plan, testing documents and a changelog entry when the work starts.
