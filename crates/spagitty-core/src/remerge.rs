// SPDX-License-Identifier: GPL-3.0-or-later

//! Code a pull request carries that was written while resolving a merge
//! conflict (FEAT-092).
//!
//! A pull request that merged its target and resolved a conflict holds lines
//! that are neither the author's own change nor the target's: they were
//! written in the merge. Read as the author's work they hide in plain sight —
//! and a resolution is exactly where one side quietly gets dropped.
//!
//! `git show --remerge-diff` (git 2.36) re-does a merge as git would have,
//! conflict markers and all, and diffs that against what was committed: what
//! it shows is the resolution and nothing else. The lines the resolution wrote
//! are followed to the pull request's head, and what each side had where they
//! conflicted is kept from the markers, so the reviewer can see both.

use std::collections::{BTreeSet, HashMap};
use std::path::Path;

use gix::ObjectId;
use serde::Serialize;

use crate::diff::{self, LineOrigin};
use crate::error::{Error, Result};
use crate::graph::short_id;
use crate::shell;

/// Past this many merges in one pull request the rest are not re-done: each
/// one is a whole merge's worth of work for git.
pub const MAX_MERGES: usize = 20;

/// Lines one merge wrote in one place of one file.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictFix {
    pub path: String,
    /// The merge, in full and short, and its subject.
    pub merge: String,
    pub short: String,
    pub summary: String,
    /// The lines of the head the fix wrote that the pull request adds, by
    /// the head's numbering.
    pub lines: Vec<u32>,
    /// What each side had where they conflicted: the target's, and the pull
    /// request's own. Both empty where the merge changed lines that nothing
    /// conflicted over.
    pub main_side: Vec<String>,
    pub branch_side: Vec<String>,
}

/// A file the pull request changes that carries a conflict fix.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FixedFile {
    pub path: String,
    /// The pull request adds lines to it besides the fixes, or removes lines
    /// that cannot be told apart.
    pub author: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConflictFixes {
    pub fixes: Vec<ConflictFix>,
    pub files: Vec<FixedFile>,
    /// Every merge that was re-done, by short id, whether it fixed anything.
    pub merges: Vec<String>,
    /// There were more merges than were re-done.
    pub truncated: bool,
}

/// One line of a remerge diff: its origin and, on the committed side, its
/// number in the merge.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Line {
    origin: char,
    number: Option<u32>,
    text: String,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Remerged {
    path: String,
    lines: Vec<Line>,
}

/// The files of `git show --remerge-diff` output, hunks laid end to end.
fn parse(text: &str) -> Vec<Remerged> {
    let mut files: Vec<Remerged> = Vec::new();
    let mut in_hunk = false;
    let mut at = 0u32;
    for raw in text.split('\n') {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        if line.starts_with("diff --git ") {
            files.push(Remerged::default());
            in_hunk = false;
            continue;
        }
        let Some(file) = files.last_mut() else { continue };
        if line.starts_with("@@ ") {
            // `@@ -a,b +c,d @@`: the committed side starts at `c`.
            at = line
                .split(' ')
                .find_map(|part| part.strip_prefix('+'))
                .and_then(|range| range.split(',').next())
                .and_then(|start| start.parse().ok())
                .unwrap_or(1);
            in_hunk = true;
            continue;
        }
        if !in_hunk {
            if let Some(path) = line.strip_prefix("+++ ") {
                let path = path.trim_matches('"');
                file.path = path.strip_prefix("b/").unwrap_or("").to_string();
            }
            continue;
        }
        let (origin, rest) = match line.chars().next() {
            Some(origin @ (' ' | '+' | '-')) => (origin, &line[1..]),
            _ => continue,
        };
        let number = (origin != '-').then(|| {
            at += 1;
            at - 1
        });
        file.lines.push(Line {
            origin,
            number,
            text: rest.to_string(),
        });
    }
    files.retain(|file| !file.path.is_empty());
    files
}

/// One place a merge wrote lines: their numbers in the merge, and the two
/// sides the markers held there.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct Region {
    lines: Vec<u32>,
    first: Vec<String>,
    second: Vec<String>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Side {
    First,
    Base,
    Second,
}

/// The places a remerge diff shows the merge wrote lines.
///
/// The removed side of a remerge diff is the merge as git would have made it,
/// conflict markers included; the added side is what was committed. Inside a
/// pair of markers, every committed line is the resolution — the side that
/// was kept as much as anything added. Git prints a change's removed lines
/// before its added ones, so the lines written in place of a conflict follow
/// its closing marker; they belong to it until an unchanged line intervenes.
/// Lines added away from any conflict are a place of their own, with no
/// sides.
fn regions(file: &Remerged) -> Vec<Region> {
    let mut out = Vec::new();
    let mut open: Option<(Region, Side)> = None;
    let mut closing: Option<Region> = None;
    let mut loose: Option<Region> = None;
    let flush = |out: &mut Vec<Region>, region: &mut Option<Region>| {
        if let Some(region) = region.take() {
            if !region.lines.is_empty() {
                out.push(region);
            }
        }
    };

    for line in &file.lines {
        let mut marker = false;
        if line.origin != '+' {
            let text = line.text.as_str();
            if text.starts_with("<<<<<<<") {
                flush(&mut out, &mut closing);
                flush(&mut out, &mut loose);
                if let Some((region, _)) = open.take() {
                    out.push(region);
                }
                open = Some((Region::default(), Side::First));
                marker = true;
            } else if open.is_some() && text.starts_with(">>>>>>>") {
                closing = open.take().map(|(region, _)| region);
                marker = true;
            } else if let Some((region, side)) = open.as_mut() {
                if text.starts_with("|||||||") && *side == Side::First {
                    *side = Side::Base;
                    marker = true;
                } else if text.starts_with("=======") {
                    *side = Side::Second;
                    marker = true;
                } else {
                    match side {
                        Side::First => region.first.push(line.text.clone()),
                        Side::Second => region.second.push(line.text.clone()),
                        Side::Base => {}
                    }
                }
            }
        }

        let Some(number) = line.number else { continue };
        if let Some((region, _)) = open.as_mut() {
            region.lines.push(number);
        } else if line.origin == '+' {
            match closing.as_mut() {
                Some(region) => region.lines.push(number),
                None => loose.get_or_insert_with(Region::default).lines.push(number),
            }
        } else if !marker {
            flush(&mut out, &mut closing);
            flush(&mut out, &mut loose);
        }
    }
    if let Some((region, _)) = open.take() {
        out.push(region);
    }
    flush(&mut out, &mut closing);
    flush(&mut out, &mut loose);
    out
}

fn id(text: &str) -> Result<ObjectId> {
    ObjectId::from_hex(text.as_bytes()).map_err(|_| Error::UnknownCommit(text.to_string()))
}

/// Is `candidate` `tip` or one of its ancestors?
fn is_ancestor(repo: &gix::Repository, candidate: ObjectId, tip: ObjectId) -> bool {
    candidate == tip || repo.merge_base(candidate, tip).is_ok_and(|base| base.detach() == candidate)
}

/// The two-parent merges between `from` and `to`, newest first.
fn merges(repo: &gix::Repository, from: ObjectId, to: ObjectId) -> Result<Vec<(ObjectId, ObjectId, ObjectId)>> {
    let walk = repo
        .rev_walk([to])
        .with_hidden([from])
        .all()
        .map_err(|e| Error::Walk(e.to_string()))?;
    let mut out = Vec::new();
    for info in walk {
        let info = info.map_err(|e| Error::Walk(e.to_string()))?;
        let commit = repo
            .find_commit(info.id)
            .map_err(|e| Error::Walk(e.to_string()))?;
        let parents: Vec<ObjectId> = commit.parent_ids().map(|parent| parent.detach()).collect();
        if let [first, second] = parents[..] {
            out.push((info.id, first, second));
        }
    }
    Ok(out)
}

/// Lines of `path` in `from` by their number in `to`, for the lines neither
/// changed. Empty when the file cannot be read in both.
fn carried(repo: &gix::Repository, from: &str, to: &str, path: &str) -> HashMap<u32, u32> {
    if from == to {
        return HashMap::new();
    }
    let Ok(file) = diff::full_file_between(repo, from, to, path, None) else {
        return HashMap::new();
    };
    file.lines
        .iter()
        .filter(|line| line.origin == LineOrigin::Context)
        .filter_map(|line| Some((line.old?, line.new?)))
        .collect()
}

/// The conflict fixes in a pull request: what each merge between
/// `merge_base` and `head` wrote while resolving, followed to the head.
///
/// `target` is the target branch's tip: of a merge's two parents, the one it
/// contains is the target's side.
pub fn conflicts(
    repo: &gix::Repository,
    dir: &Path,
    merge_base: &str,
    head: &str,
    target: &str,
) -> Result<ConflictFixes> {
    let found = merges(repo, id(merge_base)?, id(head)?)?;
    let target = id(target)?;
    let mut report = ConflictFixes {
        truncated: found.len() > MAX_MERGES,
        ..ConflictFixes::default()
    };

    // What the pull request adds, by file, as the head numbers it.
    let mut added: HashMap<String, (BTreeSet<u32>, bool)> = HashMap::new();
    let mut added_in = |path: &str| -> (BTreeSet<u32>, bool) {
        added
            .entry(path.to_string())
            .or_insert_with(|| match diff::full_file_between(repo, merge_base, head, path, None) {
                Ok(file) => {
                    let lines: BTreeSet<u32> = file
                        .lines
                        .iter()
                        .filter(|line| line.origin == LineOrigin::Added)
                        .filter_map(|line| line.new)
                        .collect();
                    (lines, file.removed > 0)
                }
                Err(_) => (BTreeSet::new(), false),
            })
            .clone()
    };

    for (merge, first, second) in found.into_iter().take(MAX_MERGES) {
        let commit = repo
            .find_commit(merge)
            .map_err(|e| Error::Walk(e.to_string()))?;
        let summary = commit
            .message()
            .map(|message| message.summary().to_string())
            .unwrap_or_default();
        let short = short_id(&merge);
        report.merges.push(short.clone());

        // The target's side is the parent the target already contains; when
        // neither or both do, the second, which is where `git merge main`
        // puts it.
        let main_is_first = is_ancestor(repo, first, target) && !is_ancestor(repo, second, target);

        let text = shell::remerge_diff(dir, &merge.to_string())?;
        for file in parse(&text) {
            let (adds, _) = added_in(&file.path);
            if adds.is_empty() {
                continue;
            }
            let moved = carried(repo, &merge.to_string(), head, &file.path);
            let at_head = |number: u32| {
                if merge.to_string() == head {
                    Some(number)
                } else {
                    moved.get(&number).copied()
                }
            };
            for region in regions(&file) {
                let lines: Vec<u32> = region
                    .lines
                    .iter()
                    .filter_map(|&number| at_head(number))
                    .filter(|number| adds.contains(number))
                    .collect();
                if lines.is_empty() {
                    continue;
                }
                let (main_side, branch_side) = if main_is_first {
                    (region.first, region.second)
                } else {
                    (region.second, region.first)
                };
                report.fixes.push(ConflictFix {
                    path: file.path.clone(),
                    merge: merge.to_string(),
                    short: short.clone(),
                    summary: summary.clone(),
                    lines,
                    main_side,
                    branch_side,
                });
            }
        }
    }

    let mut paths: Vec<String> = report.fixes.iter().map(|fix| fix.path.clone()).collect();
    paths.sort();
    paths.dedup();
    for path in paths {
        let (adds, removes) = added_in(&path);
        let fixed: BTreeSet<u32> = report
            .fixes
            .iter()
            .filter(|fix| fix.path == path)
            .flat_map(|fix| fix.lines.iter().copied())
            .collect();
        let author = adds.iter().any(|line| !fixed.contains(line)) || (adds.is_empty() && removes);
        report.files.push(FixedFile { path, author });
    }
    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::Fixture;

    const REMERGE: &str = "diff --git a/src/a.rs b/src/a.rs
remerge CONFLICT (content): Merge conflict in src/a.rs
index 0000000..1111111 100644
--- a/src/a.rs
+++ b/src/a.rs
@@ -3,9 +3,7 @@ fn cache_dir() {
 one
-<<<<<<< 1234567 (Keep the avatars folder)
-    let dir = base.join(\"avatars\");
-=======
-    let dir = base.or_else(xdg);
->>>>>>> 89abcde (Add the XDG fallback)
+    let dir = base.or_else(xdg)
+        .join(\"avatars\");
 two
+    log(dir);
 three
diff --git a/b.ts b/b.ts
--- a/b.ts
+++ b/b.ts
@@ -1,6 +1,5 @@
-<<<<<<< ours
 \tavatarPath?: string;
-=======
-\tinitials: string;
->>>>>>> theirs
+\tinitials: string;
";

    #[test]
    fn a_remerge_diff_is_read_file_by_file_with_the_merge_numbering() {
        let files = parse(REMERGE);
        assert_eq!(
            files.iter().map(|file| file.path.as_str()).collect::<Vec<_>>(),
            vec!["src/a.rs", "b.ts"]
        );
        let numbered: Vec<(char, Option<u32>)> =
            files[0].lines.iter().map(|line| (line.origin, line.number)).collect();
        assert_eq!(numbered[0], (' ', Some(3)));
        assert_eq!(numbered[6], ('+', Some(4)));
        assert_eq!(numbered[7], ('+', Some(5)));
        assert_eq!(numbered[9], ('+', Some(7)));
    }

    #[test]
    fn a_conflict_is_the_lines_written_for_it_and_what_each_side_had() {
        let files = parse(REMERGE);
        let found = regions(&files[0]);
        assert_eq!(
            found,
            vec![
                Region {
                    lines: vec![4, 5],
                    first: vec!["    let dir = base.join(\"avatars\");".into()],
                    second: vec!["    let dir = base.or_else(xdg);".into()],
                },
                // Added away from any conflict: a place with no sides.
                Region {
                    lines: vec![7],
                    first: vec![],
                    second: vec![],
                },
            ]
        );

        // A side kept as it was is as much the resolution as a line added.
        let kept = regions(&files[1]);
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].lines, vec![1, 2]);
        assert_eq!(kept[0].first, vec!["\tavatarPath?: string;".to_string()]);
        assert_eq!(kept[0].second, vec!["\tinitials: string;".to_string()]);
    }

    #[test]
    fn a_three_way_marker_keeps_the_base_out_of_both_sides() {
        let text = "diff --git a/x b/x\n--- a/x\n+++ b/x\n@@ -1,7 +1,1 @@\n-<<<<<<< ours\n-A\n-||||||| base\n-O\n-=======\n-B\n->>>>>>> theirs\n+AB\n";
        let found = regions(&parse(text)[0]);
        assert_eq!(found.len(), 1);
        assert_eq!((found[0].first.clone(), found[0].second.clone()), (vec!["A".to_string()], vec!["B".to_string()]));
        assert_eq!(found[0].lines, vec![1]);
    }

    /// Stage everything, new files too, and commit.
    fn save(fixture: &Fixture, message: &str) -> String {
        fixture.git(&["add", "-A"]);
        fixture.commit(message)
    }

    /// Run git where failing is the point.
    fn try_git(fixture: &Fixture, args: &[&str]) -> bool {
        std::process::Command::new("git")
            .current_dir(fixture.at(""))
            .args(args)
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("HOME", fixture.at(""))
            .output()
            .expect("git runs")
            .status
            .success()
    }

    /// `main` and `feature` both change line 5 of `a.rs` and add a field to
    /// `b.ts`; `feature` merges `main`, resolving both, then adds a line at
    /// the top of `a.rs`.
    fn merged_with_a_fix() -> Fixture {
        let fixture = Fixture::empty();
        let lines: Vec<String> = (1..=10).map(|n| format!("line {n}")).collect();
        fixture.write("a.rs", &(lines.join("\n") + "\n"));
        fixture.write("b.ts", "interface A {\n\tname: string;\n}\n");
        save(&fixture, "base");

        fixture.git(&["checkout", "-q", "-b", "feature"]);
        fixture.write("a.rs", &(lines.join("\n").replace("line 5", "branch five") + "\n"));
        fixture.write("b.ts", "interface A {\n\tname: string;\n\tx: number;\n}\n");
        save(&fixture, "feature work");

        fixture.git(&["checkout", "-q", "main"]);
        fixture.write("a.rs", &(lines.join("\n").replace("line 5", "main five") + "\n"));
        fixture.write("b.ts", "interface A {\n\tname: string;\n\ty: number;\n}\n");
        save(&fixture, "main work");

        fixture.git(&["checkout", "-q", "feature"]);
        assert!(!try_git(&fixture, &["merge", "-q", "main"]), "the merge conflicts");
        fixture.write("a.rs", &(lines.join("\n").replace("line 5", "merged five") + "\n"));
        fixture.write("b.ts", "interface A {\n\tname: string;\n\tx: number;\n\ty: number;\n}\n");
        fixture.git(&["add", "-A"]);
        fixture.git(&["commit", "-q", "--no-edit"]);

        fixture.write("a.rs", &format!("top\n{}\n", lines.join("\n").replace("line 5", "merged five")));
        save(&fixture, "a line on top");
        fixture
    }

    #[test]
    fn a_merge_s_resolution_is_found_and_followed_to_the_head() {
        let fixture = merged_with_a_fix();
        let repo = fixture.open();
        let head = fixture.rev("feature");
        let target = fixture.rev("main");
        let base = repo
            .merge_base(id(&head).unwrap(), id(&target).unwrap())
            .unwrap()
            .to_string();
        assert_eq!(base, target, "main is in the branch now");

        let found = conflicts(&repo, &fixture.at(""), &base, &head, &target).unwrap();
        assert_eq!(found.merges, vec![short_id(&id(&fixture.rev("feature~1")).unwrap())]);
        assert!(!found.truncated);

        let a = found.fixes.iter().find(|fix| fix.path == "a.rs").expect("a.rs");
        // Line 5 in the merge, line 6 at the head under the new top line.
        assert_eq!(a.lines, vec![6]);
        assert_eq!(a.main_side, vec!["main five".to_string()]);
        assert_eq!(a.branch_side, vec!["branch five".to_string()]);
        assert_eq!(a.summary, "Merge branch 'main' into feature");

        // The field the branch added is in the resolution: from main's side,
        // the pull request adds only what the merge wrote.
        let b = found.fixes.iter().find(|fix| fix.path == "b.ts").expect("b.ts");
        assert_eq!(b.lines, vec![3]);

        assert_eq!(
            found.files,
            vec![
                FixedFile { path: "a.rs".into(), author: true },
                FixedFile { path: "b.ts".into(), author: false },
            ]
        );
    }

    #[test]
    fn a_pull_request_with_no_merges_has_no_fixes() {
        let fixture = Fixture::empty();
        fixture.write("a", "1\n");
        let base = save(&fixture, "base");
        fixture.write("a", "2\n");
        let head = save(&fixture, "change");
        let found = conflicts(&fixture.open(), &fixture.at(""), &base, &head, &base).unwrap();
        assert_eq!(found, ConflictFixes::default());
    }
}
