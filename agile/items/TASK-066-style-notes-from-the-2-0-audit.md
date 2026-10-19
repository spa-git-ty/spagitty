<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-066 — Style notes from the 2.0 audit

**Status:** Done — on `release/3.4.0`; unmerged. Every note is done or struck below; the one that was real is BUG-094.
**Screens:** —.
**Raised by:** the audit. Things that differ from the house style, or are edge cases not worth a bug of their own, and work. None was fixed in passing.

## Notes

- **`src-tauri/src/agents.rs` is one file of about 1,500 lines** (#52): the machine file, the probes, the local and remote agent commands, assignment start and its tests. It is cohesive but hard to read; `commands.rs` is split by topic and this could be too.
  - ~~Struck~~, 2026-10-19: splitting a cohesive file is a move with no change in behaviour and a large diff for every branch in flight; it is worth doing when somebody next works across it, not on its own.
- **`design_handoff_agents/`** (#52) puts HTML source files and a canvas JSON in the repository root. Its README says the proposal wins over the shipped item while the proposal is still marked *Proposed*, and the item defers things the proposal describes (merge undo, agent-handled rebase stops). The README should point at the item.
  - **Done** before this was picked up: the README names [FEAT-115](FEAT-115-agents-in-review-and-merge.md) as the authority where the item and the spec differ.
- **`plain()` in `src-tauri/src/watch.rs`** (#53) takes `\\?\` off any verbatim path. For `\\?\Volume{…}\` or a device path that leaves a path that reads as relative. `canonicalize` returns that form only for unusual mounts, so nothing fails today.
  - **Done**, 2026-10-19: the prefix comes off only when what is left is still absolute, a drive path above all; a volume or device path keeps it. `a_verbatim_prefix_is_taken_off` covers both.
- **`tools/release-notes.mjs`** runs as a CLI only when `import.meta.url === file://${argv[1]}`, which is false on Windows. It works on the CI runners; the new `tools/attribution.mjs` uses `pathToFileURL`, and this could follow.
  - **Done**, 2026-10-19: it uses `pathToFileURL`, and `node tools/release-notes.mjs 3.3.0` now prints the notes on Windows, where before it printed nothing.
- **An unresolved CodeRabbit thread on #52** (`agents.rs`, about resuming an assignment whose target moved) was not reproduced. If it is real, it is a bug item then.
  - **Real**: [BUG-094](BUG-094-a-resumed-agent-skips-files-that-changed-since.md), fixed 2026-10-19. Resume sends the head as it is now, and the earlier record kept its old one.
- **Comment density.** The files in #53 to #58 carry long explanatory comments naming the bug that caused them. It is the house style and is left alone.
  - ~~Struck~~: as it says.

## Acceptance criteria

- Each note is either done, or struck here with a reason.
