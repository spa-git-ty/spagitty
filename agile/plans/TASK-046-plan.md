<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-046 — Plan

**Item:** [`agile/items/TASK-046-lists-you-can-read.md`](../items/TASK-046-lists-you-can-read.md)

## Approach

**One component for a file's name.** `src/lib/ui/FileName.svelte` splits the
path at its last `/` into the name and the folder. The name is `flex: 0 1 auto`
with a small minimum and the folder is `flex: 1 1 0`, so the folder takes the
spare width and is the first to give it back; neither needs `direction: rtl`,
so the left-to-right mark goes too. The badge keeps the class `glyph`, which
the tests already find it by. Working copy, the diff and stash list, and the
commit detail all render it.

**Rows keep their classes.** `.row.solid`, `.row.dashed`, `.act`, `.open` and
`.file` are what the tests and the stores' callers find, so they stay; what
they draw changes. Stage is the row's first action and discard its second.
Git's own terms — Staged, Unstaged — stay as the section names.

**Commit actions.** `cherryPick` and `revertCommit` already exist in
`graph/actions.ts`, each behind a confirmation.

**Stash.** `note()` strips `On <branch>: ` and turns `WIP on …` into "Work in
progress". The drawing of the entry hanging off its commit stays (FEAT-034).

**The strip.** `ready` is `graph.complete || graph.count > 0`.

## Files

| File | Change |
| --- | --- |
| `src/lib/ui/FileName.svelte` | New. |
| `src/lib/changes/FileColumn.svelte` | Rows, headings, on-demand actions. |
| `src/lib/diff/FileList.svelte` | Rows. |
| `src/lib/graph/CommitDetail.svelte` | Rows; working commit actions. |
| `src/lib/stash/StashList.svelte`, `src/routes/stash/+page.svelte` | Three-line entries; no duplicate count. |
| `src/lib/chrome/StatusStrip.svelte` | Ready with history on screen; ellipsis. |

## Risks and rollback

- **Actions shown on hover** are not visible on a touch screen until a row is
  selected. Selecting a row shows them.
- Rollback is a revert.
