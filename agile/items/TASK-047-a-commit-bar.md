<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-047 — A commit bar

**Status:** Open — merged into `main` on 2026-09-30; the manual sweep is still owed.
**Branch:** `task/TASK-047-a-commit-bar`
**Screens:** Working copy (1C).
**Raised by:** the author: "on the working directory page, can the commit
message — or better, hide it and have it appear when I press add description,
in a small place at the bottom — instead of it taking half the screen from the
top like that?"

## Problem

The commit message was a well across the top of the diff column: a subject
line, a rule, a three-line body, the amend chip and its note, capped at 40% of
the column. It was there on every visit, whether or not anybody was typing, so
the hunks being committed started halfway down. The Commit button was in a
separate strip along the bottom — the message and the action that uses it at
opposite ends of the screen.

## Change

- **One bar along the bottom**: the summary field, "Add description", the
  amend chip, and the Commit button, in that order.
- **The description is asked for.** A text area opens under the summary when "Add
  description" is pressed, takes the focus, and stays open while it holds
  anything; a long one scrolls inside itself.
- **Notes only when they apply**: signing, and what amending does, as one quiet
  line above the bar.
- **No bar when there is nothing to commit.** A clean working copy said so and
  also offered a disabled Commit button; it now only says so.
- The diff has the whole column.

## Non-scope

- What committing does, the subject's 50-character hint, signing's checks.

## Acceptance criteria

- With changes, the Working copy screen shows one line along its bottom — the
  summary, "Add description", amend and Commit — and the diff fills the rest.
- "Add description" opens the body with the focus in it; a body that holds text
  is shown without asking.
- A clean working copy shows no commit bar.
