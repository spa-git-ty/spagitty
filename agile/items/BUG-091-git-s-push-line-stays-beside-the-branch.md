<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-091 — git's push line stays beside the branch, hashes and all

**Status:** Fixed — on `release/3.3.0`; unmerged. Covered by unit and store tests; not looked at in the running application (the author is checking it).
**Screens:** chrome (toolbar)
**Raised by:** the author, 2026-10-19, on 3.2.0, with a screenshot of the toolbar reading `75b242b47..880b35657 sit -> origin…` beside the branch: "why everytime i do action i have this hash and action i take".

## What happens

After a fetch or a push, the toolbar shows git's last line beside the branch
name: two abbreviated hashes, the branch and an arrow. It stays there until the
next fetch or push replaces it, so it looks like something is still going on.

## Cause

- FEAT-018 kept git's last line (`summary` in `network/store.svelte.ts`) and
  the toolbar showed it as it came, under *How the last fetch or push ended*.
  The same line was also shown in a notice, so it was said twice, and the
  toolbar copy never went away.
- The line is git's ref-update table row. It says which branch moved, but in
  git's own notation.

## Acceptance criteria

- A fetch or push that worked says so in a notice, in words: *Pushed
  chore/fix-sonar-scan*, *Fetched: origin/main moved on*, *Pushed new branch x*,
  *Already up to date*. Nothing stays beside the branch.
- A fetch or push that failed still says why beside the branch, in git's words,
  until the next one.
- A line that is not a ref update is passed on as git wrote it.

## Fix

- `network/plain.ts`: `plainSummary` turns the ref-update row into words (new
  branch, new tag, deleted, forced, refused, up to date).
- `network/store.svelte.ts` keeps the plain summary and puts it in the notice.
- `chrome/Toolbar.svelte` shows only the error.
- Tests: `network/plain.test.ts`; `network/store.test.ts` reads the plain words.
