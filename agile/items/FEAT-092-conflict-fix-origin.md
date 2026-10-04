<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-092 — Conflict fixes, told apart

**Status:** Open — built on `feature/FEAT-092-conflict-fix-origin`, not yet merged.
**Branch:** `feature/FEAT-092-conflict-fix-origin`
**Screens:** 1R.
**Raised by:** the author, 2026-10-04, as slice 4 of the Review handoff
(`design_handoff_review/README.md`, "Author vs conflict fix").

## Problem

A pull request that merged its target and resolved a conflict carries code
written in that merge. In a diff it looks like the author's own change, and it
is reviewed as one — when it is the place a bad resolution quietly drops what
one side did.

## Change

- **Each merge in the pull request is re-done by git** —
  `git show --remerge-diff`, git 2.36 or later — which shows exactly what its
  resolution wrote and what each side had where they conflicted. Lines a merge
  changed away from any conflict count too. The newest twenty merges are
  looked at; the room says when there were more.
- **The lines are followed to the head**, so commits after the merge move them
  rather than lose them, and lines later rewritten stop counting as the
  merge's.
- **A changed part holding them is a conflict-fix card**: framed in sky
  (`--lane-5`), headed *Conflict fix, not in the author's own commits* and
  *Made in merge `<short>` (`<subject>`)*. The exact lines it wrote carry a sky
  marker. *main's side* and *branch's side* — named after the target — open
  what each side had there under the header.
- **The files list** marks each file holding one *conflict fix*, filters by
  *All*, *Author* (the files with the author's own changes) and *Conflict
  fixes*, and keeps a legend of the two colours. When git cannot do it, the
  legend says conflict fixes could not be looked for.
- **The record keeps which files and merges**, so the inbox's card and preview
  say *conflict fixes* before the pull request is opened again.

## Dependencies

None added. Needs git 2.36 or later on `PATH` for this one feature; without
it, the room works and says it could not look.

## Non-scope

- Saying in words what each side brought: the sides are shown, not summarised.
- The host's patch, when the head cannot be fetched: there are no merges to
  re-do.

## Acceptance criteria

- In a pull request that merged its target and resolved a conflict, the
  resolution is framed as a conflict fix and never shown as the author's own
  work.
- Each side of the conflict can be seen from its card.
- A file whose only changes came from a resolution is not listed under Author.
