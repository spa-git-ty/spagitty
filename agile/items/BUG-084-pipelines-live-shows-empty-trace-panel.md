<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-084 — Pipelines Live still draws the trace panel when a branch has no runs

**Status:** Fixed — on `fix/ui-bugs-from-screenshots`; unmerged.
**Screens:** 1X (Pipelines · live)
**Raised by:** the author, 2026-10-18, on 3.1.0: "No runs on this branch." with an empty log box and polling timer still shown.

## What happens

On the Live view, a branch with zero runs shows "No runs on this branch." at the top, but the lower half still renders the job inspector ("Select a job"), the "partial trace · polled" label, an empty black terminal rectangle, and "Updated 0 s ago". The empty log box and polling timer are misleading for a branch with no jobs.

## Cause

In `PipelinesScreen.svelte`, the empty-run message is a single `<p>` (line 54) while `PipelineGraph`, `.log-head`, `.logs` and `.updated` (lines 56–62) render unconditionally, so the inspector and poll timer appear even when there is nothing to inspect.

## Acceptance criteria

- When a branch has no runs, the graph, job inspector, log box and "Updated …" timer are not rendered; only the empty state is shown.

## Fix

- `src/lib/pipelines/PipelinesScreen.svelte`: gate the graph/inspector/log block behind `{#if run}` so it only renders when a run exists.

## Not done

- No change to polling itself; `schedule()` still governs polling for other branches.