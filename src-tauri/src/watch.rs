// SPDX-License-Identifier: GPL-3.0-or-later

//! Filesystem watching for the open repository.
//!
//! Spagitty does not poll. `notify` watches the `.git` directory and the UI is
//! told when something it displays has actually changed — a branch moved, a
//! commit landed, the index was touched. Polling a repository means either
//! being slow to notice or burning CPU on a directory that is idle almost all
//! of the time.
//!
//! The working tree is watched too (BUG-055). It was once left out to save a
//! recursive watch of a large checkout, but an edit in an editor touches
//! nothing in `.git`, so the working-copy count and the graph's
//! uncommitted-changes row stayed stale until something else wrote the index.
//! What keeps it cheap is the filter: a burst of working-tree events counts only
//! when one of its paths is not git-ignored, so a build or an install writing
//! thousands of ignored files asks for nothing.

use std::collections::HashSet;
use std::ffi::OsString;
use std::path::{Component, Path, PathBuf, Prefix};
use std::sync::mpsc::{channel, Receiver, RecvTimeoutError, Sender};
use std::thread::JoinHandle;
use std::time::Duration;

use notify::event::ModifyKind;
use notify::{EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use serde::Serialize;
use tauri::{AppHandle, Emitter, Runtime};

pub const CHANGED_EVENT: &str = "repo-changed";

/// A single git operation touches several files in quick succession — a commit
/// rewrites the index, HEAD, a ref, and the reflog. Coalesce them so the UI
/// refreshes once, after things have settled, rather than four times mid-write.
const QUIET_PERIOD: Duration = Duration::from_millis(150);

#[derive(Serialize, Clone, Default)]
#[serde(rename_all = "camelCase")]
struct ChangedEvent {
    /// A ref moved, or HEAD did: the graph's refs and the branch chip are stale.
    refs: bool,
    /// The index changed: working-copy counts are stale.
    worktree: bool,
}

impl ChangedEvent {
    fn is_empty(&self) -> bool {
        !self.refs && !self.worktree
    }
}

/// Holds the watcher alive. Dropping it stops watching and joins the thread.
pub struct RepoWatcher {
    _watcher: RecommendedWatcher,
    stop: Sender<()>,
    handle: Option<JoinHandle<()>>,
}

impl Drop for RepoWatcher {
    fn drop(&mut self) {
        let _ = self.stop.send(());
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// Start watching `git_dir`. Returns `None` if the platform watcher could not
/// be created — the app still works, it just won't notice outside changes,
/// which is better than refusing to open the repository.
pub fn watch<R: Runtime>(
    app: AppHandle<R>,
    git_dir: &Path,
    workdir: Option<&Path>,
) -> Option<RepoWatcher> {
    let (event_tx, event_rx) = channel::<notify::Result<notify::Event>>();
    let (stop_tx, stop_rx) = channel::<()>();

    let mut watcher = notify::recommended_watcher(move |res| {
        // A closed receiver means the repository was closed; nothing to do.
        let _ = event_tx.send(res);
    })
    .ok()?;

    watcher.watch(git_dir, RecursiveMode::Recursive).ok()?;
    // The working tree, when there is one. A watch that cannot be made (a
    // Linux machine out of inotify watches) leaves `.git` watched, as before.
    if let Some(workdir) = workdir {
        let _ = watcher.watch(workdir, RecursiveMode::Recursive);
    }

    let git_dir = canonical(git_dir);
    let workdir = workdir.map(canonical);
    let handle = std::thread::Builder::new()
        .name("spagitty-watch".into())
        .spawn(move || debounce(app, event_rx, stop_rx, git_dir, workdir))
        .ok()?;

    Some(RepoWatcher {
        _watcher: watcher,
        stop: stop_tx,
        handle: Some(handle),
    })
}

/// The real path, as the platform watcher names paths.
///
/// On Windows `canonicalize` answers with a verbatim path, `\\?\C:\…\.git`,
/// while the watcher reports `C:\…\.git\refs\heads\main`. Kept verbatim, no
/// event ever fell under the git directory: every `.git` write looked like a
/// working-tree change, no ref move was ever seen, and the refresh each one
/// caused touched `.git` again, for ever (BUG-059). So the prefix is taken
/// off here, and off every event path in [`classify`], without a dependency.
fn canonical(path: &Path) -> PathBuf {
    plain(&path.canonicalize().unwrap_or_else(|_| path.to_path_buf()))
}

/// `path` without Windows' verbatim prefix: `\\?\C:\x` is `C:\x`, and
/// `\\?\UNC\server\share` is `\\server\share`. Any other path is itself.
fn plain(path: &Path) -> PathBuf {
    let Some(text) = path.to_str() else {
        // Not Unicode, which Windows allows: take the prefix off by its
        // components rather than leave it on and miss every event.
        return plain_parts(path);
    };
    if let Some(rest) = text.strip_prefix(r"\\?\UNC\") {
        return PathBuf::from(format!(r"\\{rest}"));
    }
    if let Some(rest) = text.strip_prefix(r"\\?\") {
        return PathBuf::from(rest);
    }
    path.to_path_buf()
}

/// [`plain`] for a path that is not Unicode. Only Windows parses a prefix
/// component; anywhere else the path comes back as it went in.
fn plain_parts(path: &Path) -> PathBuf {
    let mut parts = path.components();
    let Some(Component::Prefix(prefix)) = parts.next() else {
        return path.to_path_buf();
    };
    let mut out = match prefix.kind() {
        Prefix::VerbatimDisk(drive) => PathBuf::from(format!("{}:", char::from(drive))),
        Prefix::VerbatimUNC(server, share) => {
            let mut unc = OsString::from(r"\\");
            unc.push(server);
            unc.push(r"\");
            unc.push(share);
            PathBuf::from(unc)
        }
        _ => return path.to_path_buf(),
    };
    out.extend(parts);
    out
}

/// What follows `base` in `full`, compared without regard to ASCII case, when
/// `full` is `base` or lies under it. Whole components only: `.gitignore` is
/// not inside `.git`.
fn fold_inside<'a>(full: &'a str, base: &str) -> Option<&'a str> {
    if full.len() < base.len()
        || !full.is_char_boundary(base.len())
        || !full[..base.len()].eq_ignore_ascii_case(base)
    {
        return None;
    }
    let rest = &full[base.len()..];
    if !rest.is_empty() && !rest.starts_with(['\\', '/']) {
        return None;
    }
    Some(rest.trim_start_matches(['\\', '/']))
}

/// Where `path` is inside `git_dir`, if it is: both without a verbatim
/// prefix, and on Windows, where a drive letter can come in either case,
/// compared without regard to case.
fn inside(path: &Path, git_dir: &Path) -> Option<PathBuf> {
    let path = plain(path);
    let git_dir = plain(git_dir);
    if let Ok(rest) = path.strip_prefix(&git_dir) {
        return Some(rest.to_path_buf());
    }
    if cfg!(windows) {
        return fold_inside(path.to_str()?, git_dir.to_str()?).map(PathBuf::from);
    }
    None
}

/// Working-tree paths a burst may name before the rest are not looked at: one
/// that git would notice among the first few hundred is enough to refresh.
const CANDIDATES: usize = 256;

fn debounce<R: Runtime>(
    app: AppHandle<R>,
    events: Receiver<notify::Result<notify::Event>>,
    stop: Receiver<()>,
    git_dir: PathBuf,
    workdir: Option<PathBuf>,
) {
    // Opened here, on this thread, the first time a working-tree path needs
    // asking about, and kept: the ignore rules are read from it.
    let mut rules: Option<spagitty_core::ignore::Rules> = None;
    let mut candidates: HashSet<PathBuf> = HashSet::new();
    loop {
        if stop.try_recv().is_ok() {
            return;
        }

        // Block until something happens, then keep collecting until the
        // repository goes quiet again.
        let mut pending = match events.recv_timeout(Duration::from_millis(250)) {
            Ok(Ok(event)) => classify(&event, &git_dir, &mut candidates),
            Ok(Err(_)) => continue,
            Err(RecvTimeoutError::Timeout) => continue,
            Err(RecvTimeoutError::Disconnected) => return,
        };

        loop {
            match events.recv_timeout(QUIET_PERIOD) {
                Ok(Ok(event)) => {
                    let next = classify(&event, &git_dir, &mut candidates);
                    pending.refs |= next.refs;
                    pending.worktree |= next.worktree;
                }
                Ok(Err(_)) => continue,
                Err(RecvTimeoutError::Timeout) => break,
                Err(RecvTimeoutError::Disconnected) => return,
            }
        }

        if !pending.worktree && !candidates.is_empty() {
            if let Some(workdir) = &workdir {
                if rules.is_none() {
                    rules = spagitty_core::ignore::Rules::open(workdir);
                }
                let paths: Vec<PathBuf> = candidates.iter().take(CANDIDATES).cloned().collect();
                pending.worktree = match &rules {
                    Some(rules) => rules.any_not_ignored(&paths),
                    None => true,
                };
            }
        }
        candidates.clear();

        if !pending.is_empty() {
            let _ = app.emit(CHANGED_EVENT, pending);
        }
    }
}

/// Does this event mean something actually *changed*?
///
/// This filter is load-bearing, not an optimization. inotify reports reads as
/// `Access` events, and reading refs is exactly what the graph walk and every
/// snapshot do. Without this, reading refs would look like refs moving, which
/// would trigger a refresh, which would read refs again — a feedback loop that
/// reloads the graph forever.
///
/// Metadata-only modifications are excluded for the same reason: an atime bump
/// is not a ref moving. A ref that genuinely moves is written (`Modify(Data)`)
/// or replaced via a lock file and a rename (`Create` / `Modify(Name)`).
fn is_change(kind: &EventKind) -> bool {
    match kind {
        EventKind::Create(_) | EventKind::Remove(_) => true,
        EventKind::Modify(ModifyKind::Metadata(_)) => false,
        EventKind::Modify(_) => true,
        // Access, Any and Other carry no evidence that anything changed.
        _ => false,
    }
}

/// Work out what an event means for the UI.
///
/// Lock files are ignored: git writes `ref.lock` before `ref`, so reacting to
/// the lock would mean reading the repository exactly while it is mid-write.
///
/// A path outside `git_dir` is in the working tree: it goes into
/// `candidates`, for the debounce to ask whether git would notice it.
///
/// Refs are recognised by path components, not by a `/refs/` substring: on
/// Windows the separator is `\`, and the substring test never matched there,
/// so a branch moved outside Spagitty went unnoticed (BUG-055).
fn classify(
    event: &notify::Event,
    git_dir: &Path,
    candidates: &mut HashSet<PathBuf>,
) -> ChangedEvent {
    let mut out = ChangedEvent::default();

    if !is_change(&event.kind) {
        return out;
    }

    for path in &event.paths {
        let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if name.ends_with(".lock") {
            continue;
        }

        let Some(inside) = inside(path, git_dir) else {
            if candidates.len() < CANDIDATES {
                candidates.insert(plain(path));
            }
            continue;
        };
        if inside.starts_with("refs") || name == "HEAD" || name == "packed-refs" {
            out.refs = true;
        } else if name == "index" {
            out.worktree = true;
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use notify::event::{CreateKind, DataChange, MetadataKind, RemoveKind};
    use std::path::PathBuf;

    fn event(kind: EventKind, paths: &[&str]) -> notify::Event {
        notify::Event {
            kind,
            paths: paths.iter().map(PathBuf::from).collect(),
            attrs: Default::default(),
        }
    }

    const WROTE: EventKind = EventKind::Modify(ModifyKind::Data(DataChange::Any));

    #[test]
    fn a_read_is_not_a_change() {
        // This is the whole reason the filter exists. inotify reports reads as
        // Access events, and reading refs is what the graph walk does; treating
        // one as a change would reload the graph forever.
        let read = event(
            EventKind::Access(notify::event::AccessKind::Read),
            &[".git/refs/heads/main"],
        );

        assert!(!is_change(&read.kind));
        assert!(classify(&read).is_empty());
    }

    #[test]
    fn an_atime_bump_is_not_a_ref_moving() {
        let touched = event(
            EventKind::Modify(ModifyKind::Metadata(MetadataKind::AccessTime)),
            &[".git/refs/heads/main"],
        );

        assert!(!is_change(&touched.kind));
        assert!(classify(&touched).is_empty());
    }

    #[test]
    fn a_written_ref_is_a_ref_change() {
        let written = event(WROTE, &[".git/refs/heads/main"]);

        let out = classify(&written);
        assert!(out.refs);
        assert!(!out.worktree);
    }

    #[test]
    fn a_ref_replaced_through_a_rename_is_a_ref_change() {
        // git writes `ref.lock` and renames it over `ref`, so the change arrives
        // as a create or a rename rather than a data write.
        let created = event(
            EventKind::Create(CreateKind::File),
            &[".git/refs/heads/main"],
        );
        let renamed = event(
            EventKind::Modify(ModifyKind::Name(notify::event::RenameMode::To)),
            &[".git/refs/heads/main"],
        );

        assert!(classify(&created).refs);
        assert!(classify(&renamed).refs);
    }

    #[test]
    fn a_deleted_branch_is_a_ref_change() {
        let removed = event(
            EventKind::Remove(RemoveKind::File),
            &[".git/refs/heads/gone"],
        );
        assert!(classify(&removed).refs);
    }

    #[test]
    fn head_and_packed_refs_count_as_refs() {
        assert!(classify(&event(WROTE, &[".git/HEAD"])).refs);
        assert!(classify(&event(WROTE, &[".git/packed-refs"])).refs);
    }

    #[test]
    fn the_index_is_a_worktree_change_and_not_a_ref_change() {
        let out = classify(&event(WROTE, &[".git/index"]));

        assert!(out.worktree);
        assert!(!out.refs);
    }

    #[test]
    fn a_lock_file_is_ignored_so_we_never_read_mid_write() {
        let lock = event(
            EventKind::Create(CreateKind::File),
            &[".git/refs/heads/main.lock"],
        );
        let index_lock = event(WROTE, &[".git/index.lock"]);

        assert!(classify(&lock).is_empty());
        assert!(classify(&index_lock).is_empty());
    }

    #[test]
    fn an_unrelated_file_inside_the_git_directory_changes_nothing() {
        assert!(classify(&event(WROTE, &[".git/COMMIT_EDITMSG"])).is_empty());
        assert!(classify(&event(WROTE, &[".git/objects/ab/cdef"])).is_empty());
    }

    #[test]
    fn one_event_touching_both_reports_both() {
        // A commit rewrites the index and moves a ref; the debounce coalesces
        // them, so a single event carrying both paths has to as well.
        let both = event(WROTE, &[".git/index", ".git/refs/heads/main"]);

        let out = classify(&both);
        assert!(out.refs);
        assert!(out.worktree);
        assert!(!out.is_empty());
    }

    #[test]
    fn a_verbatim_prefix_is_taken_off() {
        assert_eq!(
            plain(Path::new(r"\\?\C:\work\app\.git")),
            PathBuf::from(r"C:\work\app\.git")
        );
        assert_eq!(
            plain(Path::new(r"\\?\UNC\server\share\.git")),
            PathBuf::from(r"\\server\share\.git")
        );
        assert_eq!(
            plain(Path::new("/work/app/.git")),
            PathBuf::from("/work/app/.git")
        );
    }

    /// BUG-059. The git directory as `canonicalize` answers on Windows, the
    /// event as the watcher reports it — and the other way round. Both are
    /// `.git` paths: a ref move is a ref move, and nothing in `.git` is a
    /// working-tree candidate.
    #[test]
    fn a_verbatim_git_dir_still_owns_the_events_under_it() {
        let verbatim = Path::new(r"\\?\/work/app/.git");
        let plain_dir = Path::new("/work/app/.git");
        for (git_dir, path) in [
            (verbatim, r"/work/app/.git/refs/heads/main"),
            (plain_dir, r"\\?\/work/app/.git/refs/heads/main"),
        ] {
            let mut candidates = HashSet::new();
            let out = super::classify(&event(WROTE, &[path]), git_dir, &mut candidates);
            assert!(out.refs, "{git_dir:?} and {path}");
            assert!(candidates.is_empty(), "{git_dir:?} and {path}");
        }
        let mut candidates = HashSet::new();
        let quiet = super::classify(
            &event(WROTE, &["/work/app/.git/objects/ab/cd"]),
            verbatim,
            &mut candidates,
        );
        assert!(quiet.is_empty());
        assert!(
            candidates.is_empty(),
            "a write in .git is not a working-tree change"
        );
    }

    /// The case-blind comparison Windows falls back on takes whole components:
    /// the git directory itself, in another case, is inside it; a file whose
    /// name merely starts with `.git` is not.
    #[test]
    fn a_case_blind_match_takes_whole_components() {
        let base = r"C:\work\app\.git";
        assert_eq!(
            fold_inside(r"c:\work\app\.git\refs\heads\main", base),
            Some(r"refs\heads\main")
        );
        assert_eq!(fold_inside(r"c:\Work\App\.GIT", base), Some(""));
        assert_eq!(fold_inside(r"C:\work\app\.gitignore", base), None);
        assert_eq!(fold_inside(r"C:\work\app\.git-blame-ignore", base), None);
        assert_eq!(fold_inside(r"C:\work", base), None);
    }

    /// Anywhere but Windows a path has no prefix component to take off, so a
    /// path that is not Unicode comes back as it is.
    #[cfg(unix)]
    #[test]
    fn a_path_that_is_not_unicode_comes_back_as_it_is() {
        use std::os::unix::ffi::OsStrExt;
        let odd = Path::new(std::ffi::OsStr::from_bytes(b"/work/\xff/.git"));
        assert_eq!(plain(odd), odd.to_path_buf());
    }

    /// On Windows a path that is not Unicode still loses its verbatim prefix.
    #[cfg(windows)]
    #[test]
    fn a_verbatim_path_that_is_not_unicode_loses_its_prefix() {
        use std::os::windows::ffi::OsStringExt;
        let mut wide: Vec<u16> = r"\\?\C:\work\".encode_utf16().collect();
        wide.push(0xD800);
        wide.extend(r"\.git".encode_utf16());
        let odd = PathBuf::from(OsString::from_wide(&wide));
        assert!(odd.to_str().is_none());
        let mut expected: Vec<u16> = r"C:\work\".encode_utf16().collect();
        expected.push(0xD800);
        expected.extend(r"\.git".encode_utf16());
        assert_eq!(plain(&odd), PathBuf::from(OsString::from_wide(&expected)));
    }

    #[test]
    fn a_working_tree_path_is_still_a_candidate() {
        let mut candidates = HashSet::new();
        let out = super::classify(
            &event(WROTE, &["/work/app/src/main.rs"]),
            Path::new(r"\\?\/work/app/.git"),
            &mut candidates,
        );
        assert!(out.is_empty());
        assert_eq!(
            candidates.into_iter().collect::<Vec<_>>(),
            [PathBuf::from("/work/app/src/main.rs")]
        );
    }

    #[test]
    fn an_event_with_no_paths_changes_nothing() {
        assert!(classify(&event(WROTE, &[])).is_empty());
    }

    // The debounce loop takes an `AppHandle<R>` rather than a handle bound to
    // the Wry runtime (TASK-003), so `mock_app` can supply one and the
    // coalescing below can be tested for what it emits rather than only for
    // what it classifies.

    use crate::testing::{self, Emitted};

    /// `classify` for a repository whose `.git` is at `.git`.
    fn classify(event: &notify::Event) -> ChangedEvent {
        super::classify(event, Path::new(".git"), &mut HashSet::new())
    }
    use serde_json::Value;
    use std::sync::mpsc::Sender;

    /// A debounce loop running on its own thread, with the two channels it
    /// reads and the events it emits.
    struct Running {
        events: Sender<notify::Result<notify::Event>>,
        stop: Sender<()>,
        changed: Emitted<Value>,
        thread: Option<std::thread::JoinHandle<()>>,
        _app: tauri::App<tauri::test::MockRuntime>,
    }

    impl Running {
        fn start() -> Self {
            let app = testing::app();
            let changed = Emitted::<Value>::on(app.handle(), CHANGED_EVENT);

            let (events, event_rx) = channel();
            let (stop, stop_rx) = channel();
            let handle = app.handle().clone();
            let thread = std::thread::spawn(move || {
                debounce(handle, event_rx, stop_rx, PathBuf::from(".git"), None)
            });

            Running {
                events,
                stop,
                changed,
                thread: Some(thread),
                _app: app,
            }
        }

        fn send(&self, kind: EventKind, paths: &[&str]) {
            self.events.send(Ok(event(kind, paths))).expect("sending");
        }
    }

    impl Drop for Running {
        /// Wind the loop down and wait for it, so a debounce that ignored both
        /// ways out fails the test rather than leaking a thread.
        fn drop(&mut self) {
            let _ = self.stop.send(());
            if let Some(thread) = self.thread.take() {
                thread.join().expect("the debounce thread");
            }
        }
    }

    #[test]
    fn a_burst_from_one_commit_becomes_one_refresh() {
        // A commit rewrites the index, HEAD, a ref and the reflog within a few
        // milliseconds. The UI should refresh once, after it settles, not four
        // times mid-write.
        let running = Running::start();

        for path in [
            ".git/index",
            ".git/HEAD",
            ".git/refs/heads/main",
            ".git/logs/HEAD",
        ] {
            running.send(WROTE, &[path]);
        }

        let events = running.changed.at_least(1);
        assert_eq!(events[0]["refs"], true);
        assert_eq!(events[0]["worktree"], true);
        running.changed.no_more_than(1);
    }

    #[test]
    fn a_burst_that_changes_nothing_emits_nothing() {
        // Reads, lock files and git's own scratch files. Emitting for these is
        // the feedback loop `is_change` and `classify` exist to prevent, and
        // the loop must not emit an empty payload either.
        let running = Running::start();

        running.send(
            EventKind::Access(notify::event::AccessKind::Read),
            &[".git/refs/heads/main"],
        );
        running.send(EventKind::Create(CreateKind::File), &[".git/index.lock"]);
        running.send(WROTE, &[".git/COMMIT_EDITMSG"]);

        running.changed.no_more_than(0);
    }

    #[test]
    fn a_change_after_the_repository_settles_is_its_own_refresh() {
        // The complement of coalescing: two operations far enough apart are two
        // events, not one. A debounce that swallowed the second would leave the
        // UI stale until something else happened.
        let running = Running::start();

        running.send(WROTE, &[".git/refs/heads/main"]);
        assert_eq!(running.changed.at_least(1).len(), 1);

        std::thread::sleep(QUIET_PERIOD * 3);

        running.send(WROTE, &[".git/index"]);
        let events = running.changed.at_least(2);
        assert_eq!(events[1]["worktree"], true);
        assert_eq!(events[1]["refs"], false);
    }

    /// BUG-055: an edit in the working tree refreshes the working copy; a burst
    /// of git-ignored output does not.
    #[test]
    fn an_edit_in_the_working_tree_is_noticed_and_ignored_output_is_not() {
        let fixture = spagitty_core::fixture::Fixture::linear(1);
        std::fs::write(
            fixture.path().join(".gitignore"),
            "target/
",
        )
        .expect("ignore");
        std::fs::create_dir_all(fixture.path().join("target")).expect("dir");
        std::fs::write(fixture.path().join("target/out.bin"), "x").expect("out");
        std::fs::write(fixture.path().join("notes.md"), "x").expect("notes");

        let app = testing::app();
        let changed = Emitted::<Value>::on(app.handle(), CHANGED_EVENT);
        let (events, event_rx) = channel();
        let (stop, stop_rx) = channel();
        let handle = app.handle().clone();
        let work = canonical(fixture.path());
        let git = work.join(".git");
        let thread = {
            let work = work.clone();
            std::thread::spawn(move || debounce(handle, event_rx, stop_rx, git, Some(work)))
        };
        let wrote = |path: PathBuf| Ok(notify::Event::new(WROTE).add_path(path));

        events
            .send(wrote(work.join("target/out.bin")))
            .expect("send");
        changed.no_more_than(0);

        events.send(wrote(work.join("notes.md"))).expect("send");
        let got = changed.at_least(1);
        assert_eq!(got[0]["worktree"], true);
        assert_eq!(got[0]["refs"], false);

        let _ = stop.send(());
        thread.join().expect("the debounce thread");
    }

    #[test]
    fn a_ref_written_through_windows_separators_is_a_ref_change() {
        let git = Path::new("C:").join("repo").join(".git");
        let event = notify::Event::new(WROTE).add_path(git.join("refs").join("heads").join("main"));
        assert!(super::classify(&event, &git, &mut HashSet::new()).refs);
    }
}
