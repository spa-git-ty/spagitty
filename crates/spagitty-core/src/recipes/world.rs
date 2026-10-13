// SPDX-License-Identifier: GPL-3.0-or-later

//! What is true of a repository, for the validator (FEAT-133).
//!
//! [`gather`] reads what [`World`] holds that git knows: the branches and
//! remotes, what is checked out, whether there are changes, what is in
//! progress, and how the branch stands against its upstream. It is read
//! in-process, with no `git` run, because the stack is checked with every
//! change to it and the command log is for what Spagitty did.
//!
//! What git does not know — the repository's checks, its rules for agents —
//! is filled in by the application.

use std::path::Path;

use gix::ObjectId;

use super::validate::World;
use super::{Recipe, Step};
use crate::conflicts::{self, Operation};
use crate::error::Result;

/// Is anything reachable from `tip` and not from `hidden`?
fn any_beyond(repo: &gix::Repository, tip: ObjectId, hidden: ObjectId) -> bool {
    if tip == hidden {
        return false;
    }
    repo.rev_walk([tip])
        .with_hidden([hidden])
        .all()
        .map(|mut walk| walk.any(|commit| commit.is_ok()))
        .unwrap_or(false)
}

fn tip_of(repo: &gix::Repository, name: &str) -> Option<ObjectId> {
    repo.rev_parse_single(name)
        .ok()?
        .object()
        .ok()?
        .peel_to_commit()
        .ok()
        .map(|commit| commit.id)
}

/// What the repository at `path` is, as far as git knows, for `recipe`.
///
/// `rewrites_pushed` is asked only about the branches the recipe rebases
/// onto: a rebase onto one replays the commits that branch lacks, and when
/// the upstream has any of those, a push afterwards would need force.
pub fn gather(path: &Path, recipe: &Recipe) -> Result<World> {
    let repo = crate::repo::open(path)?;
    let head = crate::repo::head(&repo);

    let mut branches: Vec<String> = repo
        .references()
        .ok()
        .and_then(|platform| {
            platform.prefixed("refs/heads/").ok().map(|iter| {
                iter.flatten()
                    .map(|reference| reference.name().shorten().to_string())
                    .collect()
            })
        })
        .unwrap_or_default();
    branches.sort();

    let remotes = crate::remotes::remotes(&repo)
        .into_iter()
        .map(|remote| remote.name)
        .collect();
    let dirty = crate::status::working_copy(&repo)
        .map(|copy| copy.changed_paths() > 0)
        .unwrap_or(false);
    let operation = match conflicts::operation(&repo) {
        Operation::None => None,
        Operation::Merge => Some("merge"),
        Operation::Rebase | Operation::RebaseInteractive => Some("rebase"),
        Operation::CherryPick => Some("cherry-pick"),
        Operation::Revert => Some("revert"),
        Operation::ApplyMailbox => Some("patch series"),
        Operation::Bisect => Some("bisect"),
    }
    .map(str::to_string);

    let drift = head
        .branch
        .as_ref()
        .and_then(|branch| crate::branches::divergences(&repo).remove(branch));
    let upstream_tip = drift
        .as_ref()
        .and_then(|drift| tip_of(&repo, &format!("refs/remotes/{}", drift.upstream)));

    let mut rewrites_pushed = Vec::new();
    if let Some(upstream) = upstream_tip {
        for step in &recipe.steps {
            let Step::Rebase { onto } = step else {
                continue;
            };
            if rewrites_pushed.contains(onto) {
                continue;
            }
            if let Some(onto_tip) = tip_of(&repo, &format!("refs/heads/{onto}")) {
                if any_beyond(&repo, upstream, onto_tip) {
                    rewrites_pushed.push(onto.clone());
                }
            }
        }
    }

    Ok(World {
        branches,
        remotes,
        current: head.branch,
        dirty,
        operation,
        upstream: drift.is_some(),
        unpushed: drift.map(|drift| drift.ahead as u32).unwrap_or(0),
        rewrites_pushed,
        ..World::default()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::Fixture;

    fn save(fixture: &Fixture, message: &str) {
        fixture.git(&["add", "-A"]);
        fixture.commit(message);
    }

    fn rebase_onto(onto: &str) -> Recipe {
        Recipe {
            steps: vec![Step::Rebase { onto: onto.into() }],
            ..Recipe::named("r")
        }
    }

    #[test]
    fn it_reads_the_branches_what_is_checked_out_and_whether_there_are_changes() {
        let fixture = Fixture::empty();
        fixture.write("a.txt", "one\n");
        save(&fixture, "First");
        fixture.git(&["branch", "feature/avatars"]);
        fixture.git(&["remote", "add", "origin", "https://example.com/r.git"]);

        let clean = gather(fixture.path(), &Recipe::named("r")).expect("gathered");
        assert_eq!(clean.branches, ["feature/avatars", "main"]);
        assert_eq!(clean.remotes, ["origin"]);
        assert_eq!(clean.current.as_deref(), Some("main"));
        assert!(!clean.dirty);
        assert_eq!(clean.operation, None);
        // No upstream: nothing is pushed, and nothing is known to be.
        assert!(!clean.upstream);
        assert_eq!(clean.unpushed, 0);

        fixture.write("new.txt", "untracked\n");
        assert!(
            gather(fixture.path(), &Recipe::named("r"))
                .expect("gathered")
                .dirty
        );
    }

    #[test]
    fn a_detached_head_has_no_branch_and_a_stopped_merge_is_said() {
        let fixture = Fixture::conflicted();

        let world = gather(fixture.path(), &Recipe::named("r")).expect("gathered");

        assert_eq!(world.operation.as_deref(), Some("merge"));
        assert!(world.dirty);

        let detached = Fixture::linear(2);
        detached.git(&["checkout", "-q", "--detach", "HEAD"]);
        assert_eq!(
            gather(detached.path(), &Recipe::named("r"))
                .expect("gathered")
                .current,
            None
        );
    }

    /// A branch two commits ahead of an upstream it has pushed one commit to.
    fn pushed() -> Fixture {
        let fixture = Fixture::empty();
        fixture.write("a.txt", "one\n");
        save(&fixture, "First");
        fixture.git(&["branch", "base"]);
        fixture.write("a.txt", "one\ntwo\n");
        save(&fixture, "Pushed");
        let pushed = fixture.head();
        fixture.git(&["update-ref", "refs/remotes/origin/main", &pushed]);
        fixture.git(&["remote", "add", "origin", "https://example.com/r.git"]);
        fixture.git(&["config", "branch.main.remote", "origin"]);
        fixture.git(&["config", "branch.main.merge", "refs/heads/main"]);
        fixture.write("a.txt", "one\ntwo\nthree\n");
        save(&fixture, "Local one");
        fixture.write("a.txt", "one\ntwo\nthree\nfour\n");
        save(&fixture, "Local two");
        fixture
    }

    #[test]
    fn it_counts_the_commits_the_upstream_lacks() {
        let world = gather(pushed().path(), &Recipe::named("r")).expect("gathered");

        assert!(world.upstream);
        assert_eq!(world.unpushed, 2);
    }

    /// The rule a push after a rebase is checked by.
    #[test]
    fn a_rebase_rewrites_pushed_commits_when_the_upstream_has_some_the_other_branch_lacks() {
        let fixture = pushed();
        // `base` lacks the pushed commit: rebasing onto it replays that one.
        fixture.git(&["branch", "ahead", "refs/remotes/origin/main"]);

        let onto_base = gather(fixture.path(), &rebase_onto("base")).expect("gathered");
        assert_eq!(onto_base.rewrites_pushed, ["base"]);

        // `ahead` has everything the upstream has: only local commits move.
        let onto_ahead = gather(fixture.path(), &rebase_onto("ahead")).expect("gathered");
        assert_eq!(onto_ahead.rewrites_pushed, Vec::<String>::new());

        // A branch that is not there is the validator's to refuse.
        let onto_nothing = gather(fixture.path(), &rebase_onto("nowhere")).expect("gathered");
        assert_eq!(onto_nothing.rewrites_pushed, Vec::<String>::new());

        // And a recipe that rebases onto nothing is asked nothing.
        assert!(gather(fixture.path(), &Recipe::named("r"))
            .expect("gathered")
            .rewrites_pushed
            .is_empty());
    }
}
