<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-071 — Automated test record

**Item:** [BUG-071](../items/BUG-071-a-gemini-agent-gets-bad-request.md)

| Suite | Cases |
| --- | --- |
| `crates/spagitty-core/src/models.rs` | A signed call is sent back with its signature beside `functionCall`, never inside it; with parallel calls only the signed one carries a signature and made-up ids are not sent; a tool with no properties is declared with no `parameters` and an empty `required` is dropped; an empty assistant turn is not sent; a reply with no parts reads as an empty turn that stopped on length; an error wrapped in a list still gives Google's sentence. |
| `crates/spagitty-farm/src/assign/remote.rs` | A `default_api:`-prefixed tool name is answered and narrated as the tool it names. |

## Results — 2026-10-11

- `cargo test -p spagitty-core models`: 15 passed.
- `cargo test -p spagitty-farm remote`: 5 passed.
- `cargo fmt --all --check`, `cargo clippy -p spagitty-core -p spagitty-farm --all-targets -D warnings`: clean.
