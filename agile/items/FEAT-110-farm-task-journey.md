<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-110 — Farm task journey

**Status:** Open — built and swept against the handoff in a browser fixture; not yet run in the native app; unmerged.
**Branch:** `codex/farm-journey` (integration branch for the six handoff slices).
**Screens:** 1Q.
**Raised by:** the author, 2026-10-07, from the Farm design handoff.

## Change

Task deep links, full stepper, Output, Changes, Checks, Review and Hand-off tabs; preserve editing and extension actions.

## Acceptance criteria

- Follow the handoff's structure, copy, token colours and state-driven routing.
- Every action uses the existing Farm backend and surfaces failures.
- Tests, type checks and production build pass; visual review covers light, dark and reduced motion.
- No new dependency; prior profile-navigation and Review changes survive.

