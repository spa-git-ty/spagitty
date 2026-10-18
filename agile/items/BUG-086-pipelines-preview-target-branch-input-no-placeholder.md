<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-086 — Pipelines Preview target branch input has no placeholder

**Status:** Fixed — on `fix/ui-bugs-from-screenshots`; unmerged.
**Screens:** 1X (Pipelines · preview)
**Raised by:** the author, 2026-10-18, on 3.1.0: the input between "to" and "with" is blank with no cue.

## What happens

In the simulation bar ("If [push] to [ ] with [working copy]"), the branch/tag input has no placeholder, label text, or default value, so it is unclear what belongs there.

## Cause

`PipelinesScreen.svelte` line 37 renders `<input aria-label="Simulation branch or tag" class="ref" … />` with no `placeholder` attribute.

## Acceptance criteria

- The input shows placeholder text such as "branch or tag" when empty.

## Fix

- `src/lib/pipelines/PipelinesScreen.svelte`: add `placeholder="branch or tag"` to the simulation branch input.

## Not done

- No change to validation or default value.