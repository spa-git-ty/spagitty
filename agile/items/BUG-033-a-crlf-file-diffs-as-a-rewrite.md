<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-033 — A CRLF file diffs as a rewrite

**Status:** Fixed.
**Branch:** `bugfix/BUG-033-a-crlf-file-diffs-as-a-rewrite`
**Screens:** Working copy (1C), and the hunk staging and discarding it offers.
**Raised by:** the author, on Windows: "in working directory on diff instead of
showing diff lines only it shows that the whole old file is deleted and
rewritten! that's a core feature bug."

## Problem

Change one line of a file on Windows and the Working copy screen shows every
line of it removed and every line added again.

The unstaged diff compares the index blob with the file on disk, and it read
the file on disk raw. Git for Windows checks files out with `core.autocrlf`
set, so the working file ends every line with `\r\n` while its blob, which is
what `git add` wrote, ends them with `\n`. Every line differed by its `\r`.
Git itself converts the working file to its stored form before diffing it;
Spagitty did not. `crlf_terminators_are_not_shown` hid the `\r` when a line
is drawn, which is why nothing looked wrong except the size of the diff.

The same diff is what hunk staging and discarding are built from, so a hunk
staged from a CRLF file was the whole file.

## Reproduction

With `core.autocrlf=true`, commit a 40-line file, change lines 2 and 38 on
disk: one hunk of 40 removals and 40 additions, instead of two hunks of one
line each. `a_crlf_file_diffs_only_the_lines_that_changed` failed with
`left: 1, right: 2`.

## Scope

- Read the working side of every unstaged diff through the repository's
  to-git conversion: `core.autocrlf`, `.gitattributes` `text`/`eol`, and clean
  filters.

## Non-scope

- The 10 `spagitty-core` tests that fail on Windows because the fixture
  repositories inherit Git for Windows' system configuration through gix.
  Their own item.

## Acceptance criteria

- A CRLF working file with two changed lines diffs as two one-line hunks.
- Staging one of them stages that line only, with LF in the index.
- Discarding one of them reverts that line only, and the file keeps its CRLF.
