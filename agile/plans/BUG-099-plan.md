<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-099 — Plan

**Item:** [BUG-099](../items/BUG-099-the-hooks-side-panel-cards-do-nothing.md)

1. `src/lib/hooks/HooksScreen.svelte`: `addBlock` imports first when read-only; cards disabled only while saving or at the cap, with a disabled style.
2. `src/lib/hooks/HookRuns.svelte`: a manager's hooks drawn as plain lines (`.step.legacy`).
