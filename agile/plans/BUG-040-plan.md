<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-040 — Plan

**Item:** [`agile/items/BUG-040-the-shells-pane-class-leaks-into-five-components.md`](../items/BUG-040-the-shells-pane-class-leaks-into-five-components.md)

## Approach

Rename the shell's class rather than the components': one global rule and one
element in the layout, against five components. `.pane` becomes
`.window-pane` in `app.css` and in `+layout.svelte`'s markup and scoped rule.
The FEAT-083 tokens set on it reach every screen exactly as before, since every
screen is inside the layout's `<main>`.

A test holds it: `app.css` declares no `.pane` rule, and `window-pane` appears
in one component, the layout.

## Files

| File | Change |
| --- | --- |
| `src/app.css` | `.pane` → `.window-pane`. |
| `src/routes/+layout.svelte` | The `<main>` and its rule. |
| `src/lib/ui/flat.test.ts` | The FEAT-082 test's selectors; the new guard. |

## Risks and rollback

- A stylesheet or test still naming `.pane` for the shell. None in `src` or
  `tools`. Rollback is a revert.
