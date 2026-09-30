<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-051 — Plan

**Item:** [`agile/items/TASK-051-one-theme.md`](../items/TASK-051-one-theme.md)

## Approach

Keep `Palette`, `Variant`, `Family`, `FAMILIES`, `familyOf`, `variantOf`,
`paletteOf` and `isFamily` — the store and the Omarchy path are written against
them — and narrow them to one: `FamilyId` is `'pomodoro'`, `FAMILIES` has one
entry, the sixteen other palettes go. `isFamily` then rejects every removed id,
so `theme.init()` needs no migration of its own: a stored `nord` is not a
family and takes the default, and the store writes `pomodoro` back on commit.
FEAT-086's `catppuccin` move and `spagitty.theme.pomodoro` marker go.

`AppearanceSection` loses its swatch grid and the derived swatches; the mode
chips and Follow Omarchy stay. svelte-check is run until no unused CSS is left.

Tests: those about choosing between families, naming other families' variants,
or one family's accent differing from another's go; those that used a family as
a stand-in use Pomodoro; a stored removed family is shown to fall back.

## Files

| File | Change |
| --- | --- |
| `src/lib/themes.ts` | Pomodoro alone. |
| `src/lib/theme.svelte.ts` | No Catppuccin move. |
| `src/lib/settings/AppearanceSection.svelte` | No family grid. |
| `src/lib/themes.test.ts`, `theme.test.ts`, `omarchy.test.ts`, `settings/sections.test.ts` | One family. |
| `README.md`, `docs/screens.md`, `CHANGELOG.md` | One theme. |

## Risks and rollback

- **Somebody who liked a published palette** loses it. The author chose that;
  following the desktop's palette under Omarchy remains for anyone who themes
  their whole desktop. Rollback is a revert.
