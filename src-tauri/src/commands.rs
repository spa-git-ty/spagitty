// SPDX-License-Identifier: GPL-3.0-or-later

//! Tauri commands. This layer is deliberately thin: it holds the open session,
//! forwards to `spagitty-core`, and converts errors to strings for the webview.
//! No git logic lives here.

use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;

use serde::Serialize;
use spagitty_core::avatars;
use spagitty_core::blame::{self, Blame};
use spagitty_core::branches::{self, BranchRow};
use spagitty_core::clone::{self, Plan};
use spagitty_core::conflicts::{self, ConflictSides, ConflictState};
use spagitty_core::diff::{self, CommitDetail, CommitDiff, FileDiff, Side};
use spagitty_core::files;
use spagitty_core::forge::review::ReviewVerdict;
use spagitty_core::forge::{self, Account, Kind, MergeMethod, PullRequest, Repo};
use spagitty_core::graph::{GraphOrder, ROW_PITCH};
use spagitty_core::highlight;
use spagitty_core::home::{self, Overview};
use spagitty_core::identity::{self, Identity, Key, Scope};
use spagitty_core::investigate;
use spagitty_core::merger;
use spagitty_core::ops::{self, Integration, ResetMode, StashAction};
use spagitty_core::pull;
use spagitty_core::rebase::{self, Edit, Preview, Todo};
use spagitty_core::record::{self, Executed};
use spagitty_core::reflog;
use spagitty_core::refs::RefIndex;
use spagitty_core::remerge;
use spagitty_core::remotes;
use spagitty_core::repo::{self, RepoInfo, RepoSummary};
use spagitty_core::search::Query;
use spagitty_core::shell::PullMode;
use spagitty_core::signing::{self, Signing};
use spagitty_core::stash::{self, StashEntry};
use spagitty_core::status::{self, RepoCounts, WorkingCopy};
use spagitty_core::submodules::{self, Submodule};
use spagitty_core::tags;
use spagitty_core::tools::{self, ExternalToolsConfig};
use spagitty_core::untrack;
use spagitty_core::update;
use spagitty_core::usages;
use spagitty_core::work;
use spagitty_core::worktrees::{self, Worktree};
use spagitty_core::{Error, Result};
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

use crate::about::{About, Licenses};
use crate::accounts;
use crate::clone_worker::{self, CloneWorker};
use crate::graph_worker::{self, GraphWorker};
use crate::merger_state;
use crate::network_worker::{self, NetworkWorker};
use crate::profiles;
use crate::rebase_worker::{self, RebaseWorker};
use crate::recents;
use crate::review_state;
use crate::search_worker::{self, SearchWorker};
use crate::settings::Settings;
use crate::timing;
use crate::watch::{self, RepoWatcher};

/// One open repository and everything running against it.
///
/// Field order matters on drop: the graph worker and the watcher both hold an
/// `AppHandle<R>` and must be shut down before the repository handle goes away.
struct Session {
    path: PathBuf,
    repo: spagitty_core::gix::ThreadSafeRepository,
    graph: GraphWorker,
    /// The refs the graph is rooted at, empty for every branch.
    ///
    /// Held here rather than in the webview because a walk is restarted by the
    /// filesystem watcher as well as by the user: a ref moving must not quietly
    /// reset Solo or Smart Branch Visibility back to showing everything.
    visible: Vec<String>,
    /// The refs whose lanes are held open on the left — "pin to left".
    pinned: Vec<String>,
    /// How the walk sequences the same DAG: date order or branch (topo) order.
    order: GraphOrder,
    /// The ticket of the open that installed it. A read taken from it is
    /// delivered only while it is still the session (TASK-052).
    ticket: u64,
    _watcher: Option<RepoWatcher>,
}

/// The open repository as a command works on it: taken from the session
/// under its lock, which is let go before the work starts (TASK-052).
struct Open {
    path: PathBuf,
    repo: spagitty_core::gix::ThreadSafeRepository,
    ticket: u64,
}

/// The commands that change the repository. They run one at a time; reads
/// run beside them and beside each other. A command not named here reads, and
/// `every_command_is_read_or_write_on_purpose` refuses a new one that is in
/// neither list.
const WRITERS: &[&str] = &[
    "stage",
    "unstage",
    "stage_hunk",
    "unstage_hunk",
    "discard",
    "discard_hunk",
    "commit",
    "set_hooks_enabled",
    "checkout",
    "create_branch",
    "stash_push",
    "rebase_run",
    "rebase_continue",
    "rebase_skip",
    "rebase_abort",
    "reset",
    "revert",
    "cherry_pick",
    "integrate",
    "rebase_onto",
    "checkout_detached",
    "rename_branch",
    "delete_branch",
    "create_tag",
    "delete_tag",
    "stash_action",
    "pull",
    "pull_branch",
    "conflict_take",
    "conflict_resolve_region",
    "conflict_write",
    "conflict_resolve",
    "conflict_settle",
    "conflict_continue",
    "conflict_abort",
    "merger_land",
    "merger_rebase_open",
    "merger_rebase_continue",
    "merger_rebase_skip",
    "merger_rebase_abort",
    "merger_rebase_finish",
    "tag_create",
    "tag_delete",
    "tag_retag",
    "remote_add",
    "remote_rename",
    "remote_remove",
    "remote_set_url",
    "submodule_update",
    "submodule_sync",
    "submodule_deinit",
    "launch_external_merge",
    "worktree_add",
    "worktree_remove",
    "worktree_lock",
    "worktree_unlock",
    "worktree_prune",
    "review_check_out",
    "untrack",
];

/// The commands that only read. Listed so that a new command is put in one
/// list or the other by somebody deciding, not by default.
#[cfg(test)]
const READERS: &[&str] = &[
    "graph_request",
    "snapshot",
    "commit_detail",
    "commit_diff",
    "file_diff",
    "working_copy",
    "working_diff",
    "binary_file_diff",
    "binary_working_diff",
    "hooks",
    "head_message",
    "branches",
    "home_overview",
    "stashes",
    "rebase_todo",
    "rebase_progress",
    "start",
    "search_start",
    "blame",
    "conflicts",
    "conflict_sides",
    "tags",
    "reflog",
    "reflog_refs",
    "remotes",
    "submodules",
    "launch_external_diff",
    "worktrees",
    "review_files",
    "review_file",
    "files_resolve",
    "files_tree",
    "files_read",
    "files_find",
    "files_search",
    "files_usages",
    "files_before",
    "files_blame",
    "files_spans",
    "investigate_branch",
    "investigate_compare",
    "investigate_hunk",
    "investigate_life",
    "investigate_today",
    "terminal_links",
];

#[derive(Default)]
pub struct AppState {
    session: Mutex<Option<Session>>,
    /// Taken by every one of the [`WRITERS`] for the whole of its work, so two
    /// writes never run at once (TASK-052). Reads never take it: they read
    /// what git has on disk, as they would with git running beside Spagitty.
    /// The session lock is held only to take the repository handle.
    write: Mutex<()>,
    next_token: AtomicU64,
    /// The query running right now, if any. Replacing it cancels the one it
    /// replaces — a search is restartable, not resumable.
    search: Mutex<Option<SearchWorker>>,
    /// The todo list the Rebase screen is planning against.
    ///
    /// Kept here rather than round-tripped through the webview on every
    /// keystroke: the preview is recomputed on every edit, and sending the
    /// whole list back each time would be both wasteful and a way for the
    /// screen to plan against a list the repository never produced.
    rebase_todo: Mutex<Option<Todo>>,
    /// The rebase running right now, if any.
    ///
    /// Held so that a second one can be refused and so that Abort can wait for
    /// the worker before unwinding the state it is reading. Dropping it waits;
    /// nothing cancels it, for the reason written out in `rebase_worker`.
    rebase: Mutex<Option<RebaseWorker>>,
    /// The fetch or push running right now, if any (FEAT-018).
    ///
    /// One at a time, and refused rather than queued: two would give the screen
    /// two sets of progress to tell apart, and git would be contending for the
    /// same refs regardless.
    network: Mutex<Option<NetworkWorker>>,
    /// The clone running right now, if any.
    ///
    /// One at a time: a second clone is refused rather than queued. Two is not
    /// a workflow anyone asked for, and the state it needs — a list of running
    /// clones, each with its own progress — is more machinery than the problem
    /// has.
    clone: Mutex<Option<CloneWorker>>,
    /// The ticket of the latest open or close asked for (TASK-048).
    ///
    /// Commands run off the main thread, so two opens can overlap: a large
    /// repository asked for first can finish reading after a small one asked
    /// for second. Each open takes a ticket before it reads anything and
    /// installs its session only while that ticket is still the latest. A close
    /// takes one too, so an open it overtook cannot reopen a repository the
    /// user has just closed.
    opened: AtomicU64,
}

/// The session lock, held, and timed (TASK-052).
///
/// How long it took to get and how long it was held are kept when it is let
/// go, under the command that took it — the two numbers that say whether a
/// screen waited for its own work or for somebody else's.
struct Held<'a> {
    guard: std::sync::MutexGuard<'a, Option<Session>>,
    command: &'static str,
    waited: std::time::Duration,
    since: std::time::Instant,
}

impl std::ops::Deref for Held<'_> {
    type Target = Option<Session>;

    fn deref(&self) -> &Self::Target {
        &self.guard
    }
}

impl std::ops::DerefMut for Held<'_> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.guard
    }
}

impl Drop for Held<'_> {
    fn drop(&mut self) {
        timing::push(self.command, self.waited, self.since.elapsed());
    }
}

impl AppState {
    /// Take the session lock for `command`, timing the wait and the hold.
    fn lock_session(&self, command: &'static str) -> Held<'_> {
        let asked = std::time::Instant::now();
        let guard = self.session.lock().expect("session lock");
        let since = std::time::Instant::now();
        Held {
            guard,
            command,
            waited: since - asked,
            since,
        }
    }

    /// Run `f` against the open repository. `command` names it in the
    /// timings and says whether it writes ([`WRITERS`]).
    ///
    /// The session lock is held only to take the repository handle (TASK-052):
    /// a quick read no longer waits behind a slow one, or behind a write. A
    /// write waits for the write in flight. A read whose repository was
    /// closed or replaced while it ran is [`Error::Replaced`], so its answer
    /// never lands on the next repository's screen; a write that finished is
    /// reported as it is, since what it did is done.
    fn with_session<T>(
        &self,
        command: &'static str,
        f: impl FnOnce(&Open) -> Result<T>,
    ) -> Result<T> {
        self.with_turn(command, WRITERS.contains(&command), f)
    }

    /// [`AppState::with_session`], told whether `command` writes.
    fn with_turn<T>(
        &self,
        command: &'static str,
        writes: bool,
        f: impl FnOnce(&Open) -> Result<T>,
    ) -> Result<T> {
        let asked = std::time::Instant::now();
        let open = {
            let guard = self.session.lock().expect("session lock");
            let session = guard.as_ref().ok_or(Error::NoRepository)?;
            Open {
                path: session.path.clone(),
                repo: session.repo.clone(),
                ticket: session.ticket,
            }
        };
        let alone = writes.then(|| self.write.lock().expect("write lock"));
        let since = std::time::Instant::now();
        let done = f(&open);
        drop(alone);
        timing::push(command, since - asked, since.elapsed());
        if !writes && !self.still_open(open.ticket) {
            return Err(Error::Replaced(open.path));
        }
        done
    }

    /// Run `f` against the session itself, holding its lock while it runs:
    /// for what lives in the session rather than the repository, the graph
    /// worker above all. `f` must be quick.
    fn with_live_session<T>(
        &self,
        command: &'static str,
        f: impl FnOnce(&Session) -> Result<T>,
    ) -> Result<T> {
        let held = self.lock_session(command);
        let session = held.as_ref().ok_or(Error::NoRepository)?;
        f(session)
    }

    /// Is the session installed by `ticket` still the open one?
    fn still_open(&self, ticket: u64) -> bool {
        self.session
            .lock()
            .expect("session lock")
            .as_ref()
            .is_some_and(|session| session.ticket == ticket)
    }

    /// Where the open repository is, for work that takes longer than the
    /// session should be held for: a recipe's run (FEAT-133).
    pub(crate) fn open_path(&self, command: &'static str) -> Result<PathBuf> {
        self.with_session(command, |session| Ok(session.path.clone()))
    }

    /// A new ticket, which is now the latest. See [`AppState::opened`].
    fn ticket(&self) -> u64 {
        self.opened.fetch_add(1, Ordering::SeqCst) + 1
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenResult {
    pub info: RepoInfo,
    pub counts: RepoCounts,
    /// Identifies this walk. Rows from any other token are stale and dropped.
    pub token: u64,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Snapshot {
    pub info: RepoInfo,
    pub counts: RepoCounts,
}

/// What is known about one author, beyond what the commit already said.
///
/// Two independent answers rather than one optional record, because they are
/// found in different ways and either can be present without the other: a
/// no-reply address yields a handle with no request at all and may still have
/// no picture, and an ordinary address can have a picture and no handle
/// anybody could name without asking a host who it belongs to.
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AvatarAnswer {
    /// The account name, when the address is one that spells it out.
    pub handle: Option<String>,
    /// The picture, as a `data:` URL ready to draw.
    pub picture: Option<String>,
    /// No answer yet, for a reason that may pass: offline, rate limited, a
    /// server error. The frontend asks again later rather than settling on the
    /// generated face for the session (FEAT-081).
    pub retry: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Metrics {
    /// Mirrored by ROW_PITCH in src/lib/metrics.ts. The frontend asserts these
    /// agree at boot, so the two definitions cannot silently drift apart.
    pub row_pitch: u32,
}

/// Open a repository (or the one containing `path`) and start its graph worker.
///
/// Opening does not walk anything. The worker waits for the first
/// [`graph_request`] before it touches history.
///
/// `order` is how that walk will sequence commits once it starts. Omitted, it
/// is date order, which is what every release before this drew.
#[tauri::command(async)]
pub fn open_repo<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    path: PathBuf,
    order: Option<GraphOrder>,
) -> Result<OpenResult> {
    let ticket = state.ticket();
    open_as(app, state, path, order, ticket)
}

/// [`open_repo`] once its ticket is taken, so a test can overtake it.
fn open_as<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    path: PathBuf,
    order: Option<GraphOrder>,
    ticket: u64,
) -> Result<OpenResult> {
    let sync = repo::open_sync(&path)?;
    let local = sync.to_thread_local();

    let info = repo::info(&local)?;
    let refs = RefIndex::build(&local)?;
    let counts = status::counts(&local, &refs)?;
    let git_dir = local.git_dir().to_path_buf();
    let order = order.unwrap_or_default();

    let token = state.next_token.fetch_add(1, Ordering::Relaxed);
    let graph = graph_worker::spawn(
        app.clone(),
        info.path.clone(),
        token,
        Vec::new(),
        Vec::new(),
        order,
    );
    let watcher = watch::watch(app.clone(), &git_dir, local.workdir());

    let mut slot = state.lock_session("open_as");
    // Overtaken while it was reading: the worker and watcher it started are
    // dropped, and joined, on the way out.
    if state.opened.load(Ordering::SeqCst) != ticket {
        return Err(Error::Superseded(info.path));
    }

    // Opening is the only way a repository joins the list. Spagitty never goes
    // looking for repositories on its own.
    recents::remember(&app, &info.path);

    // Replacing the session drops the previous worker and watcher, which joins
    // their threads before we return.
    *slot = Some(Session {
        path: info.path.clone(),
        repo: sync,
        graph,
        visible: Vec::new(),
        pinned: Vec::new(),
        order,
        ticket,
        _watcher: watcher,
    });

    Ok(OpenResult {
        info,
        counts,
        token,
    })
}

/// Ask the walker for `count` more rows. Returns immediately; rows arrive on
/// the `graph-rows` event.
#[tauri::command(async)]
pub fn graph_request(state: State<'_, AppState>, token: u64, count: usize) -> Result<()> {
    state.with_live_session("graph_request", |session| {
        // A request against a walk that has been replaced is stale, not an
        // error — it just raced a refresh.
        if session.graph.token() == token {
            session.graph.request(count);
        }
        Ok(())
    })
}

/// Restart the walk after refs moved. Produces a new token; the UI clears its
/// rows and starts again.
#[tauri::command(async)]
pub fn graph_restart<R: Runtime>(app: AppHandle<R>, state: State<'_, AppState>) -> Result<u64> {
    let mut guard = state.lock_session("graph_restart");
    let session = guard.as_mut().ok_or(Error::NoRepository)?;

    let token = state.next_token.fetch_add(1, Ordering::Relaxed);
    restart_walk(app, session, token);
    Ok(token)
}

/// Choose which refs the graph is rooted at, and restart the walk.
///
/// An empty list is every branch. Hide, Solo and Smart Branch Visibility are
/// all the same command with a different list, computed on screen from the
/// branches it is already showing — the backend does not need to know which of
/// the three the user pressed, only what they want to see.
#[tauri::command(async)]
pub fn graph_visibility<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    refs: Vec<String>,
    pinned: Vec<String>,
) -> Result<u64> {
    let mut guard = state.lock_session("graph_visibility");
    let session = guard.as_mut().ok_or(Error::NoRepository)?;

    session.visible = refs;
    session.pinned = pinned;
    let token = state.next_token.fetch_add(1, Ordering::Relaxed);
    restart_walk(app, session, token);
    Ok(token)
}

/// Choose how the graph sequences commits, and restart the walk.
///
/// Date order is `git log`: newest first, parallel histories interleaved.
/// Branch order is `git log --topo-order`: a line of history stays together.
/// Lanes are assigned as the walk runs, so changing the order cannot reuse the
/// current drawing.
#[tauri::command(async)]
pub fn graph_order<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    order: GraphOrder,
) -> Result<u64> {
    let mut guard = state.lock_session("graph_order");
    let session = guard.as_mut().ok_or(Error::NoRepository)?;

    session.order = order;
    let token = state.next_token.fetch_add(1, Ordering::Relaxed);
    restart_walk(app, session, token);
    Ok(token)
}

/// Replace the session's walker, keeping the refs and order it is already on.
fn restart_walk<R: Runtime>(app: AppHandle<R>, session: &mut Session, token: u64) {
    session.graph = graph_worker::spawn(
        app,
        session.path.clone(),
        token,
        session.visible.clone(),
        session.pinned.clone(),
        session.order,
    );
}

/// HEAD and the rail counts, re-read. Called after `repo-changed`.
#[tauri::command(async)]
pub fn snapshot(state: State<'_, AppState>) -> Result<Snapshot> {
    state.with_session("snapshot", |session| {
        let local = session.repo.to_thread_local();
        let refs = RefIndex::build(&local)?;
        Ok(Snapshot {
            info: repo::info(&local)?,
            counts: status::counts(&local, &refs)?,
        })
    })
}

/// Everything the detail panel shows for one commit.
#[tauri::command(async)]
pub fn commit_detail(state: State<'_, AppState>, id: String) -> Result<CommitDetail> {
    state.with_session("commit_detail", |session| {
        diff::commit_detail(&session.repo.to_thread_local(), &id)
    })
}

/// The Diff screen's file list and header counts.
///
/// This reads every blob the commit touched, because `+n −m` per file cannot
/// be known without diffing them. It is one call when the screen opens.
#[tauri::command(async)]
pub fn commit_diff(state: State<'_, AppState>, id: String) -> Result<CommitDiff> {
    state.with_session("commit_diff", |session| {
        diff::commit_diff(&session.repo.to_thread_local(), &id)
    })
}

/// The hunks of one file, fetched as that file is selected.
#[tauri::command(async)]
pub fn file_diff(state: State<'_, AppState>, id: String, path: String) -> Result<FileDiff> {
    state.with_session("file_diff", |session| {
        diff::file_diff(&session.repo.to_thread_local(), &id, &path)
    })
}

/// The staged, unstaged and conflicted lists behind the Working copy screen.
///
/// This is the full status walk, so it is one call per refresh rather than one
/// per row.
#[tauri::command(async)]
pub fn working_copy(state: State<'_, AppState>) -> Result<WorkingCopy> {
    state.with_session("working_copy", |session| {
        status::working_copy(&session.repo.to_thread_local())
    })
}

/// One working-copy file's hunks, on either side of the index.
#[tauri::command(async)]
pub fn working_diff(state: State<'_, AppState>, path: String, side: Side) -> Result<FileDiff> {
    state.with_session("working_diff", |session| {
        diff::working_file_diff(&session.repo.to_thread_local(), &path, side)
    })
}

/// Detailed binary / image diff metadata for a commit file (FEAT-065).
#[tauri::command(async)]
pub fn binary_file_diff(
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> Result<diff::BinaryDiff> {
    state.with_session("binary_file_diff", |session| {
        diff::binary_file_diff(&session.repo.to_thread_local(), &id, &path)
    })
}

/// Detailed binary / image diff metadata for working copy (FEAT-065).
#[tauri::command(async)]
pub fn binary_working_diff(
    state: State<'_, AppState>,
    path: String,
    side: Side,
) -> Result<diff::BinaryDiff> {
    state.with_session("binary_working_diff", |session| {
        diff::binary_working_diff(&session.repo.to_thread_local(), &path, side)
    })
}

#[tauri::command(async)]
pub fn stage(state: State<'_, AppState>, paths: Vec<String>) -> Result<()> {
    state.with_session("stage", |session| {
        work::stage(&session.repo.to_thread_local(), &paths)
    })
}

#[tauri::command(async)]
pub fn unstage(state: State<'_, AppState>, paths: Vec<String>) -> Result<()> {
    state.with_session("unstage", |session| {
        work::unstage(&session.repo.to_thread_local(), &paths)
    })
}

/// Stage one hunk. `header` identifies it, so a stale view is refused rather
/// than half-applied.
#[tauri::command(async)]
pub fn stage_hunk(
    state: State<'_, AppState>,
    path: String,
    index: usize,
    header: String,
) -> Result<()> {
    state.with_session("stage_hunk", |session| {
        work::stage_hunk(&session.repo.to_thread_local(), &path, index, &header)
    })
}

#[tauri::command(async)]
pub fn unstage_hunk(
    state: State<'_, AppState>,
    path: String,
    index: usize,
    header: String,
) -> Result<()> {
    state.with_session("unstage_hunk", |session| {
        work::unstage_hunk(&session.repo.to_thread_local(), &path, index, &header)
    })
}

/// Throw away unstaged changes to whole paths.
///
/// The only command here that destroys work git cannot get back. The screen
/// confirms first; nothing in this layer asks a second time, because a
/// confirmation the caller cannot see the wording of is not a confirmation.
#[tauri::command(async)]
pub fn discard(state: State<'_, AppState>, paths: Vec<String>) -> Result<()> {
    state.with_session("discard", |session| {
        work::discard(&session.repo.to_thread_local(), &paths)
    })
}

/// Throw away one unstaged hunk. `header` identifies it, so a stale view is
/// refused rather than the wrong part of a file being lost.
#[tauri::command(async)]
pub fn discard_hunk(
    state: State<'_, AppState>,
    path: String,
    index: usize,
    header: String,
) -> Result<()> {
    state.with_session("discard_hunk", |session| {
        work::discard_hunk(&session.repo.to_thread_local(), &path, index, &header)
    })
}

/// What a hook printed while a commit ran it (FEAT-107).
pub const HOOK_OUTPUT_EVENT: &str = "hook-output";

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HookOutput {
    /// The caller's, so a window shows only the commit it was opened for.
    pub token: u64,
    pub line: String,
}

/// Commit what is staged. Returns the new commit's id.
///
/// `skip_hooks` runs none of the repository's hooks for this commit; a
/// `token` asks for everything git and its hooks print, as `hook-output`
/// events carrying it (FEAT-107). The session is released before git runs:
/// a hook can take a minute, and nothing else should wait in line behind it.
#[tauri::command(async)]
pub fn commit<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    subject: String,
    body: String,
    amend: bool,
    skip_hooks: Option<bool>,
    token: Option<u64>,
) -> Result<String> {
    let repo = state.with_session("commit", |session| Ok(session.repo.to_thread_local()))?;
    work::commit_with(
        &repo,
        &subject,
        &body,
        amend,
        skip_hooks.unwrap_or(false),
        &mut |line| {
            if let Some(token) = token {
                let _ = app.emit(
                    HOOK_OUTPUT_EVENT,
                    HookOutput {
                        token,
                        line: line.to_string(),
                    },
                );
            }
        },
    )
}

/// The repository's hooks, what each runs, and whether Spagitty runs them
/// (FEAT-107).
#[tauri::command(async)]
pub fn hooks(state: State<'_, AppState>) -> Result<spagitty_core::hooks::Hooks> {
    let path = state.with_session("hooks", |session| Ok(session.path.clone()))?;
    spagitty_core::hooks::list(&path)
}

/// Switch this repository's hooks on or off for commits made in Spagitty.
#[tauri::command(async)]
pub fn set_hooks_enabled(state: State<'_, AppState>, enabled: bool) -> Result<()> {
    let path = state.with_session("set_hooks_enabled", |session| Ok(session.path.clone()))?;
    spagitty_core::hooks::set_enabled(&path, enabled)
}

/// The message of the commit HEAD points at, for pre-filling an amend.
#[tauri::command(async)]
pub fn head_message(state: State<'_, AppState>) -> Result<String> {
    state.with_session("head_message", |session| {
        work::head_message(&session.repo.to_thread_local())
    })
}

/// Every branch, with how far it has drifted. One call per refresh.
///
/// Opens the repository again rather than reusing the session handle. A branch
/// upstream lives in `.git/config`, and `gix` reads config once when a
/// repository is opened — so a `git branch --set-upstream-to` run while Spagitty
/// is open would be invisible until it was restarted, and the screen would
/// report "no upstream" for a branch that has one. Re-discovery costs one
/// directory walk; being quietly wrong about drift costs more.
#[tauri::command(async)]
pub fn branches(state: State<'_, AppState>) -> Result<Vec<BranchRow>> {
    state.with_session("branches", |session| {
        branches::list(&repo::open(&session.path)?)
    })
}

#[tauri::command(async)]
pub fn checkout(state: State<'_, AppState>, name: String) -> Result<()> {
    state.with_session("checkout", |session| {
        branches::checkout(&session.repo.to_thread_local(), &name)
    })
}

/// Create a branch. `start` empty means `HEAD`.
#[tauri::command(async)]
pub fn create_branch(
    state: State<'_, AppState>,
    name: String,
    start: String,
    checkout: bool,
) -> Result<()> {
    state.with_session("create_branch", |session| {
        branches::create(&session.repo.to_thread_local(), &name, &start, checkout)
    })
}

/// What Home shows that only git knows (FEAT-140): the header, the recent
/// branches, the heatmap and the changelog's unreleased entries, in one read.
#[tauri::command(async)]
pub fn home_overview(state: State<'_, AppState>) -> Result<Overview> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_secs() as i64)
        .unwrap_or(0);
    state.with_session("home_overview", |session| {
        Ok(home::overview(&session.repo.to_thread_local(), now))
    })
}

/// Every stash entry, newest first. Ask `commit_diff` about an entry's `id` to
/// see what is in it — a stash is a commit.
#[tauri::command(async)]
pub fn stashes(state: State<'_, AppState>) -> Result<Vec<StashEntry>> {
    state.with_session("stashes", |session| {
        stash::list(&session.repo.to_thread_local())
    })
}

#[tauri::command(async)]
pub fn stash_push(
    state: State<'_, AppState>,
    message: String,
    include_untracked: bool,
) -> Result<()> {
    state.with_session("stash_push", |session| {
        stash::push(&session.repo.to_thread_local(), &message, include_untracked)
    })
}

/// Stop tracking files and folders on a branch, in a commit (FEAT-155).
/// Pushing it is the network worker's, as every push is.
#[tauri::command(async)]
pub fn untrack(
    state: State<'_, AppState>,
    branch: String,
    paths: Vec<untrack::Target>,
    ignore: bool,
    message: String,
) -> Result<untrack::Untracked> {
    state.with_session("untrack", |session| {
        untrack::untrack(
            &session.repo.to_thread_local(),
            &branch,
            &paths,
            ignore,
            &message,
        )
    })
}

/// The todo list `git rebase -i <upstream>` would open, before any edit.
///
/// Generated rather than read: running `git rebase -i` to see the file it opens
/// would start a rebase, which is the thing the Rebase screen exists to avoid.
#[tauri::command(async)]
pub fn rebase_todo(state: State<'_, AppState>, upstream: String) -> Result<Todo> {
    let todo = state.with_session("rebase_todo", |session| {
        rebase::todo(&session.repo.to_thread_local(), &upstream)
    })?;

    *state.rebase_todo.lock().expect("rebase lock") = Some(todo.clone());
    Ok(todo)
}

/// What a plan would produce.
///
/// Pure: a fold over the edits against the todo already read. No repository is
/// touched, and there is no path from here to `shell::rebase_interactive` —
/// executing a plan is FEAT-015.
#[tauri::command(async)]
pub fn rebase_preview(state: State<'_, AppState>, edits: Vec<Edit>) -> Result<Preview> {
    let guard = state.rebase_todo.lock().expect("rebase lock");
    let todo = guard
        .as_ref()
        .ok_or_else(|| Error::NotStageable("no rebase is being planned".into()))?;

    Ok(rebase::plan(todo, &edits))
}

// --- History operations -----------------------------------------------------
//
// Every command below writes. They share one shape on purpose: forward to
// `spagitty_core::ops`, and let the error come back as git's own sentence. No
// confirmation happens here — the screen asks, because a backend that prompted
// could not be scripted or tested — and no operation is inferred from another.
// "Reset" is three commands in the menu because it is three different things.

/// Execute the plan the Rebase screen built. Returns the token its events carry.
///
/// The todo is generated from the same order the preview was generated from, so
/// what was on screen is what git runs. It is generated here rather than sent
/// from the webview for the same reason the preview is: a plan the repository
/// never produced must not be executable by asking nicely.
///
/// The rebase itself runs on a worker (FEAT-015). Every other write holds the
/// session lock until it finishes, which is fine for a commit and wrong for a
/// hundred replayed commits: the screen showing the progress could not ask for
/// it, because the command answering would want the same lock.
#[tauri::command(async)]
pub fn rebase_run<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    edits: Vec<Edit>,
) -> Result<u64> {
    let (upstream, todo) = {
        let guard = state.rebase_todo.lock().expect("rebase lock");
        let todo = guard
            .as_ref()
            .ok_or_else(|| Error::NotStageable("no rebase is being planned".into()))?;
        (todo.upstream.clone(), rebase::todo_text(todo, &edits)?)
    };

    // A second rebase while one is running is refused rather than queued: git
    // would refuse it anyway, and the screen would then have two sets of
    // progress events to tell apart.
    if state.rebase.lock().expect("rebase worker lock").is_some() {
        return Err(Error::NotStageable("a rebase is already running".into()));
    }

    let (workdir, git_dir) = state.with_session("rebase_run", |session| {
        let repo = session.repo.to_thread_local();
        Ok((
            spagitty_core::repo::workdir(&repo)?.to_path_buf(),
            repo.git_dir().to_path_buf(),
        ))
    })?;

    let token = state.next_token.fetch_add(1, Ordering::Relaxed);
    let worker = rebase_worker::spawn(app, workdir, git_dir, upstream, todo, token)?;
    *state.rebase.lock().expect("rebase worker lock") = Some(worker);

    Ok(token)
}

/// How far a rebase that is running has got, or null when none is.
///
/// Read from git's own state directory, so a rebase started from the command
/// line while Spagitty was open is visible here too.
#[tauri::command(async)]
pub fn rebase_progress(state: State<'_, AppState>) -> Result<Option<rebase::Progress>> {
    state.with_session("rebase_progress", |session| {
        Ok(rebase::progress(&session.repo.to_thread_local()))
    })
}

/// Carry on with a rebase that stopped, once its conflicts are resolved.
#[tauri::command(async)]
pub fn rebase_continue(state: State<'_, AppState>) -> Result<()> {
    state.with_session("rebase_continue", |session| {
        ops::rebase_continue(&session.repo.to_thread_local())
    })
}

/// Drop the commit a rebase stopped on and carry on with the rest.
#[tauri::command(async)]
pub fn rebase_skip(state: State<'_, AppState>) -> Result<()> {
    state.with_session("rebase_skip", |session| {
        ops::rebase_skip(&session.repo.to_thread_local())
    })
}

/// Unwind a rebase and put the branch back where it started.
///
/// The worker is dropped first, which waits for its thread. Aborting while it
/// is still emitting progress would leave the screen showing a step count for a
/// rebase that no longer exists.
#[tauri::command(async)]
pub fn rebase_abort(state: State<'_, AppState>) -> Result<()> {
    state.rebase.lock().expect("rebase worker lock").take();
    state.with_session("rebase_abort", |session| {
        ops::rebase_abort(&session.repo.to_thread_local())
    })
}

/// Move the current branch to `commit`.
///
/// `Hard` discards uncommitted work. The mode arrives as an enum rather than a
/// string so that the webview cannot send a harder reset than the one whose
/// consequences the user read in the menu.
#[tauri::command(async)]
pub fn reset(state: State<'_, AppState>, commit: String, mode: ResetMode) -> Result<()> {
    state.with_session("reset", |session| {
        ops::reset(&session.repo.to_thread_local(), &commit, mode)
    })
}

/// Commit the inverse of `commit`. Reverting a merge takes the first parent as
/// the mainline, which is the parent the graph draws as the trunk.
#[tauri::command(async)]
pub fn revert(state: State<'_, AppState>, commit: String) -> Result<()> {
    state.with_session("revert", |session| {
        ops::revert(&session.repo.to_thread_local(), &commit)
    })
}

/// Replay `commits` onto the current branch, in the order given.
#[tauri::command(async)]
pub fn cherry_pick(state: State<'_, AppState>, commits: Vec<String>) -> Result<()> {
    state.with_session("cherry_pick", |session| {
        ops::cherry_pick(&session.repo.to_thread_local(), &commits)
    })
}

/// Merge, fast-forward or rebase `source` into the branch that is checked out.
///
/// This is what dropping one branch label onto another resolves to, once the
/// user has picked from the menu that appears.
#[tauri::command(async)]
pub fn integrate(state: State<'_, AppState>, source: String, how: Integration) -> Result<()> {
    state.with_session("integrate", |session| {
        ops::integrate(&session.repo.to_thread_local(), &source, how)
    })
}

/// Replay commits onto `onto`.
///
/// `upstream` empty moves the whole branch — "rebase this onto that". A commit
/// in `upstream` moves only what comes after it, which is the graph's "rebase
/// these N commits onto". `branch` empty means HEAD.
#[tauri::command(async)]
pub fn rebase_onto(
    state: State<'_, AppState>,
    onto: String,
    upstream: String,
    branch: String,
) -> Result<()> {
    state.with_session("rebase_onto", |session| {
        ops::rebase_onto(&session.repo.to_thread_local(), &onto, &upstream, &branch)
    })
}

/// Check out a commit with no branch attached.
#[tauri::command(async)]
pub fn checkout_detached(state: State<'_, AppState>, revision: String) -> Result<()> {
    state.with_session("checkout_detached", |session| {
        ops::checkout_detached(&session.repo.to_thread_local(), &revision)
    })
}

/// Rename a local branch.
#[tauri::command(async)]
pub fn rename_branch(state: State<'_, AppState>, from: String, to: String) -> Result<()> {
    state.with_session("rename_branch", |session| {
        ops::rename_branch(&session.repo.to_thread_local(), &from, &to)
    })
}

/// Delete a local branch. `force` is `-D`, and loses unmerged commits.
#[tauri::command(async)]
pub fn delete_branch(state: State<'_, AppState>, name: String, force: bool) -> Result<()> {
    state.with_session("delete_branch", |session| {
        ops::delete_branch(&session.repo.to_thread_local(), &name, force)
    })
}

/// Create a tag at `target`. A message makes it annotated.
#[tauri::command(async)]
pub fn create_tag(
    state: State<'_, AppState>,
    name: String,
    target: String,
    message: String,
) -> Result<()> {
    state.with_session("create_tag", |session| {
        ops::create_tag(&session.repo.to_thread_local(), &name, &target, &message)
    })
}

/// Delete a local tag.
#[tauri::command(async)]
pub fn delete_tag(state: State<'_, AppState>, name: String) -> Result<()> {
    state.with_session("delete_tag", |session| {
        ops::delete_tag(&session.repo.to_thread_local(), &name)
    })
}

/// Apply, pop or drop a stash entry.
#[tauri::command(async)]
pub fn stash_action(state: State<'_, AppState>, index: usize, action: StashAction) -> Result<()> {
    state.with_session("stash_action", |session| {
        ops::stash(&session.repo.to_thread_local(), index, action)
    })
}

/// Fetch. Returns the token its events carry (FEAT-018).
///
/// An empty remote fetches all of them. `prune` deletes remote-tracking refs
/// the remote no longer has — it used to be passed on every fetch, which meant
/// a destructive operation happening without anybody choosing it.
///
/// Runs on a worker so that git's progress arrives while it runs rather than
/// all at once at the end, and so that a slow network operation does not hold
/// the session lock against every other screen.
#[tauri::command(async)]
pub fn fetch<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    remote: String,
    prune: bool,
) -> Result<u64> {
    start(app, state, network_worker::Job::Fetch { remote, prune })
}

/// Pull. Fetches and integrates in one `git pull`, so git resolves which
/// upstream the current branch tracks rather than Spagitty guessing.
#[tauri::command(async)]
pub fn pull(state: State<'_, AppState>, remote: String, mode: PullMode) -> Result<String> {
    state.with_session("pull", |session| {
        ops::pull(&session.repo.to_thread_local(), &remote, mode)
    })
}

/// Bring a local branch up to its upstream, whichever branch is checked out
/// (FEAT-126). Only ever forward: git's refusal is the error when it cannot be.
#[tauri::command(async)]
pub fn pull_branch(state: State<'_, AppState>, name: String) -> Result<String> {
    state.with_session("pull_branch", |session| {
        ops::pull_branch(&session.repo.to_thread_local(), &name)
    })
}

/// Push. `force` is `--force-with-lease`, never a plain force.
///
/// On a worker, for the reasons in [`fetch`].
#[tauri::command(async)]
pub fn push<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    remote: String,
    refspec: String,
    force: bool,
) -> Result<u64> {
    start(
        app,
        state,
        network_worker::Job::Push {
            remote,
            refspec,
            force,
        },
    )
}

/// Start a network job, refusing a second one while the first is running.
///
/// Refused rather than queued: two of them would give the screen two sets of
/// progress events to tell apart, and git would be contending for the same
/// refs anyway.
fn start<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    job: network_worker::Job,
) -> Result<u64> {
    if state.network.lock().expect("network lock").is_some() {
        return Err(Error::NotStageable(
            "a fetch or push is already running".into(),
        ));
    }

    let workdir = state.with_session("start", |session| {
        Ok(spagitty_core::repo::workdir(&session.repo.to_thread_local())?.to_path_buf())
    })?;

    let token = state.next_token.fetch_add(1, Ordering::Relaxed);
    let worker = network_worker::spawn(app, workdir, job, token)?;
    *state.network.lock().expect("network lock") = Some(worker);

    Ok(token)
}

/// Let go of a finished network worker, so the next one may start.
///
/// Called by the screen when it sees the done event. The worker cannot release
/// itself: dropping it joins its own thread.
#[tauri::command(async)]
pub fn network_release(state: State<'_, AppState>) {
    state.network.lock().expect("network lock").take();
}

/// Start a query. Returns the token its rows will carry.
///
/// Rows arrive as `search-rows` events and the walk ends with `search-done`.
/// Starting a query cancels whichever one was running, so a store that sees
/// rows from an older token can drop them without asking.
#[tauri::command(async)]
pub fn search_start<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    query: Query,
) -> Result<u64> {
    let path = state.with_session("search_start", |session| Ok(session.path.clone()))?;
    let token = state.next_token.fetch_add(1, Ordering::Relaxed);

    let worker = search_worker::spawn(app, path, query, token);
    // Assigning drops the previous worker, which cancels it and joins its
    // thread before this call returns.
    *state.search.lock().expect("search lock") = Some(worker);

    Ok(token)
}

/// Stop the running query. Leaving the screen is the ordinary caller.
#[tauri::command(async)]
pub fn search_stop(state: State<'_, AppState>) {
    *state.search.lock().expect("search lock") = None;
}

/// Who last touched each line of `path` at `revision`. An empty revision is
/// `HEAD`.
#[tauri::command(async)]
pub fn blame(state: State<'_, AppState>, path: String, revision: String) -> Result<Blame> {
    state.with_session("blame", |session| {
        blame::file(&session.repo.to_thread_local(), &path, &revision)
    })
}

// --- Files (FEAT-131) ---------------------------------------------------
//
// Every command below takes a revision and reads git's objects at it. None of
// them holds the session lock while it works: colouring a large file or
// blaming a long one takes long enough to be noticed, and the graph, the
// toolbar and the watcher all wait on that lock. The repository handle is
// taken under the lock and the work is done with it after.

/// The open repository, to read from without holding the session.
fn files_repo(
    state: &State<'_, AppState>,
    command: &'static str,
) -> Result<spagitty_core::gix::Repository> {
    state.with_session(command, |session| Ok(session.repo.to_thread_local()))
}

/// The commit a branch, tag or commit means. An empty revision is `HEAD`.
#[tauri::command(async)]
pub fn files_resolve(state: State<'_, AppState>, rev: String) -> Result<files::Revision> {
    files::resolve(&files_repo(&state, "files_resolve")?, &rev)
}

/// One folder of the repository at `rev`. An empty `dir` is the root.
#[tauri::command(async)]
pub fn files_tree(
    state: State<'_, AppState>,
    rev: String,
    dir: String,
) -> Result<Vec<files::TreeEntry>> {
    files::tree(&files_repo(&state, "files_tree")?, &rev, &dir)
}

/// `path` as it is in `rev`: text, an image, binary, a Git LFS pointer, too
/// large, or not there.
#[tauri::command(async)]
pub fn files_read(
    state: State<'_, AppState>,
    rev: String,
    path: String,
) -> Result<files::FileRead> {
    files::read(&files_repo(&state, "files_read")?, &rev, &path)
}

/// How many paths *Find a file* shows. Past this the field says to type more.
const FILES_FIND_LIMIT: usize = 200;

/// The files of `rev` whose paths hold every word of `query`.
#[tauri::command(async)]
pub fn files_find(state: State<'_, AppState>, rev: String, query: String) -> Result<files::Found> {
    files::find(
        &files_repo(&state, "files_find")?,
        &rev,
        &query,
        FILES_FIND_LIMIT,
    )
}

/// How many lines *Search in files* returns. Past this it says there are more.
const FILES_SEARCH_LIMIT: usize = 2_000;

/// Every line of every text file at `rev` that `query` matches (FEAT-151).
#[tauri::command(async)]
pub fn files_search(
    state: State<'_, AppState>,
    rev: String,
    query: String,
    options: files::SearchOptions,
) -> Result<files::Searched> {
    files::search(
        &files_repo(&state, "files_search")?,
        &rev,
        &query,
        options,
        FILES_SEARCH_LIMIT,
    )
}

/// How many places *Usages* returns. Past this it says there are more.
const FILES_USAGES_LIMIT: usize = 2_000;

/// Where `name` is defined, used and mentioned in the files at `rev`
/// (FEAT-154).
#[tauri::command(async)]
pub fn files_usages(
    state: State<'_, AppState>,
    rev: String,
    name: String,
) -> Result<usages::Usages> {
    usages::usages(
        &files_repo(&state, "files_usages")?,
        &rev,
        &name,
        FILES_USAGES_LIMIT,
    )
}

/// The colour of lines `start..end` of a blob, from tree-sitter.
#[tauri::command(async)]
pub fn files_spans(
    state: State<'_, AppState>,
    blob: String,
    grammar: String,
    start: usize,
    end: usize,
) -> Result<Vec<highlight::Line>> {
    highlight::spans(
        &files_repo(&state, "files_spans")?,
        &blob,
        &grammar,
        start,
        end,
    )
}

/// Who wrote each line of `path` at `rev`, with each commit said once.
#[tauri::command(async)]
pub fn files_blame(
    state: State<'_, AppState>,
    rev: String,
    path: String,
) -> Result<blame::CompactBlame> {
    blame::compact(&files_repo(&state, "files_blame")?, &path, &rev)
}

/// What a line was before `commit` wrote it, for the peek.
#[tauri::command(async)]
pub fn files_before(
    state: State<'_, AppState>,
    commit: String,
    path: String,
    line: u32,
    previous: Option<String>,
    previous_path: Option<String>,
) -> Result<Option<String>> {
    let repo = files_repo(&state, "files_before")?;
    let from = previous.as_deref().zip(previous_path.as_deref());
    files::before(&repo, &commit, &path, line, from)
}

// --- Investigate (FEAT-132) ---------------------------------------------
//
// Reads, like Files', and like Files' none holds the session lock while it
// works: a page of `git log --follow` on a long history takes long enough to
// be noticed.

/// A page of the life of a file, a folder, a branch or a line.
#[tauri::command(async)]
pub fn investigate_life(
    state: State<'_, AppState>,
    ask: investigate::Ask,
) -> Result<investigate::Page> {
    investigate::life(&files_repo(&state, "investigate_life")?, &ask)
}

/// Who owns `path` at `rev` today, and what survives of each commit.
#[tauri::command(async)]
pub fn investigate_today(
    state: State<'_, AppState>,
    path: String,
    rev: String,
) -> Result<investigate::Today> {
    investigate::today(&files_repo(&state, "investigate_today")?, &path, &rev)
}

/// The largest hunk of what `commit` did to `path`.
#[tauri::command(async)]
pub fn investigate_hunk(
    state: State<'_, AppState>,
    commit: String,
    path: String,
    old_path: Option<String>,
) -> Result<Option<diff::Hunk>> {
    investigate::largest_hunk(
        &files_repo(&state, "investigate_hunk")?,
        &commit,
        &path,
        old_path.as_deref(),
    )
}

/// `path` as `commit` left it, against `today_path` at `rev`.
#[tauri::command(async)]
pub fn investigate_compare(
    state: State<'_, AppState>,
    commit: String,
    path: String,
    rev: String,
    today_path: String,
) -> Result<Vec<diff::Hunk>> {
    investigate::compare(
        &files_repo(&state, "investigate_compare")?,
        &commit,
        &path,
        &rev,
        &today_path,
    )
}

/// A branch against the default branch: its fork, where it landed, ahead and behind.
#[tauri::command(async)]
pub fn investigate_branch(
    state: State<'_, AppState>,
    name: String,
) -> Result<investigate::BranchFacts> {
    investigate::branch_facts(&files_repo(&state, "investigate_branch")?, &name)
}

/// What a terminal's link would open (FEAT-130).
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TerminalLinks {
    /// Each hex word that is a commit here, with the full id it means.
    pub commits: Vec<(String, String)>,
    /// Each path that is a file at `HEAD`.
    pub paths: Vec<String>,
}

/// Which of the hex words and paths a terminal printed are real. With no
/// repository open, none are: nothing is underlined on a guess.
#[tauri::command(async)]
pub fn terminal_links(
    state: State<'_, AppState>,
    words: Vec<String>,
    paths: Vec<String>,
) -> TerminalLinks {
    let (commits, paths) = match files_repo(&state, "terminal_links") {
        Ok(repo) => files::known(&repo, &words, &paths),
        Err(_) => (Vec::new(), Vec::new()),
    };
    TerminalLinks { commits, paths }
}

/// What operation is in progress, and every conflicted path.
///
/// Reads only. Taking a side, editing the merged result and marking a file
/// resolved are FEAT-016 and have no command here at all — a disabled button
/// backed by nothing is easier to explain than one backed by a half-written
/// write path.
#[tauri::command(async)]
pub fn conflicts(state: State<'_, AppState>) -> Result<ConflictState> {
    state.with_session("conflicts", |session| {
        conflicts::state(&session.repo.to_thread_local())
    })
}

/// The three index stages of one conflicted path, plus the file on disk.
#[tauri::command(async)]
pub fn conflict_sides(state: State<'_, AppState>, path: String) -> Result<ConflictSides> {
    state.with_session("conflict_sides", |session| {
        conflicts::sides(&session.repo.to_thread_local(), &path)
    })
}

/// Every conflict region in a file's merged text, so the screen can offer one
/// button per region rather than only per file.
#[tauri::command(async)]
pub fn conflict_regions(text: String) -> Vec<conflicts::Region> {
    conflicts::regions(&text)
}

/// Take one whole side of a conflicted file into the working tree.
///
/// The file stays conflicted afterwards. Marking it resolved is a separate
/// command on purpose: looking at the result is the point of the screen.
#[tauri::command(async)]
pub fn conflict_take(
    state: State<'_, AppState>,
    path: String,
    side: conflicts::Side,
) -> Result<()> {
    state.with_session("conflict_take", |session| {
        conflicts::take(&session.repo.to_thread_local(), &path, side)
    })
}

/// Resolve one marker region, or every region, and write the file back.
///
/// The text is re-read here rather than taken from the webview: a screen that
/// sent the whole file back could send one it had edited since it was read, and
/// the region indexes would then point somewhere else.
#[tauri::command(async)]
pub fn conflict_resolve_region(
    state: State<'_, AppState>,
    path: String,
    index: Option<usize>,
    side: conflicts::Side,
) -> Result<()> {
    state.with_session("conflict_resolve_region", |session| {
        let repo = session.repo.to_thread_local();
        let current = std::fs::read_to_string(spagitty_core::repo::workdir(&repo)?.join(&path))?;
        let resolved = match index {
            Some(index) => conflicts::resolve_region(&current, index, side)?,
            None => conflicts::resolve_all(&current, side),
        };
        conflicts::write_merged(&repo, &path, &resolved)
    })
}

/// Write the merged pane's text to the file, exactly as given.
#[tauri::command(async)]
pub fn conflict_write(state: State<'_, AppState>, path: String, text: String) -> Result<()> {
    state.with_session("conflict_write", |session| {
        conflicts::write_merged(&session.repo.to_thread_local(), &path, &text)
    })
}

/// Mark paths resolved: `git add`.
#[tauri::command(async)]
pub fn conflict_resolve(state: State<'_, AppState>, paths: Vec<String>) -> Result<()> {
    state.with_session("conflict_resolve", |session| {
        conflicts::mark_resolved(&session.repo.to_thread_local(), &paths)
    })
}

/// Settle one file with what was chosen for it and mark it resolved
/// (FEAT-102): its text, or one side whole.
#[tauri::command(async)]
pub fn conflict_settle(
    state: State<'_, AppState>,
    path: String,
    text: Option<String>,
    take: Option<conflicts::Side>,
) -> Result<()> {
    state.with_session("conflict_settle", |session| {
        conflicts::settle(
            &session.repo.to_thread_local(),
            &path,
            text.as_deref(),
            take,
        )
    })
}

/// Carry on with whatever the repository is in the middle of.
#[tauri::command(async)]
pub fn conflict_continue(state: State<'_, AppState>) -> Result<()> {
    state.with_session("conflict_continue", |session| {
        let repo = session.repo.to_thread_local();
        conflicts::continue_operation(&repo, conflicts::operation(&repo))
    })
}

/// Abandon it and put the repository back.
#[tauri::command(async)]
pub fn conflict_abort(state: State<'_, AppState>) -> Result<()> {
    state.with_session("conflict_abort", |session| {
        let repo = session.repo.to_thread_local();
        conflicts::abort_operation(&repo, conflicts::operation(&repo))
    })
}

/// What merging two branches would do, worked out without writing it
/// (FEAT-100).
///
/// A merge of a large history takes git a while, so the session is let go
/// first and the merge is worked out on its own thread, as for the review's
/// re-done merges.
#[tauri::command]
pub async fn merger_forecast(
    state: State<'_, AppState>,
    a: String,
    b: String,
) -> Result<merger::Forecast> {
    let shared = {
        let guard = state.lock_session("merger_forecast");
        guard.as_ref().ok_or(Error::NoRepository)?.repo.clone()
    };
    off_thread(move || merger::forecast(&shared.to_thread_local(), &a, &b)).await
}

/// Land a merge Merger planned (FEAT-101): commit the result and move the
/// receiving branch to it, refusing if either branch moved since the plan.
///
/// It writes, so it holds the session like every other write.
#[tauri::command(async)]
pub fn merger_land(state: State<'_, AppState>, ask: merger::LandAsk) -> Result<merger::Landed> {
    state.with_session("merger_land", |session| {
        merger::land(&session.repo.to_thread_local(), &ask)
    })
}

/// Every conflict of merging two branches, each side whole, read from a dry
/// run (FEAT-102). Writes nothing, so it lets the session go first, as the
/// forecast does.
#[tauri::command]
pub async fn merger_conflicts(
    state: State<'_, AppState>,
    a: String,
    b: String,
) -> Result<merger::MergeConflicts> {
    let shared = {
        let guard = state.lock_session("merger_conflicts");
        guard.as_ref().ok_or(Error::NoRepository)?.repo.clone()
    };
    off_thread(move || merger::conflict_files(&shared.to_thread_local(), &a, &b)).await
}

/// Start Merger's rebase in its own worktree, or find the one already
/// running for this merge (FEAT-103): where it stopped, or that it is done.
#[tauri::command(async)]
pub fn merger_rebase_open(
    state: State<'_, AppState>,
    ask: merger::LandAsk,
) -> Result<merger::Replay> {
    state.with_session("merger_rebase_open", |session| {
        merger::rebase_open(&session.repo.to_thread_local(), &ask)
    })
}

/// Settle the stop's files with what was chosen and carry the rebase on.
#[tauri::command(async)]
pub fn merger_rebase_continue(
    state: State<'_, AppState>,
    ask: merger::LandAsk,
    resolutions: Vec<merger::Resolution>,
) -> Result<merger::Replay> {
    state.with_session("merger_rebase_continue", |session| {
        merger::rebase_continue(&session.repo.to_thread_local(), &ask, &resolutions)
    })
}

/// Drop the commit Merger's rebase stopped on, and carry on.
#[tauri::command(async)]
pub fn merger_rebase_skip(
    state: State<'_, AppState>,
    ask: merger::LandAsk,
) -> Result<merger::Replay> {
    state.with_session("merger_rebase_skip", |session| {
        merger::rebase_skip(&session.repo.to_thread_local(), &ask)
    })
}

/// Undo Merger's rebase and remove its worktree. Neither branch moved.
#[tauri::command(async)]
pub fn merger_rebase_abort(state: State<'_, AppState>, ask: merger::LandAsk) -> Result<()> {
    state.with_session("merger_rebase_abort", |session| {
        merger::rebase_abort(&session.repo.to_thread_local(), &ask)
    })
}

/// Move the receiving branch to the replayed commits.
#[tauri::command(async)]
pub fn merger_rebase_finish(
    state: State<'_, AppState>,
    ask: merger::LandAsk,
) -> Result<merger::Landed> {
    state.with_session("merger_rebase_finish", |session| {
        merger::rebase_finish(&session.repo.to_thread_local(), &ask)
    })
}

/// The choices kept for one merge, or null (FEAT-102).
#[tauri::command(async)]
pub fn merger_state<R: Runtime>(app: AppHandle<R>, key: String) -> Option<serde_json::Value> {
    let root = merger_state::root(&app)?;
    merger_state::read(&merger_state::path_in(&root, &key)?)
}

/// Keep the choices made for one merge (FEAT-102). A null state forgets them.
#[tauri::command(async)]
pub fn set_merger_state<R: Runtime>(
    app: AppHandle<R>,
    key: String,
    state: serde_json::Value,
) -> Result<()> {
    let path = merger_state::root(&app)
        .and_then(|root| merger_state::path_in(&root, &key))
        .ok_or_else(|| Error::Config("nowhere to keep this merge's choices".into()))?;
    if state.is_null() {
        return merger_state::forget(&path).map_err(Error::Config);
    }
    merger_state::write(&path, &state).map_err(Error::Config)
}

/// Every tag, newest first (FEAT-051).
#[tauri::command(async)]
pub fn tags(state: State<'_, AppState>) -> Result<Vec<tags::Tag>> {
    state.with_session("tags", |session| {
        tags::tags(&session.repo.to_thread_local())
    })
}

/// Create a tag. A non-empty message makes it annotated.
#[tauri::command(async)]
pub fn tag_create(
    state: State<'_, AppState>,
    name: String,
    target: String,
    message: String,
) -> Result<()> {
    state.with_session("tag_create", |session| {
        tags::create(&session.repo.to_thread_local(), &name, &target, &message)
    })
}

/// Delete a local tag.
#[tauri::command(async)]
pub fn tag_delete(state: State<'_, AppState>, name: String) -> Result<()> {
    state.with_session("tag_delete", |session| {
        tags::delete(&session.repo.to_thread_local(), &name)
    })
}

/// Rewrite an annotated tag's message, keeping it on the same commit.
#[tauri::command(async)]
pub fn tag_retag(
    state: State<'_, AppState>,
    name: String,
    target: String,
    message: String,
) -> Result<()> {
    state.with_session("tag_retag", |session| {
        tags::retag(&session.repo.to_thread_local(), &name, &target, &message)
    })
}

/// Where a ref has been (FEAT-050).
#[tauri::command(async)]
pub fn reflog(state: State<'_, AppState>, query: reflog::ReflogQuery) -> Result<reflog::Reflog> {
    state.with_session("reflog", |session| {
        reflog::reflog(&session.repo.to_thread_local(), &query)
    })
}

/// Every ref whose reflog is worth offering, `HEAD` first.
#[tauri::command(async)]
pub fn reflog_refs(state: State<'_, AppState>) -> Result<Vec<String>> {
    state.with_session("reflog_refs", |session| {
        Ok(reflog::logged_refs(&session.repo.to_thread_local()))
    })
}

/// Every configured remote, in name order.
#[tauri::command(async)]
pub fn remotes(state: State<'_, AppState>) -> Result<Vec<remotes::Remote>> {
    state.with_session("remotes", |session| {
        Ok(remotes::remotes(&session.repo.to_thread_local()))
    })
}

/// Add a remote. Configuration only — nothing is fetched.
#[tauri::command(async)]
pub fn remote_add(state: State<'_, AppState>, name: String, url: String) -> Result<()> {
    state.with_session("remote_add", |session| {
        remotes::add(&session.repo.to_thread_local(), &name, &url)
    })
}

/// Rename a remote, its tracking refs, and every upstream pointing at it.
#[tauri::command(async)]
pub fn remote_rename(state: State<'_, AppState>, from: String, to: String) -> Result<()> {
    state.with_session("remote_rename", |session| {
        remotes::rename(&session.repo.to_thread_local(), &from, &to)
    })
}

/// List all submodules for the open repository (FEAT-067).
#[tauri::command(async)]
pub fn submodules(state: State<'_, AppState>) -> Result<Vec<Submodule>> {
    state.with_session("submodules", |session| {
        submodules::list(&session.repo.to_thread_local())
    })
}

/// Update submodules recursively (FEAT-067).
#[tauri::command(async)]
pub fn submodule_update(
    state: State<'_, AppState>,
    paths: Vec<String>,
    init: bool,
    recursive: bool,
) -> Result<String> {
    state.with_session("submodule_update", |session| {
        submodules::update(&session.repo.to_thread_local(), &paths, init, recursive)
    })
}

/// Sync submodule URLs from .gitmodules (FEAT-067).
#[tauri::command(async)]
pub fn submodule_sync(state: State<'_, AppState>, recursive: bool) -> Result<String> {
    state.with_session("submodule_sync", |session| {
        submodules::sync(&session.repo.to_thread_local(), recursive)
    })
}

/// De-initialize a submodule (FEAT-067).
#[tauri::command(async)]
pub fn submodule_deinit(state: State<'_, AppState>, path: String, force: bool) -> Result<String> {
    state.with_session("submodule_deinit", |session| {
        submodules::deinit(&session.repo.to_thread_local(), &path, force)
    })
}

/// Read configured and available external diff/merge tools (FEAT-068).
///
/// Falls back to global configuration from ~/.gitconfig if no repository is open.
#[tauri::command(async)]
pub fn external_tools_config(state: State<'_, AppState>) -> Result<ExternalToolsConfig> {
    let guard = state.lock_session("external_tools_config");
    match guard.as_ref() {
        Some(session) => tools::get_config(&session.repo.to_thread_local()),
        None => tools::get_config_global(),
    }
}

/// Set configured external tool (FEAT-068).
///
/// If no repository is open, writes to global configuration (~/.gitconfig).
#[tauri::command]
pub fn set_external_tool(
    state: State<'_, AppState>,
    tool_type: String,
    tool_name: Option<String>,
    global: bool,
) -> Result<()> {
    let guard = state.lock_session("set_external_tool");
    match guard.as_ref() {
        Some(session) => tools::set_tool(
            &session.repo.to_thread_local(),
            &tool_type,
            tool_name.as_deref(),
            global,
        ),
        None => {
            if !global {
                return Err(Error::NoRepository);
            }
            tools::set_tool_global(&tool_type, tool_name.as_deref())
        }
    }
}

/// Launch external diff tool (FEAT-068).
#[tauri::command(async)]
pub fn launch_external_diff(
    state: State<'_, AppState>,
    path: String,
    tool: Option<String>,
    commit: Option<String>,
) -> Result<()> {
    state.with_session("launch_external_diff", |session| {
        tools::launch_diff(
            &session.repo.to_thread_local(),
            &path,
            tool.as_deref(),
            commit.as_deref(),
        )
    })
}

/// Launch external merge tool (FEAT-068).
#[tauri::command(async)]
pub fn launch_external_merge(
    state: State<'_, AppState>,
    path: String,
    tool: Option<String>,
) -> Result<()> {
    state.with_session("launch_external_merge", |session| {
        tools::launch_merge(&session.repo.to_thread_local(), &path, tool.as_deref())
    })
}

/// Remove a remote, its tracking refs, and the upstreams pointing at it.
#[tauri::command(async)]
pub fn remote_remove(state: State<'_, AppState>, name: String) -> Result<()> {
    state.with_session("remote_remove", |session| {
        remotes::remove(&session.repo.to_thread_local(), &name)
    })
}

/// Change where a remote points.
#[tauri::command(async)]
pub fn remote_set_url(state: State<'_, AppState>, name: String, url: String) -> Result<()> {
    state.with_session("remote_set_url", |session| {
        remotes::set_url(&session.repo.to_thread_local(), &name, &url)
    })
}

/// List all worktrees for the open repository (FEAT-062).
#[tauri::command(async)]
pub fn worktrees(state: State<'_, AppState>) -> Result<Vec<Worktree>> {
    state.with_session("worktrees", |session| {
        worktrees::list(&session.repo.to_thread_local())
    })
}

/// Add a new linked worktree (FEAT-062).
#[tauri::command(async)]
pub fn worktree_add(
    state: State<'_, AppState>,
    path: String,
    branch: Option<String>,
    new_branch: Option<String>,
    detach: bool,
) -> Result<Worktree> {
    state.with_session("worktree_add", |session| {
        worktrees::add(
            &session.repo.to_thread_local(),
            std::path::Path::new(&path),
            branch.as_deref(),
            new_branch.as_deref(),
            detach,
        )
    })
}

/// Remove a worktree (FEAT-062).
#[tauri::command(async)]
pub fn worktree_remove(state: State<'_, AppState>, path: String, force: bool) -> Result<()> {
    state.with_session("worktree_remove", |session| {
        worktrees::remove(
            &session.repo.to_thread_local(),
            std::path::Path::new(&path),
            force,
        )
    })
}

/// Lock a worktree against pruning (FEAT-062).
#[tauri::command(async)]
pub fn worktree_lock(
    state: State<'_, AppState>,
    path: String,
    reason: Option<String>,
) -> Result<()> {
    state.with_session("worktree_lock", |session| {
        worktrees::lock(
            &session.repo.to_thread_local(),
            std::path::Path::new(&path),
            reason.as_deref(),
        )
    })
}

/// Unlock a locked worktree (FEAT-062).
#[tauri::command(async)]
pub fn worktree_unlock(state: State<'_, AppState>, path: String) -> Result<()> {
    state.with_session("worktree_unlock", |session| {
        worktrees::unlock(&session.repo.to_thread_local(), std::path::Path::new(&path))
    })
}

/// Prune stale worktrees (FEAT-062).
#[tauri::command(async)]
pub fn worktree_prune(state: State<'_, AppState>) -> Result<()> {
    state.with_session("worktree_prune", |session| {
        worktrees::prune(&session.repo.to_thread_local())
    })
}

/// Every remembered repository, as a card.
///
/// Each is read where it sits, without becoming the open one: no walk, no
/// worker, no watcher, and nothing written to it. A path that has gone comes
/// back as a card that says so rather than being dropped — a repository that
/// moved is something to see, not something to forget quietly.
#[tauri::command(async)]
pub fn recent_repos<R: Runtime>(app: AppHandle<R>) -> Vec<RepoSummary> {
    recents::load(&app)
        .iter()
        .map(|path| repo::summary(path))
        .collect()
}

/// Where a clone would land, and what is wrong with that.
///
/// Recomputed as the user types, because every refusal here is knowable without
/// the network: telling somebody after a round trip what they could have been
/// told while typing is the failure this exists to avoid.
#[tauri::command(async)]
pub fn clone_plan(url: String, parent: PathBuf) -> Plan {
    clone::plan(&url, &parent)
}

/// Start a clone. Returns the token its progress will carry.
///
/// Progress arrives as `clone-progress` events and the clone ends with
/// `clone-done`. The plan is recomputed here rather than taken from the caller:
/// a destination that filled up between the last keystroke and the button is
/// still a destination this must refuse.
#[tauri::command(async)]
pub fn clone_start<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    url: String,
    parent: PathBuf,
) -> Result<u64> {
    if state.clone.lock().expect("clone lock").is_some() {
        return Err(Error::NotStageable(
            "a clone is already running; wait for it or stop it first".into(),
        ));
    }

    let plan = clone::plan(&url, &parent);
    let destination = match (&plan.problem, plan.destination) {
        (Some(problem), _) => return Err(Error::NotStageable(problem.message())),
        (None, Some(destination)) => destination,
        (None, None) => return Err(Error::NotStageable("there is nowhere to clone to".into())),
    };

    let token = state.next_token.fetch_add(1, Ordering::Relaxed);
    let worker = clone_worker::spawn(app, url, destination, plan.creates_destination, token)?;

    *state.clone.lock().expect("clone lock") = Some(worker);
    Ok(token)
}

/// Stop the running clone, and forget it either way.
///
/// One command for both "the user pressed Stop" and "it finished, let go of
/// it", because they are the same operation: dropping the worker kills a
/// process that is still running and joins a thread that is not. Cancelling
/// removes the destination only if the clone created it — a directory that was
/// already there is left exactly as it was found.
#[tauri::command(async)]
pub fn clone_release(state: State<'_, AppState>) {
    *state.clone.lock().expect("clone lock") = None;
}

/// Remove a repository from Spagitty's list. The directory is not touched.
#[tauri::command]
pub fn forget_repo<R: Runtime>(app: AppHandle<R>, path: PathBuf) {
    recents::forget(&app, &path);
}

#[tauri::command(async)]
pub fn close_repo(state: State<'_, AppState>) {
    // The ticket is taken under the lock, so an open either installed before
    // this close and is cleared by it, or checks its ticket after it and
    // finds itself overtaken.
    let mut slot = state.lock_session("close_repo");
    state.ticket();
    *slot = None;
}

/// Every `git` command Spagitty has executed since `since`, oldest first.
///
/// The panel subscribes to [`crate::command_log::EXECUTED_EVENT`] for new
/// entries, so this is the catch-up call: what ran before the panel was opened,
/// or while the webview was reloading. `since` of 0 is everything still held.
#[tauri::command(async)]
pub fn git_commands(since: u64) -> Vec<Executed> {
    record::recent(since)
}

/// How long each hold of the open repository waited and ran, after `after`
/// (TASK-052). God mode's Timings panel asks for what is new.
#[tauri::command(async)]
pub fn command_timings(after: u64) -> Vec<timing::Timed> {
    timing::since(after)
}

/// Forget the recorded commands. The user's action, from the panel.
#[tauri::command]
pub fn clear_git_commands() {
    record::clear();
}

#[tauri::command(async)]
pub fn metrics() -> Metrics {
    Metrics {
        row_pitch: ROW_PITCH,
    }
}

#[tauri::command(async)]
pub fn about() -> About {
    crate::about::about()
}

/// Every dependency this binary is made of, with its license.
///
/// Generated from the lockfiles at build time rather than typed, so it cannot
/// quietly fall behind a `cargo update`. A build that could not generate it
/// returns the reason instead of an empty list.
#[tauri::command(async)]
pub fn licenses() -> Licenses {
    crate::about::licenses()
}

/// `user.name` and `user.email`, per scope, and which one is in effect.
///
/// Settings does not need an open repository: with none, the global and system
/// configuration is read on its own and the local scope is reported as absent.
#[tauri::command(async)]
pub fn identity(state: State<'_, AppState>) -> Result<Identity> {
    let guard = state.lock_session("identity");
    match guard.as_ref() {
        Some(session) => {
            // The shared handle keeps the config from when the repository was
            // opened. Read from disk after a save or an external edit (BUG-082).
            let fresh = repo::open(&session.path)?;
            Ok(identity::read(&fresh))
        }
        None => identity::read_global(),
    }
}

/// Write one identity key in one scope. A blank value unsets the key.
///
/// The scope is what the user chose on the screen and is never inferred here.
/// Writing to the local scope needs a repository to write into; writing to the
/// global one does not, and runs wherever Spagitty was started.
#[tauri::command]
pub fn set_identity(
    state: State<'_, AppState>,
    scope: Scope,
    key: Key,
    value: String,
) -> Result<Identity> {
    let open = state
        .session
        .lock()
        .expect("session lock")
        .as_ref()
        .map(|session| session.path.clone());

    let directory = match (scope, open) {
        (_, Some(path)) => path,
        (Scope::Local, None) => return Err(Error::NoRepository),
        (Scope::Global, None) => std::env::current_dir()?,
    };

    identity::write(&directory, scope, key, &value)?;
    identity(state)
}

/// Read all saved identity profiles (FEAT-069).
#[tauri::command(async)]
pub fn identity_profiles<R: Runtime>(app: AppHandle<R>) -> Vec<identity::IdentityProfile> {
    profiles::load(&app)
}

/// Save an identity profile (FEAT-069).
#[tauri::command]
pub fn save_identity_profile<R: Runtime>(
    app: AppHandle<R>,
    profile: identity::IdentityProfile,
) -> Result<()> {
    profiles::save(&app, profile);
    Ok(())
}

/// Delete an identity profile (FEAT-069).
#[tauri::command]
pub fn delete_identity_profile<R: Runtime>(app: AppHandle<R>, id: String) -> Result<()> {
    profiles::delete(&app, &id);
    Ok(())
}

/// Apply an identity profile to the current repository or globally (FEAT-069).
#[tauri::command]
pub fn apply_identity_profile(
    state: State<'_, AppState>,
    profile: identity::IdentityProfile,
    global: bool,
) -> Result<Identity> {
    let scope = if global {
        identity::Scope::Global
    } else {
        identity::Scope::Local
    };
    let open = state
        .session
        .lock()
        .expect("session lock")
        .as_ref()
        .map(|session| session.path.clone());

    let directory = match (scope, open) {
        (_, Some(path)) => path,
        (identity::Scope::Local, None) => return Err(Error::NoRepository),
        (identity::Scope::Global, None) => std::env::current_dir()?,
    };

    identity::apply_profile(&directory, scope, &profile)?;
    identity(state)
}

/// Spagitty's own behaviour toggles.
/// Which repository on a hosting service the open one points at (FEAT-017).
///
/// `None` when no repository is open, when its remote is not a host Spagitty
/// reads, or when it has several remotes and no `origin` — none of which is an
/// error. Plenty of repositories are not on a forge at all.
///
/// A host a connected account names counts as that account's kind, so a
/// self-hosted GitLab not called `gitlab.` anything is still read (FEAT-088).
#[tauri::command(async)]
pub fn forge_repo<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<Option<Repo>> {
    let known = accounts::load(&app);
    let guard = state.lock_session("forge_repo");
    match guard.as_ref() {
        Some(session) => forge::identify_repo_with(&session.repo.to_thread_local(), &known),
        None => Ok(None),
    }
}

/// The connected accounts. Hosts and logins; never tokens.
#[tauri::command(async)]
pub fn forge_accounts<R: Runtime>(app: AppHandle<R>) -> Vec<Account> {
    accounts::load(&app)
}

/// Connect an account by proving the token works.
///
/// The login is read back from the host rather than typed: it proves the token
/// is valid *and* gets the name in one request, and a name a person typed would
/// be a name they could get wrong in a way nothing would catch until a pull
/// request failed to be theirs.
///
/// The token goes to the OS keychain and nowhere else. It is not returned, not
/// logged, and not written to the accounts file.
#[tauri::command]
pub async fn forge_connect<R: Runtime>(
    app: AppHandle<R>,
    kind: Kind,
    host: String,
    token: String,
) -> Result<Vec<Account>> {
    let host = host.trim().to_lowercase();
    if host.is_empty() {
        return Err(Error::Forge {
            host: host.clone(),
            detail: "a host is needed".into(),
        });
    }
    // The hostname decides when it names a forge. The screen only ever sends
    // GitHub, so a `gitlab.` host was asked GitHub's GraphQL question and
    // answered "Field 'viewer' doesn't exist on type 'Query'" (BUG-043).
    if token.trim().is_empty() {
        return Err(Error::Forge {
            host,
            detail: "a token is needed".into(),
        });
    }

    // Proving the token is a network round trip, so it goes off the main
    // thread like every other one.
    // A host that names no forge is asked GitLab's question first, so a
    // self-hosted GitLab under any name connects as GitLab (FEAT-088).
    let (kind, asked) = {
        let host = host.clone();
        let token = token.trim().to_string();
        off_thread(move || forge::identify_account(&host, &token, kind)).await?
    };

    forge::keychain::store(&host, &asked, token.trim())?;

    Ok(accounts::remember(
        &app,
        Account {
            kind,
            host,
            user: asked,
        },
    ))
}

/// Disconnect an account, and forget its token.
///
/// The keychain first: a token left behind after the account is gone is a
/// credential nothing in the interface can reach to remove.
#[tauri::command]
pub fn forge_disconnect<R: Runtime>(
    app: AppHandle<R>,
    host: String,
    user: String,
) -> Result<Vec<Account>> {
    forge::keychain::forget(&host, &user)?;
    Ok(accounts::forget(&app, &host, &user))
}

/// What every forge call needs before it can make one: which repository, whose
/// token, and who that token belongs to.
///
/// Gathered in one place because the order matters and is easy to get wrong.
/// Everything local happens here — the session, the account file, the
/// keychain — and the caller then goes to the network on a thread of its own.
/// Reading the session takes a lock, and **a lock must never be held across an
/// await**, so this function takes `State` and gives it up before returning.
///
/// The three ways it can fail are three different sentences, because they are
/// three different things for the reader to do about them.
fn forge_credentials<R: Runtime>(
    app: &AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<(Repo, String, String)> {
    let Some(repo) = forge_repo(app.clone(), state)? else {
        return Err(Error::Forge {
            host: String::new(),
            detail: "this repository is not on a service Spagitty can read".into(),
        });
    };

    let connected = accounts::load(app);
    let Some(account) = accounts::for_host(&connected, &repo.host) else {
        return Err(Error::ForgeUnauthorized {
            host: repo.host.clone(),
            detail: "no account is connected for this host".into(),
        });
    };

    let Some(token) = forge::keychain::read(&account.host, &account.user)? else {
        return Err(Error::ForgeUnauthorized {
            host: repo.host.clone(),
            detail: "the token for this account is no longer in the keychain".into(),
        });
    };

    let user = account.user.clone();
    Ok((repo, token, user))
}

/// The open pull requests for the open repository.
///
/// Errors rather than an empty list when something is wrong, so the screen can
/// say whether it is offline, rate limited, or looking at a repository nobody
/// has connected an account for. An empty list means there are none.
#[tauri::command]
pub async fn pull_requests<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<Vec<PullRequest>> {
    let (repo, token, user) = forge_credentials(&app, state)?;
    let account = user.clone();
    off_thread(move || forge::pull_requests(&repo, &token, &user))
        .await
        .map_err(|error| name_account(error, &account))
}

/// A refusal says which account was refused (BUG-067). With more than one
/// account connected, "the token was refused" is a sentence about nobody.
fn name_account(error: Error, user: &str) -> Error {
    match error {
        Error::ForgeUnauthorized { host, detail } => Error::ForgeUnauthorized {
            host,
            detail: format!("{detail} (signed in as {user})"),
        },
        other => other,
    }
}

/// Create a new pull request on the configured forge (FEAT-070).
#[tauri::command]
pub async fn create_pull_request<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    title: String,
    body: String,
    head: String,
    base: String,
    draft: bool,
) -> Result<PullRequest> {
    let (repo, token, _) = forge_credentials(&app, state)?;
    off_thread(move || {
        forge::create_pull_request(&repo, &token, &title, &body, &head, &base, draft)
    })
    .await
}

/// The files one pull request changes, with the host's own diff of each.
///
/// The diff is the host's, not a local one: the head branch of a pull request
/// is usually not fetched, and fetching it to render a file list would turn
/// reading a review into a network operation on the repository (FEAT-058).
#[tauri::command]
pub async fn pull_request_files<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    number: u64,
) -> Result<Vec<FileDiff>> {
    let (repo, token, _) = forge_credentials(&app, state)?;
    off_thread(move || forge::review::pull_request_files(&repo, &token, number)).await
}

#[tauri::command]
pub async fn pull_request_commits<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    number: u64,
) -> Result<Vec<forge::review::PullRequestCommit>> {
    let (repo, token, _) = forge_credentials(&app, state)?;
    off_thread(move || forge::review::pull_request_commits(&repo, &token, number)).await
}

#[tauri::command]
pub async fn commit_files<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    sha: String,
) -> Result<Vec<FileDiff>> {
    let (repo, token, _) = forge_credentials(&app, state)?;
    off_thread(move || forge::review::commit_files(&repo, &token, &sha)).await
}

#[tauri::command]
pub async fn pull_request_comments<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    number: u64,
) -> Result<Vec<forge::review::PullRequestComment>> {
    let (repo, token, _) = forge_credentials(&app, state)?;
    off_thread(move || forge::review::pull_request_comments(&repo, &token, number)).await
}

#[tauri::command]
pub async fn submit_review<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    number: u64,
    verdict: ReviewVerdict,
    comment: String,
    draft_comments: Option<Vec<forge::review::DraftComment>>,
) -> Result<()> {
    let (repo, token, _) = forge_credentials(&app, state)?;
    let drafts = draft_comments.unwrap_or_default();
    off_thread(move || {
        forge::review::submit_review_with_comments(
            &repo, &token, number, verdict, &comment, &drafts,
        )
    })
    .await
}

#[tauri::command]
pub async fn reply_comment<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    number: u64,
    comment_id: u64,
    body: String,
) -> Result<forge::review::PullRequestComment> {
    let (repo, token, _) = forge_credentials(&app, state)?;
    off_thread(move || forge::review::reply_comment(&repo, &token, number, comment_id, &body)).await
}

/// Every comment the Review room shows, with threads and the comments on the
/// pull request as a whole (FEAT-093).
#[tauri::command]
pub async fn review_comments<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    number: u64,
) -> Result<Vec<forge::review::PullRequestComment>> {
    let (repo, token, _) = forge_credentials(&app, state)?;
    off_thread(move || forge::review::review_comments(&repo, &token, number)).await
}

/// Resolve a thread on the host, or open it again (FEAT-093).
#[tauri::command]
pub async fn resolve_thread<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    number: u64,
    thread_id: String,
    resolved: bool,
) -> Result<()> {
    let (repo, token, _) = forge_credentials(&app, state)?;
    off_thread(move || forge::review::resolve_thread(&repo, &token, number, &thread_id, resolved))
        .await
}

/// Merge a pull request on the configured forge (FEAT-071).
#[tauri::command]
pub async fn merge_pull_request<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    number: u64,
    method: MergeMethod,
    commit_title: Option<String>,
    commit_message: Option<String>,
) -> Result<()> {
    let (repo, token, _) = forge_credentials(&app, state)?;
    off_thread(move || {
        forge::review::merge_pull_request(
            &repo,
            &token,
            number,
            method,
            commit_title.as_deref(),
            commit_message.as_deref(),
        )
    })
    .await
}

/// Close / reject a pull request without merging (FEAT-071).
#[tauri::command]
pub async fn close_pull_request<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    number: u64,
) -> Result<()> {
    let (repo, token, _) = forge_credentials(&app, state)?;
    off_thread(move || forge::review::close_pull_request(&repo, &token, number)).await
}

/// Toggle draft / ready-for-review status on a pull request (FEAT-071).
#[tauri::command]
pub async fn set_pr_draft<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    number: u64,
    id: String,
    title: String,
    draft: bool,
) -> Result<()> {
    let (repo, token, _) = forge_credentials(&app, state)?;
    off_thread(move || forge::review::set_draft_status(&repo, &token, number, &id, &title, draft))
        .await
}

/// Open pull requests anywhere on a host that involve the person (FEAT-087):
/// the Review screen's "All my repos".
///
/// The host is the open repository's when it is on one, and otherwise the
/// first connected account's — the screen is still useful with a repository
/// that has no forge remote.
#[tauri::command]
pub async fn involved_pull_requests<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<Vec<PullRequest>> {
    let open = forge_repo(app.clone(), state)?;
    let connected = accounts::load(&app);
    let account = match &open {
        Some(repo) => accounts::for_host(&connected, &repo.host),
        None => connected.first(),
    }
    .cloned()
    .ok_or_else(|| Error::ForgeUnauthorized {
        host: open
            .as_ref()
            .map(|repo| repo.host.clone())
            .unwrap_or_default(),
        detail: "no account is connected for this host".into(),
    })?;

    let Some(token) = forge::keychain::read(&account.host, &account.user)? else {
        return Err(Error::ForgeUnauthorized {
            host: account.host.clone(),
            detail: "the token for this account is no longer in the keychain".into(),
        });
    };

    let user = account.user.clone();
    off_thread(move || {
        forge::involved_pull_requests(account.kind, &account.host, &token, &account.user)
    })
    .await
    .map_err(|error| name_account(error, &user))
}

/// Bring a pull request's head and its target branch into the repository,
/// unless the head already there is the one the host reports (FEAT-089).
///
/// The fetch is a network operation and runs off the main thread with the
/// session lock released: what it needs from the session — the path, the
/// repository handle, the forge remote — is taken first.
#[tauri::command]
pub async fn review_checkout<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    number: u64,
    target: String,
    head: String,
) -> Result<pull::PullHead> {
    let known = accounts::load(&app);
    let (dir, shared, remote, kind) = {
        let guard = state.lock_session("review_checkout");
        let session = guard.as_ref().ok_or(Error::NoRepository)?;
        let repository = session.repo.to_thread_local();
        let listed: Vec<(String, String)> = remotes::remotes(&repository)
            .into_iter()
            .map(|remote| (remote.name, remote.url))
            .collect();
        let (name, url) = forge::forge_remote(&listed)
            .cloned()
            .ok_or_else(|| Error::Forge {
                host: String::new(),
                detail: "no remote to fetch the pull request from".into(),
            })?;
        let kind = forge::identify_with(&url, &known)
            .map(|found| found.kind)
            .ok_or_else(|| Error::Forge {
                host: String::new(),
                detail: "this repository is not on a service Spagitty can read".into(),
            })?;
        (session.path.clone(), session.repo.clone(), name, kind)
    };

    off_thread(move || {
        if let Some(found) = pull::resolve(&shared.to_thread_local(), &remote, number, &target)? {
            if pull::is_current(&found, &head) {
                return Ok(found);
            }
        }
        pull::fetch(&dir, kind, &remote, number, &target)?;
        // Opened afresh: the refs the fetch wrote are read from disk, not from
        // whatever the shared handle had cached.
        pull::resolve(&repo::open(&dir)?, &remote, number, &target)?.ok_or_else(|| Error::Forge {
            host: kind.label().to_string(),
            detail: "the pull request's head was not fetched".into(),
        })
    })
    .await
}

/// What a pull request's merges wrote while resolving conflicts (FEAT-092).
///
/// Each merge is re-done by git, which takes a while on a large one, so the
/// session is let go first, as for the fetch.
#[tauri::command]
pub async fn review_conflicts(
    state: State<'_, AppState>,
    merge_base: String,
    head: String,
    target: String,
) -> Result<remerge::ConflictFixes> {
    let (dir, shared) = {
        let guard = state.lock_session("review_conflicts");
        let session = guard.as_ref().ok_or(Error::NoRepository)?;
        (session.path.clone(), session.repo.clone())
    };
    off_thread(move || {
        remerge::conflicts(&shared.to_thread_local(), &dir, &merge_base, &head, &target)
    })
    .await
}

/// The files a pull request changes, read from disk: from its merge base to
/// its head (FEAT-089).
#[tauri::command(async)]
pub fn review_files(
    state: State<'_, AppState>,
    from: String,
    to: String,
) -> Result<Vec<diff::FileChange>> {
    state.with_session("review_files", |session| {
        diff::changes_between(&session.repo.to_thread_local(), &from, &to)
    })
}

/// One file of a pull request, whole, read from disk (FEAT-089).
#[tauri::command(async)]
pub fn review_file(
    state: State<'_, AppState>,
    from: String,
    to: String,
    path: String,
    old_path: Option<String>,
) -> Result<diff::FullFile> {
    state.with_session("review_file", |session| {
        diff::full_file_between(
            &session.repo.to_thread_local(),
            &from,
            &to,
            &path,
            old_path.as_deref(),
        )
    })
}

/// Check a pull request's fetched head out as a branch here (FEAT-095): its
/// own name when that is free or already there, `pr-N` otherwise. The
/// branch follows the forge remote's own when that is exactly the head.
#[tauri::command(async)]
pub fn review_check_out(
    state: State<'_, AppState>,
    number: u64,
    head: String,
    source: String,
    target: String,
) -> Result<pull::CheckedOut> {
    state.with_session("review_check_out", |session| {
        // Opened afresh: the fetch just before wrote refs the shared handle
        // may not have read.
        let repository = repo::open(&session.path)?;
        let listed: Vec<(String, String)> = remotes::remotes(&repository)
            .into_iter()
            .map(|remote| (remote.name, remote.url))
            .collect();
        let remote = forge::forge_remote(&listed)
            .map(|(name, _)| name.clone())
            .unwrap_or_else(|| "origin".to_string());
        pull::check_out(&repository, &remote, number, &head, &source, &target)
    })
}

/// One pull request the Review inbox wants checks and threads for.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SummaryAsk {
    /// `owner/name`, or null for the open repository.
    repository: Option<String>,
    number: u64,
}

/// Checks and thread counts the list did not carry (FEAT-088).
///
/// GitLab's merge request list has neither; GitHub's has both and answers
/// with nothing here. Each pull request names its repository when it came
/// from "All my repos"; the rest are the open repository's.
#[tauri::command]
pub async fn review_summaries<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    asks: Vec<SummaryAsk>,
) -> Result<Vec<(Option<String>, forge::ReviewSummary)>> {
    let open = forge_repo(app.clone(), state)?;
    let connected = accounts::load(&app);
    let Some(account) = (match &open {
        Some(repo) => accounts::for_host(&connected, &repo.host),
        None => connected.first(),
    })
    .cloned() else {
        return Ok(Vec::new());
    };
    let Some(token) = forge::keychain::read(&account.host, &account.user)? else {
        return Ok(Vec::new());
    };

    off_thread(move || {
        // Grouped by repository, so each project is asked about once.
        let mut by_repo: Vec<(Option<String>, Vec<u64>)> = Vec::new();
        for ask in asks {
            match by_repo.iter_mut().find(|(slug, _)| *slug == ask.repository) {
                Some((_, numbers)) => numbers.push(ask.number),
                None => by_repo.push((ask.repository, vec![ask.number])),
            }
        }

        let mut found = Vec::new();
        for (slug, numbers) in by_repo {
            let repo = match &slug {
                None => match &open {
                    Some(repo) => repo.clone(),
                    None => continue,
                },
                Some(slug) => {
                    let Some((owner, name)) = slug.rsplit_once('/') else {
                        continue;
                    };
                    Repo {
                        kind: account.kind,
                        host: account.host.clone(),
                        owner: owner.to_string(),
                        name: name.to_string(),
                    }
                }
            };
            for summary in forge::review_summaries(&repo, &token, &account.user, &numbers) {
                found.push((slug.clone(), summary));
            }
        }
        Ok(found)
    })
    .await
}

/// Which recent repository is a clone of `slug` on `host`, if any (FEAT-087).
///
/// A row from "All my repos" belongs to some repository; reviewing it means
/// opening that repository, and this is how the screen finds where it is on
/// disk. Read from the remotes of the repositories Spagitty already knows,
/// and nothing else is searched.
#[tauri::command(async)]
pub fn local_clone_of<R: Runtime>(app: AppHandle<R>, host: String, slug: String) -> Option<String> {
    let known = accounts::load(&app);
    recents::load(&app).into_iter().find_map(|path| {
        let repository = repo::open(&path).ok()?;
        let found = forge::identify_repo_with(&repository, &known)
            .ok()
            .flatten()?;
        (found.host == host && found.slug().eq_ignore_ascii_case(&slug))
            .then(|| path.to_string_lossy().into_owned())
    })
}

/// What the reviewer has done on one pull request, or null (FEAT-087).
#[tauri::command(async)]
pub fn review_state<R: Runtime>(
    app: AppHandle<R>,
    host: String,
    owner: String,
    name: String,
    number: u64,
) -> Option<serde_json::Value> {
    let root = review_state::root(&app)?;
    review_state::read(&review_state::path_in(&root, &host, &owner, &name, number)?)
}

/// Keep what the reviewer has done on one pull request (FEAT-087). A null
/// state forgets it.
#[tauri::command(async)]
pub fn set_review_state<R: Runtime>(
    app: AppHandle<R>,
    host: String,
    owner: String,
    name: String,
    number: u64,
    state: serde_json::Value,
) -> Result<()> {
    let path = review_state::root(&app)
        .and_then(|root| review_state::path_in(&root, &host, &owner, &name, number))
        .ok_or_else(|| Error::Config("nowhere to keep this review".into()))?;
    if state.is_null() {
        return review_state::forget(&path).map_err(Error::Config);
    }
    review_state::write(&path, &state).map_err(Error::Config)
}

/// Is there a newer Spagitty than this one?
///
/// One unauthenticated request to the project's own releases endpoint. It is
/// the caller's job to decide whether to ask — `check_for_updates` gates the
/// one at startup — because a preference that stops a request has to stop it
/// before it is made, not after.
///
/// **`async`, and the work goes to a blocking thread.** Tauri runs a
/// synchronous command on the main thread, so a request with a thirty-second
/// timeout freezes the window until it answers — which on a machine with no
/// route out is the whole thirty seconds, at startup, before anything is on
/// screen. Every command here that touches the network is shaped this way for
/// that reason.
#[tauri::command]
pub async fn check_update() -> Result<update::Update> {
    off_thread(update::check).await
}

/// Run blocking work off the main thread, keeping the error type.
///
/// A panic inside is reported rather than swallowed: it means a bug, and a
/// command that quietly returned nothing would hide it.
async fn off_thread<T, F>(work: F) -> Result<T>
where
    F: FnOnce() -> Result<T> + Send + 'static,
    T: Send + 'static,
{
    match tauri::async_runtime::spawn_blocking(work).await {
        Ok(result) => result,
        Err(error) => Err(Error::Forge {
            host: String::new(),
            detail: format!("the request could not be run: {error}"),
        }),
    }
}

/// Signing, as git would resolve it here (FEAT-019).
///
/// Read from the open repository's cascade when there is one, and from the
/// global configuration when there is not — Settings does not need a repository
/// open, and the global value is the one it can still offer.
#[tauri::command(async)]
pub fn signing(state: State<'_, AppState>) -> Result<Signing> {
    let guard = state.lock_session("signing");
    match guard.as_ref() {
        Some(session) => Ok(signing::read(&session.repo.to_thread_local())),
        None => signing::read_global(),
    }
}

/// Turn `commit.gpgsign` on or off in one scope, and report what that left.
///
/// The same shape as `set_identity`, because it is the same problem: a git
/// config key with a global value and a repository override, written through
/// `git config` so every other tool sees it.
#[tauri::command]
pub fn set_signing(state: State<'_, AppState>, scope: Scope, on: bool) -> Result<Signing> {
    let open = state
        .session
        .lock()
        .expect("session lock")
        .as_ref()
        .map(|session| session.path.clone());

    let directory = match (scope, open) {
        (_, Some(path)) => path,
        (Scope::Local, None) => return Err(Error::NoRepository),
        (Scope::Global, None) => std::env::current_dir()?,
    };

    signing::set(&directory, scope, on)?;
    signing(state)
}

/// Clear `commit.gpgsign` in one scope, so the next one up decides again.
#[tauri::command]
pub fn clear_signing(state: State<'_, AppState>, scope: Scope) -> Result<Signing> {
    let open = state
        .session
        .lock()
        .expect("session lock")
        .as_ref()
        .map(|session| session.path.clone());

    let directory = match (scope, open) {
        (_, Some(path)) => path,
        (Scope::Local, None) => return Err(Error::NoRepository),
        (Scope::Global, None) => std::env::current_dir()?,
    };

    signing::clear(&directory, scope)?;
    signing(state)
}

#[tauri::command(async)]
pub fn settings<R: Runtime>(app: AppHandle<R>) -> Settings {
    crate::settings::load(&app)
}

/// Store the behaviour toggles.
///
/// A failed write is reported rather than swallowed: a toggle that did not
/// persist looks exactly like one that did, until the next restart.
///
/// **Turning the avatars off empties their cache.** A preference that stops
/// future requests but leaves the pictures already on disk being drawn is a
/// preference that did not do what it says; the pictures go with the switch.
#[tauri::command]
pub fn set_settings<R: Runtime>(
    app: AppHandle<R>,
    settings: Settings,
) -> std::result::Result<(), String> {
    if !settings.fetch_avatars {
        if let Some(cache) = avatar_cache(&app) {
            let _ = avatars::forget(&cache);
        }
    }
    crate::settings::save(&app, settings)
}

/// One author's picture and handle, for a node on the graph (FEAT-079).
///
/// Answers for a single address, because that is the unit that is cached and
/// the unit the frontend deduplicates to: a screen of three hundred rows is a
/// handful of distinct authors, and asking per author is what makes the second
/// look at a repository cost nothing.
///
/// Both halves are absent far more often than not, and neither absence is a
/// failure. A `handle` needs an address that carries one; a `picture` needs
/// somebody to have uploaded one, and a network to fetch it over. The graph
/// draws its generated face when this says nothing, which is what it drew
/// before this command existed.
///
/// **The preference is checked here, not by the caller.** A setting that stops
/// a request has to stop it in the place the request is made — a frontend that
/// forgot to ask would otherwise be the whole opt-out.
///
/// **`commit` is one of this author's commits (FEAT-081).** An ordinary address
/// says nothing about which account wrote it, and the repository's forge does:
/// when the open repository is on GitHub, the resolver asks it about this one
/// commit, once per author. A connected account's token is read for that
/// request only, and only when the disk has no answer already.
#[tauri::command]
pub async fn avatar<R: Runtime>(
    app: AppHandle<R>,
    email: String,
    commit: Option<String>,
) -> AvatarAnswer {
    let handle = avatars::source(&email).handle;

    if !crate::settings::load(&app).fetch_avatars {
        // The handle still stands: it was read out of the address, and reading
        // it sent nothing anywhere.
        return AvatarAnswer {
            handle,
            picture: None,
            retry: false,
        };
    }

    let Some(root) = avatar_cache(&app) else {
        return AvatarAnswer {
            handle,
            picture: None,
            retry: false,
        };
    };

    // Taken from the app rather than as a command argument: an async command
    // that borrows `State` has to return a `Result`, and nothing here is one.
    let repo = forge_repo(app.clone(), app.state::<AppState>())
        .ok()
        .flatten();

    // Off the main thread for the reason on `check_update`: this is a network
    // request with a timeout on it, and a synchronous command holds the window
    // for as long as it takes. The keychain read is blocking too.
    let answer = tauri::async_runtime::spawn_blocking(move || {
        static RETIRED: std::sync::Once = std::sync::Once::new();
        RETIRED.call_once(|| avatars::retire_legacy(&root));

        let token = || {
            let host = &repo.as_ref()?.host;
            let connected = accounts::load(&app);
            let account = accounts::for_host(&connected, host)?;
            forge::keychain::read(&account.host, &account.user)
                .ok()
                .flatten()
        };
        let cache = avatars::cache_dir(&root);

        // An ordinary address in a GitHub or Bitbucket repository, asked
        // without a commit it authored — a committer in the detail pane. Only
        // the forge can say who that is, so nothing is sent: a Gravatar miss
        // written now would hide the forge's answer when a row this person
        // authored asks properly. GitLab is asked by address, so it needs no
        // commit (BUG-053).
        let by_commit = repo.as_ref().is_some_and(|repo| repo.kind != Kind::GitLab);
        if by_commit && commit.is_none() && avatars::source(&email).handle.is_none() {
            return avatars::Answer {
                picture: avatars::cached(&cache, &email),
                ..avatars::Answer::default()
            };
        }

        let commit = match (&repo, &commit) {
            (Some(repo), Some(id)) => Some(avatars::Commit {
                repo,
                id,
                token: &token,
            }),
            (Some(repo), None) if repo.kind == Kind::GitLab => Some(avatars::Commit {
                repo,
                id: "",
                token: &token,
            }),
            _ => None,
        };
        avatars::picture(&cache, &email, commit.as_ref(), &avatars::Live)
    })
    .await
    .unwrap_or_default();

    AvatarAnswer {
        handle: answer.handle.or(handle),
        picture: answer.picture,
        retry: answer.retry,
    }
}

/// Where fetched pictures are kept.
///
/// The cache directory rather than the configuration one: this is data that
/// can be thrown away and rebuilt from the network, which is what a cache
/// directory is for and what makes it right for a backup tool to skip.
fn avatar_cache<R: Runtime>(app: &AppHandle<R>) -> Option<PathBuf> {
    app.path()
        .app_cache_dir()
        .ok()
        .map(|dir| dir.join("avatars"))
}

/// The path the app was launched with, if any: `spagitty /path/to/repo`.
#[tauri::command(async)]
pub fn launch_path<R: Runtime>(app: AppHandle<R>) -> Option<PathBuf> {
    let _ = app.webview_windows();
    std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .filter(|p| p.exists())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::{self, Emitted};
    use serde_json::Value;
    use spagitty_core::fixture::Fixture;
    use tauri::test::MockRuntime;
    use tauri::{App, Manager};

    /// An application and the repository it is about to be given.
    ///
    /// Every command takes `State<'_, AppState>`, which can only be borrowed
    /// from a built application, so the app has to outlive the calls — hence
    /// holding it rather than returning the state.
    fn app() -> App<MockRuntime> {
        testing::app()
    }

    fn open(app: &App<MockRuntime>, at: &std::path::Path) -> Result<OpenResult> {
        open_repo(
            app.handle().clone(),
            app.state::<AppState>(),
            at.to_path_buf(),
            None,
        )
    }

    #[test]
    fn a_command_against_no_open_repository_says_so() {
        // The first thing every screen does on a cold start. It has to be an
        // answer rather than a panic, and the same answer whichever command
        // asked.
        let app = app();
        let state = app.state::<AppState>();

        assert!(matches!(snapshot(state.clone()), Err(Error::NoRepository)));
        assert!(matches!(branches(state.clone()), Err(Error::NoRepository)));
        assert!(matches!(stashes(state.clone()), Err(Error::NoRepository)));
        assert!(matches!(
            commit_detail(state.clone(), "HEAD".into()),
            Err(Error::NoRepository)
        ));
        assert!(matches!(
            graph_request(state.clone(), 0, 10),
            Err(Error::NoRepository)
        ));
        assert!(matches!(
            graph_order(app.handle().clone(), state, GraphOrder::Branch),
            Err(Error::NoRepository)
        ));
    }

    const READ: bool = false;
    const WRITE: bool = true;

    /// Hold a turn at the repository for 150 ms on another thread, and time
    /// how long a second, empty command takes to get its turn beside it.
    fn waited_beside(state: &AppState, slow: bool, quick: bool) -> std::time::Duration {
        std::thread::scope(|scope| {
            let (holding, held) = std::sync::mpsc::channel();
            scope.spawn(move || {
                state
                    .with_turn("t052_slow", slow, |_| {
                        holding.send(()).expect("told");
                        std::thread::sleep(std::time::Duration::from_millis(150));
                        Ok(())
                    })
                    .expect("the slow command");
            });
            held.recv().expect("the slow command has started");
            let asked = std::time::Instant::now();
            state
                .with_turn("t052_quick", quick, |_| Ok(()))
                .expect("the quick command");
            asked.elapsed()
        })
    }

    const WAITED: std::time::Duration = std::time::Duration::from_millis(100);

    /// TASK-052: a long blame no longer holds up selecting a commit.
    #[test]
    fn a_quick_read_does_not_wait_for_a_slow_one() {
        let fixture = Fixture::woven();
        let app = app();
        open(&app, fixture.path()).expect("opening the fixture");
        let state: &AppState = &app.state::<AppState>();

        assert!(waited_beside(state, READ, READ) < WAITED);
    }

    #[test]
    fn two_writes_never_overlap_and_reads_wait_for_neither() {
        let fixture = Fixture::woven();
        let app = app();
        open(&app, fixture.path()).expect("opening the fixture");
        let state: &AppState = &app.state::<AppState>();

        assert!(waited_beside(state, WRITE, WRITE) >= WAITED);
        // A read runs beside a write, as it would beside git itself, and a
        // write beside a read.
        assert!(waited_beside(state, WRITE, READ) < WAITED);
        assert!(waited_beside(state, READ, WRITE) < WAITED);
    }

    /// The timings still say how long a command held the repository.
    #[test]
    fn the_timings_say_how_long_a_command_held_the_repository() {
        let fixture = Fixture::woven();
        let app = app();
        open(&app, fixture.path()).expect("opening the fixture");
        let state: &AppState = &app.state::<AppState>();

        let before = crate::timing::since(0).last().map_or(0, |entry| entry.seq);
        state
            .with_turn("t052_timed", READ, |_| {
                std::thread::sleep(std::time::Duration::from_millis(30));
                Ok(())
            })
            .expect("the read");
        let timed = crate::timing::since(before)
            .into_iter()
            .find(|entry| entry.command == "t052_timed")
            .expect("timed");
        assert!(timed.held_us >= 30_000, "{timed:?}");
    }

    #[test]
    fn a_read_that_outlives_its_repository_is_not_delivered() {
        let first = Fixture::woven();
        let second = Fixture::woven();
        let app = app();
        open(&app, first.path()).expect("opening the first");
        let state: &AppState = &app.state::<AppState>();

        std::thread::scope(|scope| {
            let (reading, read) = std::sync::mpsc::channel();
            let (opened, go_on) = std::sync::mpsc::channel::<()>();
            let slow = scope.spawn(move || {
                state.with_session("blame", |session| {
                    reading.send(()).expect("told");
                    go_on.recv().expect("the other repository is open");
                    Ok(session.path.clone())
                })
            });
            read.recv().expect("the read has its repository");
            open(&app, second.path()).expect("opening the second");
            opened.send(()).expect("told");
            assert!(matches!(
                slow.join().expect("joined"),
                Err(Error::Replaced(_))
            ));
        });

        // A read that starts now reads the second.
        let now = state
            .with_session("blame", |session| Ok(session.path.clone()))
            .expect("the read");
        assert_eq!(
            now.canonicalize().unwrap(),
            second.path().canonicalize().unwrap()
        );
    }

    /// Every command that reaches the repository says whether it writes, so
    /// a new one is classified by somebody deciding (TASK-052).
    #[test]
    fn every_command_is_read_or_write_on_purpose() {
        let source = include_str!("commands.rs");
        let code = &source[..source.find("mod tests {").unwrap_or(source.len())];
        let named = session_commands(code);
        assert!(named.len() > 80, "found only {} commands", named.len());
        for name in &named {
            let reads = READERS.contains(&name.as_str());
            let writes = WRITERS.contains(&name.as_str());
            assert!(
                reads != writes,
                "`{name}` must be in exactly one of READERS and WRITERS"
            );
        }
        for listed in READERS.iter().chain(WRITERS) {
            assert!(
                named.iter().any(|name| name == listed),
                "`{listed}` is listed but no command uses it"
            );
        }
    }

    /// The names given to `with_session`, `with_live_session` and `files_repo`.
    fn session_commands(code: &str) -> Vec<String> {
        let mut names = Vec::new();
        for opener in ["with_session(", "with_live_session(", "files_repo(&state,"] {
            let mut rest = code;
            while let Some(at) = rest.find(opener) {
                rest = &rest[at + opener.len()..];
                if let Some(quoted) = rest.trim_start().strip_prefix('"') {
                    if let Some(end) = quoted.find('"') {
                        names.push(quoted[..end].to_string());
                    }
                }
            }
        }
        names.sort();
        names.dedup();
        names
    }

    #[test]
    fn opening_a_repository_starts_a_walk_and_reports_what_is_there() {
        let fixture = Fixture::woven();
        let app = app();
        let rows = Emitted::<Value>::on(app.handle(), crate::graph_worker::ROWS_EVENT);

        let opened = open(&app, fixture.path()).expect("opening the fixture");

        assert_eq!(opened.info.head.branch.as_deref(), Some("main"));
        // Opening walks nothing. The worker waits for the first request.
        rows.no_more_than(0);

        graph_request(app.state::<AppState>(), opened.token, 3).expect("asking for rows");
        assert_eq!(rows.at_least(1)[0]["rows"].as_array().unwrap().len(), 3);
    }

    #[test]
    fn opening_a_second_repository_replaces_the_first() {
        // Replacing the session drops the previous worker and watcher, which
        // joins both threads. If either ignored the shutdown this would hang
        // rather than fail, so it is given a deadline.
        let first = Fixture::woven();
        let second = Fixture::woven();
        let app = app();

        let one = open(&app, first.path()).expect("opening the first");

        let paths = (second.path().to_path_buf(), one.token);
        let handle = app.handle().clone();
        let state_is_replaced = std::sync::Arc::new(std::sync::Mutex::new(None));
        let out = state_is_replaced.clone();

        testing::finishes_promptly("replacing the open repository", move || {
            let app = handle;
            let two = open_repo(app.clone(), app.state::<AppState>(), paths.0, None)
                .expect("opening the second");
            *out.lock().expect("result") = Some(two.token);
        });

        let two = state_is_replaced.lock().expect("result").expect("a token");
        assert_ne!(two, one.token, "a replaced walk gets a new token");

        // The old repository is gone: a command reads the new one.
        let info = snapshot(app.state::<AppState>()).expect("a snapshot");
        assert_eq!(
            info.info.path.canonicalize().unwrap(),
            second.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn a_request_against_a_replaced_walk_is_ignored_rather_than_refused() {
        // The UI can send a request that raced a refresh. Rows carry the token
        // they belong to and stale ones are dropped on arrival, so a stale
        // request is nothing to report.
        let fixture = Fixture::woven();
        let app = app();
        let rows = Emitted::<Value>::on(app.handle(), crate::graph_worker::ROWS_EVENT);

        let opened = open(&app, fixture.path()).expect("opening the fixture");
        let stale = opened.token.wrapping_add(1);

        assert!(graph_request(app.state::<AppState>(), stale, 5).is_ok());
        rows.no_more_than(0);

        // And the walk that is actually running still answers.
        graph_request(app.state::<AppState>(), opened.token, 2).expect("asking for rows");
        assert_eq!(rows.at_least(1)[0]["token"], opened.token);
    }

    #[test]
    fn restarting_the_walk_produces_a_new_token_and_leaves_the_old_one_stale() {
        let fixture = Fixture::woven();
        let app = app();

        let opened = open(&app, fixture.path()).expect("opening the fixture");
        let restarted = graph_restart(app.handle().clone(), app.state::<AppState>())
            .expect("restarting the walk");

        assert_ne!(restarted, opened.token);
        // The old token now names nothing, which is not an error.
        assert!(graph_request(app.state::<AppState>(), opened.token, 5).is_ok());
    }

    #[test]
    fn changing_the_graph_order_produces_a_new_token() {
        let fixture = Fixture::woven();
        let app = app();

        let opened = open(&app, fixture.path()).expect("opening the fixture");
        let reordered = graph_order(
            app.handle().clone(),
            app.state::<AppState>(),
            GraphOrder::Branch,
        )
        .expect("changing the order");

        assert_ne!(reordered, opened.token);
        assert!(graph_request(app.state::<AppState>(), opened.token, 5).is_ok());
    }

    #[test]
    fn identity_saves_and_reads_use_the_current_configuration_in_both_scopes() {
        let fixture = Fixture::empty();
        let app = app();
        open(&app, fixture.path()).expect("opening the fixture");

        // `app()` puts HOME in a temporary directory, so the global writes
        // here never reach the configuration of the person running the tests.
        for scope in [Scope::Global, Scope::Local] {
            let fields = match scope {
                Scope::Global => [
                    (Key::Name, "Grace Hopper"),
                    (Key::Email, "grace@example.com"),
                ],
                Scope::Local => [
                    (Key::Name, "Margaret Hamilton"),
                    (Key::Email, "margaret@example.com"),
                ],
            };
            for (key, text) in fields {
                let saved = set_identity(app.state::<AppState>(), scope, key, text.into())
                    .expect("saving identity");
                let reread = identity(app.state::<AppState>()).expect("reading identity");
                let (saved, reread) = match key {
                    Key::Name => (saved.name, reread.name),
                    Key::Email => (saved.email, reread.email),
                };
                let (saved_value, reread_value) = match scope {
                    Scope::Global => (saved.global, reread.global),
                    Scope::Local => (saved.local, reread.local),
                };
                assert_eq!(
                    saved_value.as_deref(),
                    Some(text),
                    "save returned stale config for {scope:?} {key:?}"
                );
                assert_eq!(
                    reread_value.as_deref(),
                    Some(text),
                    "read returned stale config for {scope:?} {key:?}"
                );
                // Fixture::git gives its subprocess a different HOME. Use the
                // same isolated home as the command under test for this read.
                let config = spagitty_core::shell::program("git")
                    .current_dir(fixture.path())
                    .args(["config", scope.flag(), "--get", key.key()])
                    .output()
                    .expect("reading git config");
                assert!(config.status.success());
                assert_eq!(String::from_utf8_lossy(&config.stdout).trim(), text);
            }
        }

        let cleared = set_identity(app.state::<AppState>(), Scope::Local, Key::Name, "".into())
            .expect("clearing the local name");
        assert_eq!(cleared.name.local, None);
        assert_eq!(cleared.name.global.as_deref(), Some("Grace Hopper"));
        assert_eq!(cleared.name.effective.as_deref(), Some("Grace Hopper"));
        fixture.git(&["config", "--local", "user.name", "Katherine Johnson"]);
        let edited = identity(app.state::<AppState>()).expect("external config edit");
        assert_eq!(edited.name.local.as_deref(), Some("Katherine Johnson"));
        assert_eq!(edited.name.global.as_deref(), Some("Grace Hopper"));
    }

    #[test]
    fn closing_puts_the_session_back_to_having_no_repository() {
        let fixture = Fixture::woven();
        let app = app();

        open(&app, fixture.path()).expect("opening the fixture");

        testing::finishes_promptly("closing the repository", {
            let handle = app.handle().clone();
            move || close_repo(handle.state::<AppState>())
        });

        assert!(matches!(
            snapshot(app.state::<AppState>()),
            Err(Error::NoRepository)
        ));
    }

    #[test]
    fn opening_something_that_is_not_a_repository_leaves_the_session_alone() {
        // The failure must not tear down the repository the user already had
        // open, which is what a half-replaced session would do.
        let fixture = Fixture::woven();
        let empty = tempfile::tempdir().expect("a directory that is not a repository");
        let app = app();

        open(&app, fixture.path()).expect("opening the fixture");
        assert!(open(&app, empty.path()).is_err());

        let still_there = snapshot(app.state::<AppState>()).expect("the first repository");
        assert_eq!(
            still_there.info.path.canonicalize().unwrap(),
            fixture.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn an_open_overtaken_by_a_later_one_is_dropped() {
        // TASK-048. Off the main thread, a large repository asked for first can
        // finish reading after a small one asked for second. The second is what
        // the user wants; the first must not land on top of it.
        let first = Fixture::woven();
        let second = Fixture::woven();
        let app = app();
        let state = app.state::<AppState>();

        let slow = state.ticket();
        open(&app, second.path()).expect("the later open");

        let overtaken = open_as(
            app.handle().clone(),
            app.state::<AppState>(),
            first.path().to_path_buf(),
            None,
            slow,
        );
        assert!(matches!(overtaken, Err(Error::Superseded(_))));

        let open_now = snapshot(app.state::<AppState>()).expect("the later repository");
        assert_eq!(
            open_now.info.path.canonicalize().unwrap(),
            second.path().canonicalize().unwrap()
        );
    }

    #[test]
    fn an_open_overtaken_by_a_close_does_not_reopen() {
        // Closing the last tab while its repository is still being read must
        // leave nothing open (BUG-019's symptom, by another route).
        let fixture = Fixture::woven();
        let app = app();

        let slow = app.state::<AppState>().ticket();
        close_repo(app.state::<AppState>());

        let overtaken = open_as(
            app.handle().clone(),
            app.state::<AppState>(),
            fixture.path().to_path_buf(),
            None,
            slow,
        );
        assert!(matches!(overtaken, Err(Error::Superseded(_))));
        assert!(matches!(
            snapshot(app.state::<AppState>()),
            Err(Error::NoRepository)
        ));
    }

    #[test]
    fn an_open_after_a_close_still_opens() {
        let fixture = Fixture::woven();
        let app = app();

        close_repo(app.state::<AppState>());
        open(&app, fixture.path()).expect("opening after closing");

        assert!(snapshot(app.state::<AppState>()).is_ok());
    }

    #[test]
    fn only_the_ordered_writers_run_on_the_main_thread() {
        // TASK-048. A plain `#[tauri::command]` runs on the main thread, so
        // anything it reads there — a repository, a lock another command holds
        // — is time the window does not paint. The ones left there write the
        // application's own files or the user's git configuration: a few
        // milliseconds each, and their order is the order the user asked in.
        // A new command goes off the main thread unless it is one of these.
        const ORDERED: &[&str] = &[
            "apply_identity_profile",
            "clear_git_commands",
            "clear_signing",
            "delete_identity_profile",
            "forge_disconnect",
            "forget_repo",
            "save_identity_profile",
            "set_external_tool",
            "set_identity",
            "set_settings",
            "set_signing",
        ];

        let mut on_main = Vec::new();
        let mut lines = include_str!("commands.rs").lines();
        while let Some(line) = lines.next() {
            if line.trim_end() != "#[tauri::command]" {
                continue;
            }
            let next = lines.next().unwrap_or_default();
            if let Some(rest) = next.strip_prefix("pub fn ") {
                let name: String = rest
                    .chars()
                    .take_while(|c| c.is_alphanumeric() || *c == '_')
                    .collect();
                on_main.push(name);
            }
        }
        on_main.sort();

        assert_eq!(on_main, ORDERED);
    }
}
