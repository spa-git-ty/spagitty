<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-098 — automated verification

2026-10-06, continuation of the existing CodeRabbit feature checkout.

- Passed: cargo test -p spagitty-farm --test supplemental — 12 tests on Windows and Linux.
- Real task worktrees cover missing required evidence at both merge entry points, current passes, provider removal, restart, policy edits/downgrades, changed code, bounded repairs, cancellation, duplicate starts, independent verification/review authority and preserving the user checkout.
- Passed: full Rust workspace tests on Linux, including the existing farm unit and pipeline suites.
- Passed: farm UI settings and explicit review-result tests; policy edits do not grant merge autonomy.
- Final suite/coverage/build results and remaining prerequisites are recorded in extensions-continuation-review.md.

Deterministic providers prove farm authority; they do not prove a live CodeRabbit account or an unattended paid repair cycle.
