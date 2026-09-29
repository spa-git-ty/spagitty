<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-033 — Plan

**Item:** [`agile/items/BUG-033-a-crlf-file-diffs-as-a-rewrite.md`](../items/BUG-033-a-crlf-file-diffs-as-a-rewrite.md)

## Approach

`worktree_bytes` in `diff.rs` is the only place the working side of an
unstaged diff is read — the text diff, the hunk patch and the binary diff all
call it. It now passes what it read through `to_git`, which asks gix for the
repository's filter pipeline and runs `convert_to_git`: the same attributes,
`core.autocrlf` and clean filters git applies before it diffs. An unchanged
outcome keeps the original buffer; a converted one replaces it.

Hunk staging applies the patch to the index with `git apply --cached`, and
discarding applies it in reverse to the working file with `git apply`, which
reads the file through the same conversion and writes the result back through
its inverse. With both sides in stored form, the patch matches what each
expects.

## Alternatives considered

- **Stripping `\r` before diffing.** It fixes the common case and is wrong for
  a file that is *meant* to be CRLF in the repository, and for every other
  filter.
- **Reading the working file with `git diff`.** A subprocess per file shown,
  for a conversion gix already provides.

## Files

| File | Change |
| --- | --- |
| `crates/spagitty-core/src/diff.rs` | `worktree_bytes` converts; `to_git`. |
| `crates/spagitty-core/src/work.rs` | Three tests on a CRLF checkout. |

## Risks and rollback

- **One filter pipeline per file read.** It builds an attribute stack, which is
  cheap next to the diff itself.
- **A clean filter that is slow or fails** now runs when the diff is shown, as
  it does for `git diff`. A failure is reported as a diff error.
- Rollback is a revert.
