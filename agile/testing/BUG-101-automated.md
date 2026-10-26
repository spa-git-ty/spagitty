<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-101 — Automated test record

**Item:** [BUG-101](../items/BUG-101-every-edit-in-calls-says-could-not-save.md)

- `src/lib/calls/CallsScreen.test.ts`, *writes an edit to the collection a moment later*: the save passes the collection's stem as both `stem` and `was`.
- `calls::store`, *a_collection_is_written_under_its_slug_and_listed*: a collection is written over its own file as often as it is saved when `was` is its stem, and refused when `was` is empty.

## Results — 2026-10-26

- `bunx vitest run src/lib/calls`: 66 passed.
