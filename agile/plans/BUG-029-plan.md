<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-029 — Plan

**Item:** [`agile/items/BUG-029-the-window-is-ringed-with-blurred-desktop.md`](../items/BUG-029-the-window-is-ringed-with-blurred-desktop.md)

## Approach

The window already has a vocabulary for "do not draw a card": `data-window`, set
on the root element by `chrome/window.ts`, with `maximized` dropping the gap,
the corner and the shadow. The whole fix is a third value in that vocabulary and
one question to decide it.

So the change is small in three places and nowhere else:

1. `chrome/window.ts` gains `shellState`, a pure function from *(maximized,
   user agent)* to `floating | maximized | flush`, and publishes its answer
   where `watchMaximized` already published a boolean.
2. `app.css` and `+layout.svelte` add `flush` beside `maximized` on the rules
   that already exist, plus one rule of its own for the outline.
3. `src-tauri/tauri.linux.conf.json` makes the Linux window opaque.

Nothing is added to the paint path, and nothing changes on the other two hosts.

## Decisions

- **A third state, not a fourth condition on the second.** `maximized` and
  `flush` draw the same picture and are asked for different reasons: one is
  "this window fills the screen", the other is "somebody else already drew the
  edge". Collapsing them would make restoring a Linux window hand the gap back,
  and would make the CSS unreadable the first time somebody asked why a tiled
  window is not maximized.
- **`flush` outranks `maximized`.** The compositor decorates the window whether
  it is maximized or not, so the Linux answer does not change with the size.
  Checked first, and the test says so.
- **The user agent, and a whole-word match.** The real question is "does this
  compositor decorate windows for me", and no desktop has an API for it. The
  proxy is the platform, the platform is in `navigator.userAgent`, and taking
  it from there costs nothing — `@tauri-apps/plugin-os` would be a dependency
  added to learn something already in the document. `\bLinux\b` rather than
  `includes('Linux')`, so a product name in a UA string cannot decide how the
  window is drawn.
- **An empty agent reads as "draw the card".** A test environment with no
  `navigator` must not silently take the Linux branch and hide the card from
  every other test.
- **Opaque on Linux as well as flush.** Flush alone fixes the picture: with no
  transparent pixel there is nothing for the compositor to blur. Opaque makes
  it cheap as well as right — a window the compositor need not blur behind or
  blend, on a renderer that is already rasterising every frame on the CPU
  (`platform.rs`) — and it closes the door on the WebKitGTK failure recorded
  there, where an accelerated transparent window loses its buffer and goes
  invisible.
- **The duplication in `tauri.linux.conf.json` is pinned by a test.** Tauri
  merges the platform config with RFC 7396 JSON Merge Patch, which replaces
  arrays rather than merging their elements, so the override cannot say "the
  same window, but opaque" — it has to repeat the whole window object.
  `tools/window.test.ts` asserts the two objects differ in exactly one field,
  and carries the reasoning that JSON cannot hold.

## Alternatives rejected

- **Keep the gap and paint it.** Filling the margin with the window's own
  background would stop the blur and leave a 10px border of flat colour outside
  the corner, which is worse than either honest answer.
- **Detect the compositor.** `XDG_CURRENT_DESKTOP` or the Hyprland socket would
  name it, and then there is a list of compositors to maintain, in the frontend,
  for a difference nobody can see: the ones that do not decorate windows get a
  plain rectangle, which is what a window looks like there anyway.
- **Turn Hyprland's blur off for this window.** A window rule in somebody's
  `hypr.conf` is not a fix; it is asking every user to work around us.
- **Drop `transparent` everywhere.** macOS and Windows draw no shadow of their
  own for an undecorated window. Taking the card away there would be trading
  this bug for a worse-looking application on two platforms that never had it.

## Files

- `src/lib/chrome/window.ts` — `ShellState`, `decoratesItself`, `shellState`.
- `src/lib/chrome/window.test.ts` — the policy, per host.
- `src/app.css` — `flush` beside `maximized` on `--window-gap`.
- `src/routes/+layout.svelte` — `flush` on the corner and shadow rules, and the
  outline rule.
- `src-tauri/tauri.linux.conf.json` — new; the opaque Linux window.
- `tools/window.test.ts` — new; the two configs agree, and why the file exists.
