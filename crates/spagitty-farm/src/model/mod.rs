// SPDX-License-Identifier: GPL-3.0-or-later

//! The farm's vocabulary: what a goal, a task, an agent and a run *are*.
//!
//! Everything here is data. No process is spawned, no file is written and no
//! git command runs from this module tree — that is deliberate, because these
//! types are what the scheduler, the persistence layer, the Tauri commands and
//! the webview all agree on, and a type that can do something is a type that
//! two of those four will use to do it in the wrong layer.
//!
//! # Why the identifiers are newtypes and not `String`
//!
//! A farm juggles four families of identifier at once — tasks, agents, runs and
//! farms — and they are all strings underneath. Passing a run id where a task id
//! belongs is the kind of mistake that produces a farm which looks fine and
//! schedules nothing. Newtypes make it a compile error instead. They serialise
//! as plain strings, so the webview never sees the wrapper.

pub mod agent;
pub mod event;
pub mod farm;
pub mod goal;
pub mod handoff;
pub mod run;
pub mod setup;
pub mod task;

pub use agent::{
    AgentAvailability, AgentCapabilities, AgentCapability, AgentDefinition, AgentInputMode,
    AgentProvider, AgentRole, AgentTraits,
};
pub use event::{FarmEvent, Recorded};
pub use farm::{Autonomy, Farm, FarmStatus, Permissions, WorkspaceMode};
pub use goal::{Goal, GoalSource};
pub use handoff::{Handoff, HandoffStatus, ProposedTask, TestOutcome, TestReport};
pub use run::{AgentRun, RunOutcome, RunPhase};
pub use setup::{PastFarm, Setup};
pub use task::{Task, TaskKind, TaskOrigin, TaskPriority, TaskStatus};

use std::fmt;

use serde::{Deserialize, Serialize};

/// Build a string newtype: a distinct type at compile time, a plain string on
/// the wire.
///
/// A macro rather than four hand-written copies, because the four are identical
/// and a hand-written set drifts — one of them gains `PartialOrd`, another does
/// not, and a sort somewhere stops compiling for no reason a reader can see.
macro_rules! id_type {
    ($name:ident, $what:literal) => {
        #[doc = concat!("Identifies one ", $what, ".")]
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl From<&str> for $name {
            fn from(value: &str) -> Self {
                Self(value.to_string())
            }
        }
    };
}

id_type!(FarmId, "farm");
id_type!(GoalId, "goal");
id_type!(TaskId, "task");
id_type!(AgentId, "registered agent");
id_type!(RunId, "single execution of one agent against one task");

/// Task identifiers are `TASK-0042`: sortable as text, and the same shape the
/// plan's structured-task example uses.
///
/// Four digits rather than three because the repository's own working record
/// already uses three for its items, and a farm task in a log next to a
/// `TASK-012` from `agile/` should not read as the same thing.
pub fn task_id(sequence: u32) -> TaskId {
    TaskId::new(format!("TASK-{sequence:04}"))
}

/// Wall-clock milliseconds since the Unix epoch.
///
/// A plain integer rather than a date type: the farm never does arithmetic on
/// calendars, only "which happened first" and "how long did it take", and the
/// webview formats it with `Intl` anyway. Adding `chrono` to render a timestamp
/// nothing in Rust reads would be a dependency for a conversion.
pub fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|since| since.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifiers_are_plain_strings_on_the_wire() {
        let json = serde_json::to_string(&TaskId::new("TASK-0042")).unwrap();
        assert_eq!(json, "\"TASK-0042\"");
        let back: TaskId = serde_json::from_str(&json).unwrap();
        assert_eq!(back, TaskId::new("TASK-0042"));
    }

    #[test]
    fn task_identifiers_sort_as_text_in_numeric_order() {
        let mut ids = [task_id(10), task_id(2), task_id(1)];
        ids.sort();
        assert_eq!(
            ids.iter().map(|id| id.to_string()).collect::<Vec<_>>(),
            ["TASK-0001", "TASK-0002", "TASK-0010"]
        );
    }

    #[test]
    fn the_clock_is_after_this_was_written() {
        // 2026-01-01, which every machine that can build this is past.
        assert!(now_ms() > 1_767_225_600_000);
    }
}
