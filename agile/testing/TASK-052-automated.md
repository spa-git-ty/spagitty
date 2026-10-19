<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-052 — Automated test record

**Item:** [TASK-052](../items/TASK-052-nothing-waits-in-line.md)

## Step 2 — 2026-10-19

In `src-tauri/src/commands.rs`:

- `a_quick_read_does_not_wait_for_a_slow_one`: a read started while another holds its turn for 150 ms gets its own in under 100 ms. Step 1's test asserted the opposite and is replaced by this one.
- `two_writes_never_overlap_and_reads_wait_for_neither`: a write waits for the write in flight; a read beside a write, and a write beside a read, do not wait.
- `the_timings_say_how_long_a_command_held_the_repository`: the Timings panel still gets the hold.
- `a_read_that_outlives_its_repository_is_not_delivered`: a read in flight when a second repository opens answers `Replaced`; the next read reads the second.
- `every_command_is_read_or_write_on_purpose`: every command name given to `with_session`, `with_live_session` or `files_repo` is in exactly one of `READERS` and `WRITERS`, and neither list names a command that does not exist.
- TASK-048's tests of overlapping opens and closes pass unchanged.

The waits are timed by the tests themselves rather than read from the shared timing buffer, which other tests running beside them fill.

### Results

- `cargo test -p spagitty --lib` in WSL, four runs in a row: 197 passed, one existing ignored test.
- `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --all --check` in WSL: passed.
- Not measured in the application: the baseline in [TASK-052-baseline.md](TASK-052-baseline.md) is the author's to take.
