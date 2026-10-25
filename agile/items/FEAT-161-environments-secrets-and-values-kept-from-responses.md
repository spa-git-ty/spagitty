<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-161 — Environments, secrets and values kept from responses

**Status:** Backlog — after FEAT-160.
**Screens:** 2C (Calls › Environments, at `/calls/environments`; the keep card and the variables panel on Calls).
**Raised by:** the author, 2026-10-25: environments that are easy to manage, values from responses filled in automatically, and an easy way to put any response value into a variable.
**Proposal:** [`docs/proposals/calls.md`](../../docs/proposals/calls.md) §6. **Designs:** [`design_handoff_calls/`](../../design_handoff_calls/) — `Environments.dc.html`, `Main.dc.html`.

## Change

Environments are named sets of variables with a colour. Plain values are in the repository. Secrets and kept values are in the OS keychain. Clicking a value in a response keeps it as a variable, once or on every call.

## Acceptance criteria

- **Environments screen** (§6.1):
  - The list shows a colour dot, host and *in use*, with *+ New environment*.
  - One environment shows its name, a colour from five, *Ask before sending* and *Duplicate*.
  - A variables table has name, value, kind and kept in, plus *+ Add variable*. Shared variables follow below.
  - The footer names the repository file and the keychain.
- **Switching**: the header's picker, and `Ctrl+E` anywhere in Calls. The active environment is per person, per repository.
- **Storage**:
  - `.spagitty/calls/env/<name>.json` and `shared.json` hold plain values, and only the names of secrets and kept values.
  - Secret and kept values go to the keychain through the `forge/keychain.rs` seam, keyed by repository, environment and name.
  - A test writes every kind of variable through every writer and asserts that no secret or kept value appears in any file.
- **Keeping** (§6.2):
  - Clicking a value in a formatted JSON or XML response opens the keep card inline under its line. The card has: the path (JSONPath or XPath), *as* with a suggested name, *in*, *Update on every call*, *Cancel* and *Keep*.
  - The kept value is underlined in `--ok` with `→ name`, and is in the variables panel at once.
  - Clicking it again shows *Kept as …* and *Forget*.
- **Rules** are stored on the call (`keep: [{ var, from, every }]`). After each send, *every call* rules update. A path that finds nothing marks the variable stale and keeps the old value.
- **Kept from responses** (Environments) lists each rule with a switch. Off freezes the value.
- **Production**: an environment named `production`, or coloured red, is created with *Ask before sending* on.
- **Tests**:
  - Rust tests for JSONPath and XPath keeping, the name suggestions, stale values, and keychain round trips.
  - Component tests for the keep card and the environments screen.

## Not in this item

- Sharing environments (FEAT-164).
- Scripts that set variables.
