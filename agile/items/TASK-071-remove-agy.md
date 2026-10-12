<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-071 — agy is removed

**Status:** Done — on `task/TASK-071-remove-agy`; unmerged. The sweep is not run.
**Screens:** 1K, 1Q, 1R, 1S.
**Raised by:** the author, 2026-10-12, on 2.2.1, after a review assigned to agy failed.

## Problem

A review given to agy stopped at its first step:

```
agy jetski: no output produced — a tool required the "command" permission that headless mode cannot prompt for, so it was auto-denied.
```

agy's own log and transcript for that run say what happened. Its print mode cannot ask for a permission, so the first tool that needs one is denied, and the turn ends there with nothing printed. Given the plan step of a review, agy 1.3.3 asked to run `git status` before answering. The same prompt replayed in a scratch repository did it six times in six, with `--mode plan`, without it and with `--sandbox`. `--mode plan` is also not only a read-only switch: it puts agy's own planning workflow after the prompt, about four thousand characters asking for an implementation plan document.

With a sentence added to the prompt forbidding the command tool, agy read files instead and answered in three of five replays; the other two ended the same way on a file it asked to read outside its working directory. The only setting that makes it dependable is **Auto-approve tools**, which is `--dangerously-skip-permissions`.

The author decided not to fix it. agy is not good enough to keep, the author does not want Spagitty driving a Google account without a person at it, and asked that the changelog say Google is not worth supporting.

## Change

- agy is not a built-in agent. Settings › Agents does not list it, Farm does not detect it, and Review and Merger do not offer it.
- The **Auto-approve tools** choice, its saved value and its command are gone.
- What Spagitty knew about agy's command line is gone: print mode, plan mode, the permission denial read from its output, and its install directory on the search path.
- A Farm agent list saved with agy in it opens without agy and keeps every other agent.
- Gemini through an API key, under remote agents, is a different thing and is unchanged.

## Acceptance criteria

- Nothing in `src/`, `src-tauri/` or `crates/` runs, detects or names agy, apart from the reading of an old saved list.
- An `agents.json` that still has `agyAutoApprove` loads.
- A Farm registry that still has an agy definition loads, without it, and with the others as they were.
- The tests pass and the build succeeds. No dependency is added or removed.

## Decision

The provider is taken out of the closed set, not hidden. `AgentProvider` is what the compiler checks every adapter, label and colour against, so a variant kept for old files would have to keep an adapter too.

That makes an old saved definition unreadable, and a registry was read all or nothing: one agy entry would have opened the Farm with no agents configured at all. `AgentRegistry` now reads its definitions one at a time and leaves out the ones it cannot read (`readable` in `crates/spagitty-farm/src/agent/registry.rs`).

Anyone who still wants agy can add it as a custom agent, with their own arguments.

## Not done

- A Farm task agy was working on when the application is updated keeps its branch, `spagitty/farm/<task>/agy`. Spagitty no longer reads `agy` in that name, so it treats the task as a custom agent's and will not find its worktree to remove. No such task is known to exist.
- BUG-065 and TASK-060 describe agy as it was built. They are left as the record of that.
- `tools/attribution.mjs` still refuses agy and Antigravity as the author of a commit. That is TASK-064's rule and has nothing to do with running it.

## What was done

`crates/spagitty-farm/src/agent/adapters/agy.rs` is deleted, with `AgentProvider::Agy`, its label, slug and adapter, the plan-mode flags and the permission-denial check in `assign/local.rs`, and `agy/bin` in `detector.rs`. In `src-tauri/src/agents.rs` the saved `agy_auto_approve`, `agy_access` and `agents_set_agy_auto_approve` are gone. In the frontend, the provider, its label, colour and monogram, the two chips in `AgentsSection.svelte` and `setAgyAutoApprove`.

Tests that used agy as an example use something else: the Windows shim tests name `agent.cmd`, the backend tests that needed "some other provider" use Cursor, and the Settings test that a changed access choice is saved before **Test** can run uses Codex's **Full Access**.
