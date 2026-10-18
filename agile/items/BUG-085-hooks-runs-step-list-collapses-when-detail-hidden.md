<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-085 — Hooks Runs step list collapses when the detail panel is hidden

**Status:** Fixed — on `fix/ui-bugs-from-screenshots`; unmerged.
**Screens:** 1X (Hooks · runs)
**Raised by:** the author, 2026-10-18, on 3.1.0: the step list squashed to ~15–20 px, text wrapping one character per line.

## What happens

On the Hooks Runs view, when the right-hand detail panel is hidden, the left step list collapses to a sliver (~15–20 px) and its labels wrap character-by-character, making the view unusable.

## Cause

`HookRuns.svelte` sizes `.left` with `width: var(--detail-w); flex: none`. `--detail-w` is published as `0` when the detail panel is hidden (`panels.svelte.ts`), so the step list inherits that zero width. The step list is not the detail panel and should not share its width variable.

## Acceptance criteria

- Hiding the detail panel does not collapse the Hooks Runs step list; it keeps a readable width (at least its minimum).

## Fix

- `src/lib/hooks/HookRuns.svelte`: give `.left` its own width (`var(--detail-w)` replaced with a fixed minimum-backed width) and add `min-width: 200px`.

## Not done

- The step list is not made independently resizable in this fix; it keeps a fixed readable width.