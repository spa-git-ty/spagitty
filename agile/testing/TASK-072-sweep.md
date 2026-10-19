<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-072 — Sweep

**Item:** [TASK-072](../items/TASK-072-the-declared-minimum-rust-is-older-than-a-dependency-needs.md)

| Ticket | Preconditions | Steps | Expected result | Result |
| --- | --- | --- | --- | --- |
| SWEEP-TASK072-01 | A pull request with this change | Let gate 2 run | *No dependency needs a newer Rust than Cargo.toml says* passes | Not run. Needs the workflow on GitHub. |
| SWEEP-TASK072-02 | A Rust 1.90 toolchain (`rustup toolchain install 1.90`) | `cargo +1.90 build --workspace` | It builds | Not run |
