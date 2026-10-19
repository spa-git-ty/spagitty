<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-060 — Sweep

**Item:** [`agile/items/FEAT-060-spagitty-brand.md`](../items/FEAT-060-spagitty-brand.md)

## What a human checks at the window

Swift, per Amendment 4: the wheel stays with the author. The whole identity is
laid out on one page — open `assets/brand/preview.html` in a browser and walk
the sections; the checklist below maps to them.

1. **The mark.** The bare mark: the amber plate and the four dark strands read
   as the author's own drawing — the tangle at the top, the strands running
   straight toward the bottom. Nothing here should look re-drawn or smeared.
2. **App icon sizes.** 16 → 512 in a row (`src-tauri/icons/`): the plate and
   the strands survive down to 16 px.
3. **The lockups.** On both surfaces: mark and wordmark on one optical
   centreline; the wordmark's letterspacing deliberate, not broken.
4. **The hero.** At README width: mark left, wordmark + tagline. No banding,
   no clipped text.
5. **Tray/menubar.** White on a dark tray, black on a bright one, monochrome
   template in macOS; the strands-only mark still carries at 22 px.
6. **The accent.** The app's `--accent` matches the brand amber: `#EEB04D` in
   the dark theme, `#976317` in the light one — and `preview.html` flags that
   `--warn` sits ~1.09–1.35:1 against `--accent` in the light themes, accepted.
7. **The prohibition.** The whole set must contain nothing that reads as the
   Git project's mark or orange.

## What this sweep cannot say

- It cannot judge whether the identity is *good* — only whether it is
  consistent. If the shape wants a change, that is a new item, not a
  hand-edit of the bytes (the gate-2 drift check refuses hand-edits anyway).

**Result:** Obsolete, 2026-10-19 — FEAT-085 replaced the mark, the palette and the wordmark; its sweep checks the new ones.
