// SPDX-License-Identifier: GPL-3.0-or-later

//! What a farm starts with that rarely changes, and the farms that came before
//! (FEAT-141).
//!
//! Starting a farm asks one thing every time — the goal — and a handful of
//! things that are nearly always the same as last time: which agents, how much
//! they do alone, which checks every task must pass. The second group is
//! remembered per repository, so the wizard can ask "Same crew and rules as
//! last time?" and mean it.

use serde::{Deserialize, Serialize};

use super::{AgentId, Autonomy, Farm, FarmId, FarmStatus, Goal, Permissions, TaskStatus};

/// The crew and the rules, as the last farm in this repository started with
/// them.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Setup {
    #[serde(default)]
    pub agents: Vec<AgentId>,
    #[serde(default)]
    pub autonomy: Autonomy,
    #[serde(default)]
    pub permissions: Permissions,
    #[serde(default = "two")]
    pub max_parallel: usize,
    #[serde(default = "three")]
    pub max_attempts: u32,
    #[serde(default)]
    pub verification: Vec<String>,
    #[serde(default)]
    pub supplemental: crate::supplemental::Policy,
    /// Where each task works, `worktree` or `slice` (FEAT-143).
    #[serde(default)]
    pub workspace: super::WorkspaceMode,
    /// Who resolves a conflict (FEAT-146).
    #[serde(default)]
    pub conflicts: super::ConflictResolver,
    /// The goal of the farm these came from, to name it in the question.
    #[serde(default)]
    pub from_title: String,
    #[serde(default)]
    pub from_ms: u64,
}

fn two() -> usize {
    2
}

fn three() -> u32 {
    3
}

impl Setup {
    /// What `farm` was started with.
    pub fn of(farm: &Farm) -> Self {
        Setup {
            agents: farm.agents.clone(),
            autonomy: farm.autonomy,
            permissions: farm.permissions,
            max_parallel: farm.max_parallel,
            max_attempts: farm.max_attempts,
            verification: farm.verification.clone(),
            supplemental: farm.supplemental.clone(),
            workspace: farm.workspace,
            conflicts: farm.conflicts.clone(),
            from_title: farm.goal.title.clone(),
            from_ms: farm.created_ms,
        }
    }
}

/// An earlier farm in this repository, as the wizard lists it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PastFarm {
    pub id: FarmId,
    pub goal: Goal,
    pub status: FarmStatus,
    pub merged: usize,
    pub total: usize,
    pub updated_ms: u64,
    /// Its crew and rules, for *Use as a starting point*.
    pub setup: Setup,
}

impl PastFarm {
    pub fn of(farm: &Farm) -> Self {
        let counted = farm
            .tasks
            .iter()
            .filter(|task| task.status != TaskStatus::Cancelled);
        let total = counted.clone().count();
        let merged = counted
            .filter(|task| task.status == TaskStatus::Done)
            .count();
        PastFarm {
            id: farm.id.clone(),
            goal: farm.goal.clone(),
            status: farm.status,
            merged,
            total,
            updated_ms: farm.updated_ms.max(farm.created_ms),
            setup: Setup::of(farm),
        }
    }
}
