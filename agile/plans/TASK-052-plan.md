<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-052 — Plan

**Item:** [`agile/items/TASK-052-nothing-waits-in-line.md`](../items/TASK-052-nothing-waits-in-line.md)

**Where this stands:** step 1 is built. Steps 2 to 5 are below for review;
step 2 waits for the author's go-ahead, as the item asks.

**Branch base.** `task/TASK-052-nothing-waits-in-line` is cut from the Review
stack's tip (`feature/FEAT-093-threads-done-properly`), not from `main`: it
times every command in `commands.rs`, which the stack also changes, and step 3
reuses the stack's measured virtual list. It merges after the stack.

## Step 1 — Measure (built)

- **The backend's half.** Every hold of the session lock goes through one
  guard, `AppState::lock_session(command)`; `with_session` takes the command's
  name and uses it. The guard keeps two numbers when it is let go — the wait
  to get the lock and the time it was held — in `src-tauri/src/timing.rs`, a
  ring buffer of the last thousand holds shaped like `spagitty_core::record`.
  `command_timings(after)` reads it. All 78 `with_session` calls and the 12
  direct locks are named after the command they are in, by a script, so none
  was missed.
- **No `tracing`.** The item suggests it. Two numbers per hold, read by one
  panel, are a buffer and a clock; a subscriber would be the first two
  dependencies added for that (AGENTS.md asks for a reason, and this is the
  reason not to). Should the author want spans for a profiler later, the guard
  is the one place to add them.
- **The frontend's half.** `api.ts`'s `invoke` — the only way to the backend —
  times every call, answered or refused (`src/lib/timing.svelte.ts`).
  Navigation (from `beforeNavigate` to two frames after `afterNavigate`) and
  `DiffPane` (from being handed a file to two frames after it is laid out) are
  measured and left as `performance` marks and measures.
- **God mode › Timings** shows, worst first: what held the repository and for
  how long, what waited for it, every call's round trip, and the last screens
  and diffs until painted, with line counts. It reads the backend every two
  seconds while open.
- **A test pins today's behaviour**: a quick read started while a slow one
  holds the repository waits for it, and the timings say how long. Step 2
  turns it around.
- **The baseline**: `agile/testing/TASK-052-baseline.md` says how to
  measure and holds the table. The author asked to take the numbers
  themselves, from the Timings panel and from
  `crates/spagitty-core/examples/baseline.rs`, which times the work behind
  each screen without the application around it.

## Step 2 — Stop holding the lock for the work (for review)

The proposal, in the order it would be done:

1. **A view instead of the session.** Most closures use `session.repo` and
   `session.path` only. `with_session` becomes: lock, clone the
   `ThreadSafeRepository` (an `Arc` and a few handles; it shares the object
   store), the path and the session's ticket, let go, then run the closure on
   that view. The handful that read the graph worker, the visible refs or the
   pins keep a short hold for just that.
2. **Writes are serialised by a lock of their own**, `write: Mutex<()>` on
   `AppState`, taken for the whole of every writing command: stage, unstage,
   discard, apply hunks, commit, amend, checkout, branch and tag changes,
   stash, merge, cherry-pick, revert, reset, rebase and its continue, skip and
   abort, conflict resolution, submodules, worktrees, remotes, fetch/pull/push
   start, the review checkout and worktree. A read never takes it: it reads
   what git has on disk, as it would if git ran beside Spagitty, which it
   already tolerates through the watcher.
3. **Classified on purpose.** Every command in `commands.rs` is in one of two
   lists, readers and writers, and a test fails on any command in neither —
   the TASK-048 test's way of making a new command a decision.
4. **Tickets kept.** A read carries the ticket it started under; when it
   finishes after an open or a close has taken a newer one, it answers
   `Superseded` instead of a result for the wrong repository. TASK-048's
   tests stay as they are and must pass.
5. **The watcher's graph restart** takes the session lock briefly to swap the
   worker, as now; the walk itself runs on the worker's thread. It does not
   race a write any more than it does now, which the review should confirm.
6. **The timing guard stays**, so the Timings panel shows the waits gone.

The risk the item names: two things writing at once. The writers' list and its
test are what stop that; the review should check the list.

## Step 3 — Virtualise the diff

- `DiffPane` (unified and split) draws through `ui/VirtualRows.svelte` from
  FEAT-091: hunks flattened into header and line rows, measured, an overscan
  of a screen. The split view's rows are pairs, one row each.
- Highlighting runs only for drawn rows, cached by row; a Web Worker only if
  the step-1 numbers say tokenising still costs.
- Above a threshold (20k lines or 2 MB to start, tuned from the baseline) a
  file is shown collapsed with *Load diff*.
- The same for the Working copy's `HunkPane`, `PRDiffPane`, and any list in
  `FileHistoryView` or `BlameStrip` that grows with the repository.
- Search within a patch searches the row model and scrolls the match in.

## Step 4 — The wire (only if step 1 points there)

Check whether any answer is large enough to matter on its own — the
working-copy list, a commit's whole file list — and page it the way
`graph_request` pages the graph; confirm nothing asks for every file's hunks
up front.

## Step 5 — Re-measure

The same baseline, beside the first, in `agile/testing/TASK-052-baseline.md`.

## Files (step 1)

| File | Change |
| --- | --- |
| `src-tauri/src/timing.rs`, `lib.rs` | The buffer; the module. |
| `src-tauri/src/commands.rs` | `lock_session`, named `with_session`, `command_timings`, the test. |
| `src/lib/timing.svelte.ts`, `api.ts`, `types.ts` | Round trips, measures, the call. |
| `src/routes/+layout.svelte`, `src/lib/diff/DiffPane.svelte` | Navigation and diff measures. |
| `src/lib/settings/TimingsPanel.svelte`, `GodModeSection.svelte` | The panel. |
| `crates/spagitty-core/examples/baseline.rs` | The baseline's measurements. |

## Risks and rollback

- Step 1 changes no behaviour: a clock read around each hold and each call,
  and a bounded buffer. Rollback is a revert.
- Step 2 is the risky one, and waits.
