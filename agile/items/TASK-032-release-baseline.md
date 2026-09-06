<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-032 — Restore the release baseline and record the reliability plan

**Status:** Open
**Branch:** `task/TASK-032-release-baseline`
**Screens:** —.

## Problem

The published 0.5.0 release was not carried back into dev. The author approved
execution of the farm reliability plan after the project sweep reproduced
cancellation, verification-output, and automatic-merge defects.

## Scope and acceptance criteria

- Carry the published main history into dev through this work branch and a PR.
- Preserve the author's existing work and the separate amendments update.
- Record the approved sequence, validation, and release conditions in the plan.
- Version consistency and working-record tests pass; no application behavior changes.

## Non-scope

The implementation fixes each receive their own item and branch. Publication
waits for all release conditions, including platform and manual sweep evidence.
