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
use spagitty_core::forge::review::ReviewVerdict;
use spagitty_core::forge::{self, Account, Kind, MergeMethod, PullRequest, Repo};
use spagitty_core::graph::ROW_PITCH;
use spagitty_core::identity::{self, Identity, Key, Scope};
use spagitty_core::ops::{self, Integration, ResetMode, StashAction};
use spagitty_core::rebase::{self, Edit, Preview, Todo};
use spagitty_core::record::{self, Executed};
use spagitty_core::reflog;
use spagitty_core::refs::RefIndex;
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
use spagitty_core::update;
use spagitty_core::work;
use spagitty_core::worktrees::{self, Worktree};
use spagitty_core::{Error, Result};
use tauri::{AppHandle, Manager, Runtime, State};

use crate::about::{About, Licenses};
use crate::accounts;
use crate::clone_worker::{self, CloneWorker};
use crate::graph_worker::{self, GraphWorker};
use crate::network_worker::{self, NetworkWorker};
use crate::profiles;
use crate::rebase_worker::{self, RebaseWorker};
use crate::recents;
use crate::search_worker::{self, SearchWorker};
use crate::settings::Settings;
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
    _watcher: Option<RepoWatcher>,
}

#[derive(Default)]
pub struct AppState {
    session: Mutex<Option<Session>>,
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
}

impl AppState {
    fn with_session<T>(&self, f: impl FnOnce(&Session) -> Result<T>) -> Result<T> {
        let guard = self.session.lock().expect("session lock");
        let session = guard.as_ref().ok_or(Error::NoRepository)?;
        f(session)
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
#[tauri::command]
pub fn open_repo<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    path: PathBuf,
) -> Result<OpenResult> {
    let sync = repo::open_sync(&path)?;
    let local = sync.to_thread_local();

    let info = repo::info(&local)?;
    let refs = RefIndex::build(&local)?;
    let counts = status::counts(&local, &refs)?;
    let git_dir = local.git_dir().to_path_buf();

    let token = state.next_token.fetch_add(1, Ordering::Relaxed);
    let graph = graph_worker::spawn(
        app.clone(),
        info.path.clone(),
        token,
        Vec::new(),
        Vec::new(),
    );
    let watcher = watch::watch(app.clone(), &git_dir);

    // Opening is the only way a repository joins the list. Spagitty never goes
    // looking for repositories on its own.
    recents::remember(&app, &info.path);

    // Replacing the session drops the previous worker and watcher, which joins
    // their threads before we return.
    *state.session.lock().expect("session lock") = Some(Session {
        path: info.path.clone(),
        repo: sync,
        graph,
        visible: Vec::new(),
        pinned: Vec::new(),
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
#[tauri::command]
pub fn graph_request(state: State<'_, AppState>, token: u64, count: usize) -> Result<()> {
    state.with_session(|session| {
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
#[tauri::command]
pub fn graph_restart<R: Runtime>(app: AppHandle<R>, state: State<'_, AppState>) -> Result<u64> {
    let mut guard = state.session.lock().expect("session lock");
    let session = guard.as_mut().ok_or(Error::NoRepository)?;

    let token = state.next_token.fetch_add(1, Ordering::Relaxed);
    let (visible, pinned) = (session.visible.clone(), session.pinned.clone());
    session.graph = graph_worker::spawn(app, session.path.clone(), token, visible, pinned);
    Ok(token)
}

/// Choose which refs the graph is rooted at, and restart the walk.
///
/// An empty list is every branch. Hide, Solo and Smart Branch Visibility are
/// all the same command with a different list, computed on screen from the
/// branches it is already showing — the backend does not need to know which of
/// the three the user pressed, only what they want to see.
#[tauri::command]
pub fn graph_visibility<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    refs: Vec<String>,
    pinned: Vec<String>,
) -> Result<u64> {
    let mut guard = state.session.lock().expect("session lock");
    let session = guard.as_mut().ok_or(Error::NoRepository)?;

    session.visible = refs;
    session.pinned = pinned;
    let token = state.next_token.fetch_add(1, Ordering::Relaxed);
    let (visible, pinned) = (session.visible.clone(), session.pinned.clone());
    session.graph = graph_worker::spawn(app, session.path.clone(), token, visible, pinned);
    Ok(token)
}

/// HEAD and the rail counts, re-read. Called after `repo-changed`.
#[tauri::command]
pub fn snapshot(state: State<'_, AppState>) -> Result<Snapshot> {
    state.with_session(|session| {
        let local = session.repo.to_thread_local();
        let refs = RefIndex::build(&local)?;
        Ok(Snapshot {
            info: repo::info(&local)?,
            counts: status::counts(&local, &refs)?,
        })
    })
}

/// Everything the detail panel shows for one commit.
#[tauri::command]
pub fn commit_detail(state: State<'_, AppState>, id: String) -> Result<CommitDetail> {
    state.with_session(|session| diff::commit_detail(&session.repo.to_thread_local(), &id))
}

/// The Diff screen's file list and header counts.
///
/// This reads every blob the commit touched, because `+n −m` per file cannot
/// be known without diffing them. It is one call when the screen opens.
#[tauri::command]
pub fn commit_diff(state: State<'_, AppState>, id: String) -> Result<CommitDiff> {
    state.with_session(|session| diff::commit_diff(&session.repo.to_thread_local(), &id))
}

/// The hunks of one file, fetched as that file is selected.
#[tauri::command]
pub fn file_diff(state: State<'_, AppState>, id: String, path: String) -> Result<FileDiff> {
    state.with_session(|session| diff::file_diff(&session.repo.to_thread_local(), &id, &path))
}

/// The staged, unstaged and conflicted lists behind the Working copy screen.
///
/// This is the full status walk, so it is one call per refresh rather than one
/// per row.
#[tauri::command]
pub fn working_copy(state: State<'_, AppState>) -> Result<WorkingCopy> {
    state.with_session(|session| status::working_copy(&session.repo.to_thread_local()))
}

/// One working-copy file's hunks, on either side of the index.
#[tauri::command]
pub fn working_diff(state: State<'_, AppState>, path: String, side: Side) -> Result<FileDiff> {
    state.with_session(|session| {
        diff::working_file_diff(&session.repo.to_thread_local(), &path, side)
    })
}

/// Detailed binary / image diff metadata for a commit file (FEAT-065).
#[tauri::command]
pub fn binary_file_diff(
    state: State<'_, AppState>,
    id: String,
    path: String,
) -> Result<diff::BinaryDiff> {
    state
        .with_session(|session| diff::binary_file_diff(&session.repo.to_thread_local(), &id, &path))
}

/// Detailed binary / image diff metadata for working copy (FEAT-065).
#[tauri::command]
pub fn binary_working_diff(
    state: State<'_, AppState>,
    path: String,
    side: Side,
) -> Result<diff::BinaryDiff> {
    state.with_session(|session| {
        diff::binary_working_diff(&session.repo.to_thread_local(), &path, side)
    })
}

#[tauri::command]
pub fn stage(state: State<'_, AppState>, paths: Vec<String>) -> Result<()> {
    state.with_session(|session| work::stage(&session.repo.to_thread_local(), &paths))
}

#[tauri::command]
pub fn unstage(state: State<'_, AppState>, paths: Vec<String>) -> Result<()> {
    state.with_session(|session| work::unstage(&session.repo.to_thread_local(), &paths))
}

/// Stage one hunk. `header` identifies it, so a stale view is refused rather
/// than half-applied.
#[tauri::command]
pub fn stage_hunk(
    state: State<'_, AppState>,
    path: String,
    index: usize,
    header: String,
) -> Result<()> {
    state.with_session(|session| {
        work::stage_hunk(&session.repo.to_thread_local(), &path, index, &header)
    })
}

#[tauri::command]
pub fn unstage_hunk(
    state: State<'_, AppState>,
    path: String,
    index: usize,
    header: String,
) -> Result<()> {
    state.with_session(|session| {
        work::unstage_hunk(&session.repo.to_thread_local(), &path, index, &header)
    })
}

/// Throw away unstaged changes to whole paths.
///
/// The only command here that destroys work git cannot get back. The screen
/// confirms first; nothing in this layer asks a second time, because a
/// confirmation the caller cannot see the wording of is not a confirmation.
#[tauri::command]
pub fn discard(state: State<'_, AppState>, paths: Vec<String>) -> Result<()> {
    state.with_session(|session| work::discard(&session.repo.to_thread_local(), &paths))
}

/// Throw away one unstaged hunk. `header` identifies it, so a stale view is
/// refused rather than the wrong part of a file being lost.
#[tauri::command]
pub fn discard_hunk(
    state: State<'_, AppState>,
    path: String,
    index: usize,
    header: String,
) -> Result<()> {
    state.with_session(|session| {
        work::discard_hunk(&session.repo.to_thread_local(), &path, index, &header)
    })
}

/// Commit what is staged. Returns the new commit's id.
#[tauri::command]
pub fn commit(
    state: State<'_, AppState>,
    subject: String,
    body: String,
    amend: bool,
) -> Result<String> {
    state.with_session(|session| {
        work::commit(&session.repo.to_thread_local(), &subject, &body, amend)
    })
}

/// The message of the commit HEAD points at, for pre-filling an amend.
#[tauri::command]
pub fn head_message(state: State<'_, AppState>) -> Result<String> {
    state.with_session(|session| work::head_message(&session.repo.to_thread_local()))
}

/// Every branch, with how far it has drifted. One call per refresh.
///
/// Opens the repository again rather than reusing the session handle. A branch
/// upstream lives in `.git/config`, and `gix` reads config once when a
/// repository is opened — so a `git branch --set-upstream-to` run while Spagitty
/// is open would be invisible until it was restarted, and the screen would
/// report "no upstream" for a branch that has one. Re-discovery costs one
/// directory walk; being quietly wrong about drift costs more.
#[tauri::command]
pub fn branches(state: State<'_, AppState>) -> Result<Vec<BranchRow>> {
    state.with_session(|session| branches::list(&repo::open(&session.path)?))
}

#[tauri::command]
pub fn checkout(state: State<'_, AppState>, name: String) -> Result<()> {
    state.with_session(|session| branches::checkout(&session.repo.to_thread_local(), &name))
}

/// Create a branch. `start` empty means `HEAD`.
#[tauri::command]
pub fn create_branch(
    state: State<'_, AppState>,
    name: String,
    start: String,
    checkout: bool,
) -> Result<()> {
    state.with_session(|session| {
        branches::create(&session.repo.to_thread_local(), &name, &start, checkout)
    })
}

/// Every stash entry, newest first. Ask `commit_diff` about an entry's `id` to
/// see what is in it — a stash is a commit.
#[tauri::command]
pub fn stashes(state: State<'_, AppState>) -> Result<Vec<StashEntry>> {
    state.with_session(|session| stash::list(&session.repo.to_thread_local()))
}

#[tauri::command]
pub fn stash_push(
    state: State<'_, AppState>,
    message: String,
    include_untracked: bool,
) -> Result<()> {
    state.with_session(|session| {
        stash::push(&session.repo.to_thread_local(), &message, include_untracked)
    })
}

/// The todo list `git rebase -i <upstream>` would open, before any edit.
///
/// Generated rather than read: running `git rebase -i` to see the file it opens
/// would start a rebase, which is the thing the Rebase screen exists to avoid.
#[tauri::command]
pub fn rebase_todo(state: State<'_, AppState>, upstream: String) -> Result<Todo> {
    let todo =
        state.with_session(|session| rebase::todo(&session.repo.to_thread_local(), &upstream))?;

    *state.rebase_todo.lock().expect("rebase lock") = Some(todo.clone());
    Ok(todo)
}

/// What a plan would produce.
///
/// Pure: a fold over the edits against the todo already read. No repository is
/// touched, and there is no path from here to `shell::rebase_interactive` —
/// executing a plan is FEAT-015.
#[tauri::command]
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
#[tauri::command]
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

    let (workdir, git_dir) = state.with_session(|session| {
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
#[tauri::command]
pub fn rebase_progress(state: State<'_, AppState>) -> Result<Option<rebase::Progress>> {
    state.with_session(|session| Ok(rebase::progress(&session.repo.to_thread_local())))
}

/// Carry on with a rebase that stopped, once its conflicts are resolved.
#[tauri::command]
pub fn rebase_continue(state: State<'_, AppState>) -> Result<()> {
    state.with_session(|session| ops::rebase_continue(&session.repo.to_thread_local()))
}

/// Drop the commit a rebase stopped on and carry on with the rest.
#[tauri::command]
pub fn rebase_skip(state: State<'_, AppState>) -> Result<()> {
    state.with_session(|session| ops::rebase_skip(&session.repo.to_thread_local()))
}

/// Unwind a rebase and put the branch back where it started.
///
/// The worker is dropped first, which waits for its thread. Aborting while it
/// is still emitting progress would leave the screen showing a step count for a
/// rebase that no longer exists.
#[tauri::command]
pub fn rebase_abort(state: State<'_, AppState>) -> Result<()> {
    state.rebase.lock().expect("rebase worker lock").take();
    state.with_session(|session| ops::rebase_abort(&session.repo.to_thread_local()))
}

/// Move the current branch to `commit`.
///
/// `Hard` discards uncommitted work. The mode arrives as an enum rather than a
/// string so that the webview cannot send a harder reset than the one whose
/// consequences the user read in the menu.
#[tauri::command]
pub fn reset(state: State<'_, AppState>, commit: String, mode: ResetMode) -> Result<()> {
    state.with_session(|session| ops::reset(&session.repo.to_thread_local(), &commit, mode))
}

/// Commit the inverse of `commit`. Reverting a merge takes the first parent as
/// the mainline, which is the parent the graph draws as the trunk.
#[tauri::command]
pub fn revert(state: State<'_, AppState>, commit: String) -> Result<()> {
    state.with_session(|session| ops::revert(&session.repo.to_thread_local(), &commit))
}

/// Replay `commits` onto the current branch, in the order given.
#[tauri::command]
pub fn cherry_pick(state: State<'_, AppState>, commits: Vec<String>) -> Result<()> {
    state.with_session(|session| ops::cherry_pick(&session.repo.to_thread_local(), &commits))
}

/// Merge, fast-forward or rebase `source` into the branch that is checked out.
///
/// This is what dropping one branch label onto another resolves to, once the
/// user has picked from the menu that appears.
#[tauri::command]
pub fn integrate(state: State<'_, AppState>, source: String, how: Integration) -> Result<()> {
    state.with_session(|session| ops::integrate(&session.repo.to_thread_local(), &source, how))
}

/// Replay commits onto `onto`.
///
/// `upstream` empty moves the whole branch — "rebase this onto that". A commit
/// in `upstream` moves only what comes after it, which is the graph's "rebase
/// these N commits onto". `branch` empty means HEAD.
#[tauri::command]
pub fn rebase_onto(
    state: State<'_, AppState>,
    onto: String,
    upstream: String,
    branch: String,
) -> Result<()> {
    state.with_session(|session| {
        ops::rebase_onto(&session.repo.to_thread_local(), &onto, &upstream, &branch)
    })
}

/// Check out a commit with no branch attached.
#[tauri::command]
pub fn checkout_detached(state: State<'_, AppState>, revision: String) -> Result<()> {
    state.with_session(|session| ops::checkout_detached(&session.repo.to_thread_local(), &revision))
}

/// Rename a local branch.
#[tauri::command]
pub fn rename_branch(state: State<'_, AppState>, from: String, to: String) -> Result<()> {
    state.with_session(|session| ops::rename_branch(&session.repo.to_thread_local(), &from, &to))
}

/// Delete a local branch. `force` is `-D`, and loses unmerged commits.
#[tauri::command]
pub fn delete_branch(state: State<'_, AppState>, name: String, force: bool) -> Result<()> {
    state.with_session(|session| ops::delete_branch(&session.repo.to_thread_local(), &name, force))
}

/// Create a tag at `target`. A message makes it annotated.
#[tauri::command]
pub fn create_tag(
    state: State<'_, AppState>,
    name: String,
    target: String,
    message: String,
) -> Result<()> {
    state.with_session(|session| {
        ops::create_tag(&session.repo.to_thread_local(), &name, &target, &message)
    })
}

/// Delete a local tag.
#[tauri::command]
pub fn delete_tag(state: State<'_, AppState>, name: String) -> Result<()> {
    state.with_session(|session| ops::delete_tag(&session.repo.to_thread_local(), &name))
}

/// Apply, pop or drop a stash entry.
#[tauri::command]
pub fn stash_action(state: State<'_, AppState>, index: usize, action: StashAction) -> Result<()> {
    state.with_session(|session| ops::stash(&session.repo.to_thread_local(), index, action))
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
#[tauri::command]
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
#[tauri::command]
pub fn pull(state: State<'_, AppState>, remote: String, mode: PullMode) -> Result<String> {
    state.with_session(|session| ops::pull(&session.repo.to_thread_local(), &remote, mode))
}

/// Push. `force` is `--force-with-lease`, never a plain force.
///
/// On a worker, for the reasons in [`fetch`].
#[tauri::command]
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

    let workdir = state.with_session(|session| {
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
#[tauri::command]
pub fn network_release(state: State<'_, AppState>) {
    state.network.lock().expect("network lock").take();
}

/// Start a query. Returns the token its rows will carry.
///
/// Rows arrive as `search-rows` events and the walk ends with `search-done`.
/// Starting a query cancels whichever one was running, so a store that sees
/// rows from an older token can drop them without asking.
#[tauri::command]
pub fn search_start<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    query: Query,
) -> Result<u64> {
    let path = state.with_session(|session| Ok(session.path.clone()))?;
    let token = state.next_token.fetch_add(1, Ordering::Relaxed);

    let worker = search_worker::spawn(app, path, query, token);
    // Assigning drops the previous worker, which cancels it and joins its
    // thread before this call returns.
    *state.search.lock().expect("search lock") = Some(worker);

    Ok(token)
}

/// Stop the running query. Leaving the screen is the ordinary caller.
#[tauri::command]
pub fn search_stop(state: State<'_, AppState>) {
    *state.search.lock().expect("search lock") = None;
}

/// Who last touched each line of `path` at `revision`. An empty revision is
/// `HEAD`.
#[tauri::command]
pub fn blame(state: State<'_, AppState>, path: String, revision: String) -> Result<Blame> {
    state.with_session(|session| blame::file(&session.repo.to_thread_local(), &path, &revision))
}

/// Read commit history for a single file path (FEAT-063).
#[tauri::command]
pub fn file_history(
    state: State<'_, AppState>,
    path: String,
    limit: usize,
) -> Result<Vec<blame::FileHistoryEntry>> {
    state.with_session(|session| blame::history(&session.repo.to_thread_local(), &path, limit))
}

/// What operation is in progress, and every conflicted path.
///
/// Reads only. Taking a side, editing the merged result and marking a file
/// resolved are FEAT-016 and have no command here at all — a disabled button
/// backed by nothing is easier to explain than one backed by a half-written
/// write path.
#[tauri::command]
pub fn conflicts(state: State<'_, AppState>) -> Result<ConflictState> {
    state.with_session(|session| conflicts::state(&session.repo.to_thread_local()))
}

/// The three index stages of one conflicted path, plus the file on disk.
#[tauri::command]
pub fn conflict_sides(state: State<'_, AppState>, path: String) -> Result<ConflictSides> {
    state.with_session(|session| conflicts::sides(&session.repo.to_thread_local(), &path))
}

/// Every conflict region in a file's merged text, so the screen can offer one
/// button per region rather than only per file.
#[tauri::command]
pub fn conflict_regions(text: String) -> Vec<conflicts::Region> {
    conflicts::regions(&text)
}

/// Take one whole side of a conflicted file into the working tree.
///
/// The file stays conflicted afterwards. Marking it resolved is a separate
/// command on purpose: looking at the result is the point of the screen.
#[tauri::command]
pub fn conflict_take(
    state: State<'_, AppState>,
    path: String,
    side: conflicts::Side,
) -> Result<()> {
    state.with_session(|session| conflicts::take(&session.repo.to_thread_local(), &path, side))
}

/// Resolve one marker region, or every region, and write the file back.
///
/// The text is re-read here rather than taken from the webview: a screen that
/// sent the whole file back could send one it had edited since it was read, and
/// the region indexes would then point somewhere else.
#[tauri::command]
pub fn conflict_resolve_region(
    state: State<'_, AppState>,
    path: String,
    index: Option<usize>,
    side: conflicts::Side,
) -> Result<()> {
    state.with_session(|session| {
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
#[tauri::command]
pub fn conflict_write(state: State<'_, AppState>, path: String, text: String) -> Result<()> {
    state.with_session(|session| {
        conflicts::write_merged(&session.repo.to_thread_local(), &path, &text)
    })
}

/// Mark paths resolved: `git add`.
#[tauri::command]
pub fn conflict_resolve(state: State<'_, AppState>, paths: Vec<String>) -> Result<()> {
    state.with_session(|session| conflicts::mark_resolved(&session.repo.to_thread_local(), &paths))
}

/// Carry on with whatever the repository is in the middle of.
#[tauri::command]
pub fn conflict_continue(state: State<'_, AppState>) -> Result<()> {
    state.with_session(|session| {
        let repo = session.repo.to_thread_local();
        conflicts::continue_operation(&repo, conflicts::operation(&repo))
    })
}

/// Abandon it and put the repository back.
#[tauri::command]
pub fn conflict_abort(state: State<'_, AppState>) -> Result<()> {
    state.with_session(|session| {
        let repo = session.repo.to_thread_local();
        conflicts::abort_operation(&repo, conflicts::operation(&repo))
    })
}

/// Every tag, newest first (FEAT-051).
#[tauri::command]
pub fn tags(state: State<'_, AppState>) -> Result<Vec<tags::Tag>> {
    state.with_session(|session| tags::tags(&session.repo.to_thread_local()))
}

/// Create a tag. A non-empty message makes it annotated.
#[tauri::command]
pub fn tag_create(
    state: State<'_, AppState>,
    name: String,
    target: String,
    message: String,
) -> Result<()> {
    state.with_session(|session| {
        tags::create(&session.repo.to_thread_local(), &name, &target, &message)
    })
}

/// Delete a local tag.
#[tauri::command]
pub fn tag_delete(state: State<'_, AppState>, name: String) -> Result<()> {
    state.with_session(|session| tags::delete(&session.repo.to_thread_local(), &name))
}

/// Rewrite an annotated tag's message, keeping it on the same commit.
#[tauri::command]
pub fn tag_retag(
    state: State<'_, AppState>,
    name: String,
    target: String,
    message: String,
) -> Result<()> {
    state.with_session(|session| {
        tags::retag(&session.repo.to_thread_local(), &name, &target, &message)
    })
}

/// Where a ref has been (FEAT-050).
#[tauri::command]
pub fn reflog(state: State<'_, AppState>, query: reflog::ReflogQuery) -> Result<reflog::Reflog> {
    state.with_session(|session| reflog::reflog(&session.repo.to_thread_local(), &query))
}

/// Every ref whose reflog is worth offering, `HEAD` first.
#[tauri::command]
pub fn reflog_refs(state: State<'_, AppState>) -> Result<Vec<String>> {
    state.with_session(|session| Ok(reflog::logged_refs(&session.repo.to_thread_local())))
}

/// Every configured remote, in name order.
#[tauri::command]
pub fn remotes(state: State<'_, AppState>) -> Result<Vec<remotes::Remote>> {
    state.with_session(|session| Ok(remotes::remotes(&session.repo.to_thread_local())))
}

/// Add a remote. Configuration only — nothing is fetched.
#[tauri::command]
pub fn remote_add(state: State<'_, AppState>, name: String, url: String) -> Result<()> {
    state.with_session(|session| remotes::add(&session.repo.to_thread_local(), &name, &url))
}

/// Rename a remote, its tracking refs, and every upstream pointing at it.
#[tauri::command]
pub fn remote_rename(state: State<'_, AppState>, from: String, to: String) -> Result<()> {
    state.with_session(|session| remotes::rename(&session.repo.to_thread_local(), &from, &to))
}

/// List all submodules for the open repository (FEAT-067).
#[tauri::command]
pub fn submodules(state: State<'_, AppState>) -> Result<Vec<Submodule>> {
    state.with_session(|session| submodules::list(&session.repo.to_thread_local()))
}

/// Update submodules recursively (FEAT-067).
#[tauri::command]
pub fn submodule_update(
    state: State<'_, AppState>,
    paths: Vec<String>,
    init: bool,
    recursive: bool,
) -> Result<String> {
    state.with_session(|session| {
        submodules::update(&session.repo.to_thread_local(), &paths, init, recursive)
    })
}

/// Sync submodule URLs from .gitmodules (FEAT-067).
#[tauri::command]
pub fn submodule_sync(state: State<'_, AppState>, recursive: bool) -> Result<String> {
    state.with_session(|session| submodules::sync(&session.repo.to_thread_local(), recursive))
}

/// De-initialize a submodule (FEAT-067).
#[tauri::command]
pub fn submodule_deinit(state: State<'_, AppState>, path: String, force: bool) -> Result<String> {
    state.with_session(|session| submodules::deinit(&session.repo.to_thread_local(), &path, force))
}

/// Read configured and available external diff/merge tools (FEAT-068).
///
/// Falls back to global configuration from ~/.gitconfig if no repository is open.
#[tauri::command]
pub fn external_tools_config(state: State<'_, AppState>) -> Result<ExternalToolsConfig> {
    let guard = state.session.lock().expect("session lock");
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
    let guard = state.session.lock().expect("session lock");
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
#[tauri::command]
pub fn launch_external_diff(
    state: State<'_, AppState>,
    path: String,
    tool: Option<String>,
    commit: Option<String>,
) -> Result<()> {
    state.with_session(|session| {
        tools::launch_diff(
            &session.repo.to_thread_local(),
            &path,
            tool.as_deref(),
            commit.as_deref(),
        )
    })
}

/// Launch external merge tool (FEAT-068).
#[tauri::command]
pub fn launch_external_merge(
    state: State<'_, AppState>,
    path: String,
    tool: Option<String>,
) -> Result<()> {
    state.with_session(|session| {
        tools::launch_merge(&session.repo.to_thread_local(), &path, tool.as_deref())
    })
}

/// Remove a remote, its tracking refs, and the upstreams pointing at it.
#[tauri::command]
pub fn remote_remove(state: State<'_, AppState>, name: String) -> Result<()> {
    state.with_session(|session| remotes::remove(&session.repo.to_thread_local(), &name))
}

/// Change where a remote points.
#[tauri::command]
pub fn remote_set_url(state: State<'_, AppState>, name: String, url: String) -> Result<()> {
    state.with_session(|session| remotes::set_url(&session.repo.to_thread_local(), &name, &url))
}

/// List all worktrees for the open repository (FEAT-062).
#[tauri::command]
pub fn worktrees(state: State<'_, AppState>) -> Result<Vec<Worktree>> {
    state.with_session(|session| worktrees::list(&session.repo.to_thread_local()))
}

/// Add a new linked worktree (FEAT-062).
#[tauri::command]
pub fn worktree_add(
    state: State<'_, AppState>,
    path: String,
    branch: Option<String>,
    new_branch: Option<String>,
    detach: bool,
) -> Result<Worktree> {
    state.with_session(|session| {
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
#[tauri::command]
pub fn worktree_remove(state: State<'_, AppState>, path: String, force: bool) -> Result<()> {
    state.with_session(|session| {
        worktrees::remove(
            &session.repo.to_thread_local(),
            std::path::Path::new(&path),
            force,
        )
    })
}

/// Lock a worktree against pruning (FEAT-062).
#[tauri::command]
pub fn worktree_lock(
    state: State<'_, AppState>,
    path: String,
    reason: Option<String>,
) -> Result<()> {
    state.with_session(|session| {
        worktrees::lock(
            &session.repo.to_thread_local(),
            std::path::Path::new(&path),
            reason.as_deref(),
        )
    })
}

/// Unlock a locked worktree (FEAT-062).
#[tauri::command]
pub fn worktree_unlock(state: State<'_, AppState>, path: String) -> Result<()> {
    state.with_session(|session| {
        worktrees::unlock(&session.repo.to_thread_local(), std::path::Path::new(&path))
    })
}

/// Prune stale worktrees (FEAT-062).
#[tauri::command]
pub fn worktree_prune(state: State<'_, AppState>) -> Result<()> {
    state.with_session(|session| worktrees::prune(&session.repo.to_thread_local()))
}

/// Every remembered repository, as a card.
///
/// Each is read where it sits, without becoming the open one: no walk, no
/// worker, no watcher, and nothing written to it. A path that has gone comes
/// back as a card that says so rather than being dropped — a repository that
/// moved is something to see, not something to forget quietly.
#[tauri::command]
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
#[tauri::command]
pub fn clone_plan(url: String, parent: PathBuf) -> Plan {
    clone::plan(&url, &parent)
}

/// Start a clone. Returns the token its progress will carry.
///
/// Progress arrives as `clone-progress` events and the clone ends with
/// `clone-done`. The plan is recomputed here rather than taken from the caller:
/// a destination that filled up between the last keystroke and the button is
/// still a destination this must refuse.
#[tauri::command]
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
#[tauri::command]
pub fn clone_release(state: State<'_, AppState>) {
    *state.clone.lock().expect("clone lock") = None;
}

/// Remove a repository from Spagitty's list. The directory is not touched.
#[tauri::command]
pub fn forget_repo<R: Runtime>(app: AppHandle<R>, path: PathBuf) {
    recents::forget(&app, &path);
}

#[tauri::command]
pub fn close_repo(state: State<'_, AppState>) {
    *state.session.lock().expect("session lock") = None;
}

/// Every `git` command Spagitty has executed since `since`, oldest first.
///
/// The panel subscribes to [`crate::command_log::EXECUTED_EVENT`] for new
/// entries, so this is the catch-up call: what ran before the panel was opened,
/// or while the webview was reloading. `since` of 0 is everything still held.
#[tauri::command]
pub fn git_commands(since: u64) -> Vec<Executed> {
    record::recent(since)
}

/// Forget the recorded commands. The user's action, from the panel.
#[tauri::command]
pub fn clear_git_commands() {
    record::clear();
}

#[tauri::command]
pub fn metrics() -> Metrics {
    Metrics {
        row_pitch: ROW_PITCH,
    }
}

#[tauri::command]
pub fn about() -> About {
    crate::about::about()
}

/// Every dependency this binary is made of, with its license.
///
/// Generated from the lockfiles at build time rather than typed, so it cannot
/// quietly fall behind a `cargo update`. A build that could not generate it
/// returns the reason instead of an empty list.
#[tauri::command]
pub fn licenses() -> Licenses {
    crate::about::licenses()
}

/// `user.name` and `user.email`, per scope, and which one is in effect.
///
/// Settings does not need an open repository: with none, the global and system
/// configuration is read on its own and the local scope is reported as absent.
#[tauri::command]
pub fn identity(state: State<'_, AppState>) -> Result<Identity> {
    let guard = state.session.lock().expect("session lock");
    match guard.as_ref() {
        Some(session) => Ok(identity::read(&session.repo.to_thread_local())),
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
#[tauri::command]
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
#[tauri::command]
pub fn forge_repo(state: State<'_, AppState>) -> Result<Option<Repo>> {
    let guard = state.session.lock().expect("session lock");
    match guard.as_ref() {
        Some(session) => forge::identify_repo(&session.repo.to_thread_local()),
        None => Ok(None),
    }
}

/// The connected accounts. Hosts and logins; never tokens.
#[tauri::command]
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
    if token.trim().is_empty() {
        return Err(Error::Forge {
            host,
            detail: "a token is needed".into(),
        });
    }

    // Proving the token is a network round trip, so it goes off the main
    // thread like every other one.
    let asked = {
        let host = host.clone();
        let token = token.trim().to_string();
        off_thread(move || forge::whoami(kind, &host, &token)).await?
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
    let Some(repo) = forge_repo(state)? else {
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
    off_thread(move || forge::pull_requests(&repo, &token, &user)).await
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
#[tauri::command]
pub fn signing(state: State<'_, AppState>) -> Result<Signing> {
    let guard = state.session.lock().expect("session lock");
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

#[tauri::command]
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
#[tauri::command]
pub async fn avatar<R: Runtime>(app: AppHandle<R>, email: String) -> AvatarAnswer {
    let source = avatars::source(&email);
    let handle = source.handle;

    if !crate::settings::load(&app).fetch_avatars {
        // The handle still stands: it was read out of the address, and reading
        // it sent nothing anywhere.
        return AvatarAnswer {
            handle,
            picture: None,
        };
    }

    let Some(cache) = avatar_cache(&app) else {
        return AvatarAnswer {
            handle,
            picture: None,
        };
    };

    // Off the main thread for the reason on `check_update`: this is a network
    // request with a timeout on it, and a synchronous command holds the window
    // for as long as it takes.
    let picture = tauri::async_runtime::spawn_blocking(move || avatars::picture(&cache, &email))
        .await
        .unwrap_or(None);

    AvatarAnswer { handle, picture }
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
#[tauri::command]
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
            graph_request(state, 0, 10),
            Err(Error::NoRepository)
        ));
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
            let two = open_repo(app.clone(), app.state::<AppState>(), paths.0)
                .expect("opening the second");
            *out.lock().expect("result") = Some(two.token);
        });

        let two = state_is_replaced.lock().expect("result").expect("a token");
        assert_ne!(two, one.token, "a replaced walk gets a new token");

        // The old repository is gone: a command reads the new one.
        let info = snapshot(app.state::<AppState>()).expect("a snapshot");
        assert_eq!(info.info.path, second.path().canonicalize().unwrap());
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
            still_there.info.path,
            fixture.path().canonicalize().unwrap()
        );
    }
}
