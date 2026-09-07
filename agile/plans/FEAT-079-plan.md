<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-079 — Plan

**Item:** [`agile/items/FEAT-079-a-node-that-says-who.md`](../items/FEAT-079-a-node-that-says-who.md)

## The shape of the problem

The lane canvas repaints every visible node on every scroll frame,
synchronously. It cannot await anything, and a picture that arrives half a
second after a row was painted has to reach the screen without the caller
asking again. Everything below follows from that.

The answer is the arrangement `store.svelte.ts` already uses for rows: an
untracked cache with a reactive counter beside it. A synchronous read that never
blocks, and a `version` that tells the canvas a repaint would now show something
new.

## The layers, and what each is not allowed to know

| Layer | Knows | Must not know |
| --- | --- | --- |
| `spagitty-core/src/avatars.rs` | How an address resolves, the disk cache, what a picture is | Anything about Tauri, or where the cache directory is |
| `forge/http.rs` | How to fetch bytes | Why |
| `commands.rs::avatar` | The cache directory, the preference | Geometry |
| `graph/avatars.svelte.ts` | Deduplication, the queue, decoding | Painting |
| `graph/lanes.ts` | How to draw a face into a circle | That a network exists |
| `CommitRows.svelte` | Which rows are on screen, where a node is | How a picture is found |

`lanes.ts` takes a `picture` **function** rather than importing the store. It is
the module with the most tests and the least business knowing about a network,
and a lookup passed in is also what makes the node testable — a test hands in a
value.

## Decisions

- **The resolution ladder is a pure function with a table of tests.** Everything
  this feature discloses is decided in `avatars::source`, so "what does Spagitty
  send" is answered by reading one function rather than by following a fetch.
- **GitHub no-reply resolves by account *id*, not login.** An id is permanent
  and a login is not. Somebody who renames their account keeps every commit they
  ever made, and a URL built from the old login would 404 for the rest of that
  repository's life. The older address form has no id in it and falls back to
  the login, which is the best available.
- **Gravatar with `d=404`, never `d=identicon`.** Spagitty already has a
  generated face and it is the better one: same on every machine, every launch,
  offline. Accepting Gravatar's would replace a deterministic face with a
  fetched one that says no more.
- **A miss is cached, and cached for longer than a hit.** The expensive case is
  an address with nobody's picture behind it: it costs a request that returns
  404, and re-asking it for every author in a large repository on every launch
  would be most of the traffic this feature can generate. Thirty days for a
  picture, ninety for its absence.
- **The cache is named by hash, not by address.** A cache directory is a place
  people look, and a listing that is every email address in every repository
  they have opened is a worse thing to leave on a disk than the pictures are
  worth.
- **The type is read from the bytes, not the `Content-Type`.** The header is the
  host's claim; the magic number is the fact. A `data:` URL with the wrong type
  renders as nothing at all, silently, and a host answering a picture request
  with an HTML error page is the common way that happens.
- **`data:` URLs, not the asset protocol.** The webview's existing content
  policy already allows `data:` for images. The asset protocol would need a
  scope, an escaping story, and a second path into the cache directory, to
  deliver a few kilobytes.
- **`get_bytes` takes no token, as a matter of signature.** Not "an empty token
  sends no header", which is what `get_json` allows — no parameter to pass one
  through at all. An avatar is a public image on a public URL, and a request for
  one that *could* carry a token is a way for a redirect or a typo to leak it.
- **A read limit on the response.** This path runs for every author in a
  repository; a host answering with a gigabyte would otherwise be a gigabyte in
  memory. A body over the limit is discarded rather than truncated — half a PNG
  is not an avatar, and a caller that got one would cache it.
- **The rows ask; the canvas draws.** An effect over the visible range asks
  once per range change. Asking from the canvas would ask once per author *per
  frame* and spend a fling filling and draining a queue.
- **The store starts disabled.** Not because the preference is off — it is on —
  but so nothing leaves the machine in the window between the graph's first
  paint and the preference being read. The backend checks the preference too and
  is the authority; the store only decides whether to bother it.
- **Turning it off empties the cache, in the same command that stores the
  preference.** A picture that goes on being drawn from disk after somebody
  opted out is a preference that did not do what it says.
- **`version` bumps on `onload`, not on arrival.** Bumping earlier makes the
  canvas draw an image with no dimensions, which paints nothing and leaves the
  node blank until some unrelated repaint.
- **The hover target is sized to the node exactly.** A generous target would
  answer "who is this" for a pointer over the lane *beside* the node, which
  belongs to a different row — and a tooltip naming the wrong person is worse
  than one that takes a second attempt.
- **SHA-256 and base64 are written out rather than depended on.** Both crates
  are already in the tree, so this is not about binary size: it is that each is
  one function with published test vectors, and the alternative is two more
  entries on the application's own dependency surface for something a repository
  walker never needed. Both have vector tests.

## Alternatives rejected

- **Keep the marble and add only the hover.** Half the fix. The hover answers
  "who" for one node at a time, on demand; a picture answers it for the whole
  screen at a glance, which is what a face on a node is for.
- **Fetch through a forge API.** Argued in the item's non-scope.
- **Prefetch every author in the repository at open.** A repository with two
  thousand contributors would make two thousand requests for the four whose
  commits are on screen.
- **`Vec<u8>` over the IPC boundary.** Tauri serialises it as a JSON array of
  numbers — roughly four characters per byte. Base64 is 1.33.

## Files

**Rust**
- `crates/spagitty-core/src/avatars.rs` — new. Resolution, cache, hashing,
  encoding, and fourteen tests.
- `crates/spagitty-core/src/forge/http.rs` — `get_bytes`.
- `crates/spagitty-core/src/lib.rs` — the module.
- `src-tauri/src/settings.rs` — `fetch_avatars`.
- `src-tauri/src/commands.rs` — `avatar`, `AvatarAnswer`, `avatar_cache`, and
  the cache-emptying in `set_settings`.
- `src-tauri/src/lib.rs` — the handler.

**Frontend**
- `src/lib/graph/avatars.svelte.ts` — new. The cache, the queue, the counter.
- `src/lib/graph/lanes.ts` — the `picture` lookup, and `drawHead`'s three faces.
- `src/lib/graph/LaneCanvas.svelte` — passes the lookup, depends on the counter.
- `src/lib/graph/CommitRows.svelte` — the asking effect, the hover target, the
  author column.
- `src/lib/settings/BehaviourSection.svelte` — the toggle.
- `src/lib/types.ts`, `src/lib/api.ts` — `AvatarAnswer`, `avatar`.
- `src/routes/+layout.svelte` — the preference reaching the store.
