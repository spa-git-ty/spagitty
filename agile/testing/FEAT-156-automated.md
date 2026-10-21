<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-156 — Automated test record

**Item:** [FEAT-156](../items/FEAT-156-calmer-palettes-and-an-accent-of-your-own.md)

- `src/lib/themes.test.ts`: the new grounds; the tomato is the default and first, and leaves a palette as it is; every accent on each variant keeps text on it at 4.5:1 and itself at 3:1 on the background and the panel; an accent replaces the accent, the text on it and the selection and nothing else; every value parses; only the listed accents are recognised, and any other falls back. The existing readability checks pass on the new Giorno and Notte.
- `src/lib/theme.test.ts`: a fresh start is on the tomato; a picked accent is on the root with its own text and selection, the lanes, danger and ground unchanged; the dark shade in dark; it is stored and restored; an unknown one falls back; it is in the palette cached for the boot script; it moves the revision.
- `src/lib/settings/sections.test.ts`: Appearance has a swatch for each accent, marks the tomato, and marks and applies another on a click.

## Results — 2026-10-21

- `bunx vitest run`: 229 files, 5081 tests passed (one Files test that times out under the full run's load passed on its own).
- `bun run check`: 0 errors, 0 warnings.
