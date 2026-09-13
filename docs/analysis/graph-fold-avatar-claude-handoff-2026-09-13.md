# Graph fold and avatar follow-up — Claude handoff

## Decision

FEAT-081 is still open. The latest runtime recording is
`/home/maxmya/Videos/screenrecording-2026-09-13_11-19-06.mp4` (20.77 seconds,
1918 x 1038, 60 fps). It shows that the resize work is close, but two visible
defects remain:

1. Narrowing separates overflow commit nodes from their lanes instead of
   folding the graph into the surviving lane.
2. Author circles change size between parts of history and most visible authors
   still show generated initials instead of their hosting-service pictures.

The user's latest description is the acceptance authority: at the narrowest
width the graph must merge into the main visible lane as in GitKraken, and
ordinary commit avatars must remain full-sized and use real pictures when the
repository's forge can identify the author.

The canonical amendments book is currently unavailable at
`/home/maxmya/dev/agents/docs/AMENDMENTS.md`. `docs/AMENDMENTS.md` is only a
pointer. Read the canonical book if it is restored before implementation and
report this limitation in the implementation handoff if it remains unavailable.

## Defect 1: the painter splits one graph into two coordinate systems

The latest FEAT-081 revision implemented the wrong intermediate model:

- `src/lib/metrics.ts::laneX` keeps every track at its resting x.
- `laneClipX` cuts off tracks before the moving graph boundary.
- `laneNodeX` independently parks nodes against that boundary.
- `laneOverflow` fades those parked nodes.
- `src/lib/graph/lanes.ts` draws edges with `laneX` and nodes with `laneNodeX`.

That is exactly the detached stack in the recording: a column of circles beside
Commit Message whose corresponding colored paths have been clipped away. The
stack moves smoothly, but it no longer reads as a commit graph.

Do not change `crates/spagitty-core/src/graph.rs::LaneState` for this symptom.
Its `step` method already converges duplicate waiting lanes, releases them, and
reuses free slots. This defect is the frontend projection of valid logical lane
indices during manual resizing.

### Required geometry

Use one displayed x function for all graph objects. A suitable model is a
fixed-pitch clamp:

```text
restingPitch = lanePitch(laneCount, laneSpanOf(density), density)
naturalOffset = min(clampedLaneIndex * restingPitch, laneSpanOf(density))
displayedOffset = min(naturalOffset, draggedSpan)
displayedX = (LANE_X0 + displayedOffset) * zoom
```

This has the behavior the recording needs:

- Leading lanes remain at their resting positions while they fit.
- When the boundary reaches a deeper lane, that whole lane folds onto the
  boundary continuously; it is not deleted while its node survives elsewhere.
- More lanes progressively share that x as the graph narrows.
- At `draggedSpan == 0`, every track, edge endpoint, node, ghost path and hover
  target is at lane 0. The graph has merged into one visible lane.
- Widening releases lanes back to their original positions in reverse order.

Feed this same result to edge `from` and `to`, commit nodes, merge dots, stash
stubs, ghost paths, and DOM hover targets. Remove the canvas-wide clipping model
and the track/node coordinate split. Keeping logical lane and color identity is
still necessary; only displayed x folds.

Do not return to the first FEAT-081 formula that recomputed every lane's pitch
from the dragged span. It pulled all visible lanes toward lane 0 at once. The
fixed resting pitch plus direct clamp above keeps fitting lanes still and folds
only the part reached by the boundary. Do not use `floor(span / pitch)` to
choose a displayed lane count; that was the source of the 14-pixel jumps.

The earlier FEAT-046 item is the relevant repository precedent. It explicitly
says that lanes fold behind full-size portraits and nodes end at essentially
one x. The later FEAT-081 review overruled that with clipping and parking; the
new runtime recording and the user's explicit correction overrule that review.

## Defect 2a: regular avatar size changes with visible history depth

Regular node radius is currently calculated as:

```text
laneNodeRadius(laneCount, laneSpanOf(density), density)
```

`laneCount` is derived from the visible range. As scrolling enters a deeper or
shallower part of history, the same kind of ordinary commit can therefore move
between a small mark and a full portrait. Manual drag no longer changes it, but
viewport topology still does. That explains the recording's inconsistent
circles.

Make the radius stable for the selected density:

- Comfortable: every ordinary, non-merge commit uses `NODE_R` (11px at zoom 1,
  22px diameter) regardless of dragged width, visible lane count, or scroll.
- Compact: every ordinary commit uses the compact density's node radius
  consistently. Changing the explicit density preference may change size;
  merely scrolling or resizing may not.
- Merge commits may remain `MERGE_R` plain dots. GitKraken also distinguishes
  merge events from authored commits, so these smaller dots are not avatar-size
  inconsistency.

Use the same radius in canvas paint, hit testing, lane-span reservation and
overflow bounds. If the full-size portraits overlap when several lanes fold to
one x, the portraits win and the lines run behind them, as FEAT-046 requires.

There is also duplicated DOM avatar sizing: `CommitRows.svelte` uses `2em`,
while `CommitDetail.svelte` uses a fixed `20px`, and the graph node is 22px at
zoom 1. Replace the duplicate markup with a small shared author-avatar component
or at least one shared size token. For the normal author mark, use the same
22px design diameter and the same image/generated fallback logic. Keep it
square with fixed inline/block size, `aspect-ratio: 1`, circular clipping and no
flex shrink. A deliberately larger profile treatment would need its own named
variant; no current surface needs one for this fix.

## Defect 2b: the current resolver cannot identify most GitHub authors

This is not primarily a canvas decode failure. The machine's actual avatar
cache proves that fetching and decoding can work:

- Numeric GitHub no-reply identities such as `30683038+thisisgm@...` and
  `62413+cmyk@...` have valid cached 96 x 96 images.
- The visible Flea history is mostly authored as
  `gianmarcomorales@icloud.com`. That address has a cached miss, so the graph
  correctly falls back to `GM` under the current resolver even though GitHub
  associates those commits with the `thisisgm` account.

`crates/spagitty-core/src/avatars.rs::source` only understands two cases:
numeric/handle GitHub no-reply addresses, or Gravatar for every ordinary email.
It never asks the repository's forge who authored a commit. GitKraken can show
the real picture because it uses forge commit metadata, where a GitHub commit
response includes the associated user's login and avatar URL (see GitHub's
official [Get a commit response](https://docs.github.com/en/rest/commits/commits#get-a-commit)).

Make avatar resolution repository and commit aware:

1. Keep the normalized email/name as the in-memory identity key so repeated
   commits by one author still deduplicate.
2. Pass a representative commit id with the first lookup for that identity.
   `GraphRow` already has both the id and author email.
3. If the open repository maps to a connected or public GitHub remote, query
   that repository's commit endpoint and use its associated `author.login` and
   `author.avatar_url`. Cache the resulting identity mapping so this remains one
   lookup per author, not one lookup per row.
4. Keep direct numeric GitHub no-reply resolution as the cheap first path.
5. Use Gravatar as the fallback for repositories or commits with no forge user
   association. Preserve the existing local-address privacy rule and the
   user-facing fetch preference.
6. Fetch the final public image through the backend and continue returning a
   validated `data:` URL. Do not put forge or image-host credentials into an
   `<img>` request.

Use the existing forge identification/account infrastructure rather than adding
a second remote parser. A disconnected public GitHub repository may use an
unauthenticated commit lookup within rate limits; a connected account should use
its existing token only for the GitHub API request. The subsequent avatar-image
request carries no authorization header.

### Redirect and negative-cache correction

Old GitHub no-reply addresses currently resolve to
`https://github.com/<handle>.png?size=96`. GitHub redirects that URL, while the
shared HTTP agent sets `max_redirects(0)`. `avatars::picture` then treats the
redirect as a definitive absence and writes a `.miss` valid for 90 days. The
same code also caches rate limits and other non-2xx transient statuses as
90-day misses.

Fix both behaviors:

- Either resolve handle-only addresses through the GitHub user API or add a
  public-image fetch path that follows a small number of HTTPS redirects only
  to an explicit image-host allowlist. Do not relax redirects on the existing
  token-bearing JSON client.
- Cache a negative answer only when the authoritative resolver returns a
  definitive not-found result. Timeouts, offline errors, 429, 5xx, rejected
  redirects and malformed transient responses must remain retryable with a
  bounded backoff.
- Invalidate the existing poisoned `.miss` entries when shipping this change.
  Versioning the avatar-cache directory or its record format is safer than
  trying to identify which old zero-byte misses came from a legitimate 404.

## Tests Claude must add or revise

Geometry tests should prove the picture, not merely continuity:

- Every object on logical lane `n` uses the same displayed x at each width.
- A fitting leading lane does not move; a reached lane clamps continuously to
  the boundary.
- At the minimum span, every lane maps to lane 0.
- One-pixel pointer movement never causes a larger jump, and no floor-based
  threshold remains.
- Widening restores each lane's original x and identity.
- Ordinary node diameter stays constant while width and visible lane count
  change; compact mode is separately constant.
- Painter tests cover folded edge paths and nodes together and prove that no
  clipping leaves a node without its lane.

Avatar tests should cover behavior the existing pure source tests miss:

- An ordinary commit email with a GitHub commit association resolves to the
  forge avatar and is reused for later commits by that identity.
- A numeric no-reply address still uses the direct image path.
- A handle-only no-reply address succeeds through the redirect-safe path.
- A definitive 404 is negatively cached; 429, 5xx, timeout and offline failure
  are not cached for 90 days.
- Old cache misses do not suppress the new resolver.
- The Author column, graph node and commit detail all switch from the same
  generated fallback to the same decoded image.

## Runtime acceptance

Use Flea for the final Tauri sweep because it reproduces both defects and has a
known GitHub association for the visible `GM` commits.

1. Start with the new avatar cache empty and real pictures enabled.
2. Open the same `appshelf-packages` graph and wait for the visible identities
   to settle. The repeated `GM` circles must become the real `thisisgm` picture,
   and every surface showing that author must agree.
3. Scroll between shallow and deep portions. Ordinary commit circles must keep
   the same diameter. Only merge dots may be smaller.
4. Drag wide to the minimum slowly, reverse quickly, and release outside the
   header. Fitting lanes stay still until reached; overflow paths and nodes fold
   together; at minimum they share lane 0; expanding restores the graph.
5. Verify hover targets, stash marks and ghost paths at wide, intermediate and
   minimum widths.
6. Record the actual Tauri webview at 60 fps and fill the FEAT-081 manual sweep
   with observed results. Chromium unit tests alone are not visual acceptance.

Run the targeted frontend and Rust tests, the full test suite, `bun run check`,
and `bun run build`. Complete the repository's required review after the final
source revision. No new dependency is needed for either correction; if Claude
adds one, the handoff must explain why.
