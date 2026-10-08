<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# BUG-065 — Command-line agents cannot start or finish reviews correctly

**Status:** Open — implemented on `bugfix/BUG-065-agent-cli-access`; local validation complete, PR awaiting review and merge. OMP's authenticated smoke test needs the selected model/profile.
**Screens:** Settings → Agents, Review, Merger, Farm.

## What happens

Codex's Settings test rejects its temporary directory as untrusted. On this
Windows machine its sandbox cannot start shell tools, but Codex can still exit
zero and supply an empty verdict. Oh My Pi is installed as `omp`, which is not
searched, and its Bun runtime is outside the desktop process's PATH. The `agy`
CLI has no built-in adapter. Completed agent file steps leave viewed ticks unset.

## Acceptance criteria

- Codex's temporary-directory test and scratch-worktree launches pass the Git
  startup check and explicitly select the intended directory.
- Settings offers a persisted, opt-in Codex Full Access choice used by Test,
  Review and Merger. The default remains sandboxed; no automatic fallback.
- A known Windows sandbox startup failure fails the step even if Codex exits zero.
- `omp` is detected before legacy names, launched in print mode, and receives a
  PATH containing the native Bun runtime when installed via npm.
- Settings persists OMP's model and optional profile and passes them to fresh
  Test, Review and Merger launches. Blank fields inherit OMP's defaults.
- `agy` is detected, offered in Settings and Farm, and launched headlessly.
  Review and Merger use its plan mode; unattended Farm permissions are explicit.
- Settings offers persisted, opt-in agy tool auto-approval for headless Test,
  Review and Merger. A permission denial fails the assignment with guidance.
- Settings Test requires its requested answer; a configuration error is not a
  successful test just because the CLI exits zero.
- Completed file steps tick the read blob once. Failed or unfinished steps and
  assignments for an older head do not tick files. A person's untick survives sync.
- Relevant tests, type checks and builds pass. No dependencies are added.
