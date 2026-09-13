<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-081 — The graph resizes like a hand moves

**Status:** Open. Reopened twice on 2026-09-13. First by
[`docs/analysis/graph-resize-review-2026-09-13.md`](../../docs/analysis/graph-resize-review-2026-09-13.md):
the first pass squeezed the lanes instead of holding them still, and acceptance
was claimed without the runtime sweep. Then by
[`docs/analysis/graph-fold-avatar-claude-handoff-2026-09-13.md`](../../docs/analysis/graph-fold-avatar-claude-handoff-2026-09-13.md),
against a Tauri recording of the second pass: nodes stood beside Commit Message
with their lanes clipped away, author circles changed size between parts of
history, and most authors showed initials where the forge knows their picture.
All three are corrected in code; the sweep in the Tauri window is still owed.
**Screen:** Graph.
**Raised by:** the author, with a reference recording and an implementation
brief (`docs/analysis/graph-resize-video-handoff-2026-09-13.md`): dragging the
graph column should feel continuous, fold the lanes to one column at its
narrowest, and hovering should show a commit's whole message.

## Problem

Five things visible only while the pointer is moving, and then three the
runtime recording of the corrected drag showed.

**Nodes left their lanes.** The second pass kept tracks still and clipped the
ones past the edge, then parked their nodes against the boundary with a second
function (`laneNodeX`) and faded them. Each half moved smoothly; together they
were two coordinate systems. The recording shows a column of faces beside
Commit Message whose coloured paths had been cut away — no longer a graph. The
user's correction, which is the acceptance authority: at the narrowest width
the graph merges into the one visible lane, as the reference and FEAT-046 do.

**Author circles changed size as history scrolled.** The node radius followed
`laneNodeRadius(laneCount)`, and the lane count is measured over the rows on
screen, so the same kind of commit was a full portrait in a shallow stretch
and a small mark in a deep one. The Author column's mark was a separate `2em`
and the commit detail's a separate 20px, against a 22px node.

**Most authors drew initials.** The resolver knew two cases — GitHub no-reply
addresses and Gravatar — and never asked the repository's forge who authored a
commit, so an ordinary address with no Gravatar (most of Flea's history, by an
account GitHub knows) was a cached miss. Worse, it cached *every* non-2xx as a
90-day miss: GitHub's redirect for an older no-reply address, which the
transport declines to follow, a rate limit, a server error.

**Narrowing squeezed the graph.** Found by the review, against the reference
recording, after the first pass. The reference holds its leading tracks at the
same screen x while the message boundary moves left (26px apart throughout),
cuts off the tracks that no longer fit, and parks their nodes, dimmed, right
beside the boundary. Spagitty's leading tracks went from about 31px apart to
16px over the same drag: `laneX` shared the *dragged* span out between the
lanes, so the pitch itself followed the hand. The first pass fixed only the
step at the end of that formula, and its tests proved the squeeze was
continuous rather than that it should not happen.

**Lanes jumped.** `laneX` folded lanes that no longer fit by counting how many
whole lanes the span held — `floor(span / 14) + 1` — and putting every deeper
lane on the last of them. The count is a step function of the width, so at the
comfortable density with twelve lanes a one-pixel drag from 185px to 184px moved
lane 10 from x=156 to x=142. Every threshold was a fourteen-pixel jump.

**Every pointer move was a write.** The divider called `columns.resize` per
`pointermove`, which re-laid the table and serialised the whole layout into
`localStorage` synchronously — several times per displayed frame on a mouse
that reports faster than the screen repaints.

**The collapsed header said "Gr…".** Nothing replaced the word when the column
was too narrow for it.

**Hovering said nothing new.** The subject's `title` repeated the subject, which
was already on screen, and the body — paragraphs, trailers — was only reachable
by selecting the commit, which replaces the detail panel.

## Change

- **One x for everything on a lane: the fold.** `laneX(lane, columns, zoom,
  span, density)` is `LANE_X0 + min(naturalOffset, draggedSpan)`, where the
  natural offset uses the density's *resting* pitch. Edge ends, nodes, merge
  dots, stash stubs, the ghost path and the DOM hover target all read it. A
  lane that fits does not move; the boundary reaching a lane carries its track
  and its node onto the boundary together, a pixel per pixel; deeper lanes join
  one by one; at a span of 0 every lane is at lane 0. Widening releases them in
  reverse order. Logical lane and colour identity are untouched. No clip, no
  parking, no fade — `laneClipX`, `laneNodeX`, `laneOverflow` and
  `OVERFLOW_GAP` are gone. No `floor(span / pitch)` and no pitch recomputed
  from the drag.
- **The Graph column's minimum is 40px,** the width at which the span is 0 at
  both densities at 100% zoom, so the narrowest column really is one lane. At
  48 the lanes stopped 3px short of merging.
- **A column nobody dragged is unchanged,** at every zoom from 0.8 to 2, both
  densities and every lane count.
- **The node is the density's size, always.** `laneNodeRadius(density)` takes
  no lane count: 11px comfortable, 6px compact, whatever the width, depth or
  scroll. Merges stay `MERGE_R` dots. Where lanes fold, the portraits overlap
  and the lines run behind them.
- **`AuthorAvatar.svelte`** is the one author mark for the Author column and
  the commit detail: `--avatar-d` (the node's 22px, × zoom, from
  `applyMetrics`), square by `inline-size`/`block-size`/`aspect-ratio`, no flex
  shrink, and the same picture-or-initials choice the canvas makes.
- **Avatars are resolved through the repository's forge.** The rows pass one
  authored commit id with the first lookup for an address. When the open
  repository is on GitHub, the backend asks
  `GET /repos/{owner}/{name}/commits/{id}` for `author.login` and
  `author.avatar_url`, remembers that account per host and address for 30
  days, and fetches the picture with no credential. A connected account's token
  is read from the keychain for that API request only, and only when the disk
  has no answer. Numeric no-reply addresses still go straight to the image;
  Gravatar is the fallback. Obviously local addresses still never leave.
  A committer looked up without a commit in a GitHub repository sends nothing
  (a Gravatar miss then would hide the forge's answer), and the store asks
  again, once, when a row that address authored supplies a commit.
- **Only a definitive not-found is a miss.** 404/410 from the image host is
  remembered for 90 days. Offline, timeouts, 429, 403, 5xx, a non-image body and
  a redirect off the image hosts back off from 5 minutes, doubling, to 6 hours,
  and the frontend asks again a minute after a `retry` answer. Picture requests
  follow up to three HTTPS redirects, only to GitHub's and Gravatar's image
  hosts or the repository's own GitHub Enterprise host; the token-bearing JSON
  client still follows none. The cache moved to `avatars/v2`, and the old
  format's records are deleted, so the poisoned misses are not believed.
- **`resize-drag.ts`** owns a divider drag's life: keep the latest pointer x,
  apply it once per animation frame, flush it on release, save once. Release,
  `pointercancel`, `lostpointercapture` and unmount all end the drag the same
  way. The column store gained `drag` (set without saving) and `settle` (save).
- **`LaneCanvas` caches resolved lane colours** by palette revision rather than
  calling `getComputedStyle` on every draw.
- **The Graph header shows the graph icon** below 72px (× zoom), keeping its
  accessible name, a hover title and its resize handle. It is not a toggle.
- **Hovering a subject shows the full message** after a 450ms rest, from a
  per-commit cache or the selected commit's loaded detail, with stale answers
  dropped by token. It never selects.
- **Hovering a bare commit names its branch faintly** in the Branch/Tag gutter,
  on that row only, as muted text rather than a chip. The branch is found by
  topology: first-parent children upward, same lane preferred at a fork, a merge
  child as the last resort (`branchOf` in `highlight.ts`). Nothing dims.

## What was left alone

- **`LaneState` in `graph.rs`.** The detached stack was the frontend's
  projection of valid lane indices, not the walk; the walk already converges,
  releases and reuses lanes.
- **Other forges.** GitLab and Bitbucket repositories resolve as before
  (no-reply form or Gravatar). The commit lookup is written against GitHub's
  response only.
- **`LANE_PITCH_MIN`.** Scaling every lane proportionally into a narrow span was
  the other candidate for continuity and was rejected twice: it takes the pitch
  under the floor that exists to stop the picket fence `git/git` produced, and
  it is the squeeze the reference recording shows is wrong.
- **The graph's look.** The review also notes the reference's thinner, more
  saturated tracks, coloured ref chips and row colour bands. Those are
  appearance, not resize behaviour, and are not part of this item; the review
  itself says the two recordings used different repositories, themes and zoom,
  so the next comparison should align those before judging them.
- **The Branches table's divider**, which still saves per move. It is not the
  column that repaints a canvas, and the brief was about the graph.

## Acceptance

**Not yet met.** The fold, the fixed radius, the shared avatar and the resolver
are protected by unit, component and Rust tests
([`testing/FEAT-081-automated.md`](../testing/FEAT-081-automated.md)). None of
that is the Tauri window, and the canonical amendments book
(`/home/maxmya/dev/agents/docs/AMENDMENTS.md`) was unavailable when this
revision was made, so it was not re-read. The item is `Done` when
[`testing/FEAT-081-sweep.md`](../testing/FEAT-081-sweep.md) has been run in the
application, recorded at 60fps beside the reference, and every High row carries
an observed result.
