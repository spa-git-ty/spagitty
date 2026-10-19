<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-151 — Search the text of files, by word or regular expression

**Status:** Done — on `release/3.4.0`; unmerged. Covered by Rust and component tests; not looked at in the running application (the author is checking it).
**Branch:** `release/3.4.0`.
**Screens:** 1U (Files).
**Raised by:** the author, 2026-10-19: "in files view i want global text search on all files for specific word or regex, also in one file search for word or regex".

## Change

- **Search in files.** The tree's field looks for *Names* or *Text*. With *Text*, what is typed is looked for in every text file at the revision Files is showing — any branch, tag or commit, read from git's objects like the rest of Files, never the working copy. The lines found are listed under their files with the match marked; a click opens the file at that line, with the match marked and the search carried on by `n` and `N`.
- **Search in a file.** `/` and `?` keep their place in the status line, and gain the same three options.
- **Three options, shared by both:** *Regular expression* (`.*`, Alt+R), *Whole word* (`W`, Alt+W) and *Match case* (`Aa`, Alt+C). Without *Match case*, case counts only when what is typed has a capital, as `/` always did; an escape such as `\S` is not a capital. A regular expression that does not parse says why and finds nothing.
- Binary files, Git LFS pointers and files over 2 MB are not searched. Up to 2,000 lines are listed, the first match on each; past that it says there are more.

## Acceptance criteria

- Typing in *Text* lists every matching line of every text file at the revision, by file, with the match marked.
- The three options change both searches, and turning one re-runs a search on screen.
- Opening a found line puts the cursor on its match; `n` goes on to the next one.
- A bad regular expression is said, not searched for.
- Changing the revision searches the new one.
- Nothing reads the working copy.
