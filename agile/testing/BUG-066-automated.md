<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-066 — Automated test record

**Item:** [BUG-066](../items/BUG-066-extensions-refuse-spagitty-2.md)

| Suite | Cases |
| --- | --- |
| `crates/spagitty-extensions/tests/bundled_manifests.rs` | Each shipped manifest (CodeRabbit, its bundle copy when built, the example, the SDK template) admits this version of Spagitty and the extension API's version; the crate's version is the one in `package.json`. |

## Results — 2026-10-11

- `cargo test -p spagitty-extensions --test bundled_manifests`: 2 passed.
