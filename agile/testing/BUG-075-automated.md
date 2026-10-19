<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-075 — Automated test record

**Item:** [BUG-075](../items/BUG-075-a-reply-that-cannot-be-read-is-taken-as-empty.md)

- `crates/spagitty-core/src/models/http.rs`, `a_reply_cut_off_part_way_is_an_error_not_an_empty_answer`: a reader that gives part of a JSON body and then fails with a reset.
- `a_reply_over_the_limit_is_an_error_not_an_empty_answer`: eleven bytes against a ten-byte limit are refused; ten are read.
- `a_reply_that_is_not_text_is_an_error_not_an_empty_answer`: bytes that are not UTF-8.

The error-status path (an unreadable body still reports its status) is a branch in `finish` that needs a live response; it is covered by the sweep.

## Results — 2026-10-19

- `cargo test -p spagitty-core --lib models::http` on Windows: 6 passed.
- `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check` in WSL: passed.
