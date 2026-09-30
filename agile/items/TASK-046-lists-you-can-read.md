<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-046 — Lists you can read

**Status:** Open — merged into `main` on 2026-09-30; the manual sweep is still owed.
**Branch:** `task/TASK-046-lists-you-can-read`
**Screens:** Working copy (1C), Stash (1G), Diff (1B) and the commit detail
panel — every list of changed files; the status strip.
**Raised by:** the author, in the running application: "this list looks so
crowded, file names appear from their end, not readable, rounded corners x and
+ … I don't feel engaged."

## Problem

Every list of changed files was drawn the same way, and each part of it worked
against reading:

- **Paths cut from their start.** The path column used `direction: rtl` so the
  ellipsis fell on the head, which made every long path begin mid-word:
  `…s/services/debit-card-pair-cache.service.ts`. A dotfile needed an invisible
  left-to-right mark just to stay `.gitignore`.
- **A box round every file.** Working copy drew each row as a bordered card,
  raised or flat, so fourteen changes were fourteen boxes.
- **Glyphs that say nothing.** `~` for modified and renamed, `?` for untracked,
  `+` and `−` — the same `+` that was also the stage button.
- **Permanent actions.** A `+` and a `✕` beside every unstaged file, all the time.
- **Dead controls.** The commit detail's "Merge into" and "Revert" chips were
  titled "Not built yet" and did nothing — the pattern BUG-030 took off the
  toolbar.
- **Stash**: each entry squeezed a chip and the whole message onto one line
  beside its drawing, so every row read "On en…"; the header repeated the count
  the Stash tab carries, where it read as the Reflog's.
- **"Loading history…" for ever.** The graph walks in windows and fetches more
  on scroll, so on a long history it is never complete, and the strip said it
  was loading for the whole session.

## Change

- **`FileName`**, one component for a changed file everywhere: the name first
  and whole, then its folder, quieter, which is what gives way when there is no
  room; and a lettered badge in its own colour — M, A, D, R, U, ! — the letters
  `git status --short` prints.
- **Rows are lines**, with a rounded highlight for hover and selection, in
  Working copy, the diff and stash file list, and the commit detail.
- **Actions on demand.** Stage, unstage and discard are round icon buttons —
  plus, minus, undo — shown while the row is hovered, focused or selected.
  Section headings are a name, a count in a capsule, and quiet text actions.
- **Commit actions that work.** Cherry-pick and Revert, both of which ask before
  they change anything, replace the two dead chips.
- **Stash entries in three lines**: what you wrote (without git's `On <branch>:`
  prefix), then which entry and when, then the commit it hangs off. The
  duplicate count goes.
- **Ready means ready**: the strip says so once there is history on screen, and
  a label that does not fit ends in an ellipsis instead of being cut.

## Non-scope

- The diff itself, the commit message box, and the widths columns open at.
- The Branches, Tags and Reflog tables.

## Acceptance criteria

- No file path anywhere starts mid-word; the name is always shown before its
  folder.
- A Working copy row shows its actions only while it is hovered, focused or
  selected, and has no border.
- Every chip in the commit detail's actions does something.
- The status strip says "Repository ready" once the first commits are shown.
