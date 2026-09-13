<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-081 — Plan

**Item:** [`agile/items/FEAT-081-the-graph-resizes-like-a-hand-moves.md`](../items/FEAT-081-the-graph-resizes-like-a-hand-moves.md)

## Approach

Fix the geometry first, because no amount of frame scheduling hides a node that
jumps fourteen pixels. Then take the work out of the pointer loop, then add the
two hover answers, each as a pure module with the component only wiring it.

## Decisions

- ~~**Clamp the offset to the span.**~~ Superseded after review. It was
  continuous, but the pitch it clamped was still derived from the dragged span,
  so every track moved towards lane 0 as the column narrowed — the opposite of
  the reference.
- **One folded x for every object on a lane (second revision).** From
  `docs/analysis/graph-fold-avatar-claude-handoff-2026-09-13.md` and the Tauri
  recording it cites: `laneX(lane, columns, zoom, span, density) = X0 +
  min(index × restingPitch, restingSpan, draggedSpan)`. Continuous, fitting lanes
  still, reached lanes carried onto the boundary whole, lane 0 at span 0. Chosen
  over the clip-and-park model below because that model put nodes where their
  lanes were not drawn. The Graph column minimum drops to 40px so that span 0 is
  reachable.
- **The node radius is the density's alone.** Depth in view changed it during
  scroll. One `AuthorAvatar` component at the node's diameter for the DOM marks.
- **Avatars through the forge, by commit.** GitHub's commit endpoint names the
  account; cached per host and address. Through the existing `forge::http` and
  `forge::identify_repo`, a `Net` trait for tests, no new dependency. Redirects
  are followed by the avatar module, to a fixed image-host list, on the
  credential-free request only. Negative cache only on 404/410; everything else
  a bounded on-disk backoff. Cache format versioned as `avatars/v2`.
- ~~**Tracks keep their resting x; nodes ride the edge (revision).**~~ Superseded
  by the fold above. Taken from
  close frames of the reference at 0:00, 0:01–0:03.5 and 0:04.5: a track that no
  longer fits is cut off a node's width short of the boundary, and its node stays
  where it was, dimmed, until the boundary pushes it — so nothing snaps into
  place. Written as three functions rather than one shared formula, because
  tracks and nodes are treated differently: `laneX` (track, no width argument),
  `laneClipX` (where tracks stop), `laneNodeX` (`min` of the track's x and the
  parking spot, so still continuous) and `laneOverflow` (the dimming ramp).
  Proportional scaling stays rejected, for the floor and for the picture.
- **Once per frame, latest position, no easing.** A queue of positions would
  replay stale ones; easing would make the boundary lag the hand.
- **A press that never moves writes nothing.** Otherwise pressing — or
  double-clicking to reset — the message column's divider would turn a filling
  column into a sized one.
- **`drag` + `settle` beside `resize`, not a flag on it.** Every other caller of
  `resize` keeps saving at once without being touched.
- **Colours cached by `theme.revision`.** The revision is already the "could the
  palette have changed" signal FEAT-080 introduced.
- **Icon threshold 72px × zoom.** "Graph" plus cell padding is just over 50px; an
  ellipsised name is worse than the icon.
- **Tooltip via `api.commitDetail`, custom element rather than `title`.** A
  native title cannot be delayed, positioned or cancelled, and would stack on
  the existing subject title — which was removed.
- **Branch by first-parent descent, lane-preferred.** The recording does not
  show the rule; `git log --first-parent` and `git name-rev` do. Bounded to a
  600-row child window and a 6,000-row budget because it runs on hover.

## Files

- `src/lib/metrics.ts`, `metrics.test.ts`
- `src/lib/ui/resize-drag.ts`, `resize-drag.test.ts` (new)
- `src/lib/ui/columns.svelte.ts`, `src/lib/graph/columns.svelte.ts`, `columns.test.ts`
- `src/lib/graph/GraphHeader.svelte`, `LaneCanvas.svelte`, `CommitRows.svelte`
- `src/lib/graph/peek.svelte.ts`, `peek.test.ts` (new)
- `src/lib/graph/highlight.ts`, `highlight.test.ts`
- `src/lib/graph/resize-hover.test.ts` (new)
- `src/lib/graph/AuthorAvatar.svelte`, `AuthorAvatar.test.ts` (new), `CommitDetail.svelte`, `avatars.svelte.ts`, `avatars.test.ts`, `src/lib/api.ts`, `src/lib/types.ts`
- `crates/spagitty-core/src/avatars.rs`, `forge/http.rs`, `src-tauri/src/commands.rs`
- `CHANGELOG.md`, `agile/` — this set and the index row

## Steps

1. `laneX` and threshold tests. Revised twice: now one folding `laneX` for edges, nodes, ghost path and hover target; clip, parking and fade removed; fixed node radius; 40px minimum.
2. `drag`/`settle`, `resize-drag.ts`, header wiring.
3. Colour cache.
4. Compact header.
5. `peek.svelte.ts`, `branchOf`, row wiring.
6. Component tests, full suite, build, a driven run of the real components.
7. Forge-aware avatar resolution, redirect-safe image fetch, retryable failures, cache version; shared author mark.

## Risks and rollback

- **Deep lanes now stack on the resting span's edge** rather than on the last
  whole lane, six pixels further right. Visible only past 21 lanes.
- **Folded lanes draw over each other.** Several tracks share the boundary's x
  and a crossing to a folded lane shortens to the boundary. That is the merge
  the user asked for; colours stay per lane.
- **Deep histories keep full-size portraits over a 14px pitch.** Heads overlap
  their neighbours past 12 lanes; the lines run behind them (FEAT-046).
- **The forge learns which commits were looked at.** One commit id per author,
  to the host the repository is already on. Turning real pictures off stops it.
- **Unauthenticated GitHub API is 60 requests an hour.** A repository with more
  new authors than that gets `retry` for the rest and fills in on backoff; a
  connected account lifts the limit.
- **A private repository with no connected account** 404s the commit lookup and
  falls back to Gravatar, whose miss is then remembered for 90 days.
- **Rollback:** the geometry, the avatar component and the resolver revert
  independently. Reverting the resolver leaves an `avatars/v2` directory the old
  code ignores.
- **A hover lookup is a backend call.** Delayed, cached and cancelled; a failure
  shows no tooltip.
- Each step reverts on its own; nothing here changes stored layout format.
