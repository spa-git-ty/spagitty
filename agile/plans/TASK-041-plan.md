<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-041 — Plan

**Item:** [`agile/items/TASK-041-the-frame-stops-competing-with-the-work.md`](../items/TASK-041-the-frame-stops-competing-with-the-work.md)

## Approach

Every item here is a number that can be measured before and after, so each one
is fixed at the place the number is written down rather than in the component
that suffers from it. That is what makes this task testable at all: nothing
below is a rendering judgement, and a rendering judgement is exactly what this
repository cannot check.

The graph density is the largest piece and it is done last, because it touches
`metrics.ts` — the most carefully argued file in the frontend — and every
existing decision in there has to survive it. The way that is guaranteed is a
default parameter: every geometry function takes a density and defaults to the
one that was already there, so the whole existing suite is the regression test.

## Decisions

- **Forty, not thirty-eight.** The toolbar's controls measure 38. Two pixels of
  air stops the row becoming the tightest thing on screen, and the metric scales
  with zoom so a zoomed window does not get a 40px bar with 24px controls in it.
- **The rail's order does not change.** Grouping is new information; moving rows
  would destroy old information. Rebase into `tools` is the one judgement call:
  it is git work but not *routine* git work.
- **Four groups, not three.** The divider before All repositories was the only
  structure the rail had, and it meant something — those two screens are not
  about the open repository. A heading says what the divider meant, so the
  fourth group is not an addition, it is the existing boundary given a name.
- **`navRows()` in `nav.ts`, not an index comparison in the component.** A
  component that worked out its own boundaries would be doing arithmetic nothing
  can read.
- **A heading is an `h2`.** A screen-reader user navigating by heading gets the
  structure the eye gets.
- **Adaptive widths run before the stored ones, never instead of them.** A
  dragged width is a decision, including after the window moves to a different
  screen. Applying the adaptation afterwards, or on every resize, would take
  that decision back.
- **The adaptive floor is not the panel's `min`.** `min` is where a panel stops
  being useful; opening there leaves nowhere to drag. Two-thirds of the way from
  min to the design value is visibly tighter and still has somewhere to go.
- **A density is one object, not three settings.** The pitch, the node and the
  column floor are consequences of one choice — a lane closer than a node is
  wide draws lines through faces — and exposing them separately would let
  somebody pick a combination the geometry forbids.
- **The preference stores a name, not the numbers.** Storing `{pitch: 16}` would
  freeze one release's idea of compact into every installation that chose it,
  and those numbers are exactly what gets retuned after somebody looks at a
  screenshot.
- **`laneSpanOf(density)`, and the node still sized by depth alone.** FEAT-039
  decided that dragging the column must not change the node. Threading the
  dragged span into `laneNodeRadius` would have reversed that silently, and did,
  until two existing tests failed. The reference span belongs to the density.
- **`COMPACT.pitch` is 16, above `LANE_PITCH_MIN`.** A resting pitch at the
  compression floor would leave a deep history in compact mode with nothing left
  to give.
- **`density.portraits` rather than a radius comparison.** "Is a portrait worth
  drawing" is a question about the preference; a painter comparing a radius
  against a magic number would be answering it by accident.
- **`gentleFly` returns `{ duration: 0 }` rather than declining to transition.**
  Svelte needs a transition object; a zero-duration one is the difference
  between arriving instantly and throwing inside the transition runtime.
- **The duplicate reduced-motion block is deleted rather than reconciled.** Two
  blocks with the same selector and different durations is one rule and a
  question about source order, which was not a thing anybody chose.

## Files

- `src/lib/metrics.ts` — the toolbar height, `Density`, `laneSpanOf`, and a
  density parameter on five geometry functions
- `src/lib/nav.ts`, `src/lib/nav.test.ts`, `src/lib/chrome/NavRail.svelte`
- `src/lib/panels.svelte.ts`, `src/lib/panels.test.ts`
- `src/lib/graph/density.svelte.ts`, `density.test.ts` (both new)
- `src/lib/graph/lanes.ts`, `CommitRows.svelte`, `LaneCanvas.svelte`
- `src/lib/motion.ts` (new), `src/routes/+layout.svelte`, `src/app.css`
- `src/lib/settings/AppearanceSection.svelte`
- `CHANGELOG.md`, `agile/` — this set and the index row

## Steps

1. `gentleFly`, and the duplicate CSS block.
2. The toolbar height.
3. The nav groups: the model, the rows helper, the rail, the assertions.
4. Adaptive widths, with the existing suite pinned to a wide window so its
   assertions keep meaning what they meant.
5. The density: the model in `metrics.ts` with defaults, then the store, then
   the three components, then the Appearance control.
6. Changelog, this set and the index row.

## Risks and rollback

- **`metrics.ts` is load-bearing and heavily argued.** The mitigation is the
  default parameter: 349 existing geometry assertions pass unchanged, and they
  are the proof that comfortable is untouched. Two of them caught a real
  regression during this work — the node briefly began following the dragged
  column again, which FEAT-039 had deliberately stopped.
- **Compact has not been looked at.** Every number in it is reasoned and
  asserted; none of it has been seen on a screen. `SWEEP-006` to `SWEEP-009`.
- **The adaptive widths change what a first run looks like** on every window
  size. They cannot change what an existing installation looks like, which is
  the half that matters for not surprising anybody.
- Rollback is per file. The density defaults to comfortable, so reverting the
  store alone restores the previous geometry exactly.
