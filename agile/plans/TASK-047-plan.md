<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-047 — Plan

**Item:** [`agile/items/TASK-047-a-commit-bar.md`](../items/TASK-047-a-commit-bar.md)

## Approach

`MessageBox` becomes the bar and moves from the top of the diff column to the
page's footer. The Commit button stays the page's — its label and its enabled
state are the page's knowledge — and is handed to the bar as an `action`
snippet, rendered last in the row.

The body is rendered only while `asked || changes.body.length > 0`: asked for
by the "Add description" button, which focuses the field once it exists, or
already holding text, so a description typed and then left is never hidden.

The page renders the footer only in the state where the file column and diff
are shown; a write error with nothing else to show keeps a footer of its own.

## Files

| File | Change |
| --- | --- |
| `src/lib/changes/MessageBox.svelte` | The bar; the body on request; `action`. |
| `src/routes/changes/+page.svelte` | The bar in the footer with the button; none when clean. |

## Risks and rollback

- **Somebody used to the body** has to press "Add description" once per
  commit. The subject is still the first field and the only one required.
- Rollback is a revert.
