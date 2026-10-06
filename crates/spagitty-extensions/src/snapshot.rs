// SPDX-License-Identifier: GPL-3.0-or-later

//! What a review covers, decided by the host before anything runs.
//!
//! A [`ReviewSnapshot`] pins both ends of the comparison to commits and hashes
//! the content in scope, so that "is this result still about the code in front
//! of me" is a comparison of two digests rather than a guess.
//!
//! - **Committed** scope: the digest is the base and head commit ids. A commit
//!   id already names its content exactly.
//! - **Working-copy** scopes: the digest adds the index file's own checksum
//!   and the bytes of every file in scope as they are on disk now. A file
//!   edited, staged or unstaged during a review changes it.
//!
//! The provider's configuration files (`configurationFiles` in its manifest)
//! are hashed separately — from the head commit for a committed review, from
//! disk otherwise — so changing them also makes a result stale.

use std::path::Path;

use serde::Serialize;
use sha2::{Digest, Sha256};
use spagitty_core::diff::FileStatus;

use crate::manifest::{ReviewScope, ReviewTarget};
use crate::review::ReviewSnapshot;
use crate::{Error, Result};

/// The most files a preview lists.
pub const PREVIEW_LIMIT: usize = 500;

/// What the user asked to have reviewed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request {
    pub target: ReviewTarget,
    pub scope: ReviewScope,
    /// A branch, tag or commit to compare against. Required for the committed
    /// and tracked scopes; the uncommitted scopes compare against `HEAD`.
    pub base: Option<String>,
    pub task_id: Option<String>,
    pub pull_request_number: Option<u64>,
}

/// Where a file in scope comes from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Origin {
    Committed,
    Staged,
    Unstaged,
    Untracked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScopeFile {
    pub path: String,
    pub status: FileStatus,
    pub origin: Origin,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Excluded {
    pub path: String,
    pub reason: String,
}

/// What the person is shown before the first upload: the base, the scope,
/// the files, and what was left out and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Preview {
    pub base_ref: Option<String>,
    pub base_commit: String,
    pub head_commit: String,
    pub scope: ReviewScope,
    pub files: Vec<ScopeFile>,
    pub excluded: Vec<Excluded>,
    pub truncated: bool,
}

/// A stable identity for a repository, shared by all its checkouts: a hash of
/// its common git directory's canonical path. Opaque to workers.
pub fn repository_id(repo: &gix::Repository) -> String {
    let common = spagitty_core::compare::common_dir(repo);
    let digest = Sha256::digest(common.to_string_lossy().as_bytes());
    let hex: String = digest.iter().take(12).map(|b| format!("{b:02x}")).collect();
    format!("repo-{hex}")
}

fn hex(digest: impl AsRef<[u8]>) -> String {
    digest.as_ref().iter().map(|b| format!("{b:02x}")).collect()
}

fn empty_tree_commitless() -> Error {
    Error::Refused(
        "This repository has no commits yet, so there is nothing to compare against.".into(),
    )
}

/// Take a snapshot of `workdir` for `request`.
pub fn take(
    workdir: &Path,
    request: &Request,
    configuration_files: &[String],
) -> Result<(ReviewSnapshot, Preview)> {
    let repo = spagitty_core::repo::open(workdir)?;
    let head = spagitty_core::repo::head(&repo)
        .id
        .ok_or_else(empty_tree_commitless)?;

    let (base_ref, base_commit) = match request.scope {
        // What is not committed is compared with HEAD, untracked files or not.
        ReviewScope::Uncommitted | ReviewScope::IncludeUntracked => {
            (Some("HEAD".to_string()), head.clone())
        }
        ReviewScope::Committed | ReviewScope::Tracked => {
            let base = request.base.as_deref().ok_or_else(|| {
                Error::Refused("Choose the branch or commit to compare against.".into())
            })?;
            let tip = spagitty_core::compare::resolve_commit(&repo, base)?;
            let merge_base =
                spagitty_core::compare::merge_base(&repo, &head, &tip)?.ok_or_else(|| {
                    Error::Refused(format!("{base} and HEAD have no history in common."))
                })?;
            (Some(base.to_string()), merge_base)
        }
    };

    let mut files = Vec::new();
    let mut excluded = Vec::new();
    let mut digest = Sha256::new();
    digest.update(format!(
        "scope:{:?}\nbase:{base_commit}\nhead:{head}\n",
        request.scope
    ));

    let committed = matches!(request.scope, ReviewScope::Committed | ReviewScope::Tracked);
    if committed && base_commit != head {
        for file in spagitty_core::diff::range_files(&repo, &base_commit, &head)? {
            files.push(ScopeFile {
                path: file.path,
                status: file.status,
                origin: Origin::Committed,
            });
        }
    }

    if request.scope != ReviewScope::Committed {
        let status = spagitty_core::status::working_copy(&repo)?;
        if !status.conflicted.is_empty() {
            return Err(Error::Refused(
                "Resolve the conflicted files before asking for a review.".into(),
            ));
        }
        // The index's own checksum: staging or unstaging anything changes it.
        if let Ok(index) = std::fs::read(repo.index_path()) {
            let tail = &index[index.len().saturating_sub(32)..];
            digest.update(b"index:");
            digest.update(hex(Sha256::digest(tail)));
            digest.update(b"\n");
        }
        for entry in &status.staged {
            files.push(ScopeFile {
                path: entry.path.clone(),
                status: entry.status,
                origin: Origin::Staged,
            });
        }
        for entry in &status.unstaged {
            if entry.status == FileStatus::Untracked {
                if request.scope == ReviewScope::IncludeUntracked {
                    files.push(ScopeFile {
                        path: entry.path.clone(),
                        status: entry.status,
                        origin: Origin::Untracked,
                    });
                } else {
                    excluded.push(Excluded {
                        path: entry.path.clone(),
                        reason: "not added to Git; choose “include untracked files” to review it"
                            .into(),
                    });
                }
            } else {
                files.push(ScopeFile {
                    path: entry.path.clone(),
                    status: entry.status,
                    origin: Origin::Unstaged,
                });
            }
        }
        // The bytes of every working-copy file in scope, as they are now.
        let mut paths: Vec<&str> = files
            .iter()
            .filter(|f| f.origin != Origin::Committed)
            .map(|f| f.path.as_str())
            .collect();
        paths.sort_unstable();
        paths.dedup();
        for path in paths {
            digest.update(format!("file:{path}\n"));
            match std::fs::read(workdir.join(path)) {
                Ok(bytes) => digest.update(hex(Sha256::digest(&bytes))),
                Err(_) => digest.update(b"absent"),
            }
            digest.update(b"\n");
        }
    }

    let configuration_digest =
        configuration(&repo, workdir, request.scope, &head, configuration_files);
    let content_digest = format!("sha256:{}", hex(digest.finalize()));
    let repository_id = repository_id(&repo);
    let id = {
        let mut idd = Sha256::new();
        idd.update(&repository_id);
        idd.update(&content_digest);
        idd.update(configuration_digest.as_deref().unwrap_or(""));
        idd.update(request.task_id.as_deref().unwrap_or(""));
        format!("snap-{}", &hex(idd.finalize())[..20])
    };

    let truncated = files.len() > PREVIEW_LIMIT;
    let preview = Preview {
        base_ref: base_ref.clone(),
        base_commit: base_commit.clone(),
        head_commit: head.clone(),
        scope: request.scope,
        files: files.into_iter().take(PREVIEW_LIMIT).collect(),
        excluded,
        truncated,
    };
    let snapshot = ReviewSnapshot {
        id,
        repository_id,
        target: request.target,
        task_id: request.task_id.clone(),
        pull_request_number: request.pull_request_number,
        base_commit,
        head_commit: head,
        base_ref,
        scope: request.scope,
        content_digest,
        configuration_digest,
        taken_at: now_iso(),
    };
    Ok((snapshot, preview))
}

/// The provider's configuration files, hashed, or `None` when it declares none.
fn configuration(
    repo: &gix::Repository,
    workdir: &Path,
    scope: ReviewScope,
    head: &str,
    files: &[String],
) -> Option<String> {
    if files.is_empty() {
        return None;
    }
    let mut digest = Sha256::new();
    for file in files {
        digest.update(format!("config:{file}\n"));
        let bytes = if scope == ReviewScope::Committed {
            blob_at(repo, head, file)
        } else {
            std::fs::read(workdir.join(file)).ok()
        };
        match bytes {
            Some(bytes) => digest.update(hex(Sha256::digest(&bytes))),
            None => digest.update(b"absent"),
        }
        digest.update(b"\n");
    }
    Some(format!("sha256:{}", hex(digest.finalize())))
}

fn blob_at(repo: &gix::Repository, commit: &str, path: &str) -> Option<Vec<u8>> {
    let id = gix::ObjectId::from_hex(commit.as_bytes()).ok()?;
    let tree = repo.find_commit(id).ok()?.tree().ok()?;
    let entry = tree.lookup_entry_by_path(Path::new(path)).ok()??;
    let object = repo.find_object(entry.object_id()).ok()?;
    Some(object.data.clone())
}

use spagitty_core::gix;

/// The current time as an RFC 3339 UTC timestamp, without a date library.
pub fn now_iso() -> String {
    let millis = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0);
    iso(millis)
}

/// `millis` since the epoch as `YYYY-MM-DDTHH:MM:SS.mmmZ`.
pub fn iso(millis: i64) -> String {
    let seconds = millis.div_euclid(1000);
    let ms = millis.rem_euclid(1000);
    let days = seconds.div_euclid(86_400);
    let secs = seconds.rem_euclid(86_400);
    // Howard Hinnant's civil-from-days.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{ms:03}Z",
        secs / 3600,
        (secs % 3600) / 60,
        secs % 60
    )
}

/// Bases worth offering, best first: the branch's upstream, the remote's
/// default branch, then local `main`, `master`, `develop`, `dev` — each only
/// if it exists and is not the branch itself. Nothing is fetched to find
/// them; a base that is not on disk is not offered.
pub fn suggested_bases(workdir: &Path) -> Result<Vec<String>> {
    let repo = spagitty_core::repo::open(workdir)?;
    let branch = spagitty_core::repo::head(&repo).branch;
    let mut out: Vec<String> = Vec::new();
    let mut push = |name: String| {
        if Some(&name) != branch.as_ref() && !out.contains(&name) {
            out.push(name);
        }
    };
    if let Some(branch) = &branch {
        if let Ok(Some(reference)) = repo
            .find_reference(format!("refs/heads/{branch}").as_str())
            .map(Some)
        {
            if let Some(Ok(upstream)) =
                reference.remote_tracking_ref_name(gix::remote::Direction::Fetch)
            {
                push(upstream.shorten().to_string());
            }
        }
    }
    if let Ok(reference) = repo.find_reference("refs/remotes/origin/HEAD") {
        if let gix::refs::TargetRef::Symbolic(target) = reference.target() {
            push(target.shorten().to_string());
        }
    }
    for name in ["main", "master", "develop", "dev"] {
        if repo
            .find_reference(format!("refs/heads/{name}").as_str())
            .is_ok()
        {
            push(name.to_string());
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use spagitty_core::fixture::Fixture;

    fn request(scope: ReviewScope, base: Option<&str>) -> Request {
        Request {
            target: ReviewTarget::WorkingCopy,
            scope,
            base: base.map(str::to_string),
            task_id: None,
            pull_request_number: None,
        }
    }

    #[test]
    fn an_uncommitted_review_lists_staged_and_unstaged_and_leaves_untracked_out() {
        let fixture = Fixture::dirty();
        let (snapshot, preview) = take(
            fixture.path(),
            &request(ReviewScope::Uncommitted, None),
            &[],
        )
        .unwrap();
        assert_eq!(snapshot.base_commit, snapshot.head_commit);
        assert!(preview.files.iter().any(|f| f.origin == Origin::Staged));
        assert!(preview.files.iter().any(|f| f.origin == Origin::Unstaged));
        assert!(!preview.files.iter().any(|f| f.origin == Origin::Untracked));
        assert!(
            !preview.excluded.is_empty(),
            "untracked files are named as left out"
        );
        assert!(snapshot.content_digest.starts_with("sha256:"));
    }

    #[test]
    fn including_untracked_files_is_a_choice_that_changes_the_scope() {
        let fixture = Fixture::dirty();
        let (_, preview) = take(
            fixture.path(),
            &request(ReviewScope::IncludeUntracked, None),
            &[],
        )
        .unwrap();
        assert!(preview.files.iter().any(|f| f.origin == Origin::Untracked));
        assert!(
            !preview.files.iter().any(|f| f.origin == Origin::Committed),
            "it is the uncommitted scope plus untracked files, as `--uncommitted --include-untracked` is"
        );
        assert!(preview.excluded.is_empty());
    }

    #[test]
    fn editing_a_file_in_scope_changes_the_digest_and_nothing_else_does() {
        let fixture = Fixture::dirty();
        let r = request(ReviewScope::Uncommitted, None);
        let (first, _) = take(fixture.path(), &r, &[]).unwrap();
        let (again, _) = take(fixture.path(), &r, &[]).unwrap();
        assert!(first.same_code(&again));
        assert_eq!(first.id, again.id);

        let wc = spagitty_core::status::working_copy(
            &spagitty_core::repo::open(fixture.path()).unwrap(),
        )
        .unwrap();
        let path = &wc
            .unstaged
            .iter()
            .find(|e| e.status != FileStatus::Untracked)
            .unwrap()
            .path;
        let mut text = std::fs::read_to_string(fixture.path().join(path)).unwrap();
        text.push_str("one more line\n");
        std::fs::write(fixture.path().join(path), text).unwrap();
        let (after, _) = take(fixture.path(), &r, &[]).unwrap();
        assert!(!first.same_code(&after));
    }

    #[test]
    fn staging_a_file_changes_the_digest() {
        let fixture = Fixture::dirty();
        let r = request(ReviewScope::Uncommitted, None);
        let (before, _) = take(fixture.path(), &r, &[]).unwrap();
        let wc = spagitty_core::status::working_copy(
            &spagitty_core::repo::open(fixture.path()).unwrap(),
        )
        .unwrap();
        let path = wc
            .unstaged
            .iter()
            .find(|e| e.status != FileStatus::Untracked)
            .unwrap()
            .path
            .clone();
        fixture.git(&["add", &path]);
        let (after, _) = take(fixture.path(), &r, &[]).unwrap();
        assert!(!before.same_code(&after));
    }

    #[test]
    fn a_committed_review_pins_the_merge_base_and_lists_the_range() {
        let fixture = Fixture::woven();
        fixture.git(&["switch", "-q", "-c", "work"]);
        fixture.write("added.txt", "new\n");
        fixture.git(&["add", "added.txt"]);
        fixture.commit("Add a file");
        let (snapshot, preview) = take(
            fixture.path(),
            &request(ReviewScope::Committed, Some("main")),
            &[],
        )
        .unwrap();
        let repo = spagitty_core::repo::open(fixture.path()).unwrap();
        assert_eq!(
            snapshot.base_commit,
            spagitty_core::compare::resolve_commit(&repo, "main").unwrap()
        );
        assert_eq!(preview.files.len(), 1);
        assert_eq!(preview.files[0].path, "added.txt");
        assert_eq!(preview.files[0].origin, Origin::Committed);
        assert!(take(fixture.path(), &request(ReviewScope::Committed, None), &[]).is_err());
        assert!(take(
            fixture.path(),
            &request(ReviewScope::Committed, Some("no-such")),
            &[]
        )
        .is_err());
    }

    #[test]
    fn configuration_files_are_part_of_the_identity() {
        let fixture = Fixture::woven();
        let r = request(ReviewScope::Uncommitted, None);
        let files = vec![".coderabbit.yaml".to_string()];
        let (absent, _) = take(fixture.path(), &r, &files).unwrap();
        fixture.write(".coderabbit.yaml", "reviews: {}\n");
        let (present, _) = take(
            fixture.path(),
            &request(ReviewScope::Uncommitted, None),
            &files,
        )
        .unwrap();
        assert_ne!(absent.configuration_digest, present.configuration_digest);
        assert!(!absent.same_code(&present));
    }

    #[test]
    fn a_main_checkout_and_its_worktree_are_one_repository() {
        let fixture = Fixture::woven();
        let other = tempfile::tempdir().unwrap();
        let wt = other.path().join("wt");
        fixture.git(&["worktree", "add", "-q", wt.to_str().unwrap(), "-b", "side"]);
        let main = spagitty_core::repo::open(fixture.path()).unwrap();
        let tree = spagitty_core::repo::open(&wt).unwrap();
        assert_eq!(repository_id(&main), repository_id(&tree));
        assert_eq!(
            std::fs::canonicalize(spagitty_core::compare::main_workdir(&tree).unwrap()).unwrap(),
            std::fs::canonicalize(fixture.path()).unwrap()
        );
    }

    #[test]
    fn bases_are_offered_only_when_they_exist_locally() {
        let fixture = Fixture::woven();
        fixture.git(&["switch", "-q", "-c", "work"]);
        let bases = suggested_bases(fixture.path()).unwrap();
        assert_eq!(bases.first().map(String::as_str), Some("main"));
        assert!(!bases.contains(&"work".to_string()));
    }

    #[test]
    fn timestamps_are_rfc_3339() {
        assert_eq!(iso(0), "1970-01-01T00:00:00.000Z");
        assert_eq!(iso(1_791_000_000_123), "2026-10-03T04:00:00.123Z");
        assert!(now_iso().ends_with('Z'));
    }
}
