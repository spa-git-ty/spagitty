<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-070 — Automated test record

**Item:** [BUG-070](../items/BUG-070-local-agents-do-not-get-their-instructions.md)

| Suite | Cases |
| --- | --- |
| `crates/spagitty-farm/src/agent/adapter.rs` | Only a Windows `.cmd`/`.bat` agent given a many-line prompt needs stdin; an executable, another platform or a one-line prompt does not. |
| `crates/spagitty-farm/src/agent/adapters/{codex,claude,pi}.rs` | A `.cmd` agent on Windows gets the prompt on stdin (Codex with `-`, Claude Code with no argument, Oh My Pi in stdin mode); elsewhere the prompt stays an argument. |

Pull request #59's own cases (BUG-065) run with these.

## Results — 2026-10-11

- `cargo test -p spagitty-farm adapters`: 30 passed.
- `bunx vitest run src/lib/agents src/lib/review`: 147 passed. `bun run check`: zero errors, zero warnings.
- `cargo clippy --workspace --all-targets -D warnings`: clean. The `execution::process` tests that use `/bin/sh` fail on Windows as they do on `main`.
