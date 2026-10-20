// SPDX-License-Identifier: GPL-3.0-or-later

//! Stop tracking files and folders on a branch, in a commit (FEAT-155).
//!
//! `git rm --cached` takes a path out of the index and leaves it on disk; a
//! commit then records it gone from the branch. Without a `.gitignore` line
//! the file is untracked but not ignored, and the next `git add .` puts it
//! straight back, so the line is written in the same commit when asked.
//!
//! # On the branch checked out here
//!
//! In place, and only when nothing else is staged: the commit is the
//! person's ordinary commit, hooks and signing as configured, and it must
//! hold the untracking and nothing they had staged for another reason.
//!
//! # On any other branch
//!
//! In a worktree made for the commit with nothing checked out — its index is
//! the branch's tree, its folder is empty — so neither the working copy nor
//! the branch checked out here moves. Hooks do not run there: the branch's
//! files are not on disk for them to check, and a lint that finds nothing
//! fails. Signing does, as configured. The worktree is removed afterwards,
//! whether the commit was made or not.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::{Error, Result};
use crate::{hooks, shell, signing};

/// One path to stop tracking.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Target {
    /// From the top of the repository, `/`-separated.
    pub path: String,
    pub folder: bool,
}

/// What [`untrack`] made.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Untracked {
    pub commit: String,
    pub branch: String,
    /// Where the branch is pushed: its upstream's remote, else `origin`, else
    /// the only remote. `None` when there is nowhere to push it.
    pub remote: Option<String>,
    /// Whether the commit ran the hooks (FEAT-107). Not on a branch that is
    /// not checked out here.
    pub hooks_ran: bool,
}

/// The `.gitignore` line that ignores exactly `path`, from the top of the
/// repository: anchored with `/`, a folder with a trailing `/`, and the
/// characters a pattern would read as a glob escaped.
pub fn ignore_line(path: &Target) -> String {
    let mut line = String::from("/");
    let name = path.path.trim_matches('/');
    for (index, c) in name.char_indices() {
        let trailing_space = c == ' ' && name[index..].trim_end().is_empty();
        if matches!(c, '*' | '?' | '[' | ']' | '\\') || trailing_space {
            line.push('\\');
        }
        line.push(c);
    }
    if path.folder {
        line.push('/');
    }
    line
}

/// `existing` with a line for each path it does not already have.
pub fn with_ignored(existing: &str, paths: &[Target]) -> String {
    let mut out = existing.to_string();
    for path in paths {
        let line = ignore_line(path);
        if out.lines().any(|kept| kept.trim_end() == line) {
            continue;
        }
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        out.push_str(&line);
        out.push('\n');
    }
    out
}

/// A path someone could name, and not one that leaves the repository.
fn checked(paths: &[Target]) -> Result<Vec<String>> {
    if paths.is_empty() {
        return Err(Error::Untrack("Choose a file or folder to untrack.".into()));
    }
    paths
        .iter()
        .map(|path| {
            let name = path.path.trim_matches('/');
            let escapes = name.split('/').any(|part| part == ".." || part.is_empty());
            if name.is_empty() || escapes || name.starts_with(':') {
                return Err(Error::Untrack(format!(
                    "“{}” is not a path in this repository.",
                    path.path
                )));
            }
            Ok(name.to_string())
        })
        .collect()
}

/// A worktree that is removed when it is dropped, made or not.
struct Scratch<'a> {
    repo: &'a Path,
    dir: PathBuf,
}

impl Drop for Scratch<'_> {
    fn drop(&mut self) {
        if shell::worktree_remove(self.repo, &self.dir, true).is_err() {
            let _ = std::fs::remove_dir_all(&self.dir);
            let _ = shell::worktree_prune(self.repo);
        }
    }
}

/// Stop tracking `paths` on `branch` and commit it with `message`, adding a
/// `.gitignore` line for each when `ignore` is set.
pub fn untrack(
    repo: &gix::Repository,
    branch: &str,
    paths: &[Target],
    ignore: bool,
    message: &str,
) -> Result<Untracked> {
    let names = checked(paths)?;
    let message = message.trim();
    let (subject, body) = message.split_once('\n').unwrap_or((message, ""));
    if subject.trim().is_empty() {
        return Err(Error::EmptyMessage);
    }
    let dir = repo
        .workdir()
        .ok_or_else(|| Error::Untrack("A bare repository has no files to untrack.".into()))?;

    let checked_out = repo
        .head_name()
        .map_err(|error| Error::Config(error.to_string()))?
        .map(|name| name.shorten().to_string());
    let here = checked_out.as_deref() == Some(branch);
    let signing = signing::read(repo);

    let (commit, hooks_ran) = if here {
        let staged = shell::staged_names(dir)?;
        if !staged.is_empty() {
            return Err(Error::Untrack(format!(
                "{} {} already staged here, and would go into this commit too. Commit or unstage {} first.",
                staged.len(),
                if staged.len() == 1 { "change is" } else { "changes are" },
                if staged.len() == 1 { "it" } else { "them" },
            )));
        }
        if ignore && !shell::unstaged_names(dir, &[".gitignore"])?.is_empty() {
            return Err(Error::Untrack(
                ".gitignore has changes that are not committed, and adding to it would commit them. \
                 Commit or discard them first, or untrack without adding to .gitignore."
                    .into(),
            ));
        }
        shell::untrack_paths(dir, &names)?;
        if ignore {
            let file = dir.join(".gitignore");
            let existing = std::fs::read_to_string(&file).unwrap_or_default();
            let next = with_ignored(&existing, paths);
            if next != existing {
                std::fs::write(&file, next)?;
                shell::stage(dir, &[".gitignore".to_string()])?;
            }
        }
        let run_hooks = hooks::enabled(dir)?;
        let commit = shell::commit_with(
            dir,
            subject.trim(),
            body,
            false,
            signing.enabled,
            !run_hooks,
            &mut |_| {},
        )
        .map_err(|error| signing::as_signing_failure(error, &signing.program))?;
        (commit, run_hooks)
    } else {
        let scratch = Scratch {
            repo: dir,
            dir: std::env::temp_dir().join(format!(
                "spagitty-untrack-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|elapsed| elapsed.as_nanos())
                    .unwrap_or_default()
            )),
        };
        shell::worktree_add_unchecked(dir, &scratch.dir, branch)?;
        shell::untrack_paths(&scratch.dir, &names)?;
        if ignore {
            let existing =
                shell::show_text(&scratch.dir, "HEAD", ".gitignore")?.unwrap_or_default();
            let next = with_ignored(&existing, paths);
            if next != existing {
                std::fs::write(scratch.dir.join(".gitignore"), next)?;
                shell::stage(&scratch.dir, &[".gitignore".to_string()])?;
            }
        }
        let commit = shell::commit_with(
            &scratch.dir,
            subject.trim(),
            body,
            false,
            signing.enabled,
            true,
            &mut |_| {},
        )
        .map_err(|error| signing::as_signing_failure(error, &signing.program))?;
        (commit, false)
    };

    Ok(Untracked {
        commit,
        branch: branch.to_string(),
        remote: remote_for(repo, dir, branch)?,
        hooks_ran,
    })
}

/// Where `branch` is pushed: its upstream's remote, else `origin`, else the
/// only remote there is.
fn remote_for(repo: &gix::Repository, dir: &Path, branch: &str) -> Result<Option<String>> {
    if let Some(remote) = shell::get_config(dir, &format!("branch.{branch}.remote"))? {
        if remote != "." {
            return Ok(Some(remote));
        }
    }
    let names: Vec<String> = crate::remotes::remotes(repo)
        .into_iter()
        .map(|remote| remote.name)
        .collect();
    if names.iter().any(|name| name == "origin") {
        return Ok(Some("origin".into()));
    }
    Ok(match names.as_slice() {
        [only] => Some(only.clone()),
        _ => None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::Fixture;

    fn file(path: &str) -> Target {
        Target {
            path: path.into(),
            folder: false,
        }
    }

    fn folder(path: &str) -> Target {
        Target {
            path: path.into(),
            folder: true,
        }
    }

    fn a_project() -> Fixture {
        let fixture = Fixture::empty();
        fixture.write("README.md", "# Project\n");
        fixture.write(".gitignore", "target/\n");
        fixture.write("secrets.env", "TOKEN=1\n");
        fixture.write("build/out.js", "1\n");
        fixture.write("build/deep/map.js", "2\n");
        fixture.git(&["add", "-A"]);
        fixture.commit("A project");
        fixture
    }

    fn tracked(fixture: &Fixture, rev: &str) -> Vec<String> {
        fixture
            .git(&["ls-tree", "-r", "--name-only", rev])
            .lines()
            .map(String::from)
            .collect()
    }

    #[test]
    fn untracks_on_the_branch_checked_out_and_leaves_the_files_on_disk() {
        let fixture = a_project();
        let repo = fixture.open();
        let made = untrack(
            &repo,
            "main",
            &[file("secrets.env"), folder("build")],
            true,
            "Stop tracking secrets.env and build/",
        )
        .expect("untracked");

        assert_eq!(made.branch, "main");
        assert!(made.hooks_ran);
        assert_eq!(tracked(&fixture, "HEAD"), [".gitignore", "README.md"]);
        assert!(fixture.path().join("secrets.env").exists());
        assert!(fixture.path().join("build/deep/map.js").exists());
        assert_eq!(
            std::fs::read_to_string(fixture.path().join(".gitignore")).unwrap(),
            "target/\n/secrets.env\n/build/\n"
        );
        // Ignored now, so nothing is waiting to be added back.
        assert_eq!(fixture.git(&["status", "--porcelain"]).trim(), "");
    }

    #[test]
    fn untracks_on_another_branch_without_touching_the_working_copy() {
        let fixture = a_project();
        fixture.git(&["branch", "release"]);
        fixture.write("notes.txt", "mine\n");
        fixture.git(&["add", "notes.txt"]);
        let repo = fixture.open();
        let before = fixture.git(&["rev-parse", "main"]);

        let made = untrack(
            &repo,
            "release",
            &[file("secrets.env")],
            true,
            "Untrack secrets",
        )
        .expect("untracked");

        assert!(!made.hooks_ran);
        assert_eq!(fixture.git(&["rev-parse", "release"]).trim(), made.commit);
        assert!(!tracked(&fixture, "release").contains(&"secrets.env".to_string()));
        assert_eq!(
            fixture.git(&["show", "release:.gitignore"]),
            "target/\n/secrets.env\n"
        );
        // Here, nothing moved: the branch, the staged file, the .gitignore.
        assert_eq!(fixture.git(&["rev-parse", "main"]), before);
        assert_eq!(
            fixture.git(&["diff", "--cached", "--name-only"]).trim(),
            "notes.txt"
        );
        assert_eq!(
            std::fs::read_to_string(fixture.path().join(".gitignore")).unwrap(),
            "target/\n"
        );
        // And the worktree it was made in is gone.
        assert_eq!(fixture.git(&["worktree", "list"]).lines().count(), 1);
    }

    #[test]
    fn refuses_to_sweep_staged_changes_into_the_commit() {
        let fixture = a_project();
        fixture.write("README.md", "# Changed\n");
        fixture.git(&["add", "README.md"]);
        let repo = fixture.open();
        let refused = untrack(&repo, "main", &[file("secrets.env")], false, "Untrack")
            .unwrap_err()
            .to_string();
        assert!(refused.contains("1 change is already staged"), "{refused}");
        assert!(tracked(&fixture, "HEAD").contains(&"secrets.env".to_string()));
    }

    #[test]
    fn refuses_a_dirty_gitignore_only_when_it_would_be_written() {
        let fixture = a_project();
        fixture.write(".gitignore", "target/\n*.log\n");
        let repo = fixture.open();
        assert!(untrack(&repo, "main", &[file("secrets.env")], true, "Untrack").is_err());
        let made = untrack(&repo, "main", &[file("secrets.env")], false, "Untrack");
        assert!(made.is_ok(), "{made:?}");
    }

    #[test]
    fn refuses_nothing_a_path_out_of_the_repository_and_no_message() {
        let fixture = a_project();
        let repo = fixture.open();
        assert!(untrack(&repo, "main", &[], true, "x").is_err());
        assert!(untrack(&repo, "main", &[file("../elsewhere")], true, "x").is_err());
        assert!(untrack(&repo, "main", &[file("README.md")], true, "  ").is_err());
        // A path git does not track is git's own refusal.
        assert!(untrack(&repo, "main", &[file("nowhere.txt")], false, "x").is_err());
    }

    #[test]
    fn ignore_lines_are_anchored_and_escaped() {
        assert_eq!(ignore_line(&file("a/b.txt")), "/a/b.txt");
        assert_eq!(ignore_line(&folder("node_modules")), "/node_modules/");
        assert_eq!(ignore_line(&file("weird[1]*.txt")), "/weird\\[1\\]\\*.txt");
        assert_eq!(ignore_line(&file("space ")), "/space\\ ");
        assert_eq!(
            with_ignored("a\n/b/", &[folder("b"), file("c")]),
            "a\n/b/\n/c\n"
        );
    }
}
