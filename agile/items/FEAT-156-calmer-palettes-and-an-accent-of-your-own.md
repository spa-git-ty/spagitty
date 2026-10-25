<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-156 — Calmer light and dark palettes, and an accent of your own

**Status:** Done — shipped in 3.9.0, through pull request #88. Covered by palette, theme store and Appearance tests; not looked at in the running application (the author is checking it).
**Branch:** `feature/FEAT-156-palettes-and-accents`.
**Screens:** every screen; Settings › Appearance.
**Raised by:** the author, 2026-10-21: "light theme looks yellowish! and its not pretty :P , also dark it's good ambiant spatial but lets enhance colors for dark mode and light mode to be more comfy , plus allow users to pick acent colors instead of our orange one , on appernce settings( include our color by deafut allow change to others pick what you see suitable )".

## Cause

Giorno's ground was warm cream (`#fbf7f1`, panel `#f3ece2`), and the spatial environment laid two tomato washes over it: the accent and lane 1, which is also tomato. Together they made the whole window read as yellow-orange. Notte was near-black umber under near-white cream text, which is more contrast than is comfortable to read all day.

## Change

- **Giorno** on paper white with a trace of warmth in the greys: `bg #fdfdfc`, `panel #f1f0ee`, `ink #24201e`. The light environment is a faint accent wash and a faint cool one opposite it.
- **Notte** a step softer and less brown: `bg #1e1b1a`, `panel #171514`, `ink #ebe4dc`. The dark environment keeps its glow, now from the accent alone, with a little aubergine for depth.
- **An accent picker** in Settings › Appearance: Tomato (the family's own, the default), Ocean, Violet, Jade and Graphite, as round swatches in the shade the current mode paints. None is red, amber or green, which already mean danger, warning and success. A picked accent replaces three tokens — the accent, the text on it and the selection tint — and nothing else; the lanes, the semantic colours and the wordmark's tomato stay.
- The accent is stored beside the family (`spagitty.theme.accent`), cached for the first frame, and moves the theme's revision so the graph repaints. It is never laid over a followed desktop palette; picking one stops following it.

## Acceptance criteria

- The light window no longer reads as yellow; the dark one keeps its ambient glow.
- Every accent, in each mode, keeps text on it at 4.5:1 and itself at 3:1 against the background and the panel.
- Picking an accent recolours the application at once, survives a restart, and paints on the first frame.
