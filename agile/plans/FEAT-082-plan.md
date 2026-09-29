<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-082 — Plan

**Item:** [`agile/items/FEAT-082-a-spatial-shell.md`](../items/FEAT-082-a-spatial-shell.md)

## Approach

**Materials first, in `app.css`.** Three new pieces, each a token and a class:

- `--environment` — the window's ground. The theme's panel colour lit by three
  soft radial washes of its own accent, first lane and second lane. Every value
  is a `color-mix` of palette tokens, so each of the eight families lights its
  own environment and nothing names a colour of its own.
- `.pane` — opaque `--bg`, the glass edge (lit top, hairline sides), a two-part
  shadow, `--r-pane`.
- `.ornament` — `--glass-thick` with `--blur-thick`, the same edge, a tighter
  shadow. The one class the rail and the toolbar share.

The radius scale steps up together (field 7, button 9, panel 10, floating 16,
pane 18, ornament 24; pill is a capsule again). `metrics.ts` republishes them
at zoom, and `metrics.test.ts` still reads the stylesheet to keep the two
copies in step.

**Then the shell, in `+layout.svelte`.** `TitleBar`, then a `.stage` holding
the rail and `<main class="pane">`, then `StatusStrip` with the `Toolbar` as its
child. The rail's splitter goes.

**Then each piece, keeping its behaviour.**

- `TitleBar` renders `RepoTabs` in its leading column and the name only when
  there are no tabs. Still three columns with equal outer tracks when it shows
  the name; with tabs, the tabs take the room.
- `RepoTabs` loses its row and draws pills.
- `NavRail` is rewritten around one idea: every row always renders its icon,
  label and count, and CSS decides what shows. Closed, it is 52px of icons with
  a dot for changed files or conflicts; hovered or focused, it widens to 216px
  over the pane, after a 160ms rest so a pointer crossing it does not open it.
  The rail sits in a fixed-width slot, so opening it moves nothing.
- `Toolbar` drops the repository name and the Settings gear, and becomes a pill
  of capsule buttons: branch picker, Pull, Fetch, Push, Clone | Branch, Stash,
  Rebase, and Commands when that setting is on. Labels give way to icons below
  1180px.
- `StatusStrip` takes a `children` snippet and becomes a three-track grid: the
  identity and state, the snippet, the counts and the licence.

## Alternatives considered

- **Ornament over the pane's bottom edge**, as visionOS draws it. It would
  cover the last rows of every list and the Working copy screen's commit box.
  Below the pane costs a row the old status strip already had.
- **A translucent pane over the environment.** It would have to repaint its
  backdrop on every scroll of the graph. The glass that reads as spatial is on
  the objects that float; the pane is a surface.
- **Operating-system materials now.** Mica and vibrancy need the running
  application on each platform to verify. Their own item.

## Files

| File | Change |
| --- | --- |
| `src/app.css` | `--environment`, `.pane`, `.ornament`, pane and ornament radii, the rounder scale. |
| `src/lib/metrics.ts` | The radii. |
| `src/routes/+layout.svelte` | The stage, the pane, the toolbar in the strip; no rail splitter. |
| `src/lib/chrome/TitleBar.svelte` | The tabs in its row; the name only without them. |
| `src/lib/chrome/RepoTabs.svelte` | Pills. |
| `src/lib/chrome/NavRail.svelte` | The ornament. |
| `src/lib/chrome/Toolbar.svelte` | The ornament. |
| `src/lib/chrome/StatusStrip.svelte` | The row the toolbar sits in. |

## Risks and rollback

- **The rail's collapse state and width** are still in `panels.svelte.ts`, and
  nothing reads them now. Removing them is a follow-up, so this change does not
  also rewrite the panel store's persistence.
- **Blur on the software path.** Two ornaments at `blur(10px)`, which TASK-022
  measured at frame-time parity; the rail's open state blurs a list while it is
  open. To be measured on Linux in the sweep.
- Rollback is a revert.
