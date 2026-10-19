<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-146 — A conflict resolver chosen in Rules

**Status:** Done — on `release/3.3.0`; unmerged. Covered by Rust tests (natively and in WSL) and component tests; not looked at in the running application (the author is checking it).
**Branch:** `release/3.3.0`.
**Screens:** 1Q, wizard step 3 (*More rules*).
**Proposal:** [A local hub for parallel agents](../../design_handoff_home_farm_notifications/HUB.md), step 5.

## Change

When the landing lane (FEAT-144) meets a conflict, the farm's Rules choose who
resolves it: the person (today's behaviour, and the default), the agent that
wrote the task, or a command. An agent or a command works in the task's own
workspace — its worktree or its copy, never the person's working copy — and
the resolution is committed on the task's branch as an ordinary merge of the
destination. The task then goes through its checks and its review again before
the lane merges it.

## Answers to the questions in `HUB.md`

Mine, for the author to overrule:

- **Who resolves by default:** nobody but the person. *You, in Merger* is the
  default; a farm written before reads that way.
- **When an agent resolves, which one:** the agent that wrote the task. It knows
  the change; any other agent would have to learn it first.
- **Does a resolved conflict need a second review?** Yes. The resolution is a
  new commit on the task's branch, so the evidence for the old commit no longer
  counts (BUG-027): the checks run again and the reviewer reviews it, at the
  levels that review.

## Acceptance criteria

- **Rules › More rules › When a merge conflicts:** *You, in Merger* / *The task's
  agent* / *A command*, with the command's text. Saved on the farm as
  `conflicts: { by: "ask" | "agent" | "tool", command }` and remembered with the
  crew and rules. A command left empty is the default.
- **You, in Merger:** unchanged — *Conflicts with <destination>*, the task
  blocked, nothing written.
- **The task's agent:** the merge is started in the task's workspace and the task
  goes back to its agent with the conflicted paths and how to finish the merge.
  Its note reads *Resolving conflicts with <destination> in <paths>*. A task that
  has had all its tries stops and asks instead.
- **A command:** run in the task's workspace with the conflicted paths after it.
  When it leaves a conflict marker, or fails, the merge is undone, and the task
  stops with the reason and the command's output.
- **Again from the top:** a committed resolution is verified (and, for a copy,
  taken at the door, which accepts that one merge) and reviewed before it is
  merged. The lane then merges a branch that already contains the destination.
- **The person's working copy and the destination** are not touched by any of it.

## What was built

- `workspace/resolve.rs`: `open` (the merge, fetched in first for a copy),
  `unresolved`, `finish`, `abort`, `in_progress`.
- `ConflictResolver` on `Farm` and `Setup`; `farm_configure` takes `conflicts`.
- `service.rs`: `resolve_conflict`, called where the lane stops on a conflict;
  `start_task` keeps a slice whole while a merge waits in it.
- `RulesEditor.svelte`: *When a merge conflicts*.

## Not in this item

- Another agent of the crew as the resolver, or a resolver per destination.
- Resolving through Merger's screen with the agent's proposals (FEAT-117's
  room): the resolution here is a commit on the task's branch.
