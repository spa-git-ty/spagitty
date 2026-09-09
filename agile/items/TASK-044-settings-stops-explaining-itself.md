<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-044 — Settings stops explaining itself

**Status:** Done.
**Screen:** Settings (1K).
**Raised by:** the author, reading the screen: "remove the chatty chatter — we
are not idiots as humans."

## Problem

TASK-038 took out the section subtitles and the paragraphs under individual
controls, and kept "everything that says what leaves the machine". That rule was
right and the result still reads as a lecture, because the kept paragraphs are
long. The one the author quoted is five sentences under a token field:

> Spagitty reads your repositories from disk and uploads none of them. A
> connected account adds one request: to the host you named, with the token you
> issued, for pull requests you can already see in a browser. It reads — it
> never approves, merges or comments. The token is in this machine's keychain,
> never in a file; disconnecting deletes it. The only other request is the
> update check under Behaviour, which can be turned off.

Every clause of that is true and load-bearing. None of it needs to be *on the
screen*. A person connecting an account wants the scopes; a person who wants
the privacy argument is asking a question, and a question has an answer rather
than a paragraph in the way.

## Change

The claims are kept and moved to `title` attributes; the visible line is what
somebody is scanning for.

| Where | Was | Is |
| --- | --- | --- |
| Accounts, scopes | 3 lines of prose | `Read-only token. Fine-grained: … Classic: repo.` |
| Accounts, privacy | 5 sentences | `Read-only, to the host you named. Token in the keychain; disconnecting deletes it.` + hover |
| License, GPL | 3 sentences | one line + hover for why the commit matters |
| License, generated list | 3 lines | `What is linked into this binary.` + hover |
| License, undeclared | 2 lines | `{n} declare no license.` + hover |
| Updates, privacy | 2 sentences | `No account, no identifier.` + hover |
| Updates, development build | 2 lines | `Development build. Latest release: …` |
| Remotes, empty | `This repository has no remotes. Nothing can be fetched or pushed until it has one.` | `No remotes.` |
| Remotes, add | `Adding a remote writes configuration and fetches nothing…` | gone |
| God mode | two long lines | shortened; one to a hover |

## What was left alone

**The one-line notes that say what a control does and cannot show.** "Not
honoured yet: …", "Selecting a level plays it.", "Earns nothing; ignores
Personality.", "Straight into the record, no rule consulted." Each is under a
dozen words and each says something the control itself cannot demonstrate. That
is the line TASK-038 drew and it is still the right one; what this task fixes is
length, not the existence of a note.

**Every state message.** "Reading…", "Not checked yet", "No repository is open".
Those are the screen answering a question, not explaining itself.

## The tests changed too, and why that is not cheating

Four assertions read the rendered text for phrases that are now in a `title`.
The contract they were written for was never "these words are on the screen" —
it was "a reader who asks can find this out", and a claim only in a source
comment would fail that while a claim in a hover passes. `sections.test.ts` has
a `readable()` helper that reads the text **and** every title, so what leaves
the machine is still asserted; what changed is the volume on the screen.

## Acceptance criteria

- No visible paragraph in Settings runs to more than one sentence, except the
  God mode notes that describe a debug control's effect.
- Every privacy claim is still reachable — text or hover — and still asserted.

## Dependencies

TASK-038 did the first pass and set the rule this narrows.
