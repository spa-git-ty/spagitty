<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Proposal: a local hub for parallel agents

**Decided 2026-10-18:** the local hub (option 1). Step 1 is FEAT-143, steps 2 and 3 are FEAT-144, step 4 is FEAT-145 and step 5 FEAT-146; own copies, from [the fuller proposal](../docs/proposals/landing-lane-and-local-hub.md), are FEAT-147.

**Built 2026-10-19 (3.3.0):** FEAT-145, FEAT-146 and FEAT-147. The questions below were answered in those items, for the author to overrule: the person resolves by default, and when an agent does it is the task's own; a resolved conflict is checked and reviewed again; a slice takes *May touch* and *Reads* (FEAT-143). Batch merging is still not built.

Raised by the author, 2026-10-18, after reading gitai
(github.com/feder-cr/git-ai), an MIT implementation of the server-centric
design Anthropic described in "Redesigning source control". The question:
can Spagitty treat the **local repository as the single source of truth**,
with a Rust layer on top that hands each agent only the part of the project
it needs?

## What Spagitty already has

`crates/spagitty-farm` already gives every task its own branch
(`farm/T-xx-…`) and its own worktree (`workspace/worktree.rs`), and stops two
tasks editing the same file with leases (`workspace/lock.rs`). Reviews and
merges run in scratch worktrees (`assign/world.rs`). So a hub is a step on
from what exists, not a new start.

## What gitai does, in short

- Clients fetch **only the objects and paths they need** (sparse trees), not
  a clone.
- **No merge queue**: a change is rebased onto the branch tip and the ref is
  moved with a **compare-and-swap**; concurrent changes to one branch are
  batched into one ref update.
- Every accepted ref update goes into an ordered **changeset log** you can
  watch live, with the agent that made it.
- **History as an API**: blame, path history and search run on the server and
  are cached per commit.
- A pluggable **conflict resolver** (an agent, a tool, or none).

## Three ways to do it

### 1 · Local hub (recommended)

A Rust layer inside the app (in `spagitty-farm` or a new crate) over the
local repository.

| Gains | Costs |
| --- | --- |
| Grows out of the worktrees and leases already there | Each agent still has real files on disk (sparse cuts it down) |
| Git does the hard parts: `sparse-checkout` for slices, `update-ref <ref> <new> <old>` is already a compare-and-swap | One machine. In practice the limit is API rate and RAM, not the hub |
| Offline, Windows-friendly, no protocol to design | A worktree is not a security boundary |

### 2 · A real server (gRPC, as gitai)

| Gains | Costs |
| --- | --- |
| Agents anywhere: containers, a VM, a friend's machine | Auth tokens, a protocol, a daemon to keep alive, a client on the agent side |
| Real isolation; scales past one box | CLI agents expect a real folder, so the client writes the files anyway — the weight moves, it does not go |
| | Months of work for a project of "me and friends" |

### 3 · A virtual filesystem (ProjFS / FUSE)

| Gains | Costs |
| --- | --- |
| Instant "checkout", next to no disk | A different API per OS: ProjFS (Windows), FUSE (Linux), macFUSE (painful) |
| The hub sees every file an agent **reads** — ideal for leases and for knowing what an agent looked at | `cargo` and `bun` walk the whole tree, so everything is pulled in anyway, slowly; hard to debug |

## The plan

Do **1**, behind a trait, so **2** is a new implementation later and not a
rewrite:

```rust
trait SliceProvider {
    /// A place for one task to work in, holding at least `paths`.
    fn open(&self, task: &TaskId, base: &Oid, paths: &[PathSpec]) -> Result<Slice>;
    fn close(&self, slice: Slice) -> Result<()>;
}
// WorktreeSlice now (worktree + sparse-checkout); RemoteSlice later.
```

In order, each useful by itself:

1. **Slices.** A task's worktree with `sparse-checkout` limited to its
   *May touch* paths, plus what the planner says it must read.
   **Catch:** a sparse slice breaks `cargo test` in a Rust workspace when
   member crates are missing. Agents *edit* in the slice; the checks run in
   a full tree when the task is verified.
2. **Merging without a queue.** Rebase the task onto the tip, run the checks,
   then `git update-ref refs/heads/<target> <new> <old>`. If the tip moved,
   rebase again and retry. Several ready tasks become one ref update.
   (This sits under Merger — every merge is still made in Merger, FEAT-117.)
3. **A changeset log.** SQLite: each ref update, its task, its agent, the
   before and after. It feeds the Farm's Activity, Home and the notification
   center.
4. **History as tools.** Blame, a path's history and search, offered to
   agents over MCP and cached per commit, so an agent asks instead of walking
   history itself.
5. **A conflict resolver**, chosen in Rules: an agent, a tool, or stop and
   ask.

In the wizard this appears as one choice in step 3, *Where each task works*:
a worktree (today) or a slice (`new`). It stays hidden until step 1 ships.

## Questions for later

- Which agent resolves conflicts by default, and does a resolved conflict
  need a second review?
- Does a slice take only *May touch*, or also files the planner says to read?
- Batch merges: how many tasks, or how many seconds, per batch?
