<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-066 — Plan

**Item:** [TASK-066](../items/TASK-066-style-notes-from-the-2-0-audit.md)

Each note is taken in turn and either done or struck in the item, with the reason.

1. `agents.rs` split: struck. A move with no behaviour in it, and a large diff for every branch in flight.
2. `design_handoff_agents/README.md`: already done; it points at FEAT-115.
3. `plain()` in `watch.rs`: keep the `\?\` prefix unless what is left is absolute; test a volume and a device path.
4. `tools/release-notes.mjs`: the `pathToFileURL` guard `attribution.mjs` uses.
5. The CodeRabbit thread on resuming: read the resume path end to end. It is real, and becomes BUG-094.
6. Comment density: struck, as the note says.
