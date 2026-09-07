<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-038 — Settings that stop lecturing

**Status:** Done
**Branch:** `feature/FEAT-079-avatars-and-quieter-settings`
**Screens:** Settings.

## What and why

The Settings screen explained itself at length. Every section had a subtitle
under its heading; most controls had a paragraph under them; several sections
closed with two or three more paragraphs arguing for decisions the reader had
not asked about. The author's report: it reads as though somebody is being made
to sit an exam to change a checkbox.

The prose was not padding — it was written carefully and most of it was true and
useful *once*. The problem is what a screen of it does: when everything is
explained, nothing stands out, and the controls — which are the point — are
outnumbered by sentences about the controls. A person opening Settings to change
one thing has to read past four paragraphs to find it.

## Scope and acceptance criteria

- No section carries a subtitle that only restates its heading.
- No control carries a paragraph that only restates its label.
- Detail that is genuinely useful moves to the control's `title`, where a person
  who wants it can ask and a person who does not is not made to read it.
- **Three kinds of text stay**, at full length where needed:
  1. anything that says what leaves the machine;
  2. anything a person cannot work out from the control — a keyboard shortcut,
     a token's required scopes, where a value in effect came from;
  3. anything a control says about itself being broken or not yet honoured.
- Labels are rewritten to stand alone rather than being propped up by the
  sentence that was under them.
- The tests that assert on this copy assert the *claim*, not the sentence.

## Non-scope

- Reorganising the sections, or which section a control lives in.
- Changing what any control does.
- The God mode section's structure. It is a developer surface with four
  sub-headings and they are load-bearing; its prose was trimmed, its shape kept.
- Writing help documentation to move the deleted text into. What was deleted was
  deleted because nobody needed it there, not because it needed a new home.

## What was removed, in one line each

| Section | Gone |
| --- | --- |
| Appearance | The subtitle; the paragraph about first-run defaults; two paragraphs under the sliders, replaced by a short note each — the Zoom one keeps the keyboard shortcut, which is the one thing a slider cannot tell you |
| Behaviour | The subtitle; the description under all three toggles, now the chip's `title`. Labels rewritten to carry their own meaning — "Prune deleted **remote** branches when fetching" says what the sentence under it used to |
| You | The subtitle; the closing paragraph about `git config` — the "In effect" line above each field already names the file — and a long sentence about what Clear does, which is now on the Clear button |
| Signing | The subtitle; two closing paragraphs; the "holds nothing, so the value above comes from somewhere else" clause, which the "In effect" line above it already answers |
| Updates | The subtitle; the privacy paragraph reduced to its two claims, both kept |
| Personality | Both subtitles shortened; the three level descriptions tightened |
| Accounts | The per-account "its token is in the keychain" line, now on the Disconnect button; four closing paragraphs merged into one that keeps every claim in them |
| External Tools | The introductory paragraph; the scope checkbox's label shortened to the path it writes |
| God mode | The subtitle and five explanatory paragraphs shortened; all four sub-headings kept |
