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

use std::path::Path;

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

/// Where Check out branch left the working copy (FEAT-095).
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CheckedOut {
    /// The branch now checked out.
    pub branch: String,
    /// The remote-tracking branch it follows, when the pull request is the
    /// remote's own branch.
    pub upstream: Option<String>,
    /// The pull request's own branch name is another branch here, so it is
    /// `pr-N` instead.
    pub renamed: bool,
}

/// A local branch's commit, if there is a branch of that name.
fn branch_tip(repo: &gix::Repository, name: &str) -> Option<gix::ObjectId> {
    let mut reference = repo.find_reference(name).ok()?;
    Some(reference.peel_to_id().ok()?.detach())
}

/// Is `candidate` `tip` or one of its ancestors?
fn is_ancestor(repo: &gix::Repository, candidate: gix::ObjectId, tip: gix::ObjectId) -> bool {
    candidate == tip
        || repo
            .merge_base(candidate, tip)
            .is_ok_and(|base| base.detach() == candidate)
}

/// A name a branch can have.
fn is_branch_name(name: &str) -> bool {
    !name.is_empty() && gix::refs::FullName::try_from(format!("refs/heads/{name}")).is_ok()
}

/// Check the pull request's fetched head out as a branch here (FEAT-095).
///
/// No branch of yours is ever moved. The pull request's own name, `source`,
/// is used when there is no branch of that name — it is made at the head —
/// or when the branch of that name is already at the head. Anything else, or
/// a name that is the target's (a fork's `main` into `main`), gets
/// `pr-N`, Spagitty's own, which only ever moves forward: one with commits
/// the pull request does not have is refused rather than moved.
///
/// A new branch follows `remote/source` when that is exactly the head: the
/// pull request is the remote's own branch, so a pull or a push goes where
/// the author's does. Uncommitted changes go across, or git refuses and says
/// what would be overwritten, as for any checkout.
pub fn check_out(
    repo: &gix::Repository,
    remote: &str,
    number: u64,
    head: &str,
    source: &str,
    target: &str,
) -> Result<CheckedOut> {
    let dir = crate::repo::workdir(repo)?;
    let head_id = gix::ObjectId::from_hex(head.as_bytes())
        .map_err(|_| Error::UnknownCommit(head.to_string()))?;

    if source != target && is_branch_name(source) {
        match branch_tip(repo, &format!("refs/heads/{source}")) {
            None => {
                shell::create_branch(dir, source, head, true)?;
                let tracked = format!("{remote}/{source}");
                let upstream = (branch_tip(repo, &format!("refs/remotes/{tracked}"))
                    == Some(head_id))
                .then(|| shell::set_upstream(dir, source, &tracked).map(|()| tracked))
                .transpose()?;
                return Ok(CheckedOut {
                    branch: source.to_string(),
                    upstream,
                    renamed: false,
                });
            }
            Some(at) if at == head_id => {
                shell::checkout(dir, source)?;
                return Ok(CheckedOut {
                    branch: source.to_string(),
                    upstream: None,
                    renamed: false,
                });
            }
            Some(_) => {}
        }
    }

    let own = format!("pr-{number}");
    if let Some(at) = branch_tip(repo, &format!("refs/heads/{own}")) {
        if !is_ancestor(repo, at, head_id) {
            return Err(Error::NotStageable(format!(
                "{own} has commits the pull request does not; check it out yourself"
            )));
        }
    }
    shell::switch_reset(dir, &own, head)?;
    let renamed = source != own;
    Ok(CheckedOut {
        branch: own,
        upstream: None,
        renamed,
    })
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
        // The list carries the blobs a viewed tick is kept against, the same
        // ones the whole file reports.
        let whole =
            diff::full_file_between(&repo, &pull.merge_base, &pull.head, "lib.rs", None).unwrap();
        assert_eq!(files[0].new_blob, whole.new_blob);
        assert_eq!(files[0].old_blob, whole.old_blob);
        assert_eq!(files[1].old_blob, None);
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

    /// The clone with `main` checked out and pull request 7 fetched.
    fn fetched() -> (Fixture, Fixture, String) {
        let (host, clone, head) = host_and_clone();
        clone.git(&["fetch", "-q", "origin", "main"]);
        clone.git(&["checkout", "-q", "-b", "main", "FETCH_HEAD"]);
        fetch(clone.path(), Kind::GitHub, "origin", 7, "main").unwrap();
        (host, clone, head)
    }

    fn current(clone: &Fixture) -> String {
        clone.git(&["branch", "--show-current"]).trim().to_string()
    }

    #[test]
    fn a_pull_request_is_checked_out_under_its_own_name() {
        let (_host, clone, head) = fetched();
        let done = check_out(&clone.open(), "origin", 7, &head, "feature", "main").unwrap();
        assert_eq!(
            done,
            CheckedOut {
                branch: "feature".into(),
                upstream: None,
                renamed: false
            }
        );
        assert_eq!(current(&clone), "feature");
        assert_eq!(clone.rev("HEAD"), head);

        // Again, from elsewhere: the branch is already at the head.
        clone.git(&["checkout", "-q", "main"]);
        let again = check_out(&clone.open(), "origin", 7, &head, "feature", "main").unwrap();
        assert_eq!(again.branch, "feature");
        assert_eq!(current(&clone), "feature");
    }

    #[test]
    fn the_remote_s_own_branch_is_followed() {
        let (_host, clone, head) = fetched();
        clone.git(&[
            "fetch",
            "-q",
            "origin",
            "feature:refs/remotes/origin/feature",
        ]);
        let done = check_out(&clone.open(), "origin", 7, &head, "feature", "main").unwrap();
        assert_eq!(done.upstream.as_deref(), Some("origin/feature"));
        assert_eq!(
            clone
                .git(&["rev-parse", "--abbrev-ref", "feature@{upstream}"])
                .trim(),
            "origin/feature"
        );
    }

    #[test]
    fn a_branch_of_yours_is_never_moved() {
        let (_host, clone, head) = fetched();
        let main = clone.rev("main");
        // A `feature` of your own, somewhere else.
        clone.git(&["branch", "feature", "main"]);
        let done = check_out(&clone.open(), "origin", 7, &head, "feature", "main").unwrap();
        assert_eq!(
            done,
            CheckedOut {
                branch: "pr-7".into(),
                upstream: None,
                renamed: true
            }
        );
        assert_eq!(clone.rev("feature"), main);
        assert_eq!(clone.rev("HEAD"), head);

        // A fork's `main` into `main` never takes the target's name.
        clone.git(&["checkout", "-q", "main"]);
        let done = check_out(&clone.open(), "origin", 7, &head, "main", "main").unwrap();
        assert_eq!(done.branch, "pr-7");
        assert_eq!(clone.rev("main"), main);
    }

    #[test]
    fn its_own_pr_branch_moves_forward_and_never_back() {
        let (host, clone, head) = fetched();
        clone.git(&["branch", "feature", "main"]);
        check_out(&clone.open(), "origin", 7, &head, "feature", "main").unwrap();

        // The author pushes again; checked out again, pr-7 follows.
        host.git(&["checkout", "-q", "feature"]);
        host.write("new.rs", "fresher\n");
        let newer = save(&host, "again");
        host.git(&["update-ref", "refs/pull/7/head", &newer]);
        fetch(clone.path(), Kind::GitHub, "origin", 7, "main").unwrap();
        let done = check_out(&clone.open(), "origin", 7, &newer, "feature", "main").unwrap();
        assert_eq!(done.branch, "pr-7");
        assert_eq!(clone.rev("HEAD"), newer);

        // A commit of yours on pr-7 is never thrown away.
        clone.write("mine.rs", "mine\n");
        let mine = save(&clone, "mine");
        assert!(check_out(&clone.open(), "origin", 7, &newer, "feature", "main").is_err());
        assert_eq!(clone.rev("pr-7"), mine);
    }

    #[test]
    fn uncommitted_work_it_would_overwrite_stops_it() {
        let (_host, clone, head) = fetched();
        clone.write("lib.rs", "mine, not committed\n");
        assert!(check_out(&clone.open(), "origin", 7, &head, "feature", "main").is_err());
        assert_eq!(current(&clone), "main");
        assert_eq!(
            std::fs::read_to_string(clone.path().join("lib.rs")).unwrap(),
            "mine, not committed\n"
        );
    }
}
