<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-088 — CI problem file badge wraps awkwardly on the hyphen

**Status:** Fixed — on `fix/ui-bugs-from-screenshots`; unmerged. Carried into the rebuilt screen of [BUG-090](BUG-090-pipelines-screen-does-not-match-its-artboards.md).
**Screens:** 1Y (Pipelines · preview)
**Raised by:** the author, 2026-10-18, on 3.1.0: ".gitlab-" / "ci.yml:1" split across two lines.

## What happens

The badge showing `problem.file:problem.line` (e.g. `.gitlab-ci.yml:1`) breaks across two lines on the hyphen.

## Cause

`PipelinesScreen.svelte` line 44 renders the badge in a flex `.problem` container with no `white-space: nowrap`, so the hyphen is a break opportunity.

## Acceptance criteria

- The file:line badge stays on one line.

## Fix

- `src/lib/pipelines/PipelinesScreen.svelte`: add `white-space: nowrap` to the file:line span (or its container).

## Not done

- No change to other wrapping in the problems row.
