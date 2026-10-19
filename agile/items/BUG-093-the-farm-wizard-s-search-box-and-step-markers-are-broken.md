<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-093 — The farm wizard's search box and step markers are broken

**Status:** Fixed — on `release/3.3.0`; unmerged. Not looked at in the running application (the author is checking it).
**Screens:** 1Q (starting a farm)
**Raised by:** the author, 2026-10-19, on 3.2.0, with a screenshot of step 2: "also check this ui bug".

## What happens

- **Issues › Search issues:** the magnifier sits alone in a dark bar, and the
  field is a second box hanging below it, over the card's edge.
- **The steps in the header:** each step is a tiny accent dot with its number
  printed under it, instead of a ringed number; the done step's tick sits on a
  dot.

## Cause

Both are farm.css's shared rules winning over the wizard's own. Each pair has
the same specificity, and farm.css comes later:

- `.farm-journey label` is a column, and `.farm-journey input` gives every input
  a border, padding and background. The search box is a `<label>` holding the
  icon and the input, so the icon went on top and the input became a box of its
  own.
- `.farm-journey .dot` is the 7px status dot used in Activity and Building. The
  step markers were also called `.dot`.

## Fix

- `WizardSteps.svelte`: the markers are `.marker`.
- `StepFrom.svelte`: the search box says `flex-direction: row`, and its input's
  rule is `.search input[type='search']`, more specific than the shared one.

## Not covered

No test: this is which CSS rule wins, which the test DOM does not compute.
