// SPDX-License-Identifier: GPL-3.0-or-later

//! A repository of its own for a task, and the door its work comes in by
//! (FEAT-147).
//!
//! A linked worktree shares every ref, remote, credential, hook and the stash
//! with the person's repository: `git checkout main && git merge`, or `git
//! push`, works from inside one. An own copy shares none of that. It is a clone
//! with no remote, so there is nowhere to push and no branch of the person's to
//! move.
//!
//! ```text
//! <repo>/.spagitty/farm/copies/task-0042/        the copy, on spagitty-farm/TASK-0042/claude
//! <repo>/.spagitty/farm/copies/task-0042.start   the commit it started from
//! ```
//!
//! The start is kept beside the copy, not in it, so the agent cannot rewrite
//! what its work is checked against.
//!
//! # Intake: Spagitty takes the work
//!
//! The agent pushes nothing. When its run ends, Spagitty fetches its branch
//! into `refs/spagitty/inbox/<task>/<provider>` — not a branch, never pushed —
//! and checks it at the door: it descends from where it started, it changes
//! only the paths the task may touch, it leaves the protected paths alone, it
//! adds no file larger than the limit, and it has no merge commits. Then, and
//! only then, Spagitty moves the farm branch to it by compare-and-swap. From
//! there on the task is checked, reviewed and merged as a worktree task is:
//! nothing above this module knows which kind of workspace it had.
//!
//! # What it is not
//!
//! A guard against mistakes, not against malice. A local agent runs as the
//! person and can open any file they can, including the repository's `.git` by
//! path. Own copies remove every *git* route to the person's branches; they are
//! not a sandbox.

use std::path::{Path, PathBuf};

use spagitty_core::shell;

use crate::error::{Error, Result};
use crate::model::{AgentProvider, TaskId};
use crate::workspace::branch::{branch_name, worktree_dir};
use crate::workspace::lock::{overlaps, EVERYTHING};
use crate::workspace::worktree::{exclude_farm_dir, Workspace, FARM_DIR};

/// The largest file a task may add, unless the farm says otherwise: 5 MB.
pub const SIZE_LIMIT: u64 = 5 * 1024 * 1024;

/// Paths a task changes only when it names them itself: CI, submodules and
/// hooks run code on other people's machines.
pub const PROTECTED: &[&str] = &[
    ".github/workflows/",
    ".gitlab-ci.yml",
    ".gitmodules",
    ".githooks/",
    ".husky/",
];

/// Where a task's copy lives.
pub fn copy_path(repo: &Path, task: &TaskId) -> PathBuf {
    copies_dir(repo).join(worktree_dir(task))
}

fn copies_dir(repo: &Path) -> PathBuf {
    repo.join(FARM_DIR).join("farm").join("copies")
}

fn start_file(repo: &Path, task: &TaskId) -> PathBuf {
    copies_dir(repo).join(format!("{}.start", worktree_dir(task)))
}

fn merge_file(repo: &Path, task: &TaskId) -> PathBuf {
    copies_dir(repo).join(format!("{}.merge", worktree_dir(task)))
}

/// Let the next intake take a merge of `parent` (FEAT-146).
///
/// A conflict resolved in a copy is committed as a merge of the destination's
/// tip, which Spagitty started there itself. Intake takes that merge, and
/// measures the paths from `parent`, so what is checked is what the lane will
/// add to the destination and not the destination's own history.
pub fn allow_merge(repo: &Path, task: &TaskId, parent: &str) -> Result<()> {
    std::fs::write(merge_file(repo, task), format!("{parent}\n"))?;
    Ok(())
}

/// The ref intake writes a task's work to, in the person's repository.
pub fn inbox_ref(task: &TaskId, provider: AgentProvider) -> String {
    let branch = branch_name(task, provider);
    let rest = branch
        .split_once('/')
        .map_or(branch.as_str(), |(_, rest)| rest);
    format!("refs/spagitty/inbox/{rest}")
}

/// Whether `task` works in a copy now.
pub fn exists(repo: &Path, task: &TaskId) -> bool {
    copy_path(repo, task).join(".git").is_dir()
}

/// Clone a copy for `task`, on its farm branch, starting from `start`.
///
/// A copy that is already there is handed back as it is, for the reason
/// [`super::worktree::create`] gives: a task sent back is run again in the
/// workspace it has, with its uncommitted work. A farm branch left from an
/// earlier attempt is where a new copy starts, so the agent sees its commits.
pub fn create(
    repo: &Path,
    task: &TaskId,
    provider: AgentProvider,
    start: &str,
) -> Result<Workspace> {
    let branch = branch_name(task, provider);
    let path = copy_path(repo, task);
    exclude_farm_dir(repo);

    if exists(repo, task) {
        return Ok(Workspace {
            task: task.clone(),
            branch,
            path,
        });
    }

    let from = shell::commit_id(repo, &format!("refs/heads/{branch}"))
        .or_else(|_| shell::commit_id(repo, start))?;
    std::fs::create_dir_all(copies_dir(repo))?;

    let made = (|| -> Result<()> {
        let source = repo.to_string_lossy();
        let target = path.to_string_lossy();
        // `--local` hard-links the objects, so a copy costs a checkout and
        // almost no storage. Not `--shared`: alternates break when the
        // person's repository is garbage-collected under a copy.
        shell::ran_ok(
            repo,
            &[
                "clone",
                "--local",
                "--no-tags",
                "--no-checkout",
                "--quiet",
                &source,
                &target,
            ],
        )?;
        shell::ran_ok(&path, &["checkout", "--quiet", "-B", &branch, &from])?;
        // No remote at all: nowhere to push to, nothing to fetch, and none of
        // the person's other branches in sight.
        shell::ran_ok(&path, &["remote", "remove", "origin"])?;
        // The clone made a branch for the person's HEAD as well; the agent
        // sees the branch it works on and nothing else.
        let local = shell::ran_ok(
            &path,
            &["for-each-ref", "--format=%(refname:short)", "refs/heads/"],
        )?;
        for other in local
            .lines()
            .map(str::trim)
            .filter(|name| !name.is_empty() && *name != branch)
        {
            shell::ran_ok(&path, &["branch", "-D", "--quiet", other])?;
        }
        // A clone takes none of the repository's own configuration. The work
        // is committed under the name it would have had in a worktree, and
        // never signed: an agent does not sign as the person.
        for key in ["user.name", "user.email"] {
            if let Some(value) = shell::get_config(repo, key)? {
                shell::ran_ok(&path, &["config", key, &value])?;
            }
        }
        shell::ran_ok(&path, &["config", "commit.gpgsign", "false"])?;
        std::fs::write(start_file(repo, task), format!("{from}\n"))?;
        Ok(())
    })();
    if let Err(error) = made {
        let _ = std::fs::remove_dir_all(&path);
        return Err(error);
    }

    Ok(Workspace {
        task: task.clone(),
        branch,
        path,
    })
}

/// What the door is told about a task.
#[derive(Debug, Clone)]
pub struct Door<'a> {
    /// The paths the task may touch. Empty means anywhere, as for leases.
    pub allowed: &'a [String],
    /// The largest file it may add.
    pub size_limit: u64,
}

/// Work that came in.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Taken {
    /// The commit the farm branch now points at.
    pub commit: String,
    /// The paths it changes, from where the task started.
    pub paths: Vec<String>,
}

/// Why work was turned away at the door. Each says what to change.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Refusal {
    /// The copy's branch does not descend from where the task started.
    NotFromStart,
    /// A changed path is outside the task's declared paths.
    OutsidePaths(String),
    /// A protected path changed and the task did not name it.
    Protected(String),
    /// A submodule pointer changed.
    Gitlink(String),
    /// A file is larger than the limit.
    TooLarge { path: String, bytes: u64 },
    /// The branch has a merge commit, which the task was not asked to make.
    Merge(String),
}

impl std::fmt::Display for Refusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Refusal::NotFromStart => {
                write!(f, "The work does not start from where the task started.")
            }
            Refusal::OutsidePaths(path) => {
                write!(f, "The work changes {path}, which the task may not touch.")
            }
            Refusal::Protected(path) => write!(
                f,
                "The work changes {path}, which a task changes only when it names it."
            ),
            Refusal::Gitlink(path) => write!(f, "The work moves the submodule at {path}."),
            Refusal::TooLarge { path, bytes } => write!(
                f,
                "The work adds {path}, {:.1} MB, over the {:.0} MB limit.",
                *bytes as f64 / 1_048_576.0,
                SIZE_LIMIT as f64 / 1_048_576.0
            ),
            Refusal::Merge(commit) => write!(
                f,
                "The work has a merge commit ({}) the task did not ask for.",
                &commit[..commit.len().min(7)]
            ),
        }
    }
}

/// What intake did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Intake {
    Taken(Taken),
    Refused(Refusal),
    /// The agent committed nothing: the branch is where it started.
    Nothing,
}

/// Take a task's work from its copy, check it, and move the farm branch.
pub fn intake(
    repo: &Path,
    task: &TaskId,
    provider: AgentProvider,
    door: &Door<'_>,
) -> Result<Intake> {
    let path = copy_path(repo, task);
    if !exists(repo, task) {
        return Err(Error::Refused(format!(
            "{task} has no copy to take work from"
        )));
    }
    let branch = branch_name(task, provider);
    let inbox = inbox_ref(task, provider);
    let start = std::fs::read_to_string(start_file(repo, task))
        .map(|text| text.trim().to_string())
        .map_err(|_| Error::Refused(format!("{task}'s copy does not say where it started")))?;

    let source = path.to_string_lossy();
    let refspec = format!("+refs/heads/{branch}:{inbox}");
    shell::ran_ok(
        repo,
        &[
            "fetch",
            "--quiet",
            "--no-tags",
            "--no-write-fetch-head",
            &source,
            &refspec,
        ],
    )?;
    let commit = shell::commit_id(repo, &inbox)?;

    let merged = std::fs::read_to_string(merge_file(repo, task))
        .ok()
        .map(|text| text.trim().to_string())
        .filter(|parent| !parent.is_empty());
    if let Some(refusal) = refuse(repo, &start, &commit, door, merged.as_deref())? {
        return Ok(Intake::Refused(refusal));
    }

    let farm_branch = format!("refs/heads/{branch}");
    let old = shell::commit_id(repo, &farm_branch).ok();
    let unmoved = match old.as_deref() {
        None => true,
        Some(old) => old == commit,
    };
    if commit == start && unmoved {
        return Ok(Intake::Nothing);
    }
    // Compare-and-swap, as every branch move the farm makes: an empty old
    // value means the branch must not exist yet.
    let reason = format!("spagitty: intake for {task}");
    shell::ran_ok(
        repo,
        &[
            "update-ref",
            "-m",
            &reason,
            &farm_branch,
            &commit,
            old.as_deref().unwrap_or(""),
        ],
    )?;

    Ok(Intake::Taken(Taken {
        paths: changed(repo, merged.as_deref().unwrap_or(&start), &commit)?
            .into_iter()
            .map(|change| change.path)
            .collect(),
        commit,
    }))
}

struct Change {
    path: String,
    gitlink: bool,
}

/// What `from..to` changes, by `diff --raw`: renames as a delete and an add, so
/// both sides of a move are checked.
fn changed(repo: &Path, from: &str, to: &str) -> Result<Vec<Change>> {
    let raw = shell::ran_ok(repo, &["diff", "--raw", "--no-renames", "-z", from, to])?;
    let mut changes = Vec::new();
    let mut parts = raw.split('\0').filter(|part| !part.is_empty());
    while let Some(meta) = parts.next() {
        let Some(path) = parts.next() else { break };
        // `:100644 160000 <old> <new> M`: either mode 160000 is a submodule.
        let mut modes = meta.trim_start_matches(':').split(' ');
        let old_mode = modes.next().unwrap_or("");
        let new_mode = modes.next().unwrap_or("");
        changes.push(Change {
            path: path.to_string(),
            gitlink: old_mode == "160000" || new_mode == "160000",
        });
    }
    Ok(changes)
}

/// Whether a declared pattern names `protected` itself, not merely everything.
fn names(allowed: &[String], protected: &str) -> bool {
    allowed.iter().any(|pattern| {
        let pattern = pattern.trim_start_matches("./");
        pattern != EVERYTHING && pattern.starts_with(protected.trim_end_matches('/'))
    })
}

fn refuse(
    repo: &Path,
    start: &str,
    commit: &str,
    door: &Door<'_>,
    merged: Option<&str>,
) -> Result<Option<Refusal>> {
    if !shell::is_ancestor(repo, start, commit)? {
        return Ok(Some(Refusal::NotFromStart));
    }
    let range = format!("{start}..{commit}");
    // `<merge> <parent> <parent>` per line: a merge of the parent Spagitty
    // started is the resolution, and any other is refused.
    let merges = shell::ran_ok(repo, &["rev-list", "--merges", "--parents", &range])?;
    for line in merges.lines() {
        let mut ids = line.split_whitespace();
        let merge = ids.next().unwrap_or_default();
        if !merged.is_some_and(|parent| ids.any(|id| id == parent)) {
            return Ok(Some(Refusal::Merge(merge.to_string())));
        }
    }

    let changes = changed(repo, merged.unwrap_or(start), commit)?;
    for change in &changes {
        if change.gitlink {
            return Ok(Some(Refusal::Gitlink(change.path.clone())));
        }
        if let Some(protected) = PROTECTED.iter().find(|protected| {
            change.path == protected.trim_end_matches('/') || change.path.starts_with(*protected)
        }) {
            if !names(door.allowed, protected) {
                return Ok(Some(Refusal::Protected(change.path.clone())));
            }
        }
        if !door.allowed.is_empty()
            && !door
                .allowed
                .iter()
                .any(|pattern| overlaps(pattern, &change.path))
        {
            return Ok(Some(Refusal::OutsidePaths(change.path.clone())));
        }
    }

    // Sizes from the tree the work ends at, for the files it changed.
    let paths: Vec<&str> = changes.iter().map(|change| change.path.as_str()).collect();
    if !paths.is_empty() {
        let mut args = vec!["ls-tree", "-r", "-l", "-z", commit, "--"];
        args.extend(paths.iter().copied());
        let listed = shell::ran_ok(repo, &args)?;
        for entry in listed.split('\0').filter(|entry| !entry.is_empty()) {
            // `<mode> blob <oid> <size>\t<path>`
            let Some((meta, path)) = entry.split_once('\t') else {
                continue;
            };
            let bytes = meta
                .split_whitespace()
                .nth(3)
                .and_then(|size| size.parse::<u64>().ok())
                .unwrap_or(0);
            if bytes > door.size_limit {
                return Ok(Some(Refusal::TooLarge {
                    path: path.to_string(),
                    bytes,
                }));
            }
        }
    }
    Ok(None)
}

/// Remove a task's copy and its inbox ref.
///
/// Like a worktree, a copy with uncommitted changes survives unless `force`:
/// they are work, and intake took only what was committed.
pub fn remove(repo: &Path, task: &TaskId, provider: AgentProvider, force: bool) -> Result<()> {
    let path = copy_path(repo, task);
    if exists(repo, task) {
        let dirty = shell::ran_ok(&path, &["status", "--porcelain"])
            .map(|status| !status.trim().is_empty())
            .unwrap_or(false);
        if dirty && !force {
            return Ok(());
        }
        std::fs::remove_dir_all(&path)?;
    }
    let _ = std::fs::remove_file(start_file(repo, task));
    let _ = std::fs::remove_file(merge_file(repo, task));
    let inbox = inbox_ref(task, provider);
    if shell::commit_id(repo, &inbox).is_ok() {
        let _ = shell::ran_ok(repo, &["update-ref", "-d", &inbox]);
    }
    Ok(())
}

/// Every copy on disk, by the task it belongs to: what recovery and cleanup read.
pub fn copies(repo: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(copies_dir(repo)) else {
        return Vec::new();
    };
    let mut found: Vec<(String, PathBuf)> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| path.join(".git").is_dir())
        .filter_map(|path| Some((path.file_name()?.to_string_lossy().into_owned(), path)))
        .collect();
    found.sort();
    found
}

#[cfg(test)]
mod tests {
    use super::*;
    use spagitty_core::fixture::Fixture;

    fn task() -> TaskId {
        TaskId::new("TASK-0042")
    }

    const PROVIDER: AgentProvider = AgentProvider::ClaudeCode;

    fn door(allowed: &[String]) -> Door<'_> {
        Door {
            allowed,
            size_limit: SIZE_LIMIT,
        }
    }

    fn commit_in(dir: &Path, path: &str, text: &str) {
        let file = dir.join(path);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, text).unwrap();
        shell::stage(dir, &[path.to_string()]).unwrap();
        shell::commit(dir, &format!("change {path}"), "", false, false).unwrap();
    }

    fn git(dir: &Path, args: &[&str]) -> shell::Ran {
        shell::ran(dir, args).unwrap()
    }

    #[test]
    fn a_copy_lives_beside_the_worktrees_and_its_inbox_is_not_a_branch() {
        assert!(copy_path(Path::new("/repo"), &task()).ends_with(".spagitty/farm/copies/task-0042"));
        assert_eq!(
            inbox_ref(&task(), PROVIDER),
            "refs/spagitty/inbox/TASK-0042/claude"
        );
    }

    #[test]
    fn a_copy_is_a_clone_on_the_farm_branch_with_no_remote_and_no_other_branch() {
        let repo = Fixture::woven();
        let workspace = create(repo.path(), &task(), PROVIDER, "HEAD").unwrap();

        assert!(
            workspace.path.join(".git").is_dir(),
            "a copy has a repository of its own"
        );
        assert_eq!(
            git(&workspace.path, &["branch", "--show-current"])
                .stdout
                .trim(),
            "spagitty-farm/TASK-0042/claude"
        );
        assert_eq!(git(&workspace.path, &["remote"]).stdout.trim(), "");
        let branches = git(
            &workspace.path,
            &["for-each-ref", "--format=%(refname)", "refs/"],
        )
        .stdout;
        assert_eq!(
            branches.trim(),
            "refs/heads/spagitty-farm/TASK-0042/claude",
            "{branches}"
        );
        // Not a linked worktree: the person's repository does not list it.
        assert!(!shell::worktree_list(repo.path())
            .unwrap()
            .contains("copies"));
    }

    #[test]
    fn nothing_done_in_a_copy_moves_the_persons_branches() {
        let repo = Fixture::woven();
        let main = shell::commit_id(repo.path(), "HEAD").unwrap();
        let workspace = create(repo.path(), &task(), PROVIDER, "HEAD").unwrap();
        commit_in(&workspace.path, "docs/a.md", "agent work\n");

        // Every route a linked worktree had to the person's branches.
        assert!(!git(&workspace.path, &["push"]).ok());
        assert!(!git(&workspace.path, &["fetch", "origin"]).ok());
        let current = git(repo.path(), &["branch", "--show-current"]).stdout;
        let _ = git(&workspace.path, &["branch", "-f", current.trim(), "HEAD"]);

        assert_eq!(shell::commit_id(repo.path(), "HEAD").unwrap(), main);
        assert!(
            shell::commit_id(repo.path(), "refs/heads/spagitty-farm/TASK-0042/claude").is_err()
        );
    }

    #[test]
    fn intake_takes_the_work_into_the_inbox_and_moves_the_farm_branch() {
        let repo = Fixture::woven();
        let workspace = create(repo.path(), &task(), PROVIDER, "HEAD").unwrap();
        commit_in(&workspace.path, "docs/a.md", "agent work\n");
        let tip = shell::commit_id(&workspace.path, "HEAD").unwrap();

        let taken = intake(
            repo.path(),
            &task(),
            PROVIDER,
            &door(&["docs/**".to_string()]),
        )
        .unwrap();

        assert_eq!(
            taken,
            Intake::Taken(Taken {
                commit: tip.clone(),
                paths: vec!["docs/a.md".to_string()],
            })
        );
        assert_eq!(
            shell::commit_id(repo.path(), &inbox_ref(&task(), PROVIDER)).unwrap(),
            tip
        );
        assert_eq!(
            shell::commit_id(repo.path(), "refs/heads/spagitty-farm/TASK-0042/claude").unwrap(),
            tip
        );
    }

    #[test]
    fn a_change_outside_the_declared_paths_is_refused_and_named() {
        let repo = Fixture::woven();
        let workspace = create(repo.path(), &task(), PROVIDER, "HEAD").unwrap();
        commit_in(&workspace.path, "src/sneaky.rs", "fn x() {}\n");

        let refused = intake(
            repo.path(),
            &task(),
            PROVIDER,
            &door(&["docs/**".to_string()]),
        )
        .unwrap();

        assert_eq!(
            refused,
            Intake::Refused(Refusal::OutsidePaths("src/sneaky.rs".into()))
        );
        assert!(refused_text(&refused).contains("src/sneaky.rs"));
        // The farm branch was not moved; the work waits in the inbox and the copy.
        assert!(
            shell::commit_id(repo.path(), "refs/heads/spagitty-farm/TASK-0042/claude").is_err()
        );
        assert!(exists(repo.path(), &task()));
    }

    fn refused_text(intake: &Intake) -> String {
        match intake {
            Intake::Refused(refusal) => refusal.to_string(),
            other => panic!("not refused: {other:?}"),
        }
    }

    #[test]
    fn a_protected_path_is_refused_unless_the_task_names_it() {
        let repo = Fixture::woven();
        let workspace = create(repo.path(), &task(), PROVIDER, "HEAD").unwrap();
        commit_in(&workspace.path, ".github/workflows/ci.yml", "on: push\n");

        let anywhere = intake(repo.path(), &task(), PROVIDER, &door(&[])).unwrap();
        assert_eq!(
            anywhere,
            Intake::Refused(Refusal::Protected(".github/workflows/ci.yml".into()))
        );

        let named = [".github/workflows/ci.yml".to_string()];
        assert!(matches!(
            intake(repo.path(), &task(), PROVIDER, &door(&named)).unwrap(),
            Intake::Taken(_)
        ));
    }

    #[test]
    fn a_file_over_the_limit_is_refused() {
        let repo = Fixture::woven();
        let workspace = create(repo.path(), &task(), PROVIDER, "HEAD").unwrap();
        commit_in(&workspace.path, "assets/big.bin", &"x".repeat(2048));

        let small = Door {
            allowed: &[],
            size_limit: 1024,
        };
        assert_eq!(
            intake(repo.path(), &task(), PROVIDER, &small).unwrap(),
            Intake::Refused(Refusal::TooLarge {
                path: "assets/big.bin".into(),
                bytes: 2048
            })
        );
    }

    #[test]
    fn work_that_does_not_start_where_the_task_started_is_refused() {
        let repo = Fixture::woven();
        let workspace = create(repo.path(), &task(), PROVIDER, "HEAD").unwrap();
        // The agent throws its starting point away and commits on a new root.
        git(&workspace.path, &["checkout", "--orphan", "fresh"]);
        commit_in(&workspace.path, "docs/a.md", "rewritten\n");
        git(
            &workspace.path,
            &["branch", "-f", "spagitty-farm/TASK-0042/claude", "fresh"],
        );

        assert_eq!(
            intake(repo.path(), &task(), PROVIDER, &door(&[])).unwrap(),
            Intake::Refused(Refusal::NotFromStart)
        );
    }

    #[test]
    fn a_merge_spagitty_started_is_taken_and_measured_from_the_destination() {
        let repo = Fixture::woven();
        let workspace = create(repo.path(), &task(), PROVIDER, "HEAD").unwrap();
        commit_in(&workspace.path, "docs/a.md", "task\n");
        intake(
            repo.path(),
            &task(),
            PROVIDER,
            &door(&["docs/**".to_string()]),
        )
        .unwrap();
        // The destination moves on, outside the task's paths.
        commit_in(repo.path(), "src/elsewhere.rs", "fn other() {}\n");
        let tip = shell::commit_id(repo.path(), "HEAD").unwrap();
        let branch = git(repo.path(), &["branch", "--show-current"])
            .stdout
            .trim()
            .to_string();
        let refspec = format!("+refs/heads/{branch}:refs/spagitty/destination");
        git(
            &workspace.path,
            &["fetch", "-q", &repo.path().to_string_lossy(), &refspec],
        );
        assert!(git(
            &workspace.path,
            &["merge", "--no-ff", "-q", "-m", "Merge", &tip]
        )
        .ok());

        let unasked = intake(
            repo.path(),
            &task(),
            PROVIDER,
            &door(&["docs/**".to_string()]),
        )
        .unwrap();
        assert!(
            matches!(unasked, Intake::Refused(Refusal::Merge(_))),
            "{unasked:?}"
        );

        allow_merge(repo.path(), &task(), &tip).unwrap();
        let taken = intake(
            repo.path(),
            &task(),
            PROVIDER,
            &door(&["docs/**".to_string()]),
        )
        .unwrap();
        let Intake::Taken(taken) = taken else {
            panic!("{taken:?}")
        };
        assert_eq!(taken.paths, vec!["docs/a.md".to_string()]);
    }

    #[test]
    fn a_copy_that_committed_nothing_brings_nothing_in() {
        let repo = Fixture::woven();
        create(repo.path(), &task(), PROVIDER, "HEAD").unwrap();
        assert_eq!(
            intake(repo.path(), &task(), PROVIDER, &door(&[])).unwrap(),
            Intake::Nothing
        );
    }

    #[test]
    fn a_second_copy_starts_from_the_work_already_taken() {
        let repo = Fixture::woven();
        let workspace = create(repo.path(), &task(), PROVIDER, "HEAD").unwrap();
        commit_in(&workspace.path, "docs/a.md", "first attempt\n");
        intake(repo.path(), &task(), PROVIDER, &door(&[])).unwrap();
        let taken =
            shell::commit_id(repo.path(), "refs/heads/spagitty-farm/TASK-0042/claude").unwrap();

        remove(repo.path(), &task(), PROVIDER, true).unwrap();
        let again = create(repo.path(), &task(), PROVIDER, "HEAD").unwrap();

        assert_eq!(shell::commit_id(&again.path, "HEAD").unwrap(), taken);
    }

    #[test]
    fn removing_keeps_a_copy_with_uncommitted_work_unless_forced() {
        let repo = Fixture::woven();
        let workspace = create(repo.path(), &task(), PROVIDER, "HEAD").unwrap();
        std::fs::write(workspace.path.join("draft.txt"), "not committed").unwrap();

        remove(repo.path(), &task(), PROVIDER, false).unwrap();
        assert!(
            exists(repo.path(), &task()),
            "uncommitted work was thrown away"
        );

        remove(repo.path(), &task(), PROVIDER, true).unwrap();
        assert!(!exists(repo.path(), &task()));
        assert!(copies(repo.path()).is_empty());
    }
}
