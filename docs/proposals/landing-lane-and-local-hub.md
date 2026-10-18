<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Proposal: the landing lane and the local hub

**Prepared:** 2026-10-18  
**Target:** 3.2.0 (the lane), 3.3.0 (the hub). Open — see [question 7](#12-questions-for-the-author).  
**Status:** The author chose the local hub on 2026-10-18. The items are FEAT-144 (the lane: §5.1 and §5.5 now; §5.2 and §5.3 later) and FEAT-147 (own copies, Backlog). FEAT-140 to FEAT-143 went to Home, the farm wizard, the notification center and slices first, so the numbers first proposed here were reassigned.  
**Raised by:** the author, 2026-10-18, after reading [feder-cr/git-ai](https://github.com/feder-cr/git-ai) (an open implementation of the server-centric source control described in Anthropic's *Redesigning source control*): make the local repository behave like a git server, with a layer in the middle that lets local agents take copies of their own, work in parallel, and land back into the local repository — from where pushing to the remote is ordinary.  
**Screens:** 1Q (Farm), 1K (Settings › Farm). Merger, Conflicts and Review are used, not changed.

## 1. Outcome

- **FEAT-144 — The landing lane.** Farm work lands on its branch without touching the person's working copy, and what lands is what was tested: the merge result, not only the agent's commit. Tasks that finish together can land together. Every move of a branch the farm makes is written to a ledger with who did it.
- **FEAT-147 — The local hub.** An agent can work in a clone of its own instead of a linked worktree. The clone cannot move the person's branches, read their remotes or push anywhere. Spagitty *takes* the work from it, checks it at the door, and hands it to the lane.

The lane is useful on its own, with today's worktrees. The hub reuses the lane, so the lane comes first.

## 2. Where the farm is today

Read from the code on 2026-10-18.

| What | Where | Today |
| --- | --- | --- |
| Workspace per task | `spagitty-farm/src/workspace/worktree.rs` | `git worktree add -b spagitty-farm/<task>/<provider>` under `.spagitty/farm/worktrees/` |
| Overlap | `workspace/lock.rs` | Path leases: two tasks whose declared globs intersect never run together |
| Merging | `service.rs` `merge_checked` → `shell::merge_into` | `git merge --no-ff` **in the person's working copy**; refused unless the person is on the destination branch |
| Conflict | same | The task is blocked and the conflict is left **in the working copy** |
| Evidence | `verified_commits`, `reviewed_commits` (BUG-027) | Checks and review are for the **task's commit**, not for the merge result |
| Concurrency | `merge_lock` | A process mutex; one merge at a time, nothing retried, nothing batched |
| Push, merge permissions | `model/farm.rs` `Permissions` | Off by default, and enforced by the **prompt** ("do not merge, do not push") |

Meanwhile Merger already knows how to land without a checkout (FEAT-101, `spagitty-core/src/merger/land.rs`): it builds the result with `merge-tree` and `commit-tree`, then moves the branch against the tip it read, so a branch that moved in the meantime is refused rather than overwritten. The farm does not use it.

So the gaps are four:

1. **The farm reaches into the person's working copy.** They must stand on the destination branch, and a conflict lands in their files.
2. **What lands is not what was tested.** Task A and task B each pass on their own; A lands; B's merge onto the new tip is never run.
3. **Nothing retries or batches.** At four agents that is tolerable; it is also the point where it stops being so.
4. **Permissions are words.** An agent in a linked worktree shares refs, remotes, credentials, config, hooks and stash with the person's repository. `git checkout main && git merge`, or `git push`, works from inside it.

## 3. Rules that shape the rest

1. **The person's working copy is theirs.** No farm operation writes to it, except fast-forwarding a checked-out branch with `merge --ff-only`, which refuses rather than touch uncommitted work (the rule Merger already follows).
2. **Land what was tested.** Evidence attaches to the commit that will become the branch tip.
3. **A branch moves only against the tip that was read.** Compare-and-swap, always. A moved tip is a retry, never an overwrite.
4. **Isolation is structural, not instructional.** If an agent must not be able to do something, it is given nothing to do it with. Prompts still say it; they stop being the guard.
5. **Nothing new is louder than the work.** One strip on Farm, one section in Settings. Merger, Conflicts and Review are not redesigned.
6. **No server process.** "Hub" is a role the person's repository plays, not a daemon.

## 4. Words used here

| Word | Means |
| --- | --- |
| **Lane** | The queue of finished tasks waiting to land on one destination branch. One lane per destination. |
| **Candidate** | The commit the lane builds: the destination's current tip with one or more tasks merged in. Not yet on any branch. |
| **Land** | Move the destination branch to a verified candidate, by compare-and-swap. |
| **Ledger** | The append-only record of every branch move the farm made. |
| **Own copy** | A clone of the repository made for one task, instead of a linked worktree. |
| **Intake** | Taking a task's work from its own copy into the person's repository, and the checks made at that door. |
| **Inbox** | The refs intake writes to: `refs/spagitty/inbox/<task>/<provider>`. Never a branch, never pushed. |

## 5. The landing lane (FEAT-144)

### 5.1 The flow

```
 task in Review, evidence current
              │
              ▼
   ┌──── lane for <dest> ─────────────────────────────────────────┐
   │ 1. read tip T of <dest>                                       │
   │ 2. build candidate C = T ⊕ task   (merge-tree + commit-tree)  │
   │      conflict? ──► stop: Blocked, opens in Merger (5.4)       │
   │ 3. verify C in the lane's scratch worktree                    │
   │      red? ──► task back to Waiting with the output            │
   │ 4. update-ref <dest> C T          (compare-and-swap)          │
   │      tip moved? ──► back to 1     (counts as a retry)         │
   │ 5. ledger entry; task Done; worktree removed                  │
   └───────────────────────────────────────────────────────────────┘
```

- **Step 2 is Merger's `land`**, called with `Strategy::Merge`. `--no-ff` stays, for the reason `merge_checked` gives: a farm branch that fast-forwards disappears from history and "which agent wrote this" loses its answer.
- **Step 3 runs the repository's verification commands on C**, in one scratch worktree per lane: `.spagitty/farm/lane/<dest>/`, reset to C each time. When C equals what the task already verified (T had not moved since the task started), the step is skipped: the evidence is already for this tree.
- **Step 4 when `<dest>` is checked out** — in the person's working copy or another worktree — is `merge --ff-only C` there, as in Merger. If the person's uncommitted changes are in the way, the lane waits and the strip says so. It never stashes for them.
- **The person no longer has to stand on the destination branch.** `Switch back to <x> before merging this task` goes away.

### 5.2 Landing together

When several tasks for the same destination are waiting, the lane may build one candidate for all of them: `T ⊕ A ⊕ B ⊕ C`, verify once, land once.

- **If the batch is red**, the lane splits it in halves and verifies each, and keeps splitting until it has the task that broke it. The rest land; that one goes back with the output. At four tasks this is at most three extra runs.
- **The batch lands as one ref move**, recorded as one ledger entry naming every task.
- **Off by default** (question 3). At four agents it rarely matters; it is there for slow test suites.

### 5.3 Checks before and after landing

Verification commands get a tier in Settings › Farm:

| Tier | Runs on | Blocks the land | Example |
| --- | --- | --- | --- |
| **Before landing** | Each candidate | Yes | `cargo test -p core`, lint |
| **After landing** | The new tip, once the lane is idle | No | The full suite, the slow integration tests |

When an after-landing check goes red, the lane walks the ledger entries since the last green tip, finds the one that broke it (bisecting if there are several), and:

- at **Unattended**, writes a revert commit for it and lands that through the lane, and sends the task back;
- at every other level, stops the lane and says which entry broke the tip. The person decides.

Reverting is always a new commit, never a ref moved backwards: the tip may already have been pushed.

### 5.4 Conflicts

A conflict at step 2 does not touch any working copy. The task is Blocked with *Conflicts with <dest>*, and its action opens Merger on the pair, which resolves into a candidate the same way it does for a person (FEAT-117). What Merger lands, the farm learns about through `mark_merged`, as it does now (BUG-077).

An agent as resolver — "agents do merges" from the 2.0 plan — is the same entry point with the agent filling in Merger's resolutions. It is not in this item; the lane only has to leave the door where Merger already is.

### 5.5 The ledger

`.spagitty/farm/ledger.jsonl`, one line per branch move, appended and never rewritten — the same crash rule as the event log (`persistence/store.rs`: a partial last line is dropped on read).

```json
{"seq":41,"at":"2026-10-18T13:30:12Z","ref":"refs/heads/main",
 "old":"9c1e…","new":"4b7a…","kind":"land",
 "tasks":["TASK-0042","TASK-0043"],
 "by":[{"task":"TASK-0042","agent":"codex","reviewer":"claude"},
       {"task":"TASK-0043","agent":"claude","reviewer":"codex"}],
 "approved":"auto","checks":"before:pass"}
```

`kind` is `land`, `revert` or `retry-lost` (a compare-and-swap that lost and was rebuilt). The ledger is what Investigate (1O) can read to answer "which agent landed this, and who passed it" without parsing merge messages.

### 5.6 On screen

- **Farm (1Q)** gets one strip above the activity log: the lane per destination, its queue as task chips, and the state of the head — *Building*, *Checking*, *Landing*, *Waiting for your working copy*, *Stopped: <entry> broke main*. Nothing else on Farm moves.
- **Settings › Farm**: *Land together* (switch), the two verification tiers, and *After a red check* (Stop / Revert, the latter only offered at Unattended).
- **Labels only where needed**; the rest is hover titles.

### 5.7 Acceptance criteria

- A farm merge never writes to the person's working copy, except `--ff-only` on a checked-out destination.
- A task can land while the person is on another branch.
- The commit that becomes the tip has passing before-landing checks for that exact tree.
- A tip moved by the person between steps 1 and 4 causes a rebuild, never an overwrite (test: move the ref from a second thread mid-land).
- With *Land together* on, a red batch of three in which one task breaks the build lands the other two and returns the third with its output.
- Every land and revert appears once in the ledger; killing the process mid-land leaves either no move and no entry, or both.
- **Tests:** Rust tests in `spagitty-farm` for the lane state machine, compare-and-swap loss, batching and bisection; component tests for the strip.

## 6. The local hub (FEAT-147)

### 6.1 Why own copies, when worktrees exist

A linked worktree is a second working directory on the **same** repository. It shares, with the person:

- every ref, so `git branch -f main …` or `git checkout main && git merge` from inside it moves the person's branches;
- every remote and the credential helper, so `git push` works;
- `.git/config`, `.git/hooks`, the stash and the reflog;
- and its `.git` is a file that points back into the person's repository, so it cannot be moved, mounted or shipped on its own.

An own copy shares none of that. It is a complete repository that happens to have been cloned from the person's, which also makes it the unit that can later go into a container or onto another machine — the "remote agents" of the 2.0 plan.

### 6.2 What a copy is

```
<repo>/.spagitty/farm/copies/task-0042/        (a normal, non-bare repository)
   git clone --local --single-branch --branch <destination> --no-tags <repo> <dir>
   git remote remove origin
   git switch -c spagitty-farm/TASK-0042/codex
```

The farm already starts every task from its destination branch (`service.rs`, the call to `workspace::create`), so `--branch` always has a branch to name.

- **`--local`** hard-links the object files on the same filesystem, so a copy costs a checkout, like a worktree, and almost no object storage. (Not `--shared`: alternates break if the person's repository is ever garbage-collected under a copy.)
- **`--single-branch --no-tags`**: the agent sees the branch it starts from and nothing else.
- **No remote at all.** The copy has nowhere to push to and no credentials to push with. Pushing to the person's repository by path is the only route left, and §6.4 closes what that route can reach.
- **A fresh `.git/config`**: the agent's identity from the farm, and the hooks policy from Settings (question 6). The person's signing key is not configured in the copy: agents do not sign as the person.

### 6.3 Intake: the pull, not the push

Spagitty does not wait for the agent to push. When the run ends, Spagitty fetches:

```
git -C <repo> fetch --no-tags <copy> \
    +refs/heads/spagitty-farm/TASK-0042/codex:refs/spagitty/inbox/TASK-0042/codex
```

The person's repository is only ever written by Spagitty, into a namespace that is not a branch and is never pushed. Then intake checks, before anything else sees the work:

| Check | Refused when |
| --- | --- |
| **Ancestry** | The inbox commit does not descend from the task's start |
| **Paths** | Any changed path falls outside the task's declared `allowed_paths` — today's leases only keep tasks apart at scheduling; this makes them binding |
| **Protected paths** | It touches `.github/workflows/`, `.gitmodules`, a gitlink or a hook file, unless the task declares it |
| **Size** | A blob above the limit in Settings › Farm (default 5 MB) |
| **Shape** | Commits whose author is not the task's agent, or merge commits the agent was not asked for |

A refusal is a Blocked task with the reason, and the copy is kept for the person to read. A pass goes to verification, review, and then the lane (§5), exactly as a worktree task does.

### 6.4 Why the person's repository is the hub

The earlier conversation left two shapes open: the person's own repository as the hub, or a hidden bare repository under `.spagitty/hub.git` that both the person and the agents sync with. This proposal recommends the first:

- **One source of truth.** A second bare repository is a second place a branch can be, and a sync step between it and the working repository is a new way to be out of date.
- **The pull model removes the classic obstacle.** git refuses a push into a checked-out branch of a non-bare repository; nothing is pushed here, so it never comes up.
- **Pushing to the remote stays exactly what it is today**: the toolbar's Push, from the person's repository.

The bare-hub shape comes back only if copies move off this machine and need somewhere to push to (§10).

### 6.5 One seam for both

`spagitty-farm/src/workspace` gains a trait, so nothing above it learns which kind of workspace a task has:

```rust
pub trait Workspaces {
    fn create(&self, task: &TaskId, provider: AgentProvider, start: &str) -> Result<Workspace>;
    /// The task's work, as a commit in the person's repository.
    fn collect(&self, task: &TaskId) -> Result<Collected>;   // worktree: the branch tip
                                                             // copy: intake (§6.3)
    fn remove(&self, task: &TaskId, keep_on_failure: bool) -> Result<()>;
}
```

`LinkedWorktrees` is today's `worktree.rs`. `OwnCopies` is new. The farm picks one per farm, in Settings › Farm: **Where agents work — Linked worktrees / Own copies**. Crash recovery (`service.rs` reconciling at startup) learns to find copies and inbox refs as it finds worktrees now.

### 6.6 Honest limits

- **Isolation is against mistakes, not malice.** A local agent runs as the person and can open any file they can, including the repository's `.git` by path. Own copies remove every *git* route to the person's branches; they are not a sandbox. They are what makes one possible later, because a copy can be mounted into a container on its own.
- **Disk.** Each copy has its own checkout, as a worktree does. Objects are hard-linked; packs written later by the agent are not.
- **Windows.** Hard links need the copy on the same volume, which `.spagitty/` inside the repository guarantees.

### 6.7 Acceptance criteria

- In a copy, `git push`, `git fetch origin` and `git branch -f main` cannot move anything in the person's repository.
- Intake refuses a change outside the task's declared paths, and the reason names the path.
- A task's journey after intake — verification, review, lane — is identical for worktrees and copies; the same scheduler tests pass with either `Workspaces` implementation.
- Creating a copy of this repository on Linux, macOS and Windows takes no more than twice the time of a worktree (measured in the sweep).
- Inbox refs and copies are found and reconciled after a crash.
- **Tests:** Rust tests for each intake rule, the pull, and the trait run against both implementations.

## 7. What was taken from gitai, and what was not

| gitai | Here |
| --- | --- |
| Landing by rebase and compare-and-swap, no merge queue | §5.1, with merge instead of rebase to keep attribution |
| Concurrent lands batched into one ref update | §5.2 |
| Changeset log with the actor | §5.5, the ledger |
| Test after landing, bisect, revert the culprit (`graft-verify`) | §5.3 |
| Agents as first-class users: per-agent tokens, RPC allowlist, branch policies | §6, as structure: no remote, inbox namespace, intake rules |
| Pluggable conflict resolver | Left to Merger (§5.4) |
| Sparse clients, server-side history, cache tiers, database and object storage | Not taken. They answer a network and scale problem that four agents on one machine do not have |
| The server itself | Not taken. A Go service with a bundled git build, no Windows build, and a server in the path of a local-first client. It returns as an optional backend only for shared farms (§10) |

## 8. Dependencies

- **FEAT-101 Merger `land`** — the lane's step 2 and 4.
- **FEAT-117, BUG-077** — conflicts and person-made merges stay in Merger; the farm learns of them.
- **BUG-027** — the evidence rules; the lane moves them from the task commit to the candidate.
- **FEAT-137 Hooks** — the hooks policy for copies.
- **FEAT-132 Investigate** — the first reader of the ledger.

Nothing new in `Cargo.toml`. Everything runs through `spagitty_core::shell`, so every command the lane and intake run appears in the command log.

## 9. Order of work

1. **Lane, single task** — §5.1 with Merger's `land`, the scratch worktree, compare-and-swap and the ledger. Replaces `merge_checked`. This alone closes gaps 1 and 2.
2. **Two tiers and the after-landing walk** — §5.3.
3. **Landing together** — §5.2.
4. **The `Workspaces` trait** — `LinkedWorktrees` only, no behaviour change.
5. **Own copies and intake** — §6.2 and §6.3, behind the Settings choice.

Steps 1–3 are FEAT-144 (step 1 now, 2 and 3 later); 4–5 are FEAT-147.

## 10. Later, not in either item

- **Copies off this machine** — in a container or over SSH. Intake fetches the same way; a bare hub may be needed for the copy to push to.
- **An agent as resolver** in Merger for lane conflicts.
- **A gitai server as a backend** for a farm shared by several people: the lane and intake become calls to its land and changeset APIs instead of local git.
- **Sparse copies** of `allowed_paths` for very large repositories.

## 11. Non-scope

- Changing Merger, Conflicts or Review screens.
- Any change to how a person merges or pushes by hand.
- More than four agents at once.
- Telemetry, accounts, or any server of Spagitty's own.

## 12. Questions for the author

1. **The hub** — the person's own repository with the pull model (§6.4), or a hidden bare repository under `.spagitty/`?
2. **The default for new farms** — linked worktrees, or own copies once FEAT-147 ships?
3. **Landing together** — off by default, as proposed, or on?
4. **Automatic revert** — only at Unattended, as proposed, or never?
5. **The ledger** — local JSONL only, or also as git notes (`refs/notes/spagitty`) so it can travel with a push when the person chooses?
6. **Hooks in copies** — none, the person's, or a set chosen in Settings › Farm?
7. **Versions** — the lane in 3.2.0 and the hub in 3.3.0, or both in one release?
