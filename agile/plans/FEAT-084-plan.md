<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-084 — Plan

**Item:** [`agile/items/FEAT-084-the-other-screens-in-the-spatial-language.md`](../items/FEAT-084-the-other-screens-in-the-spatial-language.md)

## Approach

**One token for the bands' rules.** Sixteen screens draw their header and
footer hairline with the same `color-mix(in srgb, var(--line) 55%,
transparent)`. Each now reads `var(--band-rule, <that colour>)`, and
`.window-pane` sets `--band-rule: transparent` beside FEAT-083's two tokens.
The fallback keeps the rule anywhere a screen is drawn outside the pane (a
test mount, a future window). The Farm's `.side` divider is a column boundary,
not a band, and keeps its line.

**The tables copy the file list's row, not a shared component.** Each table
keeps its own grid; the row gains `margin: 0 6px`, `padding-inline: 6px` (so
cells stay aligned with the header's 12px), `--r-button`, `--fs-secondary`, and
loses `border-bottom`. The actions cell is `opacity: 0` until `.row:hover` or
`.row:focus-within`: still in the tab order, still the same height, so nothing
jumps when they appear — the TASK-046 pattern.

**The tag cell.** `.what` and `.says` get `overflow: hidden`; the name becomes
`flex: 0 1 auto` with an ellipsis, scoped to `.what .name` so the create row's
name field is unaffected.

**The rest are local edits:** the Farm's `.title`, `Starter.svelte`'s `.goal`,
`.step` and `.check`; `ProfilesSection.svelte`'s `.title`; the History page's
wrapper (`flex: 1`), card and field.

## Files

| File | Change |
| --- | --- |
| `src/app.css` | `--band-rule` on `.window-pane`. |
| `src/routes/*/+page.svelte` (15) | Header and footer rules read `--band-rule`. |
| `src/lib/branches/BranchTable.svelte` | Rows; actions on hover; dividers on hover. |
| `src/routes/tags/+page.svelte` | Rows; actions on hover; the name cell. |
| `src/routes/reflog/+page.svelte` | Rows; actions on hover; the refs row's rule. |
| `src/routes/farm/+page.svelte`, `src/lib/farm/components/Starter.svelte` | Title; the starter. |
| `src/lib/settings/ProfilesSection.svelte` | The heading. |
| `src/routes/history/+page.svelte` | Width, card, field. |
| `src/lib/ui/flat.test.ts` | The above, held. |

## Risks and rollback

- **Hidden actions are less discoverable.** They appear on the row the pointer
  is on and on keyboard focus, as they do in the file lists; the current branch
  is still marked by its tick and its tint.
- Rollback is a revert.
