// SPDX-License-Identifier: GPL-3.0-or-later

//! Branches, and how far each one has drifted.
//!
//! The Branches screen is mostly a table, and this module produces its rows.
//! Everything here is a read and stays in `gix`; checking out and creating go
//! through [`crate::shell`] for the reasons in that module's header.
//!
//! # Ahead and behind are as old as the last fetch
//!
//! `ahead` and `behind` are counted against the remote-tracking ref on disk,
//! which is whatever the last fetch left there. Nothing in this module talks to
//! a network, so the numbers are exact for what is known locally and cannot be
//! more current than that. The screen says so rather than implying it is live.

use std::collections::HashMap;

use gix::ObjectId;
use serde::Serialize;

use crate::error::{Error, Result};
use crate::graph::short_id;
use crate::refs::RefKind;
use crate::shell;

/// One row of the Branches screen.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct BranchRow {
    /// Short name for display: `main`, `origin/main`.
    pub name: String,
    /// The full ref, for anything that has to be unambiguous.
    pub full_name: String,
    pub kind: RefKind,
    /// True for the branch `HEAD` points at.
    pub current: bool,
    pub id: String,
    pub short: String,
    /// First line of the branch tip's message.
    pub summary: String,
    pub author_name: String,
    /// Author time of the tip, unix seconds.
    pub time: i64,
    /// Short name of the configured upstream, when there is one.
    pub upstream: Option<String>,
    /// Commits this branch has that its upstream does not. `None` when there
    /// is no upstream to compare against.
    pub ahead: Option<usize>,
    /// Commits the upstream has that this branch does not.
    pub behind: Option<usize>,
    /// Fully contained in `HEAD` — the branch that is safe to forget.
    pub merged: bool,
}

/// Every local branch, and every remote-tracking branch, as rows.
///
/// Ordered the way the screen reads them: the current branch first, then local
/// branches, then remotes, alphabetically within each group.
pub fn list(repo: &gix::Repository) -> Result<Vec<BranchRow>> {
    let head = repo.head_id().ok().map(|id| id.detach());
    let current = current_branch(repo);

    // Once, for every row (FEAT-033). The graph's chips read the same function.
    let drifts = divergences(repo);

    let platform = repo.references().map_err(|e| Error::Refs(e.to_string()))?;
    let mut rows = Vec::new();

    for (prefix, kind) in [
        ("refs/heads/", RefKind::Branch),
        ("refs/remotes/", RefKind::Remote),
    ] {
        let iter = platform
            .prefixed(prefix)
            .map_err(|e| Error::Refs(e.to_string()))?;

        for reference in iter.filter_map(std::result::Result::ok) {
            let full_name = reference.name().as_bstr().to_string();
            let name = full_name[prefix.len()..].to_string();

            // `origin/HEAD` is a symbolic pointer at another row, not a branch.
            if kind == RefKind::Remote && name.ends_with("/HEAD") {
                continue;
            }

            let mut reference = reference;
            let Ok(id) = reference.peel_to_id() else {
                // A ref pointing at a missing object. Broken, but not a reason
                // to fail the whole screen.
                continue;
            };
            let id = id.detach();

            let Some(tip) = describe(repo, id) else {
                continue;
            };

            // Looked up rather than recomputed (FEAT-033): the graph's chips
            // read the same map, and two counts of the same thing eventually
            // disagree.
            let drift = (kind == RefKind::Branch)
                .then(|| drifts.get(&name))
                .flatten();
            let upstream = drift.map(|drift| drift.upstream.clone());
            let (ahead, behind) = match drift {
                Some(drift) => (Some(drift.ahead), Some(drift.behind)),
                None => (None, None),
            };

            rows.push(BranchRow {
                current: kind == RefKind::Branch && current.as_deref() == Some(name.as_str()),
                merged: head.is_some_and(|head| is_ancestor(repo, id, head)),
                name,
                full_name,
                kind,
                id: id.to_string(),
                short: short_id(&id),
                summary: tip.summary,
                author_name: tip.author_name,
                time: tip.time,
                upstream,
                ahead,
                behind,
            });
        }
    }

    rows.sort_by(|a, b| {
        b.current
            .cmp(&a.current)
            .then(a.kind.cmp(&b.kind))
            .then(a.name.cmp(&b.name))
    });

    Ok(rows)
}

/// Short name of the checked-out branch's upstream, when it has one that was
/// fetched. Reads one ref, unlike [`list`], which counts every branch's drift.
pub fn current_upstream(repo: &gix::Repository) -> Option<String> {
    let head = repo.head_ref().ok()??;
    upstream_of(repo, &head).map(|(_, short)| short)
}

/// The remote-tracking ref a branch is configured to follow: its id and its
/// short name. `None` for a branch with no upstream, and for remote-tracking
/// refs, which do not have one.
fn upstream_of(
    repo: &gix::Repository,
    reference: &gix::Reference<'_>,
) -> Option<(ObjectId, String)> {
    let name = reference
        .remote_tracking_ref_name(gix::remote::Direction::Fetch)?
        .ok()?;

    let full = name.as_bstr().to_string();
    let short = full
        .strip_prefix("refs/remotes/")
        .unwrap_or(&full)
        .to_string();

    // Configured but never fetched: the ref does not exist on disk yet, which
    // is not an error — there is simply nothing to compare against.
    let mut tracking = repo.find_reference(name.as_ref()).ok()?;
    let id = tracking.peel_to_id().ok()?.detach();

    Some((id, short))
}

/// How far a branch has drifted from its upstream.
///
/// Shared by the Branches screen and the graph's chips (FEAT-033). One
/// definition, computed once per walk, so the two cannot come to disagree — the
/// item made that a criterion, and two reads of the same thing eventually
/// answer differently.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Divergence {
    /// Short name of the configured upstream.
    pub upstream: String,
    pub ahead: usize,
    pub behind: usize,
}

impl Divergence {
    /// True when the two are at the same commit, which is worth saying nothing
    /// about: a chip reading `0/0` on every row is noise.
    pub fn level(&self) -> bool {
        self.ahead == 0 && self.behind == 0
    }
}

/// Every local branch that has an upstream, by short name.
///
/// One traversal for both consumers. `branches` looks its rows up in this, and
/// `refs` attaches it to the chips, so there is one place that counts.
pub fn divergences(repo: &gix::Repository) -> HashMap<String, Divergence> {
    let mut found = HashMap::new();

    let Ok(platform) = repo.references() else {
        return found;
    };
    let Ok(iter) = platform.prefixed("refs/heads/") else {
        return found;
    };

    for reference in iter.flatten() {
        let name = reference.name().shorten().to_string();
        let Ok(id) = reference.clone().into_fully_peeled_id() else {
            continue;
        };
        let Some((up, upstream)) = upstream_of(repo, &reference) else {
            continue;
        };

        let (ahead, behind) = counts(repo, id.detach(), up);
        found.insert(
            name,
            Divergence {
                upstream,
                ahead: ahead.unwrap_or(0),
                behind: behind.unwrap_or(0),
            },
        );
    }

    found
}

/// `(ahead, behind)` — what `git rev-list --count --left-right` reports.
fn counts(
    repo: &gix::Repository,
    local: ObjectId,
    upstream: ObjectId,
) -> (Option<usize>, Option<usize>) {
    (
        Some(count_excluding(repo, local, upstream)),
        Some(count_excluding(repo, upstream, local)),
    )
}

/// Commits reachable from `tip` but not from `hidden`.
fn count_excluding(repo: &gix::Repository, tip: ObjectId, hidden: ObjectId) -> usize {
    let Ok(walk) = repo
        .rev_walk([tip])
        .with_hidden([hidden])
        .sorting(gix::revision::walk::Sorting::BreadthFirst)
        .all()
    else {
        return 0;
    };
    walk.filter_map(std::result::Result::ok).count()
}

/// Is `candidate` an ancestor of `tip` — or `tip` itself?
fn is_ancestor(repo: &gix::Repository, candidate: ObjectId, tip: ObjectId) -> bool {
    if candidate == tip {
        return true;
    }
    count_excluding(repo, candidate, tip) == 0
}

struct Tip {
    summary: String,
    author_name: String,
    time: i64,
}

/// The tip commit's message and signature. `None` when the ref points at
/// something that is not a commit, such as an unpeeled tag object.
fn describe(repo: &gix::Repository, id: ObjectId) -> Option<Tip> {
    let commit = repo.find_commit(id).ok()?;
    let summary = commit
        .message()
        .map(|m| m.summary().to_string())
        .unwrap_or_default();
    let (author_name, time) = match commit.author() {
        Ok(sig) => (
            sig.name.to_string(),
            sig.time().map(|t| t.seconds).unwrap_or(0),
        ),
        Err(_) => (String::new(), 0),
    };

    Some(Tip {
        summary,
        author_name,
        time,
    })
}

fn current_branch(repo: &gix::Repository) -> Option<String> {
    let head = repo.head().ok()?;
    Some(head.referent_name()?.shorten().to_string())
}

// --- Changing which branch is checked out ---------------------------------

/// Check out a branch.
///
/// Through `git` so that filters, LFS and hooks apply, and so that a checkout
/// which would overwrite uncommitted work is refused by git itself rather than
/// by a rule of our own that would have to guess at the same thing.
pub fn checkout(repo: &gix::Repository, name: &str) -> Result<()> {
    let workdir = workdir(repo)?;
    shell::checkout(workdir, name)
}

/// Create a branch at `start`, or at `HEAD` when `start` is empty.
///
/// The name is not validated here. `git` already knows every rule
/// `git check-ref-format` enforces, and its refusal names the actual problem —
/// a second implementation of those rules would only be able to disagree.
pub fn create(repo: &gix::Repository, name: &str, start: &str, checkout_it: bool) -> Result<()> {
    let workdir = workdir(repo)?;
    shell::create_branch(workdir, name, start, checkout_it)
}

/// The working directory to run `git` in.
fn workdir(repo: &gix::Repository) -> Result<&std::path::Path> {
    repo.workdir()
        .ok_or_else(|| Error::NotStageable("a bare repository has no working copy".into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::Fixture;

    fn rows(fixture: &Fixture) -> Vec<BranchRow> {
        list(&fixture.open()).expect("branches")
    }

    /// A remote with a fetch refspec, plus `main` tracking it. Config alone is
    /// not enough: the upstream is resolved through the remote's refspecs.
    fn configure_origin(fixture: &Fixture) {
        fixture.git(&[
            "remote",
            "add",
            "origin",
            "https://example.invalid/repo.git",
        ]);
        fixture.git(&["config", "branch.main.remote", "origin"]);
        fixture.git(&["config", "branch.main.merge", "refs/heads/main"]);
    }

    fn row<'a>(rows: &'a [BranchRow], name: &str) -> &'a BranchRow {
        rows.iter().find(|r| r.name == name).unwrap_or_else(|| {
            panic!(
                "no {name} in {:?}",
                rows.iter().map(|r| &r.name).collect::<Vec<_>>()
            )
        })
    }

    #[test]
    fn every_local_branch_is_listed() {
        let rows = rows(&Fixture::woven());
        let names: Vec<&str> = rows.iter().map(|r| r.name.as_str()).collect();

        assert!(names.contains(&"main"));
        assert!(names.contains(&"feature/split-view"));
        assert!(names.contains(&"merged/already-in-main"));
        assert_eq!(rows.len(), 3, "the fixture has no remote");
    }

    #[test]
    fn the_current_branch_is_marked_and_sorts_first() {
        let fixture = Fixture::woven();
        fixture.git(&["branch", "aaa-would-sort-first"]);

        let rows = rows(&fixture);

        assert_eq!(rows[0].name, "main");
        assert!(rows[0].current);
        assert_eq!(rows.iter().filter(|r| r.current).count(), 1);
    }

    #[test]
    fn a_detached_head_leaves_no_branch_current() {
        let fixture = Fixture::woven();
        fixture.git(&["checkout", "-q", "--detach", "HEAD~1"]);

        assert!(rows(&fixture).iter().all(|r| !r.current));
    }

    #[test]
    fn each_row_describes_its_tip() {
        let fixture = Fixture::woven();
        let main = row(&rows(&fixture), "main").clone();

        assert_eq!(main.id, fixture.head());
        assert_eq!(main.short, fixture.head()[..7]);
        assert_eq!(main.summary, "Merge feature/split-view");
        assert_eq!(main.author_name, "Ada Lovelace");
        assert!(main.time > 0);
    }

    #[test]
    fn merged_matches_git_branch_merged() {
        let fixture = Fixture::woven();
        let rows = rows(&fixture);

        let expected: Vec<String> = fixture
            .git(&["branch", "--merged", "HEAD"])
            .lines()
            .map(|l| l.trim_start_matches('*').trim().to_string())
            .collect();

        for r in rows.iter().filter(|r| r.kind == RefKind::Branch) {
            assert_eq!(
                r.merged,
                expected.contains(&r.name),
                "{} disagreed with git",
                r.name
            );
        }
    }

    #[test]
    fn a_branch_ahead_of_head_is_not_merged() {
        let fixture = Fixture::woven();
        fixture.git(&["switch", "-q", "-c", "chore/tooling"]);
        fixture.write("tools.txt", "tooling\n");
        fixture.git(&["add", "tools.txt"]);
        fixture.commit("Add a tooling note");
        fixture.git(&["switch", "-q", "main"]);

        assert!(!row(&rows(&fixture), "chore/tooling").merged);
    }

    #[test]
    fn a_branch_with_no_upstream_reports_no_counts() {
        let main = row(&rows(&Fixture::woven()), "main").clone();

        assert_eq!(main.upstream, None);
        assert_eq!(main.ahead, None);
        assert_eq!(main.behind, None);
    }

    #[test]
    fn ahead_and_behind_match_git() {
        let fixture = Fixture::woven();
        // A remote-tracking ref two commits behind, and a local one commit ahead.
        configure_origin(&fixture);
        fixture.git(&["update-ref", "refs/remotes/origin/main", "HEAD~2"]);

        let main = row(&rows(&fixture), "main").clone();
        assert_eq!(main.upstream.as_deref(), Some("origin/main"));

        let expected = fixture.git(&["rev-list", "--left-right", "--count", "main...origin/main"]);
        let mut parts = expected.split_whitespace();
        let ahead: usize = parts.next().unwrap().parse().unwrap();
        let behind: usize = parts.next().unwrap().parse().unwrap();

        assert_eq!(main.ahead, Some(ahead));
        assert_eq!(main.behind, Some(behind));
    }

    #[test]
    fn the_chip_and_the_row_never_disagree_about_the_drift() {
        // FEAT-033 made this a criterion. Both come from `divergences`, so the
        // test is really that neither consumer recomputes it — but asserted on
        // the values, because that is what a reader would notice going wrong.
        let fixture = Fixture::woven();
        configure_origin(&fixture);
        fixture.git(&["update-ref", "refs/remotes/origin/main", "HEAD~2"]);
        let repo = fixture.open();

        let row = row(&rows(&fixture), "main").clone();
        let index = crate::refs::RefIndex::build(&repo).expect("refs");
        let head = repo.head_id().expect("head").detach();
        let chip = index
            .chips_for(&head)
            .iter()
            .find(|chip| chip.name == "main")
            .expect("a chip for main")
            .clone();

        let drift = chip.divergence.expect("the chip carries the drift");
        assert_eq!(Some(drift.ahead), row.ahead);
        assert_eq!(Some(drift.behind), row.behind);
        assert_eq!(Some(drift.upstream.as_str()), row.upstream.as_deref());
    }

    #[test]
    fn a_level_branch_is_marked_level_rather_than_left_to_the_caller() {
        let fixture = Fixture::woven();
        configure_origin(&fixture);
        fixture.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);

        let drift = divergences(&fixture.open())
            .remove("main")
            .expect("a divergence");

        assert!(drift.level());
    }

    #[test]
    fn a_branch_with_no_upstream_is_absent_from_the_map() {
        // Absent rather than zeroed: "level" and "nothing to compare against"
        // are different, and a chip must not draw the first for the second.
        let fixture = Fixture::woven();

        assert!(divergences(&fixture.open()).is_empty());
    }

    #[test]
    fn a_branch_level_with_its_upstream_is_zero_and_zero_rather_than_nothing() {
        let fixture = Fixture::woven();
        configure_origin(&fixture);
        fixture.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);

        let main = row(&rows(&fixture), "main").clone();

        assert_eq!((main.ahead, main.behind), (Some(0), Some(0)));
    }

    #[test]
    fn an_upstream_configured_but_never_fetched_reports_no_counts() {
        // The config names a ref that is not on disk. That is a repository
        // waiting for its first fetch, not a broken one.
        let fixture = Fixture::woven();
        configure_origin(&fixture);

        let main = row(&rows(&fixture), "main").clone();

        assert_eq!(main.upstream, None);
        assert_eq!(main.ahead, None);
    }

    #[test]
    fn remote_tracking_branches_are_listed_after_local_ones() {
        let fixture = Fixture::woven();
        fixture.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
        fixture.git(&[
            "symbolic-ref",
            "refs/remotes/origin/HEAD",
            "refs/remotes/origin/main",
        ]);

        let rows = rows(&fixture);
        let kinds: Vec<RefKind> = rows.iter().map(|r| r.kind).collect();
        let first_remote = kinds
            .iter()
            .position(|k| *k == RefKind::Remote)
            .expect("a remote");
        let last_local = kinds
            .iter()
            .rposition(|k| *k == RefKind::Branch)
            .expect("a branch");

        assert!(last_local < first_remote);
        assert_eq!(row(&rows, "origin/main").kind, RefKind::Remote);
        assert!(
            !rows.iter().any(|r| r.name.ends_with("/HEAD")),
            "origin/HEAD is a pointer at another row, not a branch"
        );
    }

    #[test]
    fn an_empty_repository_has_no_branches_and_is_not_an_error() {
        assert!(rows(&Fixture::empty()).is_empty());
    }

    // --- Checking out and creating -----------------------------------------

    #[test]
    fn checking_out_moves_head_and_the_current_row() {
        let fixture = Fixture::woven();

        checkout(&fixture.open(), "feature/split-view").expect("checkout");

        assert_eq!(
            fixture.git(&["rev-parse", "--abbrev-ref", "HEAD"]).trim(),
            "feature/split-view"
        );
        assert!(row(&rows(&fixture), "feature/split-view").current);
        assert!(!row(&rows(&fixture), "main").current);
    }

    #[test]
    fn a_checkout_that_would_overwrite_work_is_refused_with_gits_message() {
        // Nothing on the Branches screen may discard uncommitted work, and the
        // check belongs to git rather than to a rule of ours that would have to
        // guess at the same thing.
        let fixture = Fixture::woven();
        // `core.txt` differs between the two branches.
        fixture.write("core.txt", "my uncommitted work\n");

        let error = checkout(&fixture.open(), "feature/split-view").unwrap_err();

        assert!(matches!(error, Error::Git { .. }));
        assert_eq!(
            fixture.git(&["rev-parse", "--abbrev-ref", "HEAD"]).trim(),
            "main"
        );
        assert_eq!(
            std::fs::read_to_string(fixture.path().join("core.txt")).expect("read"),
            "my uncommitted work\n"
        );
    }

    #[test]
    fn checking_out_a_branch_that_is_not_there_says_so() {
        let fixture = Fixture::woven();

        let error = checkout(&fixture.open(), "no-such-branch").unwrap_err();

        assert!(matches!(error, Error::Git { .. }));
    }

    #[test]
    fn creating_a_branch_leaves_head_where_it_was() {
        let fixture = Fixture::woven();

        create(&fixture.open(), "chore/tooling", "", false).expect("create");

        assert_eq!(
            fixture.git(&["rev-parse", "--abbrev-ref", "HEAD"]).trim(),
            "main"
        );
        let created = row(&rows(&fixture), "chore/tooling").clone();
        assert_eq!(created.id, fixture.head());
        assert!(!created.current);
    }

    #[test]
    fn creating_a_branch_from_a_chosen_start_point() {
        let fixture = Fixture::woven();
        let start = fixture.rev("HEAD~1");

        create(&fixture.open(), "from-earlier", &start, false).expect("create");

        assert_eq!(row(&rows(&fixture), "from-earlier").id, start);
    }

    #[test]
    fn creating_and_checking_out_in_one_step() {
        let fixture = Fixture::woven();

        create(&fixture.open(), "chore/tooling", "", true).expect("create");

        assert_eq!(
            fixture.git(&["rev-parse", "--abbrev-ref", "HEAD"]).trim(),
            "chore/tooling"
        );
        assert!(row(&rows(&fixture), "chore/tooling").current);
    }

    #[test]
    fn a_duplicate_name_is_refused_by_git_and_the_existing_branch_is_untouched() {
        let fixture = Fixture::woven();
        let before = fixture.rev("feature/split-view");

        let error = create(&fixture.open(), "feature/split-view", "", false).unwrap_err();

        assert!(matches!(error, Error::Git { .. }));
        assert_eq!(fixture.rev("feature/split-view"), before);
    }

    #[test]
    fn an_invalid_name_is_refused_by_git() {
        // git already knows every rule `check-ref-format` enforces; a second
        // implementation could only disagree with it.
        let fixture = Fixture::woven();

        let error = create(&fixture.open(), "has spaces", "", false).unwrap_err();

        assert!(matches!(error, Error::Git { .. }));
    }
}
