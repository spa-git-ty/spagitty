<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-074 — A remote agent's token limit is read only between steps

**Status:** Fixed — on `release/3.4.0`; unmerged. Covered by unit tests with a scripted provider; not tried against a real provider.
**Screens:** 1K, 1R, 1S.
**Raised by:** CodeRabbit on #52; confirmed in the code on 2026-10-12.

## What happens

The limits an API agent is given (40,000 tokens a run and 400,000 a day by default; minutes, when set) are compared with what was spent by `over_limit` in the engine, which runs in `checkpoint`, between steps. Inside one step `RemoteDriver::run` may make up to 40 exchanges, and each one sends the whole conversation again, so what a step costs grows faster than the number of exchanges. A step on a large file can spend several times the per-run limit before anything looks at it, on a key the person pays for. The limit then stops the *next* step and says it was reached.

## Acceptance criteria

- A remote step stops when its exchanges have spent what is left of the run's token limit (and the day's), not only after the step ends. What it had gathered is kept and the sentence says so.
- A limit of *none* still means none.
- The time limit also ends a step that is waiting on a provider.
- A unit test with a scripted provider that answers with a large usage figure shows the step stopping after the exchange that crossed the line.

## For the agent who picks this up

**Who:** unassigned. Any agent working on this repository can start cold.

- **Where:** `crates/spagitty-farm/src/assign/remote.rs` (`Driver::run`), `crates/spagitty-farm/src/assign/engine.rs` (`over_limit`, `Limits`, the `Driver` trait).
- **Shape:** hand the driver what is left (tokens and a deadline) with the `Ask`, and check it after each exchange.
