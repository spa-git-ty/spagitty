<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Using extensions

Everything is in **Settings › Extensions**.

## Installing

**Install from file…** reads a `.spagitty-extension` package and shows, before
anything is installed or run:

- its identity, version and publisher — *as the package states it*; nothing
  verifies who made it;
- the computers it has a program for;
- what it asks to be allowed to do;
- every file and its checksum;
- the trust model in one sentence.

A package is refused before anything is written if it is damaged, if a file
does not match its checksum, if it contains a link, a path that escapes its
folder, two names that differ only in case, a program it does not declare, or
if it cannot run on this computer or this version of Spagitty.

Installed packages live in Spagitty's data directory — on Windows
`%APPDATA%\dev.spagitty.app\extensions\packages`, on macOS
`~/Library/Application Support/dev.spagitty.app/extensions/packages`, on Linux
`~/.local/share/dev.spagitty.app/extensions/packages` — one folder per version.
Nothing is ever installed from inside a repository.

## Turning one on

Extensions are turned on **per repository**. Turning one on grants the
permissions it requires for this repository only; optional ones are off until
you turn them on. An extension that sends code to a service says where, and
asks you to agree, before it is turned on. Turning it on starts nothing and
sends nothing: the program starts the first time you use it.

## Using one

An extension's commands appear in the command palette (Ctrl/Cmd+P) and as
buttons on the screens they belong to — Commit, a farm task, a pull request.
A command that cannot run right now is greyed, and its tooltip says why.

A review provider's results appear in a findings panel: what was reviewed,
whether it completed, every finding by severity, where it points (never a
guessed line), and what you did with each — acknowledged, dismissed, sent to an
agent. **"Reviewed" is not "approved"**; a farm decides separately whether a
review passes its gate.

Review history is kept in the repository's `.spagitty/extensions/reviews/`
folder, which Spagitty keeps out of git. It holds the findings and what the
review covered — never your files' contents — with secrets removed. The newest
50 reviews per extension are kept; **Clear** deletes them.

## Updating, rolling back, removing

- **Update from file…** installs a newer version beside the old one and moves
  to it. Permissions the new version adds are not granted until you turn them
  on. Work in progress must finish or be cancelled first.
- **Roll back** returns to the version before the last update.
- **Remove** deletes the extension's files and permissions, and offers to
  delete its review history. Programs it used that you installed yourself are
  left alone.
- An official extension is updated with Spagitty and cannot be removed; turn it
  off instead.

## When something goes wrong

**Diagnostics** on each card shows its identity, program, recent tool runs,
its log and the end of its error output, with secrets removed. An extension
that stops unexpectedly fails what it was doing and is shown as Stopped; using
it again starts it again, and after three crashes in ten minutes it waits for
**Restart**. Nothing restarts on its own, so nothing is ever retried behind
your back.

## Additional farm reviews and PR requests

Farm settings hold the supplemental Off/Advisory/Required policy, blocking
severity and repair budget. Installing or enabling CodeRabbit grants neither
autonomy nor merge permission. Required policy blocks all farm merge paths when
the exact committed evidence or live provider cannot be validated. A policy
change is recorded as a person's choice, not an approval. Selected committed
task findings go back through that task's existing repair path.

The CodeRabbit PR panel uses the connected GitHub account and optional forge
permissions. Incremental and full requests preview the exact discussion comment
before posting. A delivered comment is a request; check/review revisions decide
what was observed. Partial data and unknown thread resolution stay visible.
Uncertain delivery requires a complete refresh before a deliberate resend.
