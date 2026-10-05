<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-049 — Plan

**Item:** [`agile/items/BUG-049-small-monospace-text-ignores-the-reading-font.md`](../items/BUG-049-small-monospace-text-ignores-the-reading-font.md)

## Approach

Publish `--font-mono` from the same place as `--code-font`, so the 190-odd
places that read it follow the setting without knowing there is one, as
FEAT-090 did for the code views. `monoStack` picks the face: the code face
unless that face is marked `proportional`, then the default code face.

`--code-font` still defaults to `var(--font-mono)` in the stylesheet. Reading
sets both on the root, so neither refers to the other once it runs.

## Files

| File | Change |
| --- | --- |
| `src/lib/reading.svelte.ts` | `proportional` on Lexend; `monoStack`; `apply` publishes `--font-mono`. |
| `src/app.css` | `--font-mono` leads with Atkinson Hyperlegible Mono. |
| `src/routes/reflog/+page.svelte` | The ids cell is one line; its note is in the interface face. |
| `src/lib/reading.test.ts` | The token follows the face, never a proportional one, and matches the stylesheet. |
| `CHANGELOG.md` | Unreleased › Fixed. |

## Risks and rollback

- A wider code face — OpenDyslexic Mono — widens every id. The cells that hold
  them already end in an ellipsis or have room; the sweep looks at the
  narrowest.
- Rollback is a revert.
