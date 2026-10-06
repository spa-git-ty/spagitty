// SPDX-License-Identifier: GPL-3.0-or-later

//! The review model every provider shares (proposal §8), and the gate.
//!
//! These are Spagitty's types, not any provider's. A provider maps its own
//! output onto them; `schemas/extensions/review.v1.schema.json` describes them
//! on the wire, and `src/lib/extensions/types.ts` mirrors them by hand, kept in
//! step by a fixture both test suites read.
//!
//! # The gate is the host's
//!
//! A provider reports how its run went and what it found. Whether that is good
//! enough to merge is computed here, from the run's status and completeness,
//! whether the snapshot it reviewed is still the code in front of us, the
//! policy's threshold, and the findings. There is no field a provider can fill
//! in to pass a gate, and "completed" is not "approved".

use serde::{Deserialize, Serialize};

use crate::manifest::{ReviewScope, ReviewTarget};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum RunStatus {
    Queued,
    Running,
    Completed,
    Incomplete,
    Skipped,
    Failed,
    Cancelled,
    Stale,
    ActionRequired,
}

impl RunStatus {
    pub fn is_terminal(self) -> bool {
        !matches!(self, RunStatus::Queued | RunStatus::Running)
    }

    pub fn parse(text: &str) -> Option<RunStatus> {
        serde_json::from_value(serde_json::Value::String(text.to_string())).ok()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Gate {
    NotEvaluated,
    Pass,
    ChangesRequested,
    Blocked,
}

/// How serious a finding is, after the provider's own scale was mapped.
///
/// Ordered: `Critical` is the greatest. `Unknown` sorts below `Info` for
/// display, and is handled separately by the gate, which never lets it pass.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Severity {
    Unknown,
    Info,
    Low,
    Medium,
    High,
    Critical,
}

impl Severity {
    pub fn parse(text: &str) -> Option<Severity> {
        serde_json::from_value(serde_json::Value::String(text.to_string())).ok()
    }

    pub fn label(self) -> &'static str {
        match self {
            Severity::Critical => "Critical",
            Severity::High => "High",
            Severity::Medium => "Medium",
            Severity::Low => "Low",
            Severity::Info => "Info",
            Severity::Unknown => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Disposition {
    #[default]
    Open,
    Acknowledged,
    Dismissed,
    SentToAgent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Completeness {
    Complete,
    Partial,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Side {
    Old,
    New,
}

/// Exactly what a review covers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewSnapshot {
    pub id: String,
    pub repository_id: String,
    pub target: ReviewTarget,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub task_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pull_request_number: Option<u64>,
    pub base_commit: String,
    pub head_commit: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub base_ref: Option<String>,
    pub scope: ReviewScope,
    /// What the reviewed content was: revisions, index state and the bytes of
    /// the files in scope, hashed. Two snapshots with the same digest are the
    /// same code.
    pub content_digest: String,
    /// The provider's configuration files at the snapshot, hashed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub configuration_digest: Option<String>,
    #[serde(default)]
    pub taken_at: String,
}

impl ReviewSnapshot {
    /// Whether `other` describes the same code under the same configuration.
    pub fn same_code(&self, other: &ReviewSnapshot) -> bool {
        self.repository_id == other.repository_id
            && self.base_commit == other.base_commit
            && self.head_commit == other.head_commit
            && self.scope == other.scope
            && self.content_digest == other.content_digest
            && self.configuration_digest == other.configuration_digest
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewFinding {
    pub id: String,
    #[serde(default)]
    pub review_id: String,
    #[serde(default)]
    pub provider_id: String,
    pub severity: Severity,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub provider_severity: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub start_line: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub end_line: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<Side>,
    pub title: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub group: Option<String>,
    #[serde(default)]
    pub disposition: Disposition,
}

/// An action the provider says a person must take before the review can go on
/// — a billing confirmation, say. Never a success.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionRequired {
    pub kind: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<serde_json::Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReviewResult {
    pub review_id: String,
    pub provider_id: String,
    pub provider_version: String,
    pub snapshot_id: String,
    pub status: RunStatus,
    pub summary: String,
    pub findings: Vec<ReviewFinding>,
    pub completeness: Completeness,
    pub started_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action_required: Option<ActionRequired>,
}

/// What a repository's policy asks of a review before it may gate a merge.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatePolicy {
    /// Findings at or above this block. Medium by default (proposal §10).
    pub threshold: Severity,
}

impl Default for GatePolicy {
    fn default() -> Self {
        GatePolicy {
            threshold: Severity::Medium,
        }
    }
}

/// The gate, and why.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GateDecision {
    pub gate: Gate,
    pub reason: String,
    /// The findings that decided it, by id.
    #[serde(default)]
    pub blocking: Vec<String>,
}

impl GateDecision {
    fn new(gate: Gate, reason: impl Into<String>) -> Self {
        GateDecision {
            gate,
            reason: reason.into(),
            blocking: Vec::new(),
        }
    }
}

/// Decide the gate for `result`, reviewed at `reviewed`, against the code as it
/// is now (`current`).
///
/// - Anything but a complete, completed run blocks, each with its own reason.
/// - A snapshot that no longer matches the code blocks as stale.
/// - A skipped review never passes — a no-change result is not approval, and a
///   caller with a non-empty diff must not be able to use one.
/// - Any `unknown` severity blocks until a person assesses it.
/// - A finding at or above the threshold requests changes. Dismissing it
///   locally does **not** clear it: a dismissal is a person's note, not a code
///   change, and only a new review of new code can.
pub fn decide(
    result: &ReviewResult,
    reviewed: &ReviewSnapshot,
    current: Option<&ReviewSnapshot>,
    policy: GatePolicy,
) -> GateDecision {
    match result.status {
        RunStatus::Completed => {}
        RunStatus::Skipped => {
            return GateDecision::new(
                Gate::Blocked,
                "The review was skipped: it saw no changes to review.",
            )
        }
        RunStatus::ActionRequired => {
            let what = result
                .action_required
                .as_ref()
                .map(|a| a.message.as_str())
                .unwrap_or("the provider is waiting for a decision");
            return GateDecision::new(Gate::Blocked, format!("The review needs action: {what}"));
        }
        RunStatus::Incomplete => {
            return GateDecision::new(Gate::Blocked, "The review did not cover every file.")
        }
        RunStatus::Failed => return GateDecision::new(Gate::Blocked, "The review failed."),
        RunStatus::Cancelled => {
            return GateDecision::new(Gate::Blocked, "The review was cancelled.")
        }
        RunStatus::Stale => {
            return GateDecision::new(
                Gate::Blocked,
                "The code changed while it was being reviewed.",
            )
        }
        RunStatus::Queued | RunStatus::Running => {
            return GateDecision::new(Gate::NotEvaluated, "The review has not finished.")
        }
    }
    if result.completeness != Completeness::Complete {
        return GateDecision::new(
            Gate::Blocked,
            "The provider did not report a complete review.",
        );
    }
    if result.snapshot_id != reviewed.id {
        return GateDecision::new(
            Gate::Blocked,
            "The result does not belong to the snapshot it claims.",
        );
    }
    match current {
        Some(current) if reviewed.same_code(current) => {}
        Some(_) => {
            return GateDecision::new(Gate::Blocked, "The code has changed since it was reviewed.")
        }
        None => {
            return GateDecision::new(
                Gate::Blocked,
                "The code being merged could not be identified.",
            )
        }
    }

    let unknown: Vec<String> = result
        .findings
        .iter()
        .filter(|f| f.severity == Severity::Unknown)
        .map(|f| f.id.clone())
        .collect();
    if !unknown.is_empty() {
        return GateDecision {
            gate: Gate::Blocked,
            reason: format!(
                "{} finding{} of unknown severity need{} assessing.",
                unknown.len(),
                if unknown.len() == 1 { "" } else { "s" },
                if unknown.len() == 1 { "s" } else { "" }
            ),
            blocking: unknown,
        };
    }

    let blocking: Vec<String> = result
        .findings
        .iter()
        .filter(|f| f.severity >= policy.threshold)
        .map(|f| f.id.clone())
        .collect();
    if !blocking.is_empty() {
        return GateDecision {
            gate: Gate::ChangesRequested,
            reason: format!(
                "{} finding{} at {} or above.",
                blocking.len(),
                if blocking.len() == 1 { "" } else { "s" },
                policy.threshold.label().to_lowercase()
            ),
            blocking,
        };
    }
    GateDecision::new(
        Gate::Pass,
        "Complete, current, and nothing at or above the threshold.",
    )
}

/// The run status a result ends as, from what the provider said and what the
/// host saw. Used where the provider's word alone is not enough.
pub fn settle(provider_said: RunStatus, completeness: Completeness, stale: bool) -> RunStatus {
    if stale && provider_said.is_terminal() && provider_said != RunStatus::Cancelled {
        return RunStatus::Stale;
    }
    if provider_said == RunStatus::Completed && completeness != Completeness::Complete {
        return RunStatus::Incomplete;
    }
    provider_said
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snapshot(digest: &str) -> ReviewSnapshot {
        ReviewSnapshot {
            id: "snap-1".into(),
            repository_id: "repo".into(),
            target: ReviewTarget::FarmTask,
            task_id: Some("TASK-0001".into()),
            pull_request_number: None,
            base_commit: "a".repeat(40),
            head_commit: "b".repeat(40),
            base_ref: None,
            scope: ReviewScope::Committed,
            content_digest: format!("sha256:{digest}"),
            configuration_digest: None,
            taken_at: String::new(),
        }
    }

    fn finding(id: &str, severity: Severity) -> ReviewFinding {
        ReviewFinding {
            id: id.into(),
            review_id: "rv".into(),
            provider_id: "p".into(),
            severity,
            provider_severity: None,
            path: None,
            start_line: None,
            end_line: None,
            side: None,
            title: "t".into(),
            message: "m".into(),
            suggestion: None,
            source_url: None,
            group: None,
            disposition: Disposition::Open,
        }
    }

    fn result(status: RunStatus, findings: Vec<ReviewFinding>) -> ReviewResult {
        ReviewResult {
            review_id: "rv".into(),
            provider_id: "p".into(),
            provider_version: "1".into(),
            snapshot_id: "snap-1".into(),
            status,
            summary: String::new(),
            findings,
            completeness: Completeness::Complete,
            started_at: String::new(),
            finished_at: None,
            action_required: None,
        }
    }

    fn gate(result: &ReviewResult) -> Gate {
        let s = snapshot("1");
        decide(result, &s, Some(&s), GatePolicy::default()).gate
    }

    #[test]
    fn a_complete_current_review_with_only_low_findings_passes() {
        assert_eq!(gate(&result(RunStatus::Completed, vec![])), Gate::Pass);
        assert_eq!(
            gate(&result(
                RunStatus::Completed,
                vec![finding("1", Severity::Low), finding("2", Severity::Info)]
            )),
            Gate::Pass
        );
    }

    #[test]
    fn a_finding_at_the_threshold_requests_changes_even_once_dismissed() {
        let mut medium = finding("1", Severity::Medium);
        medium.disposition = Disposition::Dismissed;
        let decision = {
            let s = snapshot("1");
            decide(
                &result(RunStatus::Completed, vec![medium]),
                &s,
                Some(&s),
                GatePolicy::default(),
            )
        };
        assert_eq!(decision.gate, Gate::ChangesRequested);
        assert_eq!(decision.blocking, vec!["1".to_string()]);
    }

    #[test]
    fn a_higher_threshold_lets_a_medium_through() {
        let s = snapshot("1");
        let policy = GatePolicy {
            threshold: Severity::High,
        };
        let decision = decide(
            &result(RunStatus::Completed, vec![finding("1", Severity::Medium)]),
            &s,
            Some(&s),
            policy,
        );
        assert_eq!(decision.gate, Gate::Pass);
    }

    #[test]
    fn every_unfinished_or_unproven_outcome_blocks_with_its_own_reason() {
        let mut reasons = std::collections::HashSet::new();
        for status in [
            RunStatus::Skipped,
            RunStatus::ActionRequired,
            RunStatus::Incomplete,
            RunStatus::Failed,
            RunStatus::Cancelled,
            RunStatus::Stale,
        ] {
            let s = snapshot("1");
            let decision = decide(&result(status, vec![]), &s, Some(&s), GatePolicy::default());
            assert_eq!(decision.gate, Gate::Blocked, "{status:?}");
            assert!(
                reasons.insert(decision.reason),
                "{status:?} reused a reason"
            );
        }
        assert_eq!(
            gate(&result(RunStatus::Running, vec![])),
            Gate::NotEvaluated
        );
    }

    #[test]
    fn completed_but_partial_is_not_complete() {
        let mut partial = result(RunStatus::Completed, vec![]);
        partial.completeness = Completeness::Partial;
        assert_eq!(gate(&partial), Gate::Blocked);
    }

    #[test]
    fn unknown_severity_blocks_until_assessed() {
        assert_eq!(
            gate(&result(
                RunStatus::Completed,
                vec![finding("1", Severity::Unknown)]
            )),
            Gate::Blocked
        );
    }

    #[test]
    fn code_that_moved_since_the_review_blocks() {
        let reviewed = snapshot("1");
        let now = snapshot("2");
        let decision = decide(
            &result(RunStatus::Completed, vec![]),
            &reviewed,
            Some(&now),
            GatePolicy::default(),
        );
        assert_eq!(decision.gate, Gate::Blocked);
        assert!(decision.reason.contains("changed"));
        let decision = decide(
            &result(RunStatus::Completed, vec![]),
            &reviewed,
            None,
            GatePolicy::default(),
        );
        assert_eq!(decision.gate, Gate::Blocked);
    }

    #[test]
    fn a_result_for_another_snapshot_cannot_pass() {
        let mut other = result(RunStatus::Completed, vec![]);
        other.snapshot_id = "snap-9".into();
        assert_eq!(gate(&other), Gate::Blocked);
    }

    #[test]
    fn the_host_settles_what_the_provider_said() {
        assert_eq!(
            settle(RunStatus::Completed, Completeness::Complete, false),
            RunStatus::Completed
        );
        assert_eq!(
            settle(RunStatus::Completed, Completeness::Partial, false),
            RunStatus::Incomplete
        );
        assert_eq!(
            settle(RunStatus::Completed, Completeness::Complete, true),
            RunStatus::Stale
        );
        assert_eq!(
            settle(RunStatus::Cancelled, Completeness::Unknown, true),
            RunStatus::Cancelled
        );
    }

    #[test]
    fn severities_order_from_critical_down() {
        assert!(Severity::Critical > Severity::High);
        assert!(Severity::High > Severity::Medium);
        assert!(Severity::Low > Severity::Info);
        assert_eq!(Severity::parse("high"), Some(Severity::High));
        assert_eq!(Severity::parse("major"), None);
        assert_eq!(
            RunStatus::parse("actionRequired"),
            Some(RunStatus::ActionRequired)
        );
    }
}
