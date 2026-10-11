<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-066 — Sweep

**Item:** [BUG-066](../items/BUG-066-extensions-refuse-spagitty-2.md)

| Ticket | Preconditions | Steps | Expected result | Priority | Result |
| --- | --- | --- | --- | --- | --- |
| SWEEP-BUG066-01 | Spagitty 2.x with the bundled CodeRabbit extension | Settings › Extensions | CodeRabbit shows no *It needs Spagitty* reason and can be turned on | P1 | Not run. |
| SWEEP-BUG066-02 | A new extension from `bun run ext new` | Validate and install it | It is not marked incompatible | P2 | Not run. |
