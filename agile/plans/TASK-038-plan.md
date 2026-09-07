<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-038 — Plan

**Item:** [`agile/items/TASK-038-settings-that-stop-lecturing.md`](../items/TASK-038-settings-that-stop-lecturing.md)

## Approach

The hard part of deleting prose is deleting the wrong prose. Some of what is on
this screen is the only place a claim is made, and a trim that removed it would
be a regression disguised as a tidy-up.

So each paragraph was put to one question — **what does a reader lose if this is
gone?** — with three answers that keep it and one that does not:

- *They would not know what leaves the machine.* Keep, in full.
- *They could not work it out from the control.* Keep, shortened, or move to the
  control's `title` if it is detail rather than instruction.
- *The control is lying about working.* Keep.
- *They would find out by using the control.* Delete.

The fourth is most of it.

## Decisions

- **`title` rather than a help affordance.** The rest of the screen already
  answers this kind of question with a `title` — the Clear buttons, the disabled
  scope chip, the signed marker on a commit row. A disclosure triangle would be
  a new pattern, and one that still has to be opened.
- **A label that needed a paragraph is a label that was not finished.** So
  "Prune deleted branches when fetching" became "Prune deleted **remote**
  branches when fetching", and the reassurance underneath it — that local
  branches are untouched — went to the hover. The word does the sentence's job.
- **The Accounts privacy text is merged, not cut.** Four paragraphs there made
  five separate claims, and this screen is where a reader comes to ask what the
  application sends. Every claim survives; what went is the saying of each one
  twice. It is one paragraph of clauses now.
- **"In effect" lines are the replacement for prose about configuration.** The
  Signing and You sections both closed by explaining that they write through
  `git config`. The line above each field already names the file the live value
  came from, which is the same fact as a value rather than as an argument.
- **Every deletion is marked by a comment where it was.** Deleting a paragraph
  and leaving nothing invites somebody to write it again. The comments say what
  stood there and which of the three tests it failed.
- **The tests were rewritten to assert claims.** Five tests failed on the exact
  wording, which is the correct failure — copy changed. Each was updated to
  assert the thing it cares about: `Not honoured yet` rather than the old
  sentence, `This repository holds nothing` rather than a clause about where the
  value came from instead. A test that pins a sentence makes the sentence
  unimprovable.

## Alternatives rejected

- **Collapse the prose behind a "learn more" toggle.** Still on screen, still
  ordered above the controls, now with a control of its own.
- **Move it into `docs/`.** Nobody reads a document about a checkbox. The text
  that was worth keeping is worth keeping *at* the checkbox.
- **Delete the `what` strings entirely.** They are real detail, they are one
  hover away, and they cost nothing when nobody hovers.

## Files

Nine section components, plus the three test files that pinned their wording:
`AppearanceSection`, `BehaviourSection`, `IdentitySection`, `SigningSection`,
`UpdateSection`, `PersonalitySection`, `AccountsSection`,
`ExternalToolsSection`, `GodModeSection`; `sections.test.ts`,
`SigningSection.test.ts`.
