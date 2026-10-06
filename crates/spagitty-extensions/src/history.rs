// SPDX-License-Identifier: GPL-3.0-or-later

//! Review history, kept with the repository.
//!
//! One file per extension under `.spagitty/extensions/reviews/` in the main
//! checkout, so every worktree of a repository shares one history and deleting
//! the repository deletes it.
//!
//! **Minimal records.** A record holds what identifies the review — the
//! provider and its version, the snapshot, the times — and what it concluded:
//! status, completeness, summary, findings and what the person did with each.
//! It never holds a patch or a file's content. Provider text passes through
//! [`crate::redact::redact`] before it is written.
//!
//! **Retention.** The newest [`KEEP`] reviews per extension per repository;
//! older ones fall off as new ones arrive. The findings panel can delete the
//! lot, and uninstalling an extension offers to.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::redact::redact;
use crate::review::{Disposition, GateDecision, ReviewResult, ReviewSnapshot};
use crate::storage::{repository_dir, write_atomic};
use crate::{Error, Result};

pub const KEEP: usize = 50;
pub const SCHEMA_VERSION: u32 = 1;

/// Who asked for the review.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Requester {
    Person,
    Farm,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewRecord {
    pub schema_version: u32,
    pub extension: String,
    pub extension_version: String,
    pub provider: String,
    pub requester: Requester,
    pub snapshot: ReviewSnapshot,
    pub result: ReviewResult,
    /// The gate as it was decided when the review finished, for display. The
    /// farm decides again at merge time against the code as it is then.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gate: Option<GateDecision>,
    pub created_ms: u64,
}

static WRITES: Mutex<()> = Mutex::new(());

fn file(main_workdir: &Path, extension: &str) -> PathBuf {
    repository_dir(main_workdir)
        .join("reviews")
        .join(format!("{extension}.json"))
}

/// Every kept review for `extension`, oldest first.
pub fn load(main_workdir: &Path, extension: &str) -> Vec<ReviewRecord> {
    std::fs::read_to_string(file(main_workdir, extension))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or_default()
}

fn save(main_workdir: &Path, extension: &str, records: &[ReviewRecord]) -> Result<()> {
    let bytes = serde_json::to_vec_pretty(records).map_err(|e| Error::Io(e.to_string()))?;
    write_atomic(&file(main_workdir, extension), &bytes)
}

/// Redact every piece of provider text in a record.
fn scrub(record: &mut ReviewRecord) {
    record.result.summary = redact(&record.result.summary);
    for finding in &mut record.result.findings {
        finding.title = redact(&finding.title);
        finding.message = redact(&finding.message);
        finding.suggestion = finding.suggestion.as_deref().map(redact);
    }
    if let Some(action) = &mut record.result.action_required {
        action.message = redact(&action.message);
    }
}

/// Add a review, or replace the one with the same id. Keeps the newest
/// [`KEEP`].
pub fn put(main_workdir: &Path, mut record: ReviewRecord) -> Result<()> {
    let _guard = WRITES.lock().expect("history lock");
    scrub(&mut record);
    let mut records = load(main_workdir, &record.extension);
    records.retain(|held| held.result.review_id != record.result.review_id);
    records.push(record.clone());
    if records.len() > KEEP {
        let drop = records.len() - KEEP;
        records.drain(..drop);
    }
    save(main_workdir, &record.extension, &records)
}

pub fn find(main_workdir: &Path, extension: &str, review: &str) -> Option<ReviewRecord> {
    load(main_workdir, extension)
        .into_iter()
        .find(|record| record.result.review_id == review)
}

/// Record what a person did with one finding. Returns the updated record.
pub fn set_disposition(
    main_workdir: &Path,
    extension: &str,
    review: &str,
    finding: &str,
    disposition: Disposition,
) -> Result<ReviewRecord> {
    let _guard = WRITES.lock().expect("history lock");
    let mut records = load(main_workdir, extension);
    let record = records
        .iter_mut()
        .find(|record| record.result.review_id == review)
        .ok_or_else(|| Error::Refused("That review is no longer in the history.".into()))?;
    let target = record
        .result
        .findings
        .iter_mut()
        .find(|f| f.id == finding)
        .ok_or_else(|| Error::Refused("That finding is not in the review.".into()))?;
    target.disposition = disposition;
    let updated = record.clone();
    save(main_workdir, extension, &records)?;
    Ok(updated)
}

/// Delete one review, or all of them when `review` is `None`.
pub fn delete(main_workdir: &Path, extension: &str, review: Option<&str>) -> Result<()> {
    let _guard = WRITES.lock().expect("history lock");
    match review {
        Some(review) => {
            let mut records = load(main_workdir, extension);
            records.retain(|record| record.result.review_id != review);
            save(main_workdir, extension, &records)
        }
        None => match std::fs::remove_file(file(main_workdir, extension)) {
            Ok(()) => Ok(()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
            Err(error) => Err(error.into()),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manifest::{ReviewScope, ReviewTarget};
    use crate::review::*;

    pub fn record(id: &str, message: &str) -> ReviewRecord {
        let snapshot = ReviewSnapshot {
            id: format!("snap-{id}"),
            repository_id: "repo".into(),
            target: ReviewTarget::WorkingCopy,
            task_id: None,
            pull_request_number: None,
            base_commit: "a".repeat(40),
            head_commit: "b".repeat(40),
            base_ref: Some("main".into()),
            scope: ReviewScope::Uncommitted,
            content_digest: format!("sha256:{}", "0".repeat(64)),
            configuration_digest: None,
            taken_at: String::new(),
        };
        ReviewRecord {
            schema_version: SCHEMA_VERSION,
            extension: "a.b".into(),
            extension_version: "1.0.0".into(),
            provider: "review".into(),
            requester: Requester::Person,
            result: ReviewResult {
                review_id: id.into(),
                provider_id: "review".into(),
                provider_version: "0.8.1".into(),
                snapshot_id: snapshot.id.clone(),
                status: RunStatus::Completed,
                summary: message.into(),
                findings: vec![ReviewFinding {
                    id: "f1".into(),
                    review_id: id.into(),
                    provider_id: "review".into(),
                    severity: Severity::High,
                    provider_severity: Some("major".into()),
                    path: Some("src/x.rs".into()),
                    start_line: None,
                    end_line: None,
                    side: None,
                    title: "t".into(),
                    message: message.into(),
                    suggestion: None,
                    source_url: None,
                    group: None,
                    disposition: Disposition::Open,
                }],
                completeness: Completeness::Complete,
                started_at: String::new(),
                finished_at: None,
                action_required: None,
            },
            snapshot,
            gate: None,
            created_ms: 0,
        }
    }

    #[test]
    fn a_review_is_kept_redacted_and_its_dispositions_persist() {
        let fixture = spagitty_core::fixture::Fixture::woven();
        put(fixture.path(), record("rv1", "leaked Bearer abcdef")).unwrap();
        let kept = load(fixture.path(), "a.b");
        assert_eq!(kept.len(), 1);
        assert!(!kept[0].result.summary.contains("abcdef"));
        assert!(!kept[0].result.findings[0].message.contains("abcdef"));

        let updated =
            set_disposition(fixture.path(), "a.b", "rv1", "f1", Disposition::Dismissed).unwrap();
        assert_eq!(
            updated.result.findings[0].disposition,
            Disposition::Dismissed
        );
        assert_eq!(
            find(fixture.path(), "a.b", "rv1").unwrap().result.findings[0].disposition,
            Disposition::Dismissed
        );
        assert!(
            set_disposition(fixture.path(), "a.b", "rv1", "nope", Disposition::Dismissed).is_err()
        );
    }

    #[test]
    fn only_the_newest_reviews_are_kept() {
        let fixture = spagitty_core::fixture::Fixture::woven();
        for i in 0..(KEEP + 5) {
            put(fixture.path(), record(&format!("rv{i}"), "m")).unwrap();
        }
        let kept = load(fixture.path(), "a.b");
        assert_eq!(kept.len(), KEEP);
        assert_eq!(kept[0].result.review_id, "rv5");
        put(fixture.path(), record("rv54", "replaced")).unwrap();
        assert_eq!(load(fixture.path(), "a.b").len(), KEEP, "same id replaces");
    }

    #[test]
    fn history_can_be_deleted_one_or_all() {
        let fixture = spagitty_core::fixture::Fixture::woven();
        put(fixture.path(), record("one", "m")).unwrap();
        put(fixture.path(), record("two", "m")).unwrap();
        delete(fixture.path(), "a.b", Some("one")).unwrap();
        assert_eq!(load(fixture.path(), "a.b").len(), 1);
        delete(fixture.path(), "a.b", None).unwrap();
        assert!(load(fixture.path(), "a.b").is_empty());
        delete(fixture.path(), "a.b", None).unwrap();
    }
}
