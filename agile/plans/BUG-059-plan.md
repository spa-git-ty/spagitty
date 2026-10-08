<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-059 — Plan

**Item:** [BUG-059](../items/BUG-059-windows-watcher-never-sees-refs.md)

Take the verbatim prefix off the canonical git directory and work-tree paths, and off every event path before it is compared, so a `.git` event is a `.git` event whichever form either side came in. Keep it to `watch.rs` and add no dependency. Test `plain()` with Windows' own forms, and `classify()` with a verbatim git directory against plain event paths and the reverse; check a working-tree path is still a candidate. Run the watcher's tests, clippy and the workspace suite.
