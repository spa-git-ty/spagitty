<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-029 — The packaged Linux window is ringed with blurred desktop

**Status:** Fixed, awaiting sweep
**Branch:** `feature/FEAT-079-avatars-and-quieter-settings`
**Screens:** All screens; the window itself.

## Problem and reproduction

Running the release AppImage on Omarchy, the author supplied a screenshot
showing the application content inset by about ten pixels on every side, with a
band of blurred desktop filling the gap — inside Hyprland's own orange active
border. The application looked like a card sitting inside a window rather than
like a window.

1. Build or download the AppImage.
2. Run it on Hyprland (Omarchy's default) with blur enabled, which it is by
   default.
3. Look at any edge of the window.

**Observed:** a ~10px band of blurred desktop between the compositor's border
and Spagitty's content, on all four sides.
**Expected:** the content meets the compositor's border.

## Cause

Two decisions that are each correct alone and wrong together.

`src-tauri/tauri.conf.json` creates the window with `transparent: true` and
`decorations: false`, and FEAT-037 draws the window's corner, edge and shadow in
CSS. For a shadow to have anywhere to land, `app.css` gives `body` a
`--window-gap` of 10px and paints nothing in it — a transparent margin, by
design.

On macOS and Windows that margin is genuinely empty, because nothing is behind
the window. On a Linux compositor it is not empty: Hyprland blurs what is
behind a window's transparent regions, so the margin filled with smeared
desktop. The compositor was doing exactly what it is configured to do; the
margin should not have been there in the first place, because Hyprland — like
KWin and Mutter — already draws the corner, the border and the shadow the
margin exists to make room for.

`--window-gap` is already dropped for a maximized window. A tiled window on a
Hyprland workspace is not "maximized", so it kept the gap.

## Scope and acceptance criteria

- On Linux, the window's content is flush to its edges: no gap, no CSS corner,
  no CSS shadow, no CSS outline.
- On macOS and Windows nothing changes — the floating card, and the square
  maximized state, are both as they were.
- The Linux window is opaque, so there is no transparent region for a
  compositor to blur behind and no per-frame blending on a renderer that is
  already on the CPU.
- Spagitty still draws its own title bar on every host. Only the *edge* moves.
- Regression tests cover the platform decision and the two configuration files
  that must agree.

## Non-scope

- Restoring the shadow on Linux desktops that do **not** decorate windows. The
  detection available is the user-agent string, which names the platform and
  not the compositor; a per-compositor exception would be a list to maintain
  for a case where the result is merely plainer rather than broken.
- Making the gap a preference. Two ways to spell one answer.
- Anything about the AppImage's contents, which BUG-004 and TASK-037 own.
