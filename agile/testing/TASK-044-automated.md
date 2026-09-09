<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-044 — Automated tests

**Item:** [`agile/items/TASK-044-settings-stops-explaining-itself.md`](../items/TASK-044-settings-stops-explaining-itself.md)

## What was tested

No new assertions. Four existing ones were rewritten, and the rewrite is the
whole of the interest here.

| Test | Was | Is |
| --- | --- | --- |
| says what leaves the machine, rather than that nothing does | `mounted.text()` contains `uploads none of them` | `readable(mounted)` contains `uploads no repository` |
| says what leaves the machine, beside the switch that stops it | text contains `no account, no identifier` | readable contains `No account, no identifier` |
| says a build compiled here is not out of date | text contains `development build` | readable contains `Development build` |
| says a repository has none, and still offers the form | text contains `has no remotes` | text contains `No remotes` |

`readable()` joins the rendered text with every `title` attribute in the mount.

## Why that is not weakening the test

The contract these assertions were written for was never "these words are on the
screen". It was **"a reader who asks can find this out"** — that is what makes
them worth having on a privacy claim. A claim moved into a source comment would
fail that; a claim moved into a hover passes it, because a hover is something a
reader can reach.

The alternative — asserting the phrase appears anywhere in the component's
source — would pass on a comment, and would have made the test meaningless. The
narrow version reads exactly the set a person can get to.

## The whole suite

131 files, 2,821 tests, all passing. `bun run check`: 0 errors, 0 warnings.

## What is not covered

Whether the screen now reads as calm rather than terse. That is the judgement
the author asked for and it is `SWEEP-001`.
