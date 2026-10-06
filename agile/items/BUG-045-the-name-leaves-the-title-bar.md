<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-045 — The name leaves the title bar

**Status:** Fixed.
**Branch:** `bugfix/BUG-045-the-name-leaves-the-title-bar`
**Screens:** All (the title bar).
**Raised by:** the author: "the title spagitty dissaper when i open a tab".

## Problem

FEAT-082 put the repository tabs in the title row and hid the name whenever a
tab was open, on the reasoning that a tab already says what is open. The author
wants the program's name there whatever is open.

## Scope

- The mark and the wordmark stay centred in the title bar with tabs open.
- The tabs keep the left of the row; past its width they scroll, as they did.
- No changelog entry: FEAT-082 is unreleased, and its entries never said the
  name hides.

## Acceptance criteria

- With one or more tabs open, the title bar shows the mark and "spagitty",
  centred in the window.
- With no tab open, it shows them as before.
