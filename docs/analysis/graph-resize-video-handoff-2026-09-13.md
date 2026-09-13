# Graph resizing and hover behavior — implementation brief

## Reference and scope

Reference recording: `/home/maxmya/Videos/screenrecording-2026-09-13_09-41-26.mp4`.

The recording is approximately 19.63 seconds long, at 1572 × 910 pixels and 60 fps. The requested outcome is to reproduce its responsive graph resizing, stable layout, readable nodes, and hover details in Spagitty.

This brief compares inspected recording frames with the current source. Spagitty was not running during the comparison, so current runtime performance was not measured. A 60 fps recording does not establish that the reference application renders every frame without delays. Timings and dimensions below are approximate observations, not measurements of internal animation parameters.

The shared amendments book was unavailable at `/home/maxmya/dev/agents/docs/AMENDMENTS.md`. Its additional requirements remain unverified. Read the canonical book before implementation when it becomes available; `../AMENDMENTS.md` is only a pointer.

## Observed target behavior

### 1. Direct graph resizing

Around 2–9 seconds, the user repeatedly drags the divider between Graph and Commit Message, narrowing it, widening it, and finally leaving it collapsed.

- The divider follows the pointer, and messages immediately gain or lose horizontal space.
- Branch labels stay anchored on the left; the detail panel stays in place.
- Commit rows retain their vertical positions.
- Message text remains on one line and truncates to the available width.
- There is no evident bounce, overshoot, or prolonged settling.

Do not introduce easing into the active drag. The desired smoothness comes from responsive updates and continuous geometry.

### 2. Fold lanes while preserving readable nodes

As width decreases, lanes progressively converge into the available space. At the narrowest position, commit avatars and merge dots form essentially one vertical column. Avatars retain their size and circular shape, and branch colors remain identifiable.

Widening restores the branching geometry. Resizing must preserve commit order, selection, scroll position, and branch identity.

At the recording's resolution, the graph occupies approximately 384 pixels when expanded and 66 pixels when collapsed. These are reference-image measurements, not CSS constants. Account for application zoom and display scaling.

### 3. Align every visual layer

Header boundaries, message cells, canvas nodes, branch-label connectors, backgrounds, and pointer targets must use the same geometry for each displayed frame.

Rounded branch elbows remain clean. Lines terminate behind nodes, and avatars stay sharp during resizing. Do not scale the entire canvas horizontally: that would distort portraits and strokes.

### 4. Compact graph header

In the collapsed state, the GRAPH text becomes a small graph icon. The recording does not demonstrate clicking that icon, so it does not establish that the icon is a collapse toggle.

Retain an accessible header name and a usable resize handle in this state.

### 5. Hover feedback and full commit message

Around 10–16 seconds, hovering highlights a single message row. Some hovered rows also reveal a subdued branch label in the left gutter.

Around 16 seconds, a tooltip shows the full commit subject, body, and trailers with paragraph breaks. Hovering does not replace the selected commit's details on the right.

The appropriate branch association for contextual labels must be verified against repository topology; the recording alone does not establish the lookup algorithm.

The desktop recording menu at the end is outside the requested application behavior.

## Comparison with the current implementation

Paths below are relative to the repository root.

| Area | Current implementation | Implementation advice |
| --- | --- | --- |
| Lane folding | `src/lib/metrics.ts`, function `laneX()`, computes drawable lanes using `floor(span / 14)`. | Remove discontinuities from displayed positions during manual resizing. Preserve logical lane assignments while mapping them continuously into the available width. |
| Resize persistence | `src/lib/ui/columns.svelte.ts` writes to `localStorage` on every `resize()`. | Separate temporary drag updates from persistence. Save the final width when the drag completes. Preserve normal persistence for other column actions. |
| Drag handling | `src/lib/graph/GraphHeader.svelte` already measures the starting width and captures the pointer. | Retain this behavior. Coalesce updates to the latest pointer position per animation frame; flush the final position on release. Clean up on cancellation and lost capture. |
| Canvas | `src/lib/graph/LaneCanvas.svelte` redraws from reactive geometry and resolves theme colors during drawing. | Coordinate canvas and DOM updates. Cache resolved colors until the theme changes; profile before adding further rendering complexity. |
| Node size | `src/lib/graph/lanes.ts` already preserves portrait size during manual narrowing. | Keep this behavior. Avoid stretching or shrinking portraits as a side effect of resizing the graph column. |
| Collapsed header | `src/lib/graph/GraphHeader.svelte` currently displays its text label. | Add the narrow-width icon treatment while retaining an accessible name and resize handle. |
| Hover details | `src/lib/graph/CommitRows.svelte` exposes only `row.summary` through a native title. | Provide the full-message tooltip using cached commit data, without changing selection. Discard stale responses when the pointer moves. |
| Branch hover preview | Comments in `src/lib/graph/CommitRows.svelte` explicitly document removing earlier hover-related branch effects. | Reproduce the reference's subdued contextual label separately from persistent refs. Avoid dimming the whole graph. Verify the appropriate branch association from repository topology. |

### Priority: eliminate discontinuous lane folding

At normal comfortable density with 12 lanes, changing graph width from 185 to 184 pixels can move a high-index node from x=156 to x=142. This is a 14-pixel jump caused by a one-pixel resize.

The current calculation explains it:

- The usable span is width minus the first-lane offset, node radius, and trailing space: `width - 16 - 11 - 18` at zoom 1.
- At width 185, the span is 140; the drawable lane count at the minimum pitch is `floor(140 / 14) + 1 = 11`.
- At width 184, the span is 139; that count becomes `floor(139 / 14) + 1 = 10`.
- A node assigned to lane 10 or higher therefore changes its displayed lane index from 10 to 9 in one step.

A continuous clamp against the available span is one candidate solution. Validate it against intermediate recording frames before adopting it. The reference does not reveal its internal formula. Preserve the existing logical graph topology and use the same displayed coordinates for edges, nodes, connectors, and hit targets.

### Resize lifecycle

1. On pointer down, measure the actual rendered width and capture the pointer without moving the boundary.
2. During movement, retain the latest pointer coordinate and apply at most one coordinated visual update per animation frame.
3. Render the header, rows, connectors, and canvas from a shared width and geometry snapshot. Avoid a second independently delayed update for the canvas.
4. On release, apply the final pointer position, clear pending work, release capture, and persist the resulting width.
5. Handle pointer cancellation, lost capture, and component teardown so dragging cannot remain active or leave a pending update behind.

Do not put synchronous storage writes or Git requests in the resize loop. Keep existing per-repository width persistence and double-click reset behavior.

## Acceptance criteria

- Drag wide → narrow → wide repeatedly, including fast reversals and release outside the header.
- No discrete lane jumps, blank canvas frames, distorted avatars, or header/body misalignment.
- Row positions, selected commit, detail content, and scroll offsets remain stable.
- Collapsed mode retains readable nodes and exposes the compact header icon.
- Widening restores branching geometry without changing logical lane identity or commit order.
- Hover reveals full commit text without changing selection; stale tooltip responses never appear over a different row.
- Contextual branch labels remain distinct from persistent branch refs and do not dim the whole graph.
- No synchronous storage writes or Git requests occur in the resize loop.
- The final width persists correctly, and cancellation or lost capture never leaves dragging active.

## Verification and handoff

Record and inspect the actual Tauri app at comparable dimensions and zoom, using the same repository history where practical. Compare expanded, intermediate, and collapsed states, as well as rapid reversals.

Target a 16.7 ms frame budget on a 60 Hz display and report measured results. This is an engineering target, not a performance measurement extracted from the recording. Profile the drag to distinguish geometry jumps from slow layout or drawing.

Add focused geometry and pointer-lifecycle regression tests, including widths immediately around folding thresholds. Run the project's required tests and build. DOM-only tests cannot establish visual smoothness; include a runtime recording and visual review in the implementation handoff.

Preserve the existing architecture and shared geometry helpers. No new dependency is justified by this brief alone. If a dependency becomes necessary, explain why in the implementation handoff.

This document is an implementation brief. No application source changes or runtime performance verification were performed as part of the comparison.
