<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-102 — Resolve every conflict, every way

**Status:** Open — built and merged into `main`; the manual sweep is not yet run.
**Branch:** `feature/FEAT-102-resolve-every-conflict-every-way`
**Screens:** 1S, 1D, chrome (the rail's dot).
**Raised by:** the author, 2026-10-06, as slice 3 of `design_handoff_merger/`.
The same day the author answered the handoff's open questions: Merger owns the
merges it starts, Conflicts stays for what git stopped on by itself, and **the
two share one three-column resolver**; B keeps its amber, and code coloured
like a side is softened on that side's rows.

## Problem

Conflicts offered a whole side, one marker region by side, or the whole file
by hand, and the result did not say where any of its lines came from. Merger
(FEAT-100, FEAT-101) could show conflicts but not resolve them.

## Change

- **One resolver**, `src/lib/resolver/`: the conflicted files with a dot per
  conflict (hollow red, green once resolved), the files that merge on their
  own, and the legend; the chosen file's conflicts as cards; the pill.
- **Three columns per conflict**, A | Result | B whatever the direction, the
  headers saying which side lands and which comes in. Each card has its
  number, place and status chip, one sentence on why it conflicts, each side's
  own lines by its own numbers with the context dimmed, and — in Merger — the
  commit on each side that made the change, found by `blame` over the range
  since the split.
- **Every way out**: Take A, Take B, Both A first, Both B first, Pick lines (a
  checkbox on every line; A's ticked lines, then B's), Edit by hand (the result
  as a text box, prefilled with the result so far or both sides), Reset; and
  per file, All from A and All from B. A file that conflicts as a whole —
  deleted on one side, binary — is one choice, A or B.
- **Where each line came from**: every result line has its A, B or ✎ badge and
  side marker; lines that will not land fade to 40 %; an unresolved region is a
  dashed red *Choose what lands here* box with its line counts.
- **Base** shows the merge base's lines above each conflict, from the diff3
  markers the dry run writes, including *nothing: both sides added lines*.
- **Line numbers follow the choices**, so later conflicts renumber as earlier
  ones change length.
- **Syntax colours** from the shared highlighter in every column and the Base
  strip; the hand-edit box stays plain. On A's rows function names (A's blue)
  and on B's rows numbers and attributes (B's amber) step towards the ink, so
  they differ in lightness from the tint behind them. In this codebase's
  token map (FEAT-094) those are the tokens that share a side's hue; keywords
  are the purple lane and do not clash with either.
- **Merger**: Resolve N conflicts opens resolving at the first conflict, or at
  the file clicked in What changes. The header says what merges into what and
  how far along; Abort returns to the plan, drops the choices and writes
  nothing (asking first once something was chosen); Complete merge, live once
  everything is resolved, opens the commit dialog with every conflict and what
  was chosen. The choices are kept in application data per repository, both
  branches and their merge base (`merger_state`), and applied again only to a
  region whose sides are unchanged. The rail draws a red dot on Merger while a
  merge started there has unresolved conflicts.
- **Conflicts** is rebuilt on the same resolver. A is ours, B theirs. Nothing
  is written until *Mark resolved*, which writes what was chosen and stages it
  in one call (`conflicts::settle`); a file whose markers are already gone
  from disk can be marked resolved as it is. Continue and Abort are unchanged.
  The old pager, resolve bar and panes are gone.

## Non-scope

A rebase that stops (FEAT-103): for Rebase, then fast-forward, Resolve stays
disabled here. Conflicts has no Base strip where git wrote its markers without
the base (`merge.conflictStyle=merge`, git's default); it says so.

## Acceptance criteria

- Every conflict can be resolved as A, B, A then B, B then A, a line-by-line
  pick, or free text, and every result line says where it came from.
- Abort at any point leaves the repository as it was: resolving reads a dry
  run and writes nothing (`detail::tests::reading_the_conflicts_writes_nothing`).
- Both themes and keyboard-only use work: every choice is a button, every pick
  a checkbox, the edit a text box.
