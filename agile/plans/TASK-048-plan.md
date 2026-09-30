<!-- SPDX-License-Identifier: GPL-3.0-or-later -->

# TASK-048 — Plan

**Item:** [`agile/items/TASK-048-commands-off-the-main-thread.md`](../items/TASK-048-commands-off-the-main-thread.md)

## Approach

`#[tauri::command]` becomes `#[tauri::command(async)]` on every `pub fn`
command in `commands.rs` except the eleven ordered writers. The macro then
answers through `respond_async_serialized`: the function runs inside a future
on Tauri's multi-threaded runtime instead of on the event loop. No signature
changes; the fourteen `async fn` forge commands are already off it.

The one place concurrency changes behaviour is the session. Every repository
command already takes the session mutex for its whole body, so they are still
serialized against each other, just not against painting. `open_repo` is the
exception: it reads the repository *before* taking the lock, and two of them
can now overlap. So:

- `AppState::opened: AtomicU64` is the latest ticket. `open_repo` takes one on
  entry; `open_as` does the work and, under the session lock, installs only if
  its ticket is still the latest, otherwise returns `Error::Superseded`.
  `recents::remember` moves after that check, so an overtaken open is not
  remembered.
- `close_repo` takes its ticket while holding the lock, so an open either
  installed before it (and is cleared) or checks after it (and is overtaken).
- `repo.svelte.ts` counts requests and applies an open's answer, or its
  failure, only if nothing was asked for since; `close` does the same and
  clears `busy`, which an overtaken open no longer does.

## Files

| File | Change |
| --- | --- |
| `src-tauri/src/commands.rs` | `(async)` on 101 commands; tickets; `open_as`; tests. |
| `crates/spagitty-core/src/error.rs` | `Error::Superseded`. |
| `src/lib/repo.svelte.ts` | Answers to replaced requests are ignored. |
| `src/lib/repo.test.ts` | Overlapping opens and closes. |

## Risks and rollback

- **Runtime workers.** A command waiting for the session lock holds a worker.
  The pool is one per core; the lock's holder needs none to finish, so this
  delays, it cannot deadlock.
- **Order among repository writes.** Two writes asked for without the webview
  waiting between them are serialized by the lock but may run in either order.
  The screens await each write before the next.
- Rollback is a revert.
