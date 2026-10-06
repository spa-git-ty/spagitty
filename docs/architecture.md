<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# Architecture

How Spagitty is put together, and why. Screen-by-screen state lives in
[screens.md](screens.md); how to run and test it lives in
[testing.md](testing.md).

## Three layers

```
src/                    SvelteKit, SPA mode. One store per screen.
  └── invoke ─────────► src-tauri/           Tauri commands, worker, watcher.
                          ├── calls ───────► crates/spagitty-core/        git, via gix.
                          ├── calls ───────► crates/spagitty-farm/        agents, via processes.
                          │                    └── calls ──► crates/spagitty-core/
                          └── calls ───────► crates/spagitty-extensions/  extension workers (FEAT-096)
                                               ├── calls ──► crates/spagitty-core/
                                               └── starts ─► extension workers and the tools they declare

crates/spagitty-process/  process trees: used by the farm and by the extension host
```

The farm and the extension host do not know about each other. The desktop
layer composes them: it supplies the extension host's forge and confirmation
services, and the farm's supplemental reviewer (FEAT-098), so neither crate can
reach into the other's state.

Each layer knows nothing about the one above it.

### `crates/spagitty-core`

Every git operation. No Tauri types, no window handles, no events — the crate
compiles and is tested without a GUI, and its examples
(`examples/graph-dump.rs`, `examples/diff-dump.rs`) run it from a terminal.

| Module | Holds |
| --- | --- |
| `repo.rs` | Opening a repository, describing it, reading HEAD |
| `graph.rs` | The log walk, lane assignment, lane colours, edges, `ROW_PITCH` |
| `refs.rs` | `RefIndex` — commit id to the refs pointing at it |
| `branches.rs` | Branch rows: drift, upstream, merged; checkout and create |
| `diff.rs` | Commit detail, per-commit file lists, per-file hunks |
| `conflicts.rs` | Index stages 1/2/3 of a conflicted path, and what operation is in progress |
| `rebase.rs` | The interactive-rebase todo list, its preview, and its execution through `GIT_SEQUENCE_EDITOR` |
| `ops.rs` | Every git operation that writes: reset, revert, cherry-pick, merge, rebase, tags, branch rename and delete, detached checkout, stash apply/pop/drop, fetch, push |
| `shell.rs` | The one place a `git` subprocess is spawned. Everything in `ops.rs` goes through it |
| `search.rs` | The filtered history walk behind Log search |
| `blame.rs` | Who last touched each line — the one read that goes through `git` |
| `stash.rs` | Stash entries, read from the reflog of `refs/stash` |
| `status.rs` | The working-copy status walk, and the counts the nav rail shows |
| `work.rs` | Changing the working copy: stage, unstage, commit |
| `identity.rs` | `user.name` and `user.email` per scope — read with `gix`, written with `git config` |
| `signing.rs` | Whether git will sign this commit and whether it can — `commit.gpgsign` is the authority, never a preference of Spagitty's |
| `clone.rs` | Where a clone will land, what is wrong with that, and what git's progress output means |
| `remotes.rs` | The remotes a repository knows about: add, rename, remove, retarget, each through `git remote` rather than a config edit |
| `reflog.rs` | Where a ref has been, and the three ways back to a point in it |
| `tags.rs` | Tags in one place: annotated and lightweight told apart, because the difference decides whether a message can exist |
| `record.rs` | What Spagitty actually ran — the buffer behind "Show the git command behind each action", written by `shell.rs` |
| `update.rs` | Whether there is a newer Spagitty than this one |
| `forge/` | The only part of the workspace that reaches a network. See below |
| `error.rs` | `Error`, whose `Display` text is user-facing |
| `fixture.rs` | Real repositories for tests, published behind the `fixture` feature so the Tauri layer can walk one too (TASK-003). Excluded from coverage |

**`forge/` is the network boundary, and it is as narrow as `shell.rs`'s.**
Reading pull requests could not survive the older promise that no HTTP client
was linked in either language, so the promise became a narrower one that is
still worth having and is still tested: exactly one client, `ureq`, declared
only in this crate, and reachable from exactly one file.

| File | Holds |
| --- | --- |
| `forge/http.rs` | The only file that may construct or use the HTTP client |
| `forge/github.rs` | One GraphQL request per refresh — REST would need 1 + 3N calls for the same rows |
| `forge/keychain.rs` | The personal access token, in the OS keychain and never in a configuration file |

The webview links no HTTP client, makes no request, and never holds a token.

Types crossing to the frontend derive `Serialize` with
`#[serde(rename_all = "camelCase")]`, and are mirrored by hand in
`src/lib/types.ts`. The two are kept in step deliberately rather than generated,
because the wire shape is small and a generator would be more machinery than the
problem needs.

### `crates/spagitty-farm`

The agent farm's control plane (FEAT-073): a goal, the tasks it was cut into,
and the agents working them. It orchestrates and nothing else — every git
operation it performs is `spagitty-core`'s, and it has no Tauri types either.

| Module | Holds |
| --- | --- |
| `model/` | `Farm`, `Goal`, `Task`, `AgentDefinition`, `AgentRun`, `Handoff`, `FarmEvent` — the data, and the status machine no path may skip |
| `agent/` | One adapter per provider, detection on `PATH`, and the per-repository registry |
| `workspace/` | A branch and a worktree per task, the leases that stop two agents touching one path, and the sweep for what is left behind |
| `execution/` | Starting an agent, reading it, stopping its whole process group; the transcript on disk; the narrator that turns a provider's stream into lines a person reads |
| `orchestrator/` | The dependency graph, the scheduler (a pure function: farm in, decisions out), the router, and the planner that turns a goal into tasks |
| `verification/` | The repository's own commands, run in the task's worktree |
| `review/` | A second agent reading the first one's change, and the decision it returns |
| `persistence/` | The farm on disk: JSON under `.spagitty/`, written by rename, events appended one object per line |
| `policy.rs` | The repository's `AGENTS.md` and its neighbours, attached to every prompt |
| `service.rs` | The API, and the only module that changes anything |

Three rules the crate is built around, each of which is load-bearing:

- **An agent saying "done" is not done.** There is no transition from an agent's
  own report to `Done`; verification and a review by a *different* agent are in
  the path and cannot be skipped.
- **Agents never talk to each other.** Every handoff goes through the service,
  so there is one audit trail and one place that decides what happens next.
- **Nothing an agent does reaches the user's checkout.** Every task runs in its
  own worktree on its own branch.

The Tauri layer's `farm.rs` mirrors `commands.rs`: it holds the open farm,
forwards, and turns `FarmEvent` into one webview event. Its commands are
`#[tauri::command(async)]` rather than plain, because they take locks that agent
threads hold and a blocking command runs on the thread that paints the window
(BUG-020).

### `crates/spagitty-process`

A process and everything it starts, stopped as one: a Unix process group, or a
Windows job object assigned while the child is still suspended. Moved out of
the farm (FEAT-096) so the extension host contains its workers and their tools
with the same code. Its test runs on all three operating systems in the
`farm processes` lane.

### `crates/spagitty-extensions`

The extension host (FEAT-096): manifests, packages, the registry, user state
(grants, enablement, consent), the JSON-RPC protocol, supervised workers,
operations, external tool runs, review snapshots, the review model and its gate,
and review history. No Tauri, no frontend, no farm. The contract it implements
is public: `schemas/extensions/`; the guides are in `docs/extensions/`.

| Module | Holds |
| --- | --- |
| `manifest.rs` | `extension.json` and every rule the schema states |
| `version.rs` | Manifest, API and application compatibility, and the target triple |
| `protocol.rs` | JSON-RPC 2.0 framing, one message per line |
| `worker.rs` | One worker: its tree, its stdin, a reader that never waits on the host, callbacks on threads of their own |
| `operations.rs` | Long-running work, ended exactly once |
| `tools.rs` | Declared tools: found on `PATH` or chosen, argv built from profiles, run in a tree |
| `zip.rs`, `package.rs` | The package container, read strictly; validation, extraction, packing |
| `registry.rs` | Bundled, installed and development extensions; install, update, rollback, uninstall |
| `storage.rs` | User state, and `.spagitty/extensions/` in the repository |
| `snapshot.rs` | What a review covers, pinned and hashed |
| `review.rs`, `history.rs` | The shared review model, the gate, and minimal records |
| `redact.rs` | Secrets removed on the way in |
| `host.rs` | The API the desktop calls, and the checked callbacks workers make |

**An extension is not sandboxed.** It is a native program with the user's
permissions. Process separation keeps a crash out of the window; capabilities
limit what the host's API does for it. The interface says so wherever an
extension is installed or turned on.

### `src-tauri`

Deliberately thin. It holds the open session — one repository at a time — and
forwards to the core.

| File | Holds |
| --- | --- |
| `commands.rs` | One `#[tauri::command]` per operation. No git logic. |
| `recents.rs` | The list of repositories the user has opened — application state, not git state |
| `settings.rs` | Spagitty's own behaviour toggles, stored beside the repository list |
| `about.rs` | Build identity, and the dependency license list generated by `../licenses.rs` at build time |
| `graph_worker.rs` | A thread that walks history and emits batches |
| `search_worker.rs` | A thread per query; starting one cancels the one before |
| `clone_worker.rs` | A thread per clone; it owns the `git` process, so cancelling is a signal rather than a request |
| `farm.rs` | The farm's commands and its event bridge. State of its own, with a different lifetime from the graph session |
| `extensions.rs` | The extension host's commands, its event bridge, and confirmations the person answers in the window (FEAT-096) |
| `forge_bridge.rs` | Forge reads and writes made on an extension's behalf, with the token kept in the backend |
| `watch.rs` | Filesystem watcher over the git directory |
| `platform.rs` | Host facts that must be true before the webview starts |
| `lib.rs` | Command registration |

Commands registered today, grouped by what they are for:

- **The walk** — `open_repo`, `close_repo`, `graph_request`, `graph_restart`,
  `graph_visibility`, `snapshot`, `metrics`.
- **Reading** — `commit_detail`, `commit_diff`, `file_diff`, `working_copy`,
  `working_diff`, `head_message`, `branches`, `stashes`, `blame`, `conflicts`,
  `conflict_sides`, `search_start`, `search_stop`.
- **Writing** — `stage`, `unstage`, `stage_hunk`, `unstage_hunk`, `commit`,
  `checkout`, `checkout_detached`, `create_branch`, `rename_branch`,
  `delete_branch`, `create_tag`, `delete_tag`, `reset`, `revert`,
  `cherry_pick`, `integrate`, `rebase_onto`, `rebase_todo`, `rebase_preview`,
  `rebase_run`, `stash_push`, `stash_action`, `fetch`, `push`.
- **Application state** — `recent_repos`, `forget_repo`, `clone_plan`,
  `clone_start`, `clone_release`, `about`, `licenses`, `identity`,
  `set_identity`, `settings`, `set_settings`, `launch_path`.
- **The farm** — `farm_open`, `farm_close`, `farm_snapshot`, `farm_events`,
  `farm_stale`, `farm_detect_agents`, `farm_save_agent`, `farm_remove_agent`,
  `farm_create`, `farm_configure`, `farm_start`, `farm_pause`, `farm_cancel`,
  `farm_write_policy`, `farm_add_task`, `farm_edit_task`, `farm_delete_task`,
  `farm_ready_task`, `farm_ready_tasks`, `farm_discard_tasks`,
  `farm_assign_task`, `farm_cancel_task`, `farm_retry_task`,
  `farm_run_task`, `farm_task_detail`, `farm_transcript`, `farm_merge_task`,
  `farm_review_task`, `farm_verify_task`, `farm_plan`, `farm_cancel_plan`,
  `farm_decompose`, `farm_sweep`.
- **Extensions** (FEAT-096) — `extensions_list`, `extensions_inspect`,
  `extensions_install`, `extensions_rollback`, `extensions_uninstall`,
  `extensions_attach`, `extensions_restart`, `extensions_enable`,
  `extensions_disable`, `extensions_set_grant`, `extensions_set_setting`,
  `extensions_choose_executable`, `extensions_detect_tool`,
  `extensions_run_command`, `extensions_preview_review`,
  `extensions_start_review`, `extensions_cancel`, `extensions_reviews`,
  `extensions_set_disposition`, `extensions_delete_reviews`, `extensions_panel`,
  `extensions_suggested_bases`, `extensions_confirm`, `extensions_location`.

`platform.rs` is the other file that is not about git at all. It arranges the
webview's environment as the first statement of `run` — before the builder, and
while the process is still single-threaded, which is the only state in which
`set_var` is sound.

It sets `WEBKIT_FORCE_COMPOSITING_MODE=1`, which keeps the whole page on the
GPU rather than letting WebKitGTK fall back to software for the parts its
heuristics misjudge.

It deliberately does **not** set `WEBKIT_DISABLE_DMABUF_RENDERER`, and that is a
reversal (FEAT-055). BUG-004 set it to `1` on every Linux start, because
WebKitGTK's DMABuf renderer cannot allocate a GBM buffer on some driver and
compositor combinations and a webview with no frame to present is a white window
for the whole session. The cost of the workaround was recorded in
`agile/plans/BUG-004-plan.md` and then lived with for too long: with the
renderer off, every Linux host repaints **through shared memory**, on the CPU —
including the hosts that never had the bug. On a real window that is a frame
that lags the pointer during a resize and a history that scrolls at a fraction
of the display's rate.

The accelerated path is the default now. A host that hits the GBM bug — a blank
window, `Failed to create GBM buffer` on stderr — sets
`WEBKIT_DISABLE_DMABUF_RENDERER=1` in its own environment, which the process
still honours, along with an explicit value for either variable in either
direction.

`graph_visibility` is the one worth naming here: hide, solo, smart visibility
and pin-to-left all resolve to a **root set** for a fresh walk rather than to a
filter over rows the frontend already has. Filtering client-side would leave the
lanes drawing edges between commits that are no longer parent and child.

### `src`

SvelteKit in SPA mode — `ssr = false`, `prerender = false` in
`src/routes/+layout.ts`. Tauri serves a static bundle from disk; there is no
server.

- `src/lib/api.ts` calls `invoke` for the core's commands. Two subsystems have
  a bridge of their own, each the only caller in its directory and each held to
  that by a test: `src/lib/farm/api.ts` and `src/lib/extensions/api.ts`.
- `src/lib/extensions/` draws everything an extension contributes from data:
  commands, actions, settings and three panel renderers. No extension code runs
  in the webview, and nothing an extension sends is handed to the browser as
  markup.
- One directory per screen under `src/lib/` — `graph/`, `diff/`, and one per
  screen as it is built — each with a `store.svelte.ts` and its components.
- `src/lib/chrome/` is the persistent frame: title bar, toolbar, nav rail,
  resize edges.
- `src/lib/ui/` holds what more than one screen needs: `Menu` (every right-click
  menu in the application is this component with a different list), `Dialog`
  (every confirmation) and `Notice` (every result). The last two are mounted
  **once, by the shell** — an operation started on the graph can finish after
  the user has navigated away, and a dialog owned by a screen would take the
  question with it.
- `src/lib/delight/` is the achievement layer (FEAT-072), and it is
  deliberately a **sink**: git actions push events into it and it pushes
  nothing back. It holds no reference to `repo` — the shell tells it which
  repository and which identity it belongs to — because almost everything that
  reports a git operation is reachable from `repo`, and importing it back would
  close a cycle through half the frontend. `engine.ts` is pure, so the rule set
  is tested as a table rather than by driving the application, and nothing in
  the directory can fail a git operation: every call site is `void`-ed and
  `record` swallows its own failure. Its record lives in `localStorage`, keyed
  by repository path, beside the theme and the column layouts — a lost record
  is a record of badges, not of work, and git remains the only authority on
  what actually happened.
- `src/lib/palette/` is a registry, not a list. `commands.ts` contributes the
  shell's commands; a feature contributes its own next to itself, so no single
  file has to import every feature in order to name it.
- Stores export a single object with getters, never the `$state` variables
  themselves, so a screen cannot write another screen's state by accident.

## Two things the layers share

**The row pitch is defined twice on purpose.** Lane elbows are described in row
units in `crates/spagitty-core/src/graph.rs`; the stylesheet needs the same
number in CSS pixels from `src/lib/metrics.ts`. Rather than let them drift, the
frontend fetches the Rust value at boot through the `metrics` command and logs
an error if they disagree.

**Structural numbers live in `src/lib/metrics.ts`** and are published as CSS
custom properties by `applyMetrics`. There is no `height: 26px` in any
component, and no second `26` anywhere in the frontend.

## The `git` binary boundary

`crates/spagitty-core/src/shell.rs` is the only module in the core that spawns
a process. Outside the core, two more places start processes on purpose and say
so: the farm, for agents and verification commands, and the extension host,
for extension workers and the tools their manifests declare — both through
`spagitty-process`, so every tree can be ended as a whole. Its header carries the full table and the reasoning; the rule
in one sentence:

> If the operation mutates state that the wider git ecosystem also reads, or
> delegates to something outside the repository, it shells out to `git`.
> Read-only history questions are answered in-process with `gix`.

So reads — log walking, refs, diffing, status, the index's conflict stages — are
`gix`. Interactive rebase execution, hooks, LFS, submodule recursion, credential
helpers, committing, staging, checkout, stash push, writing git config and
cloning are `git`. The table in that header is extended in the same change that
adds an operation to it.

`shell::clone_start` is the one function there that does not wait for git to
finish: a clone runs for minutes, reports progress as it goes, and has to be
cancellable, so the caller owns the child process. Everything else in the module
is a command that ends.

**One read breaks the rule, and it is written down rather than quietly done.**
`shell::blame` shells out because `gix::blame` 0.16 — the newest published
version — panics on an ordinary history shape rather than returning an error:
a file blamed at a merge commit whose history contains an intervening commit
that left the file alone. Every diff algorithm and both rename settings do it.
The exception carries an end condition: blame moves back in-process when the
upstream defect is fixed. It is the only read in the workspace that spawns a
process.

`gix` is MIT/Apache-2.0, which links cleanly into a GPL-3 program.

### What ran, recorded at the boundary

Because there is one module that spawns, there is one place that knows what was
executed. `crates/spagitty-core/src/record.rs` holds it: a process-wide ring
buffer of the last 200 executions, written by `shell.rs` itself as each command
finishes — argv, outcome, exit code, duration.

That location is the whole point of the feature. A screen composing its own
"the command behind this button" string would be describing what it *asked
for*: it would not know that a fetch carries `--prune --progress`, that a force
push is `--force-with-lease`, or that reverting a merge gained `-m 1` on the way
down. The record is written where the process is started, so it is evidence
rather than a claim, and every spawn goes through `shell::finish` or
`shell::record_spawn` so a new one cannot quietly skip it.

Two consequences worth stating:

- **Reads are absent, deliberately.** Log walking, refs, diff and status are
  in-process and have no command line. Nothing is synthesised for them, and the
  panel says so — inventing a `git log` Spagitty never ran would teach the user
  an invocation that does not exist.
- **Credentials are removed on the way in, not on the way out.** A clone URL can
  carry a token (`https://user:token@host/repo.git`), so `record::redact` strips
  the userinfo before the entry is stored. An entry that never held the secret
  cannot leak it through the copy button or a later reader who assumes display
  was doing the work.

`src-tauri/src/command_log.rs` registers the one observer at startup and
forwards each entry as the `git-command` event; `commands::git_commands(since)`
is the catch-up read for what happened before the panel was opened. The webview
side is `src/lib/commandlog/`, mounted once by the shell, revealed by the
Settings toggle "Show the git command behind each action" (FEAT-020).

## Data flow: opening a repository

1. `repo.open(path)` calls `open_repo`.
2. Rust opens the repository, builds the `RefIndex`, computes the cheap counts,
   spawns a graph worker and a filesystem watcher, and returns
   `{ info, counts, token }`.
3. The worker does not walk anything yet. The graph store asks for rows with
   `graph_request(token, count)`.
4. Rows arrive as `graph-rows` events in batches; `graph-done` ends the walk.
5. Rows carrying a token other than the current one are dropped. That is how a
   superseded walk is discarded without cancellation races.
6. The watcher emits `repo-changed`. Refs moving debounces into a re-read and a
   re-walk; a worktree change re-reads the counts only.

## Errors

`Error` crosses to JavaScript as a plain string, so its `Display` text is user
facing — plain, specific, no stack-trace jargon. Screens show that string
directly rather than substituting a message of their own.

## Branching and releases

Git Flow, per Amendments 13 to 15: `main` and `dev` are protected, work happens
on `feature/`, `task/`, `bugfix/`, `hotfix/` and `release/` branches named after
their work item ID in `agile/`.

**Current deviation, recorded on purpose.** The repository has no remote yet.
No pull request can be opened, so nothing can legitimately reach `dev`, and a
branch cut from `dev` would lack the test configuration and core modules that
later items depend on. Branches therefore **stack**: each item is cut from the
previous item's branch, in item order. When a remote exists they merge into
`dev` by pull request in that same order.

CI is six ordered gates, described in [ci.md](ci.md). They have not run yet:
there is no remote to run them on.
