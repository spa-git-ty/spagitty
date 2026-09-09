<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-031 — The window opens in the wrong theme, then changes its mind

**Status:** Fixed.
**Screen:** the whole window at startup, and Settings → Appearance.
**Raised by:** a UI review: "a saved dark/custom theme is applied after the
initial light markup; a flash is possible. There is no continuing Follow system
behaviour."

## Problem

**Two defects with one cause: the theme was decided too late, and then decided
too permanently.**

`src/app.html` opened with `data-theme="light"`, and `theme.init()` ran in the
layout's `onMount`. Between those two moments the window is up and `app.css` has
been parsed, so the application paints Catppuccin Latte — and somebody whose
saved theme is Mocha, or Dracula, or Gruvbox dark, watched their Git client open
white and then change its mind. On Linux this is not one frame: the release
build deliberately runs WebKitGTK's software renderer (FEAT-055), so every
repaint is rasterized on the CPU.

`init()` also read the system's light/dark preference **once**, on a first run,
and wrote the answer down through `commit()` as an explicit stored mode. From
that moment the application held a decision the user had never made:

- a desktop switching to dark in the evening moved everything except Spagitty;
- there was no way back to following it short of clearing `localStorage`;
- and nothing could tell a mode that had been *chosen* from one that had been
  *sampled*, because both were the same key with the same value.

Settings → Appearance offered Light and Dark and no third answer, which is
consistent with the storage: there was nowhere to put a third answer.

## Change

- **`src/theme-boot.js`** (new) runs in the document head, synchronously, before
  anything paints. It reads the palette the previous session cached and puts it
  on the root element — the same values `theme.svelte.ts` will set again a
  moment later, so the handover is invisible rather than a second change of
  mind. With nothing cached it takes the desktop's light/dark preference, which
  is enough on its own: `app.css` already carries both of the default family's
  palettes keyed off `data-theme`, so a fresh install on a dark desktop no
  longer opens white either.
- **The cache is written by `theme.svelte.ts` on every change**, as *resolved*
  custom properties rather than a family name — so the boot path never has to
  load the 550-line palette table, and a palette that arrives from somewhere
  other than that table is cached on the same terms (which FEAT-080 needs).
- **Everything the boot script reads is validated.** Property names must match
  `--kebab-case` and values a colour-shaped whitelist; anything else is dropped
  and the rest of the palette still applies. `localStorage` here is not an
  attack surface so much as a place a half-written value from a previous version
  can live, and the cost of trusting one is a window with no colours.
- **`spagitty.theme.source`** (new) is where the light/dark answer comes from —
  `manual` or `system` — kept separate from `mode`, which stays what is resolved
  and on screen. Choosing a mode sets the source to `manual`, because choosing
  is the act of taking over; choosing a family does not touch it.
- **`system` follows the desktop live**, through a `matchMedia` listener that
  exists only while that source is selected and is torn down by the shell.
- **Settings → Appearance gains a third chip, "Follow system"**, beside Light
  and Dark rather than above them: the three are one decision. While it is
  active the row also says which mode it is currently resolving to.
- **The migration keeps explicit choices.** An install from before the source
  existed has a mode and no source; that mode is treated as `manual`, because
  it might have been chosen and demoting a real preference is the worse of the
  two mistakes. The reverse — a sampled mode kept as manual — costs one visit
  to Appearance.

## Why the boot script is a file

`src-tauri/tauri.conf.json` sets `default-src 'self'` and names no
`script-src`, so scripts fall back to `'self'` and **an inline script is
blocked**. The alternatives were to add `'unsafe-inline'` — disarming the policy
for the whole application to save one request against the local bundle — or to
hash the script into the CSP, which means a hash in a JSON file that has to stay
in step with a script in an HTML file, failing silently when it does not. A
same-origin file needs no policy change at all.

It cannot live in the SvelteKit assets directory either: `svelte.config.js`
scopes that to `assets/brand/favicon`, which `tools/make-brand.py --check` owns
and regenerates. So the file sits beside `app.html` and a small Vite plugin
emits it into the bundle and serves it in development, from one source.

## Acceptance criteria

- A cold start on a saved dark or imported theme paints that theme on the first
  frame. No white frame, at any renderer.
- A fresh install on a dark desktop opens dark.
- With the source set to `system`, changing the desktop's preference changes
  Spagitty's mode while the window is open, without touching the family.
- Choosing Light or Dark stops it following, and survives a restart.
- A corrupt or hand-edited cache entry leaves the stylesheet's own values alone.

## Non-scope

- **Following a desktop's *palette*.** That is FEAT-080. This is the mechanism
  it needs — a source that is not a mode, and a cache that is not a family name.
- **Measuring cold launch.** The review asked for it; a stopwatch on a Tauri
  window is a sweep ticket, not a test, and `SWEEP-001` carries it.

## Dependencies

FEAT-055 is why a repaint on Linux costs what it does. TASK-038 last touched
the Appearance section.
