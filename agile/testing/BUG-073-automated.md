<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-073 — Automated test record

**Item:** [BUG-073](../items/BUG-073-two-quick-assign-clicks-start-two-agents.md)

- `src-tauri/src/agents.rs`, `a_second_start_on_one_job_is_refused_while_the_first_is_set_up`: two claims on one pull request; the second is refused with the item's sentence, while another pull request, or the same number in another repository, is free.
- `src-tauri/src/agents.rs`, `a_start_that_stops_early_leaves_no_claim_behind`: a start that returns an error after claiming leaves the job free and the list empty.

Neither needs a network or an agent.

## Results — 2026-10-19

- `cargo test -p spagitty --lib agents` in WSL: 15 passed, one existing ignored test.
- `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check` in WSL: passed.
