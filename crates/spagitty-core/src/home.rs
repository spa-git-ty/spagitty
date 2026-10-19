// SPDX-License-Identifier: GPL-3.0-or-later

//! What Home shows that only git knows (FEAT-140).
//!
//! Home's header, its Branches card, the heatmap and the Releases card's
//! changelog count are all local reads of the one repository, so they are one
//! call: one lock, one round trip. Everything here is a read, and every part is
//! optional — a repository with no remote, no tags or no changelog is an
//! ordinary repository, and a part that cannot be read costs that part only.
//!
//! # Five branches, not all of them
//!
//! Ahead and behind are counted for the five branches Home shows, never for
//! every branch: counting a thousand of them is what made Pipelines take half a
//! minute to open (BUG-090).

use std::path::Path;

use gix::ObjectId;
use serde::Serialize;

use crate::branches;
use crate::remotes;
use crate::tags;

/// How many branches the card shows.
pub const RECENT_BRANCHES: usize = 5;

/// The heatmap's reach: sixteen weeks.
pub const HEATMAP_DAYS: i64 = 16 * 7;

/// A ceiling on the commit times returned, so a monorepo's sixteen weeks do
/// not become a megabyte across the bridge. Past it the busiest days are
/// already at the top step.
const HEATMAP_MAX: usize = 20_000;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Overview {
    /// The fetch remote's address, as a person reads it: `github.com/owner/name`.
    pub remote: Option<String>,
    /// The licence's SPDX identifier, read from the licence file's words.
    pub licence: Option<String>,
    /// The latest version tag.
    pub version: Option<String>,
    /// When that tag's commit was made, unix seconds.
    pub version_time: Option<i64>,
    /// The checked-out branch against its upstream.
    pub upstream: Option<branches::Divergence>,
    /// The branch the others are measured against.
    pub default_branch: Option<String>,
    /// The most recently touched local branches, newest first.
    pub branches: Vec<RecentBranch>,
    /// How many local branches there are in all.
    pub branch_count: usize,
    /// Commit times on `HEAD`'s history within the heatmap's reach, unix
    /// seconds. Bucketed by the screen, which knows the person's time zone.
    pub commit_times: Vec<i64>,
    /// What `## [Unreleased]` in `CHANGELOG.md` holds.
    pub unreleased: Option<Unreleased>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecentBranch {
    pub name: String,
    pub author_name: String,
    /// The tip's author time, unix seconds.
    pub time: i64,
    /// Against the default branch, or against its own upstream for the default
    /// branch itself. `None` when there is nothing to measure against.
    pub ahead: Option<usize>,
    pub behind: Option<usize>,
    pub current: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Unreleased {
    pub total: usize,
    /// `(heading, entries)` in the file's order: `("Added", 8)`.
    pub sections: Vec<(String, usize)>,
}

/// Read the overview. `now` is unix seconds, passed in so tests can fix it.
pub fn overview(repo: &gix::Repository, now: i64) -> Overview {
    let default_branch = default_branch(repo);
    let (branches, branch_count) = recent(repo, default_branch.as_deref());
    let latest = latest_version(repo);
    Overview {
        remote: remote_address(repo),
        licence: repo.workdir().and_then(licence),
        version: latest.as_ref().map(|(name, _)| name.clone()),
        version_time: latest.map(|(_, time)| time),
        upstream: branches::current_divergence(repo),
        default_branch,
        branches,
        branch_count,
        commit_times: commit_times(repo, now - HEATMAP_DAYS * 24 * 60 * 60),
        unreleased: repo
            .workdir()
            .and_then(|dir| std::fs::read_to_string(dir.join("CHANGELOG.md")).ok())
            .and_then(|text| unreleased(&text)),
    }
}

/// `origin`'s address, or the first remote's.
fn remote_address(repo: &gix::Repository) -> Option<String> {
    let all = remotes::remotes(repo);
    let remote = all
        .iter()
        .find(|remote| remote.name == "origin")
        .or_else(|| all.first())?;
    Some(readable_address(&remote.url))
}

/// `git@github.com:owner/name.git` and `https://user@github.com/owner/name`
/// both read `github.com/owner/name`.
pub fn readable_address(url: &str) -> String {
    let mut rest = url.trim();
    if let Some((_, after)) = rest.split_once("://") {
        rest = after;
    } else if let Some((host, path)) = rest.split_once(':') {
        // The scp form, `user@host:path`. A Windows path such as `C:\repo`
        // has a one-letter host, and is left as it is.
        if host.len() > 1 && !path.starts_with('\\') {
            let host = host.rsplit('@').next().unwrap_or(host);
            return format!(
                "{host}/{}",
                path.trim_end_matches(".git").trim_end_matches('/')
            );
        }
        return rest.to_string();
    }
    let rest = rest.rsplit_once('@').map(|(_, host)| host).unwrap_or(rest);
    rest.trim_end_matches('/')
        .trim_end_matches(".git")
        .to_string()
}

/// The branch the others are measured against: what `origin/HEAD` points at,
/// else `main`, else `master`.
fn default_branch(repo: &gix::Repository) -> Option<String> {
    if let Ok(reference) = repo.find_reference("refs/remotes/origin/HEAD") {
        if let Some(target) = reference.target().try_name() {
            let full = target.as_bstr().to_string();
            if let Some(short) = full.strip_prefix("refs/remotes/origin/") {
                return Some(short.to_string());
            }
        }
    }
    ["main", "master"]
        .into_iter()
        .find(|name| repo.find_reference(&format!("refs/heads/{name}")).is_ok())
        .map(str::to_string)
}

fn tip(repo: &gix::Repository, full: &str) -> Option<ObjectId> {
    let mut reference = repo.find_reference(full).ok()?;
    reference.peel_to_id().ok().map(|id| id.detach())
}

/// The five most recently touched local branches, and how many there are.
fn recent(repo: &gix::Repository, default: Option<&str>) -> (Vec<RecentBranch>, usize) {
    let current = repo
        .head_ref()
        .ok()
        .flatten()
        .map(|head| head.name().shorten().to_string());

    let Ok(platform) = repo.references() else {
        return (Vec::new(), 0);
    };
    let Ok(iter) = platform.prefixed("refs/heads/") else {
        return (Vec::new(), 0);
    };

    let mut all: Vec<(String, ObjectId, String, i64)> = Vec::new();
    for mut reference in iter.flatten() {
        let name = reference.name().shorten().to_string();
        let Ok(id) = reference.peel_to_id() else {
            continue;
        };
        let id = id.detach();
        let Ok(commit) = repo.find_commit(id) else {
            continue;
        };
        let (author, time) = match commit.author() {
            Ok(sig) => (
                sig.name.to_string(),
                sig.time().map(|t| t.seconds).unwrap_or(0),
            ),
            Err(_) => (String::new(), 0),
        };
        all.push((name, id, author, time));
    }
    let count = all.len();
    all.sort_by(|a, b| b.3.cmp(&a.3).then(a.0.cmp(&b.0)));

    // The default branch measured against: the remote's copy when there is
    // one, since that is what a pull request is opened against.
    let base = default.and_then(|name| {
        tip(repo, &format!("refs/remotes/origin/{name}"))
            .or_else(|| tip(repo, &format!("refs/heads/{name}")))
    });

    let rows = all
        .into_iter()
        .take(RECENT_BRANCHES)
        .map(|(name, id, author_name, time)| {
            let against = if Some(name.as_str()) == default {
                tip(repo, &format!("refs/remotes/origin/{name}"))
            } else {
                base
            };
            let (ahead, behind) = match against {
                Some(other) => {
                    let (a, b) = branches::ahead_behind(repo, id, other);
                    (Some(a), Some(b))
                }
                None => (None, None),
            };
            RecentBranch {
                current: current.as_deref() == Some(name.as_str()),
                name,
                author_name,
                time,
                ahead,
                behind,
            }
        })
        .collect();
    (rows, count)
}

/// `v3.1.1` → `(3, 1, 1)`. Anything else is not a version.
fn version_of(name: &str) -> Option<(u64, u64, u64)> {
    let bare = name.strip_prefix('v').unwrap_or(name);
    let core = bare.split(['-', '+']).next()?;
    let mut parts = core.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next().unwrap_or("0").parse().ok()?;
    let patch = parts.next().unwrap_or("0").parse().ok()?;
    if parts.next().is_some() || bare.contains('-') {
        // A pre-release is not the latest version, and four parts is not one.
        return None;
    }
    Some((major, minor, patch))
}

/// The highest version tag, else the newest tag.
fn latest_version(repo: &gix::Repository) -> Option<(String, i64)> {
    let all = tags::tags(repo).ok()?;
    all.iter()
        .filter_map(|tag| version_of(&tag.name).map(|v| (v, tag)))
        .max_by_key(|(v, _)| *v)
        .map(|(_, tag)| (tag.name.clone(), tag.time))
        .or_else(|| all.first().map(|tag| (tag.name.clone(), tag.time)))
}

/// Commit times on `HEAD`'s history since `since`, newest first.
fn commit_times(repo: &gix::Repository, since: i64) -> Vec<i64> {
    let Ok(head) = repo.head_id() else {
        return Vec::new();
    };
    let Ok(walk) = repo
        .rev_walk([head.detach()])
        .sorting(gix::revision::walk::Sorting::ByCommitTime(
            Default::default(),
        ))
        .all()
    else {
        return Vec::new();
    };
    let mut times = Vec::new();
    for info in walk.filter_map(Result::ok) {
        let Some(time) = info.commit_time else {
            continue;
        };
        // Newest first, so the first commit older than the window ends it.
        if time < since {
            break;
        }
        times.push(time);
        if times.len() >= HEATMAP_MAX {
            break;
        }
    }
    times
}

const LICENCE_FILES: [&str; 6] = [
    "LICENSE",
    "LICENSE.md",
    "LICENSE.txt",
    "LICENCE",
    "LICENCE.md",
    "COPYING",
];

/// The licence's SPDX identifier, from the first words of the licence file.
fn licence(dir: &Path) -> Option<String> {
    let text = LICENCE_FILES
        .iter()
        .find_map(|name| std::fs::read_to_string(dir.join(name)).ok())?;
    Some(identify_licence(&text).unwrap_or("Other").to_string())
}

/// Which licence a licence file is, from the sentences each one opens with.
pub fn identify_licence(text: &str) -> Option<&'static str> {
    let head: String = text.chars().take(3000).collect::<String>().to_lowercase();
    let has = |words: &str| head.contains(words);
    if has("gnu affero general public license") {
        return Some("AGPL-3.0");
    }
    if has("gnu lesser general public license") {
        return Some(if has("version 2.1") {
            "LGPL-2.1"
        } else {
            "LGPL-3.0"
        });
    }
    if has("gnu general public license") {
        return Some(if has("version 2") && !has("version 3") {
            "GPL-2.0"
        } else {
            "GPL-3.0"
        });
    }
    if has("apache license") {
        return Some("Apache-2.0");
    }
    if has("mozilla public license") {
        return Some("MPL-2.0");
    }
    if has("permission is hereby granted, free of charge") {
        return Some("MIT");
    }
    if has("this is free and unencumbered software") {
        return Some("Unlicense");
    }
    if has("permission to use, copy, modify, and/or distribute") {
        return Some("ISC");
    }
    if has("redistribution and use in source and binary forms") {
        return Some(if has("neither the name") {
            "BSD-3-Clause"
        } else {
            "BSD-2-Clause"
        });
    }
    None
}

/// The entries under `## [Unreleased]`, by `###` heading.
///
/// An entry is a top-level list item. Continuation lines and nested items are
/// part of the entry above them, as they are in the rendered file.
pub fn unreleased(text: &str) -> Option<Unreleased> {
    let mut inside = false;
    let mut found = false;
    let mut sections: Vec<(String, usize)> = Vec::new();
    let mut total = 0;
    for line in text.lines() {
        let trimmed = line.trim_end();
        if let Some(heading) = trimmed.strip_prefix("## ") {
            if inside {
                break;
            }
            let name = heading.trim().trim_matches(['[', ']']).to_lowercase();
            if name.starts_with("unreleased") {
                inside = true;
                found = true;
            }
            continue;
        }
        if !inside {
            continue;
        }
        if let Some(heading) = trimmed.strip_prefix("### ") {
            sections.push((heading.trim().to_string(), 0));
            continue;
        }
        if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            total += 1;
            match sections.last_mut() {
                Some((_, count)) => *count += 1,
                None => sections.push((String::new(), 1)),
            }
        }
    }
    found.then(|| Unreleased {
        total,
        sections: sections
            .into_iter()
            .filter(|(_, count)| *count > 0)
            .collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::Fixture;

    #[test]
    fn an_address_reads_as_host_and_path() {
        assert_eq!(
            readable_address("git@github.com:spa-git-ty/spagitty.git"),
            "github.com/spa-git-ty/spagitty"
        );
        assert_eq!(
            readable_address("https://me@gitlab.example.com/a/b.git"),
            "gitlab.example.com/a/b"
        );
        assert_eq!(
            readable_address("https://github.com/a/b/"),
            "github.com/a/b"
        );
        assert_eq!(
            readable_address("ssh://git@host:2222/a/b.git"),
            "host:2222/a/b"
        );
        assert_eq!(
            readable_address("C:\\repos\\upstream"),
            "C:\\repos\\upstream"
        );
    }

    #[test]
    fn a_licence_is_known_by_its_opening_words() {
        assert_eq!(identify_licence("                    GNU GENERAL PUBLIC LICENSE\n                       Version 3, 29 June 2007"), Some("GPL-3.0"));
        assert_eq!(
            identify_licence(
                "MIT License\n\nPermission is hereby granted, free of charge, to any person"
            ),
            Some("MIT")
        );
        assert_eq!(
            identify_licence("Apache License\nVersion 2.0, January 2004"),
            Some("Apache-2.0")
        );
        assert_eq!(
            identify_licence(
                "Redistribution and use in source and binary forms ... Neither the name"
            ),
            Some("BSD-3-Clause")
        );
        assert_eq!(identify_licence("All rights reserved."), None);
    }

    #[test]
    fn the_unreleased_entries_are_counted_by_heading() {
        let text = "# Changelog\n\n## [Unreleased]\n\n### Added\n\n- **Home** is new.\n  It continues here.\n- The bell.\n  - a nested item\n\n### Fixed\n\n- One.\n\n### Removed\n\n## [3.1.1] - 2026-10-18\n\n### Fixed\n\n- Not counted.\n";
        let found = unreleased(text).expect("a section");
        assert_eq!(found.total, 3);
        assert_eq!(
            found.sections,
            vec![("Added".to_string(), 2), ("Fixed".to_string(), 1)]
        );
    }

    #[test]
    fn no_unreleased_heading_is_no_count_rather_than_zero() {
        assert_eq!(unreleased("# Changelog\n\n## [1.0.0]\n\n- x\n"), None);
        assert_eq!(unreleased("## Unreleased\n").map(|u| u.total), Some(0));
    }

    #[test]
    fn a_version_tag_beats_a_newer_tag_that_is_not_one() {
        assert_eq!(version_of("v3.1.1"), Some((3, 1, 1)));
        assert_eq!(version_of("2.0"), Some((2, 0, 0)));
        assert_eq!(version_of("v3.2.0-rc.1"), None);
        assert_eq!(version_of("nightly"), None);
    }

    #[test]
    fn the_overview_reads_a_real_repository() {
        let fixture = Fixture::woven();
        fixture.write(
            "LICENSE",
            "MIT License\n\nPermission is hereby granted, free of charge, to any person\n",
        );
        fixture.write("CHANGELOG.md", "## [Unreleased]\n\n### Added\n\n- One.\n");
        fixture.git(&[
            "remote",
            "add",
            "origin",
            "git@github.com:spa-git-ty/spagitty.git",
        ]);
        let repo = fixture.open();

        let found = overview(&repo, 1_700_000_000 + 3600);
        assert_eq!(
            found.remote.as_deref(),
            Some("github.com/spa-git-ty/spagitty")
        );
        assert_eq!(found.licence.as_deref(), Some("MIT"));
        assert_eq!(found.version.as_deref(), Some("v0.2.0"));
        assert_eq!(found.default_branch.as_deref(), Some("main"));
        assert_eq!(found.unreleased.map(|u| u.total), Some(1));
        // main, feature/split-view and merged/already-in-main.
        assert_eq!(found.branch_count, 3);
        let names: Vec<_> = found.branches.iter().map(|b| b.name.as_str()).collect();
        assert_eq!(names[0], "main");
        let feature = found
            .branches
            .iter()
            .find(|b| b.name == "feature/split-view")
            .unwrap();
        // Merged into main: nothing ahead, and main's own two commits behind.
        assert_eq!((feature.ahead, feature.behind), (Some(0), Some(2)));
        let merged = found
            .branches
            .iter()
            .find(|b| b.name == "merged/already-in-main")
            .unwrap();
        assert_eq!(merged.ahead, Some(0));
        // Every commit of the woven history is inside sixteen weeks of its end.
        assert_eq!(found.commit_times.len(), 6);
    }

    #[test]
    fn an_empty_repository_has_an_overview_of_nothing() {
        let fixture = Fixture::empty();
        let found = overview(&fixture.open(), 0);
        assert!(found.remote.is_none());
        assert!(found.version.is_none());
        assert!(found.branches.is_empty());
        assert!(found.commit_times.is_empty());
    }
}
