<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-070 — Plan

**Item:** [TASK-070](../items/TASK-070-the-plans-file-list-scrolls-in-a-card-the-height-of-its-neighbours.md)

The row is a wrapping flex row whose items stretch. Take *What changes* out of the row's height with `contain: size`, keep `align-items: stretch` so it is given the height the other two set, and make its list `flex: 1` with `overflow-y: auto`. Give the card a least height for the line it has to itself when the row wraps.

Not done with a fixed height or a measured one: the other two cards' height depends on the type size and on how the strategy descriptions wrap, and nothing should have to be re-measured when they change.

## What decides it is right

- The three cards are the same height, and that height does not change with the number of files.
- The rows of the list keep their height and the list scrolls.
