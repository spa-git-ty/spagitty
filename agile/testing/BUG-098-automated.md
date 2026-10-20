<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-098 — Automated test record

**Item:** [BUG-098](../items/BUG-098-pull-requests-says-only-500-internal-server-error.md)

- `crates/spagitty-core/src/forge/gitlab.rs`: a list answering 500, whose simple view lists three and whose second merge request still answers 500, comes back as the first and third, in order; a list whose simple view fails too is the server's error, saying so; a 403 is asked once; a whole list is read as it comes.

## Results — 2026-10-20

- `cargo test -p spagitty-core --lib forge`: 164 passed.
