// SPDX-License-Identifier: GPL-3.0-or-later

//! A pull request read from the repository rather than from its host
//! (FEAT-089).
//!
//! The host's patch is enough to list a pull request's changes and not enough
//! to review it: it has no whole files, nothing to fold out, and nothing to
//! build and run. So the Review room fetches the pull request's head into the
//! repository and diffs it locally, the way `git` would.
//!
//! # Where the head is kept
//!
//! Hosts publish every pull request's head under a ref of their own —
//! `refs/pull/N/head` on GitHub, `refs/merge-requests/N/head` on GitLab — that
//! a normal fetch does not bring down. It is fetched into
//! `refs/spagitty/pull/N`, which no branch list, graph or tag list reads, so
//! reviewing forty pull requests does not put forty chips on the graph. The
//! target branch is fetched into its ordinary remote-tracking ref, which is
//! what a fetch would do to it anyway.
//!
//! # What a pull request's diff is
//!
//! The host's: from where the head left the target — their merge base — to
//! the head. Changes that reached the target since are not the pull request's,
//! and a diff against the target's tip would show them reversed.

use std::path::{Path, PathBuf};

use serde::Serialize;

use crate::error::{Error, Result};
use crate::forge::Kind;
use crate::shell;

/// Where pull request `number`'s head is kept locally.
pub fn local_ref(number: u64) -> String {
    format!("refs/spagitty/pull/{number}")
}

/// The ref the host publishes pull request `number`'s head under, or `None`
/// for a host that publishes none.
pub fn host_ref(kind: Kind, number: u64) -> Option<String> {
    match kind {
        Kind::GitHub => Some(format!("refs/pull/{number}/head")),
        Kind::GitLab => Some(format!("refs/merge-requests/{number}/head")),
        // Bitbucket Cloud has no fetchable pull request ref.
        Kind::Bitbucket => None,
    }
}

/// The refspecs that bring a pull request and its target down: the head into
/// [`local_ref`], the target into its remote-tracking ref.
pub fn refspecs(kind: Kind, remote: &str, number: u64, target: &str) -> Result<Vec<String>> {
    let head = host_ref(kind, number).ok_or_else(|| Error::Forge {
        host: kind.label().to_string(),
        detail: "this host publishes no ref to fetch a pull request from".into(),
    })?;
    if !is_plain_ref_part(target) || !is_plain_ref_part(remote) {
        return Err(Error::UnknownPath(target.to_string()));
    }
    Ok(vec![
        format!("+{head}:{}", local_ref(number)),
        format!("+refs/heads/{target}:refs/remotes/{remote}/{target}"),
    ])
}

/// A branch or remote name that cannot change what a refspec means: no `:`,
/// no leading `+` or `-`, no whitespace, no `..`.
fn is_plain_ref_part(name: &str) -> bool {
    !name.is_empty()
        && !name.starts_with(['+', '-', '/'])
        && !name.contains(':')
        && !name.contains("..")
        && !name.chars().any(char::is_whitespace)
}

/// Fetch pull request `number`'s head and its target from `remote`.
///
/// A network operation, through the one place processes are spawned, with
/// `GIT_TERMINAL_PROMPT=0` like every other: a remote that wants a password
/// fails with git's own words rather than waiting on a prompt nobody sees.
pub fn fetch(dir: &Path, kind: Kind, remote: &str, number: u64, target: &str) -> Result<()> {
    let specs = refspecs(kind, remote, number, target)?;
    let specs: Vec<&str> = specs.iter().map(String::as_str).collect();
    shell::fetch_refspecs(dir, remote, &specs)
}

/// The three commits a pull request's diff is between.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PullHead {
    /// The head, as fetched.
    pub head: String,
    /// The target's tip, as fetched.
    pub base: String,
    /// Where the head left the target: the diff is from here.
    pub merge_base: String,
}

/// Resolve what was fetched, or `None` when it has not been.
pub fn resolve(
    repo: &gix::Repository,
    remote: &str,
    number: u64,
    target: &str,
) -> Result<Option<PullHead>> {
    let peel = |name: &str| -> Option<gix::ObjectId> {
        let mut reference = repo.find_reference(name).ok()?;
        reference.peel_to_id().ok().map(|id| id.detach())
    };

    let (Some(head), Some(base)) = (
        peel(&local_ref(number)),
        peel(&format!("refs/remotes/{remote}/{target}")),
    ) else {
        return Ok(None);
    };

    let merge_base = repo
        .merge_base(head, base)
        .map_err(|e| Error::Walk(format!("no common history with {target}: {e}")))?;

    Ok(Some(PullHead {
        head: head.to_string(),
        base: base.to_string(),
        merge_base: merge_base.to_string(),
    }))
}

/// True when the fetched head is the one the host reports, so a second fetch
/// would bring nothing new.
pub fn is_current(fetched: &PullHead, reported_head: &str) -> bool {
    !reported_head.is_empty() && fetched.head == reported_head
}

/// Where a pull request's worktree goes: beside the repository, named after
/// it — `spagitty` and pull request 214 make `spagitty-pr-214`.
pub fn worktree_path(dir: &Path, number: u64) -> Option<PathBuf> {
    let name = dir.file_name()?.to_string_lossy();
    Some(dir.parent()?.join(format!("{name}-pr-{number}")))
}

/// Put the pull request's head in a worktree of its own, detached, so it can
/// be built and run without touching the branch you are on (FEAT-089).
///
/// One worktree per pull request: opened again, it is moved to the new head —
/// unless it has changes of its own, which are never thrown away; git refuses
/// the switch and says why.
pub fn open_worktree(repo: &gix::Repository, number: u64, head: &str) -> Result<PathBuf> {
    let dir = crate::repo::workdir(repo)?;
    let path =
        worktree_path(dir, number).ok_or_else(|| Error::UnknownPath(dir.display().to_string()))?;

    let existing = crate::worktrees::list(repo)?;
    let canonical = |path: &Path| path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
    let already = existing
        .iter()
        .any(|worktree| canonical(Path::new(&worktree.path)) == canonical(&path));

    if already {
        shell::checkout_detached(&path, head)?;
    } else {
        crate::worktrees::add(repo, &path, Some(head), None, true)?;
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{self, FileStatus, LineOrigin};
    use crate::fixture::Fixture;

    #[test]
    fn each_host_publishes_its_heads_under_its_own_ref() {
        assert_eq!(
            host_ref(Kind::GitHub, 214).as_deref(),
            Some("refs/pull/214/head")
        );
        assert_eq!(
            host_ref(Kind::GitLab, 7).as_deref(),
            Some("refs/merge-requests/7/head")
        );
        assert_eq!(host_ref(Kind::Bitbucket, 7), None);
    }

    #[test]
    fn the_head_lands_where_no_branch_list_reads_and_the_target_where_a_fetch_puts_it() {
        assert_eq!(
            refspecs(Kind::GitHub, "origin", 214, "main").unwrap(),
            vec![
                "+refs/pull/214/head:refs/spagitty/pull/214".to_string(),
                "+refs/heads/main:refs/remotes/origin/main".to_string(),
            ]
        );
    }

    #[test]
    fn a_name_that_would_change_the_refspec_is_refused() {
        for target in ["", "a:b", "+main", "-x", "a b", "a/../b"] {
            assert!(
                refspecs(Kind::GitHub, "origin", 1, target).is_err(),
                "{target}"
            );
        }
        assert!(refspecs(Kind::GitHub, "or:igin", 1, "main").is_err());
        assert!(refspecs(Kind::Bitbucket, "origin", 1, "main").is_err());
    }

    #[test]
    fn the_worktree_sits_beside_the_repository() {
        assert_eq!(
            worktree_path(Path::new("/code/spagitty"), 214),
            Some(PathBuf::from("/code/spagitty-pr-214"))
        );
    }

    /// Stage everything, new files too, and commit.
    fn save(fixture: &Fixture, message: &str) -> String {
        fixture.git(&["add", "-A"]);
        fixture.commit(message)
    }

    /// A host with a pull request on it, and a clone that has not fetched it.
    fn host_and_clone() -> (Fixture, Fixture, String) {
        let host = Fixture::empty();
        host.write("lib.rs", "one\ntwo\nthree\n");
        save(&host, "base");
        host.git(&["checkout", "-q", "-b", "feature"]);
        host.write("lib.rs", "one\nTWO\nthree\nfour\n");
        host.write("new.rs", "fresh\n");
        let head = save(&host, "the change");
        host.git(&["update-ref", "refs/pull/7/head", &head]);
        // The target moves on after the branch left it.
        host.git(&["checkout", "-q", "main"]);
        host.write("other.rs", "landed on main\n");
        save(&host, "main moves");

        let clone = Fixture::empty();
        clone.git(&["remote", "add", "origin", &host.path().to_string_lossy()]);
        (host, clone, head)
    }

    #[test]
    fn a_pull_request_is_fetched_and_diffed_from_where_it_left_its_target() {
        let (host, clone, head) = host_and_clone();
        let repo = clone.open();
        assert_eq!(resolve(&repo, "origin", 7, "main").unwrap(), None);

        fetch(clone.path(), Kind::GitHub, "origin", 7, "main").unwrap();
        let repo = clone.open();
        let pull = resolve(&repo, "origin", 7, "main")
            .unwrap()
            .expect("fetched");
        assert_eq!(pull.head, head);
        assert_eq!(pull.base, host.rev("main"));
        assert_eq!(pull.merge_base, host.rev("main~1"));
        assert!(is_current(&pull, &head));
        assert!(!is_current(&pull, ""));

        // The change on main since is not the pull request's.
        let files = diff::changes_between(&repo, &pull.merge_base, &pull.head).unwrap();
        let paths: Vec<&str> = files.iter().map(|file| file.path.as_str()).collect();
        assert_eq!(paths, vec!["lib.rs", "new.rs"]);
        assert_eq!((files[0].added, files[0].removed), (2, 1));
        assert_eq!(files[1].status, FileStatus::Added);
    }

    #[test]
    fn a_whole_file_is_every_new_line_with_the_removed_ones_in_place() {
        let (_host, clone, _head) = host_and_clone();
        fetch(clone.path(), Kind::GitHub, "origin", 7, "main").unwrap();
        let repo = clone.open();
        let pull = resolve(&repo, "origin", 7, "main").unwrap().unwrap();

        let file =
            diff::full_file_between(&repo, &pull.merge_base, &pull.head, "lib.rs", None).unwrap();
        let shape: Vec<(LineOrigin, Option<u32>, Option<u32>, &str)> = file
            .lines
            .iter()
            .map(|line| (line.origin, line.old, line.new, line.text.as_str()))
            .collect();
        assert_eq!(
            shape,
            vec![
                (LineOrigin::Context, Some(1), Some(1), "one"),
                (LineOrigin::Removed, Some(2), None, "two"),
                (LineOrigin::Added, None, Some(2), "TWO"),
                (LineOrigin::Context, Some(3), Some(3), "three"),
                (LineOrigin::Added, None, Some(4), "four"),
            ]
        );
        assert!(file.old_blob.is_some() && file.new_blob.is_some());
        assert_ne!(file.old_blob, file.new_blob);

        let added =
            diff::full_file_between(&repo, &pull.merge_base, &pull.head, "new.rs", None).unwrap();
        assert_eq!(added.status, FileStatus::Added);
        assert_eq!(added.old_blob, None);
        assert_eq!(added.lines.len(), 1);
    }

    #[test]
    fn the_pull_request_opens_in_a_worktree_of_its_own_and_moves_with_its_head() {
        let (host, clone, head) = host_and_clone();
        clone.git(&["fetch", "-q", "origin", "main"]);
        clone.git(&["checkout", "-q", "-b", "main", "FETCH_HEAD"]);
        fetch(clone.path(), Kind::GitHub, "origin", 7, "main").unwrap();
        let repo = clone.open();

        let path = open_worktree(&repo, 7, &head).unwrap();
        assert!(path.ends_with(format!(
            "{}-pr-7",
            clone.path().file_name().unwrap().to_string_lossy()
        )));
        assert_eq!(
            std::fs::read_to_string(path.join("new.rs")).unwrap().trim(),
            "fresh"
        );

        // The author pushes again; opening it again moves the worktree.
        host.git(&["checkout", "-q", "feature"]);
        host.write("new.rs", "fresher\n");
        let next = save(&host, "again");
        host.git(&["update-ref", "refs/pull/7/head", &next]);
        fetch(clone.path(), Kind::GitHub, "origin", 7, "main").unwrap();
        let again = open_worktree(&clone.open(), 7, &next).unwrap();
        assert_eq!(again, path);
        assert_eq!(
            std::fs::read_to_string(path.join("new.rs")).unwrap().trim(),
            "fresher"
        );

        let _ = clone.git(&["worktree", "remove", "--force", &path.to_string_lossy()]);
    }
}
