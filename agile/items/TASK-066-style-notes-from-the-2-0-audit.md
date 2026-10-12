<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-066 — Style notes from the 2.0 audit

**Status:** Backlog — raised by the TASK-063 audit, 2026-10-12.
**Screens:** —.
**Raised by:** the audit. Things that differ from the house style, or are edge cases not worth a bug of their own, and work. None was fixed in passing.

## Notes

- **`src-tauri/src/agents.rs` is one file of about 1,500 lines** (#52): the machine file, the probes, the local and remote agent commands, assignment start and its tests. It is cohesive but hard to read; `commands.rs` is split by topic and this could be too.
- **`design_handoff_agents/`** (#52) puts HTML source files and a canvas JSON in the repository root. Its README says the proposal wins over the shipped item while the proposal is still marked *Proposed*, and the item defers things the proposal describes (merge undo, agent-handled rebase stops). The README should point at the item.
- **`plain()` in `src-tauri/src/watch.rs`** (#53) takes `\\?\` off any verbatim path. For `\\?\Volume{…}\` or a device path that leaves a path that reads as relative. `canonicalize` returns that form only for unusual mounts, so nothing fails today.
- **`tools/release-notes.mjs`** runs as a CLI only when `import.meta.url === file://${argv[1]}`, which is false on Windows. It works on the CI runners; the new `tools/attribution.mjs` uses `pathToFileURL`, and this could follow.
- **An unresolved CodeRabbit thread on #52** (`agents.rs`, about resuming an assignment whose target moved) was not reproduced. If it is real, it is a bug item then.
- **Comment density.** The files in #53 to #58 carry long explanatory comments naming the bug that caused them. It is the house style and is left alone.

## Acceptance criteria

- Each note is either done, or struck here with a reason.
