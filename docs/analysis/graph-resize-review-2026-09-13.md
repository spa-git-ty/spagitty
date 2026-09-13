# Review of FEAT-081 against the comparison recording

Reference: `/home/maxmya/Videos/screenrecording-2026-09-13_10-20-03.mp4`
(32.57 seconds, 1910 × 1036, recorded at 60 fps). GitKraken appears first;
Spagitty appears at approximately 15 seconds. Reviewed the current working-tree
changes and extracted full-resolution frames and half-second sequences.

**Result: the requested visual behavior is not matched. FEAT-081 should not be
accepted as complete.** The changes improve resize scheduling and remove a
geometry discontinuity, but retain a different model of narrowing the graph.

## 1. High: narrowing changes lane spacing instead of preserving the visible tracks

In GitKraken, compare 0:00 with 0:04.5: the leading tracks stay near screen x
498, 524, 550, 576 and 602 while the message boundary moves left. Overflow
portraits collect immediately beside the moving boundary. The visible tracks
keep their approximately 26-pixel spacing.

In Spagitty, compare 0:17 with 0:21: the leading tracks change from approximately
x 503, 534, 565, 595 to 503, 519, 536, 552. Narrowing compresses the interior
of the graph before the remaining lanes stack. These are approximate recording
pixel measurements, not CSS values; the useful comparison is each app against
itself during its drag.

The source explains this directly. `laneX()` in `src/lib/metrics.ts` still calls
`lanePitch(columns, span, density)`, whose pitch depends on the current dragged
span. FEAT-081 changed the final clamp to `min(index * pitch, span)` but did not
remove the span-dependent pitch. The plan's statement that a lane which fits
stays where it was does not hold while that pitch is changing.

Correction: preserve the resting track positions during manual resizing and
define overflow-node placement at the moving boundary. Verify edge clipping,
node placement, connectors and pointer targets together. The recording suggests
different treatment for tracks and overflow nodes; establish this from close
frames before choosing a shared coordinate formula. Do not replace the current
formula with uniform proportional scaling: that would retain the observed
mismatch. Keep automatic layout for deep histories a separate consideration.

## 2. Medium: the graph's visual treatment is still different

GitKraken has thin, saturated tracks, colored ref chips and subdued horizontal
color bands behind graph rows. Spagitty has heavier, muted tracks and a mostly
uniform graph background. At the intermediate width, its overflow nodes also
sit further back from the message boundary.

The FEAT-081 canvas change caches colors; it does not implement those visual
treatments. Passing resize tests therefore cannot establish appearance parity.
Theme and zoom should be aligned for the next comparison. The two halves also
show different repositories (`spagitty` and `core-banking-services`), so branch
counts, ref counts, commit identities and topology are not comparable evidence
of a rendering defect.

## 3. High: completion and runtime verification are not supported

`agile/items/FEAT-081-the-graph-resizes-like-a-hand-moves.md` says **Done** and
claims acceptance is met. Every Pass/Fail entry in
`agile/testing/FEAT-081-sweep.md` is empty.

The automated report explicitly used synthetic rows in Chromium rather than
the Tauri webview. It excludes paint and says `requestAnimationFrame` did not
run, so frame intervals could not be sampled. Its 5.2 ms p95 is not evidence
that the actual application renders smooth 60 fps drags. The report discloses
these limits, but the completion claim fails to honor them.

Correction: reopen acceptance, validate the corrected geometry against the
reference, then record the real Tauri app with slow drags, fast reversals and
collapse/expand. Fill in the manual results with observed evidence. Unit tests
should protect the target behavior, including stable visible-lane spacing,
instead of only proving continuity of the current formula.

## Useful changes to retain

The new drag controller coalesces pointer movement and saves the final width
once; the compact header, tooltip loading and stale-response protection address
parts of the brief. These improvements do not resolve the geometry mismatch.
This comparison recording primarily demonstrates resizing; it does not establish
full-message tooltip or contextual-branch correctness.

## Review limits

The canonical amendments book is absent at
`/home/maxmya/dev/agents/docs/AMENDMENTS.md`; the repository file is only a pointer.
No application source was changed during this review. The recording cannot
establish which exact source revision the running binary contains; the observed
compression is consistent with the current working-tree geometry.

`bun run check` passed with zero errors and warnings. `bun run build` passed.
The initial full test run passed 2,902 tests; five in `tools/dev-styles.test.ts`
were blocked with `spawnSync ... node EPERM`. An outside-sandbox rerun was
requested to distinguish that execution restriction from an actual test failure.
