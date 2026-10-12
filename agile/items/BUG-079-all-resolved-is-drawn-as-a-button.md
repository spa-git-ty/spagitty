<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-079 — *All resolved* is drawn as a button

**Status:** Fixed — on `release/2.2.2`, with BUG-080, FEAT-128 and FEAT-129; unmerged. Covered by a mounted test; not looked at in the running app.
**Screens:** 1D, 1S.
**Raised by:** the author, 2026-10-12, on 2.2.1, with a picture of the resolver's pill: "why all resolved is a button although it's not a button".

## What happens

The last place in the resolver's pill is **Next unresolved**, which goes to the next conflict with nothing chosen. Once every conflict is resolved the same button stays, its words changed to *All resolved* and its fill taken away. It still takes the pointer, the hover and the keyboard, and pressing it does nothing, because there is no next conflict to go to.

## Acceptance criteria

- With a conflict left, the last place is the **Next unresolved** button, as before.
- With none left, it says *All resolved* and is not a button: it cannot be pressed, focused or hovered.
- A screen reader is told when it changes.

## Fix

`ResolverPill.svelte` draws a `span` with `role="status"` in place of the button when nothing is left. It keeps the tick and the green, and has no fill and no hover.
