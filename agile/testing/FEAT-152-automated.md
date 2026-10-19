<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-152 — Automated test record

**Item:** [FEAT-152](../items/FEAT-152-the-files-tree-widens-and-copies.md)

- `src/lib/ui/VirtualRows.test.ts`: rows asked to be held stay drawn after a scroll far past them.
- `src/lib/files/files.test.ts`: a file's right-click copies its name, its relative path and its text read at the revision; a folder offers name and path only. A selection from line 1 keeps lines 1 to 1,000 drawn when scrolled 40,000 px down, and lets them go when it is cleared.
- `src/lib/panels.test.ts`: a reset puts `filesTree` back with the rest.

## Results — 2026-10-19

- `bunx vitest run src/lib/files src/lib/ui/VirtualRows.test.ts src/lib/panels.test.ts src/routes`: 514 passed.
- `bun run check`: 0 errors, 0 warnings.
