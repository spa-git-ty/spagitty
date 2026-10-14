<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-137 — Hooks that run in steps and are built from blocks

**Status:** Done — on `codex/FEAT-137-hooks`; unmerged. Covered by fixture, command and mounted tests; not looked at in the running application. Existing brand drift is recorded separately.
**Branch:** `codex/FEAT-137-hooks`.
**Screens:** 1X, chrome.
**Proposal:** [Forge views](../../docs/proposals/forge-views.md). **Designs:** [Forge views](../../design_handoff_forge_views/).

## Change

See a commit’s hooks run step by step, and build the common hooks without writing shell scripts.

## Acceptance criteria

- Build all seven catalogue blocks for pre-commit, commit-msg and pre-push, with order, stage switches and the settings in proposal §6.3.
- Generate standalone POSIX scripts with a managed marker and block hash. Refuse to overwrite unmarked hooks or silently take over a manager.
- Use git pathspecs and a NUL-delimited staged list. Never restage a path with unstaged changes. A block’s timeout and block/warn choice apply.
- Append run events and separate block logs under the common git directory; keep the newest 50 runs. Terminal commits leave the same records; watch and tail the selected run.
- Save shared blocks in .spagitty/hooks.toml and personal blocks in application data. Offer installation explicitly, dry-run the current index and try a message using the generated script.
- Offer explicit manager import, preserve the original files, and show existing commit output while a manager retains control.
- Match both Hooks artboards and link from Settings → Hooks.
- **Tests:** refusal and success cases in core fixture tests, command tests, store tests and mounted screen tests; types, coverage and build pass.

## Not in this item

- Other hosts, workflow editing, local workflow execution and partial-staging stashes are outside 3.1.
- The other three screens have their own items.

## What was built

- **Rules and standalone scripts** in `hooks/blocks.rs`, `generate.rs` and `runner.sh`: seven block kinds, stage order, path conditions, block/warn, timeout, cancellation, protected push destinations, message checks and per-block output.
- **Storage and installation** in `hooks/builder.rs` and `import.rs`: shared or personal TOML, explicit manager import, preservation of unmarked hooks, common-directory records, bounded log tails and index backups for undoing a formatter's restage.
- **Commands and watcher** in `src-tauri/src/hooks.rs`, `watch.rs` and core `shell.rs`: installed-state checks, staged matches, dry runs and message trials, read/stop/undo commands and filesystem events for the selected run.
- **Both screen views** in `src/lib/hooks/` and `src/routes/hooks/`: the stages, block editor and catalogue; runs, steps, logs, staged files and restage actions. Settings links here. The rail adds its own group above Settings.
- **Automated checks and the manual sweep** are in [the test record](../testing/FEAT-137-automated.md) and [the sweep](../testing/FEAT-137-sweep.md).

## Differences from the spec and the artboard

- **Existing chrome and navigation placement.** The shell remains as it was; the new group is above Settings, using the forge screen group.
- **The marker follows the shebang.** The executable starts with `#!/bin/sh`; the second line is the managed marker. Run ids include the process id to separate concurrent worktrees.
- **WIP skipping needs the subject.** Git provides it at commit-msg. Pre-commit runs leave the subject empty until commit-msg appends it to the record.
- **Portable timing.** Recorded durations have whole-second precision. The screen counts time from each step's start while it runs.
- **Spotless's filename boundary.** The preset supplies one property containing quoted absolute Java patterns. A comma or literal `\E` in a filename is refused with a request for a custom command, because Spotless's property cannot represent it safely.
- **Standard configuration parsers.** TOML and YAML libraries provide standard configuration parsing.
