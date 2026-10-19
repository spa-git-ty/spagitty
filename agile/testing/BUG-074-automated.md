<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-074 — Automated test record

**Item:** [BUG-074](../items/BUG-074-a-remote-agents-token-limit-is-read-only-between-steps.md)

- `crates/spagitty-farm/src/assign/remote.rs`
  - `a_step_stops_after_the_exchange_that_crosses_its_token_limit`: a scripted provider answers each exchange with 30,000 tokens, a finding and another read; with 50,000 left the step makes two exchanges, spends 60,000, says it stopped at its limit and keeps both findings.
  - `no_token_limit_still_means_none`: six exchanges of a million tokens each run to `finish_step` with no limit.
  - `the_time_limit_ends_a_step_waiting_on_a_provider`: a wait on a provider that never answers returns at a 50 ms deadline, returns at once when stopped, and a step whose time is already up asks nothing.
- `crates/spagitty-farm/src/assign/tests.rs`
  - `each_step_is_handed_what_is_left_of_the_limit`: with 1,000 tokens and steps of 150, the first two steps are handed 1,000 and 850.
  - `a_step_cut_off_at_its_limit_with_nothing_to_show_stops_rather_than_fails`: the run is *Stopped* with *Reached its limit of 100 tokens. Everything it made is kept.*
  - The existing `a_token_limit_stops_at_the_end_of_the_step` still passes.

## Results — 2026-10-19

- `cargo test -p spagitty-farm --lib` in WSL: 474 passed.
- `cargo test -p spagitty --lib` in WSL: 193 passed, one existing ignored test.
- `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check` in WSL: passed.
