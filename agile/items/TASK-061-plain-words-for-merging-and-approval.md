<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-061 — Plain words for merging and stopping for approval

**Status:** Done — on `task/TASK-061-plain-words-for-merging-and-approval`; unmerged.
**Screens:** all.
**Raised by:** the author, 2026-10-11, on 2.0.1.

## Problem

The interface uses jargon the author finds annoying: *land* for merge (*Land the merge*, *Lands the merge itself*, `LandDialog`, now `MergeDialog`) and *gate* for a point where an agent stops for approval (*waits at a gate*). People say *merge* and *approve*.

The words appear in about 40 files under `src/`. The most are in `src/lib/merger/store.svelte.ts`, `src/lib/merger/resolve.svelte.ts`, `src/lib/merger/MergerPlan.svelte`, `src/lib/merger/LandDialog.svelte`, `src/lib/agents/levels.ts`, `src/lib/agents/AssignPopover.svelte`, and the Farm components (`Building.svelte`, `WrapUp.svelte`, `TaskJourney.svelte`, `describe.ts`).

## Change

- Text the user reads says *merge* where it said *land*, and *approval* or *waits for you* where it said *gate*. Notices, buttons, tooltips, dialog titles, notifications, and the agents' sentences all change.
- `docs/branding.md` uses *lands* in the product notes. It changes in the same change, since approved wording lives there.
- User-facing documents under `docs/` change too.
- Internal identifiers (`Gate`, `lands`, `LastAct.land`) and the CI gate names stay. Renaming them costs more than it gives, and nobody reads them in the app.
- A check fails when *land* or *gate* comes back in user-facing text, in the spirit of the copy checks in TASK-050.

## Acceptance criteria

- No screen, notice or notification uses *land* or *gate* in these senses.
- The check above runs in the ordinary test suite.

## For the agent who picks this up

**Who:** unassigned. Any agent working on this repository can start cold.

- Do this after TASK-060 so the permission sentences are rewritten once.
- `LandDialog.svelte` can be renamed `MergeDialog.svelte` while you are in it.
- **Branch:** `task/TASK-061-plain-words-for-merging-and-approval`, with plan, testing documents and a changelog entry when the work starts.

## What was done

Every string and piece of markup a person reads that said *land* now says *merge* (or *merged into*, *goes to*), in the Merger, the commit dialog, Farm, the agent card and popover, the conflict resolver, notices and the Farm wrap-up; the engine's timeline sentence reads *Merged into main*. *Gate* did not appear in text a person reads in the application (it is `Gate`, `step.gate` and a CSS class), so there was nothing to change there; the check covers both words. `LandDialog.svelte` is `MergeDialog.svelte`. `docs/branding.md` says *merges*. The identifiers (`Gate`, `lands`, `LastAct.land`, `mergerLand`) and the CI gates keep their names. `tools/copy.test.ts` fails when either word returns in a string or markup text under `src/`, in the ordinary test suite.
