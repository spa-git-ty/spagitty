<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-039 — Automated tests

**Item:** [`agile/items/TASK-039-one-colour-system-in-every-dialog.md`](../items/TASK-039-one-colour-system-in-every-dialog.md)

## What was tested

`src/lib/ui/flat.test.ts`, which is where the defect was already recorded. Two
existing assertions become load-bearing again once the list is empty, and one
new assertion covers the half of the defect the file could not previously see.

| Test | Asserts |
| --- | --- |
| is true of every component but the ones already recorded | With `KNOWN` empty, **no** component reads an undefined token. This assertion existed and was true only because eleven files were excused from it. |
| keeps no stale row in the recorded list | A fixed file must lose its row. Empty, so vacuously true — kept because the mechanism is what stops the list reopening. |
| and the section that was reported is one of the fixed ones | Unchanged: FEAT-068's `ExternalToolsSection`, the first instance found. |
| **names no colour of its own, anywhere** (new) | No component stylesheet contains a hex or `rgb`/`rgba` literal. Comments are stripped first, so the accounts of what was wrong may keep quoting the colours they replaced. |

**What would have to break for these to fail.** A component that reads a token
nothing defines, or one that writes a colour a theme cannot change. That is the
complete list of ways a dialog can end up painted from a different system, and
before this change only the first half was checked.

## Run against the broken state first

`WorktreesModal.svelte` was returned to two of its original lines — the title's
`var(--fg, #eee)` and the main-worktree pill's `rgba(238, 176, 77, 0.15)`:

```
 × is true of every component but the ones already recorded
 × names no colour of its own, anywhere
   Tests  2 failed | 10 passed (12)
```

One failure per defect, and they are independent: the first assertion says
nothing about the amber, and the second says nothing about the undefined token.
That independence is the reason the second assertion had to be added rather than
the first one widened — eleven files passed the first for years while writing
literals.

Restored, the file is green again at 12 passed.

## The whole suite

126 files, 2,648 tests, all passing, and `bun run check` reports 0 errors and 0
warnings across 1,141 files. The baseline before this change was 125 files and
2,611 tests.

## What is not covered, and why

- **Rendered colour.** These tests read source. That a `--surface` card now sits
  a step *above* the ground where the old literal sat below it is a fact about
  the rendered result, and `SWEEP-001` to `SWEEP-004` are where it is looked at.
- **Contrast ratios on the final composited surfaces.** `themes.test.ts` checks
  the palette entries against each other; it cannot check a translucent chip
  over glass over a screen. The sweep carries it.
- **The type growth.** 83 sizes moved, inside dialogs with fixed widths. No test
  here lays out a dialog; `SWEEP-005` and `SWEEP-006` do.

## Coverage

The changed files are stylesheets inside components already in the coverage
scope. No statement, branch or function changed, so the numerator and the
denominator both stand still.
