<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-052 — Nothing waits in line: measure, unlock reads, virtualize the diff

**Status:** Open — step 1 (measure) built on `task/TASK-052-nothing-waits-in-line`; stopped for review before step 2, as the item asks.
**Branch:** `task/TASK-052-nothing-waits-in-line`
**Screens:** Graph (1A), Diff, Working copy, File history, Blame strip; every
screen reached by navigation.
**Raised by:** the author, after TASK-048 merged: navigation still feels slow,
and large repositories feel heavy. This item is the gate before any decision
about replacing the webview (GPUI or otherwise) — that decision is out of scope
here and should be made against the numbers this item produces.

## Problem

TASK-048 moved the repository commands off the main thread, so the window no
longer stops painting while git runs. Three causes of slowness remain, and
TASK-048 named two of them as non-scope.

1. **One lock, held for the whole operation.** `AppState::with_session` takes
   `session: Mutex<Option<Session>>` and holds the guard while the closure runs
   — the whole diff, blame, status count or walk. Every command queues behind
   the slowest one in flight. Selecting a commit while a long blame is running
   waits for the blame; switching screens while status is being counted waits
   for the count. Off the main thread this no longer freezes the window, but
   the screen still waits for its answer, which is what "slow navigation" is.
2. **The diff is rendered whole.** `src/lib/diff/DiffPane.svelte` renders every
   line of every hunk with `{#each}` (unified and split views), and
   `src/lib/diff/highlight.ts` tokenizes each line on the webview's main thread.
   The graph is virtualized (`visibleRange` in `src/lib/graph/lanes.ts`, used by
   `CommitRows.svelte`); the diff is not. A large commit or a generated file
   builds tens of thousands of DOM nodes and tokenizes all of them before
   anything paints. `PRDiffPane.svelte`, `FileHistoryView.svelte` and
   `BlameStrip.svelte` are suspected of the same and must be checked.
3. **Nobody has measured.** There is no timing on commands, no record of how
   long a command waited for the lock versus ran, and no frontend timing for
   render cost. Every fix above is a hypothesis until it is.

## Change

Do the steps in order. Each step is its own commit carrying this item's id, and
step 1 lands before any optimization so that every later step can show a
before-and-after.

### Step 1 — Measure (no behaviour change)

- Add `tracing` spans around every `#[tauri::command(async)]` in
  `src-tauri/src/commands.rs` and `src-tauri/src/farm.rs`. Record two durations
  per call: **lock wait** (time to acquire the session lock) and **run** (time
  inside the closure). Emit them through the existing command log
  (`spagitty-core::record`) or a new debug-only channel; do not add telemetry
  that leaves the machine (Privacy section of the README stands).
- In the webview, wrap screen navigation and diff rendering in
  `performance.mark`/`measure` and surface them in God mode (Settings), so the
  author can read them without devtools.
- Write `agile/testing/TASK-052-baseline.md` with numbers from a large
  repository. Suggested reference repositories: `rust-lang/rust` or the Linux
  kernel for history depth, and one commit touching a generated file (a
  lockfile) of 10k+ lines for diff size. Record: open, switch to each screen,
  select a commit, open a large diff, scroll it, blame a long file.

### Step 2 — Stop holding the lock for the work

- `ThreadSafeRepository` is cheap to clone (it shares its object store). Change
  `with_session` so the guard is held only long enough to clone what the
  closure needs — the repository handle, the path, and the session's open
  ticket — then released before the closure runs.
- Reads run concurrently. **Writes must still be serialized**: stage, unstage,
  discard, commit, checkout, branch/tag/stash operations, rebase, conflict
  resolution, submodule and worktree changes. Introduce a separate write lock
  (e.g. `write: Mutex<()>` on `AppState`, or an `RwLock` where reads take the
  read side) and make every writing command take it. Classify every command in
  `commands.rs` as read or write; add a test that lists the writers, in the same
  spirit as the TASK-048 test that lists the main-thread commands, so a new
  command must be classified on purpose.
- Keep TASK-048's ticket guarantee: a read that finishes after its repository
  was closed or replaced must not deliver a result for the wrong repository.
  Carry the ticket into the closure and drop the result (with `Superseded`) if
  it is no longer the latest.
- The graph worker and the filesystem watcher already have their own threads;
  confirm a watcher-triggered graph restart does not now race a write. If it
  can, it takes the read side.

### Step 3 — Virtualize the diff

- Render only the visible rows of `DiffPane.svelte` (unified and split), using
  the same `visibleRange` approach as `CommitRows.svelte` with a small overscan.
  Flatten hunks into one row model (hunk header rows + line rows) so a single
  virtual list covers the whole file.
- Highlight lazily: tokenize only rows in range, cache tokens per row, and
  invalidate on file change. If profiling in step 1 shows tokenizing is still
  material, move it to a Web Worker.
- Above a threshold (start at 20k lines or 2 MB, tune from step 1), show the
  file collapsed with an explicit "Load diff" action rather than rendering it
  on selection.
- Apply the same to `PRDiffPane.svelte`. Check `FileHistoryView.svelte` and
  `BlameStrip.svelte` and virtualize any list that grows with the repository.
- Search inside the patch text must keep working across rows that are not
  rendered: search the row model, then scroll the match into range.

### Step 4 — Check the wire (only if step 1 points here)

- If any command's serialized payload is large (whole working-copy lists, whole
  commit diffs with every hunk), page or slice it the way `graph_request`
  already pages the graph: `commit_diff` returns the file list, `file_diff`
  returns hunks — confirm the frontend does not call `file_diff` for every file
  up front.

### Step 5 — Re-measure

- Repeat the step-1 baseline on the same repositories and record the results
  next to it in `agile/testing/TASK-052-baseline.md`.

## Non-scope

- Replacing Tauri or the webview. The step-5 numbers are the input to that
  decision; this item does not make it.
- Making individual git operations in `spagitty-core` faster (gix tuning,
  caching walks). If step 1 shows a single operation dominates, raise a
  separate item.
- The farm's internals; only its commands get timing spans.
- Visual changes. Virtualized screens must look and behave as they do now.

## Acceptance criteria

- Every repository command reports lock-wait and run time, visible without
  devtools.
- On the reference large repository, selecting a commit or switching screens
  while a long blame or status count is running returns without waiting for it.
- Two writes never run at the same time; a test lists every writing command and
  fails if a new command is unclassified.
- An open followed quickly by another open or a close still leaves the right
  repository on screen (TASK-048's criteria still pass).
- Opening a 10k-line diff paints its first screen without rendering every line;
  scrolling it stays smooth; search inside it still finds matches anywhere.
- Frontend branch coverage stays at or above 65% and Rust line coverage at or
  above 70% (docs/testing.md).
- `CHANGELOG.md` has an `## [Unreleased]` entry in the same change as the work
  (Amendment 20).
- `agile/testing/TASK-052-baseline.md` holds before-and-after numbers.

## Notes for the agent

- Read `AGENTS.md`, `docs/architecture.md`, `docs/AMENDMENTS.md` and
  `agile/items/TASK-048-commands-off-the-main-thread.md` before changing
  anything; this item builds on TASK-048 and must not undo it.
- Respect the seams: only `src/lib/api.ts` calls the backend;
  `crates/spagitty-core` never imports Tauri.
- Write a plan to `agile/plans/TASK-052-plan.md` and stop for review before
  step 2. Step 2 changes the concurrency model and is the riskiest part.
