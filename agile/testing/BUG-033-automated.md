<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-033 — Automated test record

**Item:** [`agile/items/BUG-033-a-crlf-file-diffs-as-a-rewrite.md`](../items/BUG-033-a-crlf-file-diffs-as-a-rewrite.md)

## What was tested

Three tests in `crates/spagitty-core/src/work.rs`, on a repository with
`core.autocrlf=true` whose 40-line file has CRLF on disk and two changed lines:

| Test | Asserts |
| --- | --- |
| `a_crlf_file_diffs_only_the_lines_that_changed` | Two hunks, and the only changed lines are the two that changed. |
| `staging_one_hunk_of_a_crlf_file_stages_only_that_hunk` | The index gains that line only, with no `\r`. |
| `discarding_one_hunk_of_a_crlf_file_keeps_its_endings` | That line is reverted, the other is kept, every line keeps its CRLF. |

The first two were written before the fix and failed against it: one hunk
where two were expected.

## Test command and output

On Windows 11, natively:

```
$ cargo test -p spagitty-core --lib crlf
test diff::tests::crlf_terminators_are_not_shown ... ok
test work::tests::a_crlf_file_diffs_only_the_lines_that_changed ... ok
test work::tests::discarding_one_hunk_of_a_crlf_file_keeps_its_endings ... ok
test work::tests::staging_one_hunk_of_a_crlf_file_stages_only_that_hunk ... ok
```

The whole of `spagitty-core` on Windows: 527 passed, 10 failed — the same
fixture-configuration failures as before the change, none new.

The pipeline's gates, on Linux (WSL, Arch):

```
$ cargo fmt --all --check
(clean)

$ cargo clippy --workspace --all-targets -- -D warnings
(clean)

$ cargo test --workspace --no-fail-fast
spagitty-core: 537 passed; 0 failed
all crates: 1017 passed; 0 failed
```

## What is not covered automatically

A clean filter configured in a real repository (LFS, for instance). The
conversion is gix's and is exercised by its own suite; the sweep covers the
common Windows case by hand.
