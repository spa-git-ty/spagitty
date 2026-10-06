// SPDX-License-Identifier: GPL-3.0-or-later

//! An additional review is evidence, never authority over task transitions.
//! The desktop implements this interface; the farm has no extension dependency.
use crate::model::TaskId;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Mode {
    #[default]
    Off,
    Advisory,
    Required,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Threshold {
    Critical,
    High,
    #[default]
    Medium,
    Low,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct Policy {
    pub mode: Mode,
    pub provider: String,
    pub threshold: Threshold,
    pub max_repairs: u32,
    pub revision: u64,
}
impl Default for Policy {
    fn default() -> Self {
        Self {
            mode: Mode::Off,
            provider: "spagitty.coderabbit/review".into(),
            threshold: Threshold::Medium,
            max_repairs: 2,
            revision: 1,
        }
    }
}
#[derive(Debug, Clone)]
pub struct Input {
    pub task: TaskId,
    pub repository: PathBuf,
    pub workdir: PathBuf,
    pub base: String,
    pub head: String,
    pub policy: Policy,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Outcome {
    Pass,
    ChangesRequested,
    Blocked,
    Cancelled,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Evidence {
    pub task: TaskId,
    pub head: String,
    pub policy: Policy,
    pub identity: String,
    pub outcome: Outcome,
    pub summary: String,
    pub change_request: String,
    /// The public review record, including the exact snapshot and findings.
    /// Retained here so history trimming cannot silently erase gate evidence.
    pub record: serde_json::Value,
}
pub trait Provider: Send + Sync {
    fn review(&self, input: &Input, stop: &AtomicBool) -> Result<Evidence, String>;
    /// Recheck grants, provider version, configuration, code and outcome.
    /// Called at every merge, including after the application has restarted.
    fn validate(&self, input: &Input, evidence: &Evidence) -> Result<(), String>;
}

pub fn matching(input: &Input, evidence: &Evidence) -> Result<(), String> {
    if evidence.task != input.task || evidence.head != input.head || evidence.policy != input.policy
    {
        return Err("The supplemental review is out of date; review the task again.".into());
    }
    if evidence.outcome != Outcome::Pass {
        return Err(evidence.summary.clone());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn evidence_is_bound_to_the_task_commit_and_entire_policy() {
        let mut input = Input {
            task: TaskId::new("TASK-0001"),
            repository: "/repo".into(),
            workdir: "/work".into(),
            base: "main".into(),
            head: "abc".into(),
            policy: Policy::default(),
        };
        let mut evidence = Evidence {
            task: input.task.clone(),
            head: input.head.clone(),
            policy: input.policy.clone(),
            identity: "provider-version-and-snapshot".into(),
            outcome: Outcome::Pass,
            summary: "Reviewed".into(),
            change_request: String::new(),
            record: serde_json::Value::Null,
        };
        assert!(matching(&input, &evidence).is_ok());
        input.head = "new".into();
        assert!(matching(&input, &evidence).is_err());
        input.head = evidence.head.clone();
        input.policy.revision += 1;
        assert!(matching(&input, &evidence).is_err());
        input.policy = evidence.policy.clone();
        input.task = TaskId::new("other");
        assert!(matching(&input, &evidence).is_err());
        input.task = evidence.task.clone();
        for outcome in [
            Outcome::ChangesRequested,
            Outcome::Blocked,
            Outcome::Cancelled,
        ] {
            evidence.outcome = outcome;
            assert!(matching(&input, &evidence).is_err());
        }
        let reopened: Evidence =
            serde_json::from_str(&serde_json::to_string(&evidence).unwrap()).unwrap();
        assert_eq!(reopened, evidence);
    }
}
