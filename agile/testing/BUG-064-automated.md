<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-064 — Automated test record

**Item:** [BUG-064](../items/BUG-064-review-says-nobody-asked-you-after-you-answered.md)

| Suite | Cases |
| --- | --- |
| `crates/spagitty-core/src/forge/github.rs` | Your latest review is read with its commit, another reviewer's is not yours; pending and dismissed reviews are not counted; a comment review is; nobody signed in has none; the query asks for `latestReviews`. |
| `src/lib/review/inbox.test.ts` | Reviewed pull requests go to *Reviewed by you · you left a review*, leaving *nobody asked you yet* to the rest; pushed-past ones come first; a requested review or one with replies stays where it was; the labels name the verdict and add *· changed since* only when the head moved. |
| `src/routes/review/page.test.ts` | A reviewed pull request shows under *Reviewed by you*, the page never says *nobody asked you yet* about it, and its card says *you asked for changes · changed since*. |

The new frontend cases fail without the fix.

## Results — 2026-10-08

- `bun run coverage`: 3,555 passed across 170 files; 82.01 % statements, 71.91 % branches, 79.43 % functions, 84.76 % lines.
- `bun run check`: zero errors, zero warnings.
- `cargo fmt --check`, `cargo clippy --workspace --all-targets -D warnings`: clean.
- `cargo test --workspace`: 1,367 passed.
