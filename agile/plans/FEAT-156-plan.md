<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-156 — Plan

**Item:** [FEAT-156](../items/FEAT-156-calmer-palettes-and-an-accent-of-your-own.md)

1. `src/lib/themes.ts`: new Giorno and Notte values; `ACCENTS`, `DEFAULT_ACCENT`, `isAccent`, `accentOf` and `withAccent`, which replaces the accent, the text on it and the selection tint.
2. `src/app.css`: the boot values to match; the environment's washes from the accent (and lane 5 in light, lane 4 in dark) instead of the accent and the tomato lane.
3. `src/lib/theme.svelte.ts`: an `accent` stored under `spagitty.theme.accent`, `setAccent`, restored in `init()`, applied in `current()` to the family's palette only.
4. `src/lib/settings/AppearanceSection.svelte`: an *Accent* row of swatches, a radio group.
