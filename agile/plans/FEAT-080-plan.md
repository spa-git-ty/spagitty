<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-080 — Plan

**Item:** [`agile/items/FEAT-080-follow-omarchy.md`](../items/FEAT-080-follow-omarchy.md)

## Approach

The obvious shape — "read the file, set the colours" — is wrong in two ways that
only show up on a real machine, so the plan is arranged around both of them.

**The palette is not a UI theme.** A terminal palette's roles and an
application's roles are different requirements that happen to share names.
Measured before any code was written, two of the four roles that matter fail
contrast when copied literally. So the work splits at that seam: Rust finds and
validates, the frontend derives — because deriving is design, and design the
frontend cannot check is design nobody checks.

**The palette is not a file.** `omarchy-theme-set` replaces a directory. Every
naive implementation of this feature watches `colors.toml`, works once during
testing, and then silently stops. The watch is designed against the script's
actual behaviour rather than against the file's existence.

Everything else follows: the source model was already built by BUG-031, and the
revision is what the graph needs because `id` cannot see the case this feature
introduces.

## Decisions

- **Rust finds and parses; TypeScript derives.** Splitting it the other way
  would put contrast arithmetic in Rust and leave `themes.test.ts` unable to
  hold a followed palette to the bars it holds the built-in eight to.
- **No path crosses the boundary.** The command answers "what is the desktop
  painted with", not "read this file". A general file-reading command reachable
  from the webview is a much larger thing to have built than a theme.
- **A hand-written scanner rather than a TOML crate.** `colors.toml` is
  generated: flat `key = "value"` pairs, comments, no tables, no arrays, no
  datetimes, no multi-line strings. A crate would be a new direct dependency in
  the binary to parse a language this file is one sentence of. The scanner
  stops dead at the first `[table]` header so a file that grows sections later
  is under-read rather than mis-read, caps keys and value length, and never
  unescapes — a colour has no escapes.
- **Bounded reads.** 64 KiB and a regular-file check, because this is a path the
  application did not write and "what if it is a fifo" deserves an answer.
- **A whitelist of colour shapes, narrower than CSS.** Omarchy writes
  `#rrggbb`. Accepting `rgb()` and named colours as well would widen the
  surface for no file that exists.
- **`firstReadable`, not `mostReadable`, for the accent label.** Taking the
  maximum sounds safer and is not: on this accent it returns pure black over the
  desktop's own near-black ground, which clears by more and belongs to nobody's
  palette. Contrast is a threshold to pass, not a score to win. This was found
  by a test, not by reasoning.
- **Lanes are de-duplicated.** Rosé Pine Dawn gives `blue` and `cyan` the same
  value; copied straight across, two branches would be drawn identically. Each
  lane takes the first candidate that is readable *and* unused, then the
  built-in family's lane, then that lane nudged. Also found by a test.
- **ANSI slots stay positional.** A gap is kept as a gap: slot 4 means blue and
  slot 5 means magenta, and compacting the list would silently recolour the
  graph.
- **The mode comes from the palette.** A followed palette carries `mode`, and
  `data-theme` has to agree with it or `app.css`'s boot values fight the inline
  properties on any token the palette does not set. Taken at `followDesktop()`
  as well as on each reading, so the chips and the paint never disagree for an
  event.
- **A failed reading is dropped, not applied.** The mid-swap window would
  otherwise be a visible flash of the built-in family.
- **`theme.revision`, not `theme.id`, for colour caches.** Every followed
  palette is `omarchy-dark`. The fingerprint covers bg, panel, ink, accent, the
  three status colours and all five lanes — the values a repaint depends on —
  so an event that re-reads an identical palette is not a repaint.
- **The watch runs only while the source is `omarchy`**, from an effect in the
  shell. A machine not following a desktop palette has no reason to hold two
  inotify watches on a directory it does not read, and a machine that starts
  following one mid-session must not have to restart.
- **Choosing a family opts out of `omarchy` but not out of `system`.** A family
  and a desktop palette are two answers to the same question; a family and
  light/dark are not.

## Files

- `src-tauri/src/desktop.rs` (new), `src-tauri/src/lib.rs`
- `src/lib/colour.ts`, `src/lib/omarchy.ts` (both new) and their tests
- `src/lib/theme.svelte.ts`, `src/lib/theme.test.ts`
- `src/lib/api.ts`, `src/lib/types.ts`
- `src/lib/settings/AppearanceSection.svelte`, `src/routes/+layout.svelte`
- `src/lib/graph/LaneCanvas.svelte`
- `CHANGELOG.md`, `agile/` — this set and the index row

## Steps

1. `desktop.rs`: the scanner, the search, the typed answer, the watcher — each
   with its refusals as tests, against the real `colors.toml`.
2. `colour.ts`: parse, luminance, contrast, mix, and the two selection
   functions, pinned to WCAG's published numbers.
3. `omarchy.ts`: the derivation, held to `themes.test.ts`'s own bars.
4. The store: the third source, the desktop palette, the revision.
5. The shell and Appearance.
6. `LaneCanvas` onto the revision.
7. A run against the real machine, both sides, recorded in the test document.
8. Changelog, this set and the index row.

## Risks and rollback

- **One desktop, one machine.** The feature is written against Omarchy's current
  layout and tested against two palettes — the real dark one and a light one —
  plus a deliberately bare one. Another generator's `colors.toml` may name
  things this does not read; the failure mode is designed to be "falls back to
  a derived value", not "breaks".
- **inotify limits.** A machine with none left gets a working theme and no live
  updates. `desktop_theme_watch` returns whether a watch is running so that can
  be said rather than guessed.
- **The `notify` watch is on a directory in the user's home**, not in a
  repository. It is registered only while following, and dropped by the shell's
  cleanup along with everything else.
- Rollback is reverting the branch. With the source set to anything but
  `omarchy` the whole feature is inert, and the backend module is not called.
