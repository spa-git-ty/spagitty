<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-081 — Automated tests

**Item:** [`agile/items/FEAT-081-the-graph-resizes-like-a-hand-moves.md`](../items/FEAT-081-the-graph-resizes-like-a-hand-moves.md)

## What was tested

| File | What it holds |
| --- | --- |
| `metrics.test.ts` — *narrowing the graph column* | Second revision (fold). Replaces the clip-and-park block. At 185px (span 140) lanes 0–5 stay at `16 + lane × 26` and lanes 6, 7, 11 are all at 156, the boundary. At span 0 every lane at 2–382 lanes, both densities and zoom 1.5 is at lane 0's x, and a 40px column is span 0 at both densities. An undragged column moves no lane by more than rounding at zoom 0.8–2, both densities, 1–382 lanes. No lane moves further than the pointer or backwards, at 2–24 lanes and zoom 1 and 1.5, for every 1px step from 400px to 40px — so no threshold remains. Lane order never inverts. Every full-size node plus halo stays inside its column at every width 40–331px, both densities. Widening restores each lane, and releases lane 3 before lane 4. |
| `metrics.test.ts` — *laneNodeRadius* | Rewritten: the radius takes no lane count. `NODE_R` at 1–100,000 lanes; `COMPACT.node` for compact; both above `MERGE_R`. |
| `metrics.test.ts` — *the author mark beside a commit* | `applyMetrics` publishes `--avatar-d` as the node diameter × zoom. |
| `metrics.test.ts` — *stacks only once the pitch has nowhere left to go* | Lanes past the floor stack on the resting span's edge (302px), not the last whole lane (296px). |
| `lanes.test.ts` — *squeezing the column keeps the portraits* | Against the painter with a recording context. At 3, 14 and 30 lanes and widths 331–40px the only radii drawn are `NODE_R` and its halo. At 331, 120, 80, 60 and 40px every node's x is also an x a track was stroked at — no node without its lane — and no `rect` or `clip` happens before the first node. At 80px nodes are at 16, 42 and 51 (the boundary) and nothing is stroked at lane 2's resting 68. At 40px every node and every stroke is at 16. A two- or five-lane history keeps the 26px pitch. |
| `density.test.ts` | The compact node is `COMPACT.node` at any depth; both densities keep every node inside the column down to 40px and start lane 0 at the same x. |
| `columns.test.ts`, `resize-hover.test.ts` | The graph column clamps to 40px. |
| `AuthorAvatar.test.ts` (new) | Two mounted marks for one person (Author column's with row initials, detail's with differently spelled address) show the same generated initials, then both switch to the same decoded `data:` picture when it arrives, and the canvas's `avatars.drawable` is that same image. The component's CSS is a fixed `--avatar-d` square with `aspect-ratio: 1` and `flex: none`, and neither `CommitRows.svelte` nor `CommitDetail.svelte` keeps its own `.avatar` rule. |
| `avatars.test.ts` | Existing cases plus: the first commit given for an address is the one sent, once; an address first asked without a commit is asked again once when a commit arrives, and never a third time; a `retry` answer is held for a minute and then asked again; one decoded image is shared across spellings of an address. |
| `avatars.rs` (Rust) | With a fake `Net`: an ordinary address resolves through `GET /repos/…/commits/{id}` to `author.login` and `author.avatar_url` (`&s=96`), never touches Gravatar, and a later commit by the same address sends nothing; a connected token reaches the JSON request only; a commit with `author: null` falls back to Gravatar; a numeric no-reply address fetches the direct image and asks no API; a handle-only no-reply address follows GitHub's 302 to `avatars.githubusercontent.com`; a redirect to a host off the list is not followed and not cached as a miss; a 404 is a miss that is believed without a request, while 429, 503, offline and an HTML 200 all return `retry` and write no miss; a 403 from the forge is `retry`, not a skip to Gravatar; backoff is 5 min, doubling, capped at 6 h, and nothing is sent inside it; a legacy `.miss` in the old root does not suppress the `v2` resolver and `retire_legacy` removes it; `cached` reads only a picture; the image host list rejects http, userinfo, ports, look-alike hosts and an Enterprise host that is not the repository's; a malformed commit id never reaches a URL. |
| `resize-drag.test.ts` | Press moves nothing; many moves → one apply per frame with the latest x; no save mid-drag; release flushes, cancels the pending frame and saves once; a cancelled frame never lands later; fast reversals keep the final x; `pointerup` + `lostpointercapture` is idempotent; cancellation keeps the last seen x; a press that never moved neither applies nor saves; a second start finishes the first. |
| `columns.test.ts` | `drag` moves the width without writing storage and `settle` writes it once; `drag` clamps and rounds like `resize`; `resize`, `unsize` and `toggle` still save at once. *Never inverts the drag* now reads the arithmetic from `resize-drag.ts`. |
| `peek.test.ts` | Waits for a rest; crossing twenty rows asks nothing; a stale answer after moving to another row is dropped; an answer after leaving is dropped; one lookup per commit; the selected commit's loaded detail is used without a lookup; the cache is bounded; a failed lookup shows nothing; paragraphs and trailers are preserved. |
| `highlight.test.ts` — `branchOf` | Rows with their own branch get nothing; first-parent descent; a topic commit is named for its topic, not the branch that merged it; the same-lane child wins at a fork; another lane's child is the fallback; a merge child is the last resort; tags are ignored and local beats remote; no branch in range → null; the child window bounds the search. |
| `resize-hover.test.ts` | The real header and rows mounted together: moves apply per frame and save once on release; header cell and lane cell agree on the width; cancellation and lost capture leave nothing pending; a press without movement does not size; unmounting mid-drag flushes and saves; the compact header shows the icon with its name and divider; hovering a bare row captions its branch on that row only with nothing dimmed; resting on a subject shows the full message and selects nothing. |

## Rendered in Chrome (second revision)

The real `drawLanes` was bundled and painted in headless Chrome at 331
(resting), 260, 200, 150, 100, 60 and 40px, with a twelve-lane staircase and a
merge line from lane 11 to lane 1.

- 331px is the design layout. At 260 to 100px the lanes that fit are exactly
  where they were at 331; the lanes the boundary has reached are drawn *on* the
  boundary, track and node together, with the merge line ending there too.
- No node stands anywhere its lane is not drawn. Full-size heads overlap where
  lanes have folded, with the lines running behind them.
- At 40px every track and node is on lane 0.

This is Chrome painting the canvas, not WebKitGTK inside Tauri, and it is
stills, not a drag. The first revision's clip-and-park stills are superseded.

## Runtime measurement (first pass)

The real `CommitRows`, `GraphHeader` and `LaneCanvas` were mounted in Chromium
against the test graph store with 600 synthetic rows across twelve lanes, and
driven by script — Spagitty's own frontend, not the Tauri webview. 288 width
updates from 380px down to 48px, back up, and ten fast 90 ↔ 330px reversals,
each timed from pointer event through Svelte's flush, the canvas redraw and a
forced layout read:

| p50 | p95 | max | over 16.7ms |
| --- | --- | --- | --- |
| 3.0ms | 5.2ms | 8.4ms | 0 |

At every one of the 288 steps the header cell, the row's lane cell and the canvas
had the same width, and the header's message boundary matched the rows'. Paint
is not included; `requestAnimationFrame` did not run in that unfocused window,
so frame intervals could not be sampled. The compact icon appeared below 72px.

## The whole suite

After the second revision: 137 files, 2,910 frontend tests, all passing.
`cargo test --workspace`: all passing (534 in `spagitty-core`), `cargo clippy
--workspace --all-targets -D warnings` clean. `bun run check`: 0 errors, 0
warnings. `bun run build` succeeds.

## What is not covered

Whether it *feels* smooth in the Tauri window on WebKitGTK at 60Hz, and paint
cost there. DOM tests cannot establish that; it is `SWEEP-001` and `SWEEP-002`.

A real GitHub API and image round trip. The Rust tests stand a fake `Net` in
for the network; whether Flea's `GM` commits resolve to `thisisgm` against the
live API is `SWEEP-011`.
