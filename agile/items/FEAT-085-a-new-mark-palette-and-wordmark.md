<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-085 — A new mark, palette and wordmark

**Status:** Open — merged into `main` on 2026-09-30; the rest of the manual sweep is still owed.
**Branch:** `feature/FEAT-085-a-new-mark-palette-and-wordmark`
**Screens:** the application icon on every platform, the tray and menu bar,
the title row with nothing open, All repositories when empty, Settings → About,
and the brand collateral.
**Raised by:** the author, 2026-09-30: "I want to redesign the icon and
identity and branding, it looks bad." Asked for the direction, they chose to
keep the pasta but have it drawn well, a new palette, and a new wordmark; from
a concepts page they picked the *Untangle* mark, the *Pomodoro* palette and
*Sora*.

## Problem

The mark was an S of many strands with a fork in its middle and commit nodes
at its ends, dark grey on an amber plate. At 16 and 32 pixels it was a smudge;
at every size it read as clip art. The wordmark was Inter with wide tracking,
the same face as half the tools on a developer's desktop.

## Change

- **The mark:** three cream strands on a tomato plate. They cross — one passes
  over the other two — and then run straight, each ending in a commit.
  `assets/brand/mark.svg`, a 100 × 100 viewBox in a small SVG subset.
- **Tomato, not Git's orange.** The plate is `#CC3B2C`, deeper and redder than
  the concept's `#E0492F`, which sat next to Git's own logo colour
  (`#F05133`), a colour the brand guide already rules out.
- **The wordmark:** `spagitty` in Sora SemiBold, tracking −0.02 em, with
  **git** in the brand's tomato. Sora is bundled (SIL OFL 1.1); Inter is gone.
- **Generation:** `tools/make-icons.py` renders strokes, round caps, the
  crossing's halo and the commit dots from the SVG — before, it could only fill
  hand-drawn outlines — and `tools/make-brand.py` sets the lockups, the README
  banner and a new preview page in Sora. Every icon, tray mark, favicon and
  lockup is regenerated; both `--check` modes pass.
- **In the application:** `Wordmark.svelte` is the name as the brand sets it;
  the title row with no tab open shows the mark and the wordmark, and so does
  All repositories when it is empty. `--brand` is a token of its own —
  `#b8321f` on light, `#f2715a` on dark — so the name stays tomato whatever
  theme family is chosen. `BrandMark` is square.
- **The brand guide** describes the new mark, colours, type and rules.

## Non-scope

- The theme families. The Pomodoro theme is its own item.
- The words: the tagline and descriptors are unchanged.

## Acceptance criteria

- The application icon, taskbar and tray icons, favicon and lockups are the new
  mark, and `python3 tools/make-icons.py --check` and
  `python3 tools/make-brand.py --check` pass.
- The mark is legible at 16 px.
- The title row with no repository open shows the mark and "spa**git**ty".
