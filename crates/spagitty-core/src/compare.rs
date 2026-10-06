// SPDX-License-Identifier: GPL-3.0-or-later

//! Naming the two ends of a comparison (FEAT-096).
//!
//! A review is of a range — a base and a head — and the review is only worth
//! anything if both ends are pinned to commits before the work starts. These
//! are the reads that pin them: a revision to a commit id, the merge base of
//! two commits, and the identity two checkouts of one repository share.
//!
//! In-process, like every other read in this crate. Nothing here writes.

use std::path::{Path, PathBuf};

use crate::{Error, Result};

/// The full commit id `revision` names: a branch, a tag, `HEAD`, a remote
/// branch or an id.
///
/// Peeled to a commit, so an annotated tag names the commit it tags rather than
/// the tag object.
pub fn resolve_commit(repo: &gix::Repository, revision: &str) -> Result<String> {
    let unknown = || Error::UnknownCommit(revision.to_string());
    let id = repo
        .rev_parse_single(revision)
        .map_err(|_| unknown())?
        .object()
        .map_err(|_| unknown())?
        .peel_to_commit()
        .map_err(|_| unknown())?
        .id;
    Ok(id.to_string())
}

/// The best common ancestor of two commits, or `None` when their histories
/// never meet.
pub fn merge_base(repo: &gix::Repository, a: &str, b: &str) -> Result<Option<String>> {
    let a = gix::ObjectId::from_hex(a.as_bytes()).map_err(|_| Error::UnknownCommit(a.into()))?;
    let b = gix::ObjectId::from_hex(b.as_bytes()).map_err(|_| Error::UnknownCommit(b.into()))?;
    match repo.merge_base(a, b) {
        Ok(id) => Ok(Some(id.detach().to_string())),
        Err(_) => Ok(None),
    }
}

/// What every checkout of one repository has in common: the canonical path of
/// its *common* git directory.
///
/// A main checkout and each worktree cut from it have different working
/// directories and different `.git` entries, and the same common directory.
/// Grants and enablement are stored under this, so a farm task's worktree is
/// the repository the user enabled an extension for, not a stranger.
pub fn common_dir(repo: &gix::Repository) -> PathBuf {
    let common = repo.common_dir();
    std::fs::canonicalize(common).unwrap_or_else(|_| common.to_path_buf())
}

/// The main checkout's working directory, when it has one.
///
/// For a non-bare repository the common directory is `<main>/.git`, so its
/// parent is the main checkout — whichever worktree the question was asked
/// from.
pub fn main_workdir(repo: &gix::Repository) -> Option<PathBuf> {
    let common = common_dir(repo);
    if common.file_name().is_some_and(|name| name == ".git") {
        return common.parent().map(Path::to_path_buf);
    }
    repo.workdir().map(Path::to_path_buf)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::Fixture;

    #[test]
    fn a_branch_resolves_to_the_commit_it_points_at() {
        let fixture = Fixture::woven();
        let repo = crate::repo::open(fixture.path()).unwrap();

        let head = crate::repo::head(&repo).id.unwrap();
        assert_eq!(resolve_commit(&repo, "HEAD").unwrap(), head);
        assert!(resolve_commit(&repo, "no-such-branch").is_err());
    }

    #[test]
    fn the_merge_base_of_a_commit_and_its_parent_is_the_parent() {
        let fixture = Fixture::woven();
        let repo = crate::repo::open(fixture.path()).unwrap();
        let head = resolve_commit(&repo, "HEAD").unwrap();
        let parent = resolve_commit(&repo, "HEAD~1").unwrap();

        assert_eq!(merge_base(&repo, &head, &parent).unwrap(), Some(parent));
    }

    #[test]
    fn a_range_lists_the_files_between_its_ends() {
        let fixture = Fixture::woven();
        let repo = crate::repo::open(fixture.path()).unwrap();
        let head = resolve_commit(&repo, "HEAD").unwrap();
        let parent = resolve_commit(&repo, "HEAD~1").unwrap();

        let files = crate::diff::range_files(&repo, &parent, &head).unwrap();
        let detail = crate::diff::commit_diff(&repo, &head).unwrap();
        let expected: Vec<_> = detail.files.iter().map(|f| f.path.clone()).collect();
        assert_eq!(
            files.iter().map(|f| f.path.clone()).collect::<Vec<_>>(),
            expected
        );
        assert!(crate::diff::range_files(&repo, &head, &head)
            .unwrap()
            .is_empty());
    }

    #[test]
    fn the_main_checkout_is_found_from_itself() {
        let fixture = Fixture::woven();
        let repo = crate::repo::open(fixture.path()).unwrap();

        let main = main_workdir(&repo).unwrap();
        assert_eq!(
            std::fs::canonicalize(main).unwrap(),
            std::fs::canonicalize(fixture.path()).unwrap()
        );
        assert!(common_dir(&repo).ends_with(".git"));
    }
}
