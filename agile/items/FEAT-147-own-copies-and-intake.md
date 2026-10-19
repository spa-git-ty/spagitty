<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# FEAT-147 — Own copies, and intake at the door

**Status:** Done — on `release/3.3.0`; unmerged. Covered by Rust tests (natively and in WSL) and component tests; not looked at in the running application (the author is checking it).
**Branch:** `release/3.3.0`.
**Screens:** 1Q, wizard step 3.
**Proposal:** [The landing lane and the local hub](../../docs/proposals/landing-lane-and-local-hub.md), §6 (named FEAT-141 there before the numbers were handed out).

## Change

An agent can work in a clone of its own (`git clone --local`, no remote) instead
of a linked worktree, so it cannot move the person's branches or push anywhere.
Spagitty fetches the work into `refs/spagitty/inbox/…`, checks it at the door
(ancestry, declared paths, protected paths, size, shape), and only then moves
the task's farm branch, from where the task is checked, reviewed and merged as
any other.

## Answers to the proposal's questions (§12)

The author chose the local hub on 2026-10-18. The rest were left open; these are
the answers this item was built on, mine, for the author to overrule:

- **The hub** is the person's own repository, with the pull model (§6.4): no
  bare repository, nothing pushed into it.
- **The default for new farms** stays *A worktree per task*. Copies are a third
  choice in *Where each task works*, tagged `new`.
- **Hooks in copies:** none. A clone takes no hooks from the repository it was
  made from, and the farm adds none. The copy is not signed as the person
  (`commit.gpgsign=false`); it commits under the person's name and email, as a
  worktree would.

## Acceptance criteria

- **A copy** is `<repo>/.spagitty/farm/copies/<task>/`, cloned with `--local`
  (hard-linked objects) and `--no-tags`, on the task's farm branch, with no
  remote and no other branch. The commit it started from is kept beside it, not
  in it.
- **No git route out:** in a copy, `git push`, `git fetch origin` and `git branch
  -f <person's branch>` move nothing in the person's repository.
- **Intake** fetches the copy's branch into `refs/spagitty/inbox/<task>/<provider>`
  when the run ends (and before every verification), and refuses, naming the
  path or commit:
  - work that does not descend from where the task started;
  - a changed path outside the task's *May touch* (none declared means anywhere);
  - a change to `.github/workflows/`, `.gitlab-ci.yml`, `.gitmodules`,
    `.githooks/` or `.husky/` the task did not name itself;
  - a submodule pointer moved;
  - a file over 5 MB;
  - a merge commit, except the one Spagitty starts to resolve a conflict
    (FEAT-146).
  A refusal blocks the task with the reason and keeps the copy. A pass moves the
  farm branch by compare-and-swap.
- **The same journey after intake:** verification, review and the lane read the
  farm branch and the copy's checkout, as they read a worktree's.
- **A sent-back task** runs again in its copy; a new copy starts from work
  already taken.
- **Removal and cleanup:** a copy with uncommitted changes survives unless
  forced. The leftover sweep removes copies no farm claims whose commit is in
  HEAD, with their inbox ref.
- **The agent is told** it is in a copy, to commit and not push.

## What was built

- `workspace/copy.rs`: `create`, `intake` (`Door`, `Intake`, `Refusal`), `remove`,
  `copies`, `allow_merge`.
- `WorkspaceMode::Copy`; `service.rs` cuts a copy in `start_task` and takes from
  it in `take_from_copy` before every verification; `worktree::remove` removes a
  copy too; `cleanup::sweep` sweeps copies.
- `context.rs`: *YOUR COPY* in the implementation prompt.
- `RulesEditor.svelte`: *A copy per task*; `types.ts`: `'copy'`.

## Not in this item

- Copies off this machine (containers, SSH), and a bare hub for them to push to.
- A size limit or protected list set in Settings: both are constants for now.
- The proposal's *Shape* check on authors: agents commit under the person's name.
