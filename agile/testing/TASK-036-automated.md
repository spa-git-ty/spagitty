<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-036 — Automated checks

## Checks

- `bun run test`: existing frontend and record assertions, including item/index
  consistency and complete item documents.
- `bun run build`: frontend production build.
- `git diff --check`: whitespace errors in tracked edits.
- Compare the pointer's range, dates, and amendment subjects with the canonical
  file, and confirm that the canonical file has not changed.

## Results

Verified 2026-09-05:

- `bun run test`: 110 files passed, 2,464 tests passed. The existing record tests
  also checked the new item's index row and document set.
- `bun run build`: passed; static production output generated successfully.
- `git diff --check`: passed.
- Pointer and agent instructions compared with the canonical book: matched.
  Its modification timestamp remains 2026-08-28 22:54:12 +0300; this change
  does not edit the canonical file.
- Local diff review completed: documentation only, no dependencies added,
  existing agent rules retained. PR review and merge have not occurred.

No new tests are needed for this documentation-only change. Coverage is unchanged
because no application source or test configuration changes.
