<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-050 — The chatter goes

**Status:** Open — merged into `main` on 2026-09-30; the rest of the manual sweep is still owed.
**Branch:** `task/TASK-050-the-chatter-goes`
**Screens:** Settings (1K) above all; Rebase (1E), Tags (1N), Reflog (1M),
All repositories (1J), Conflicts (1D), Log (1I), File history (1O), Diff (1B),
Stash (1G), Farm (1Q), Pull requests (1H), the command log, the clone dialog,
the commit detail.
**Raised by:** the author, 2026-09-30: an earlier agent "was so chatty … and
added a lot of labels on settings that are not needed; recheck them and remove
the non-needed labels, also sweep the app for such chatter."

## Problem

TASK-038 and TASK-044 cut Settings' paragraphs, and the screens gathered prose
again: a label in front of every chip row, a sentence under every choice, an
"In effect" line under every field repeating the field, footers explaining a
screen to someone already using it, keyboard hints, and empty states that
taught what a stash or a reflog is. Some of it was wrong: an unset
`commit.gpgsign` borrowed the identity's "Git refuses to commit without it".

## The rule

A line stays if it states a value or a state the controls do not already show,
warns about a consequence, or reports an error. It goes if it explains what a
control is, what a screen is for, what the application does not do, or a
shortcut. Anything a person may still want to know moves to the control's
title.

## Change

**Settings**

- *You:* no "Editing" label; the repository chip is disabled with no
  repository open instead of a note saying so; "In effect" only when the value
  comes from a scope other than the one being edited.
- *Signing:* "GPG · key" beside the switch; "In effect" only when the value
  comes from elsewhere; no key sentence; the scope's value and Clear only when
  there is one. The signer names are GPG, SSH and S/MIME; the two problems are
  one short sentence each.
- *Identity profiles:* no description, no empty-state instruction, no form
  title.
- *Accounts:* no "Connected." or "No account is connected."; no privacy line.
  The token scopes stay.
- *Appearance:* no Mode, Desktop or Theme labels, no "Theme: name" row, no
  density sentence, no "Type and row height.", the zoom shortcut in the
  slider's title.
- *External tools:* "Diff tool" and "Merge tool", no current value repeated
  beside the picker, "Built-in", "(not installed)" and no "(detected)", no
  catalogue of `$PATH`.
- *Personality and Sound:* no header notes, no sentence under each level (the
  title keeps it); the chips sit in one row.
- *Updates:* no "No account, no identifier." (the title keeps it), no "Not
  checked yet.".
- *About:* no sub-heading, no licence sentence under the licence row, no "What
  is linked into this binary.". The trademark notice stays.
- *Remotes:* "No repository is open."

**The rest of the application**

- Footers gone: Rebase's "May conflict …", Tags' "Newest first …", Reflog's
  "Git expires …", All repositories' "Repositories are read …", the command
  log's "Reads … are answered in-process".
- Explanations gone: Rebase's intro, Conflicts' "When git cannot merge …",
  Log's "Search by author …", Blame's "Name a file …", File history's
  description, Stash's "A stash puts your uncommitted work aside …", the Farm
  starter's lede, its step sentences and "Nothing runs until you say so.", and
  six explanations beside the Farm's own settings.
- Hints gone: Diff's and Stash's keyboard lines.
- Shorter: "Nothing to commit.", "No repositories yet.", "Nothing has run
  yet.", "Not here any more.", "Signed, not verified here.", "No tasks yet.";
  the clone dialog's password paragraph and two paragraphs on Pull requests'
  empty states gone.

## Non-scope

- The delight layer's Badges page and God mode, which are off by default.
- Dialogs whose sentences state a consequence (merge methods, discard, amend).

## Acceptance criteria

- Settings shows no sentence that only explains a control, and no label a chip
  or a swatch already makes redundant.
- No screen carries an explanatory footer or a keyboard hint.
- Everything removed that still carries information is in a title.
