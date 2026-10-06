// SPDX-License-Identifier: GPL-3.0-or-later

//! What the farm says as it works.
//!
//! One enum, serialised with an internal tag, rather than a Tauri event per
//! kind. The reason is the UI: the Farm screen keeps one activity list and one
//! subscription, and a screen that has to subscribe to fourteen channels to
//! stay current is a screen that will miss one.
//!
//! Every event carries enough to act on without a follow-up query — a task
//! identifier rather than an index, a whole status rather than "it changed" —
//! because a webview that has to ask a question for every event is a webview
//! that polls, which is the thing the event system exists to stop.

use serde::{Deserialize, Serialize};

use super::{AgentId, FarmStatus, RunId, TaskId, TaskStatus};

/// An event, and when it happened.
///
/// The event itself does not carry a time: the variants say *what*, and adding
/// a timestamp field to each of the sixteen would be sixteen places to forget
/// it. It is added once, here, where the event is recorded.
///
/// Flattened on the wire, so a recorded event serialises as the event's own
/// object with one more key. `atMs` defaults to zero when it is missing, which
/// is what every line written before the field existed looks like — an old log
/// still reads, and a zero is shown as no time rather than as 1970.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Recorded {
    #[serde(default)]
    pub at_ms: u64,
    #[serde(flatten)]
    pub event: FarmEvent,
}

impl Recorded {
    /// Stamp an event with the moment it is being recorded.
    pub fn now(event: FarmEvent) -> Self {
        Recorded {
            at_ms: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|since| since.as_millis() as u64)
                .unwrap_or(0),
            event,
        }
    }

    pub fn is_transcript(&self) -> bool {
        self.event.is_transcript()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "kind")]
pub enum FarmEvent {
    /// The farm as a whole moved.
    FarmStatusChanged {
        status: FarmStatus,
    },

    TaskCreated {
        task: TaskId,
        title: String,
    },
    TaskStatusChanged {
        task: TaskId,
        status: TaskStatus,
        /// Why, when there is a reason worth saying — a verification failure,
        /// a review's words, a blocked agent's explanation.
        #[serde(default)]
        note: Option<String>,
    },
    TaskAssigned {
        task: TaskId,
        agent: AgentId,
    },

    AgentStarted {
        run: RunId,
        task: TaskId,
        agent: AgentId,
        /// The command line, so the activity log shows what ran.
        command: String,
    },
    /// A chunk of the agent's transcript.
    ///
    /// Lines rather than bytes, and one event per line: the UI appends to a
    /// list, and re-assembling partial lines in the webview would put a parser
    /// on the wrong side of the boundary.
    AgentOutput {
        run: RunId,
        task: TaskId,
        line: String,
    },
    AgentStopped {
        run: RunId,
        task: TaskId,
        /// True when it exited zero.
        ok: bool,
        #[serde(default)]
        reason: Option<String>,
    },

    VerificationStarted {
        task: TaskId,
        command: String,
    },
    VerificationFinished {
        task: TaskId,
        command: String,
        passed: bool,
        /// The tail of the command's own output. Truncated by the verifier —
        /// a failing test suite can produce more than an event should carry.
        #[serde(default)]
        output: String,
    },

    ReviewRequested {
        task: TaskId,
        reviewer: AgentId,
    },
    ReviewCompleted {
        task: TaskId,
        reviewer: AgentId,
        approved: bool,
        #[serde(default)]
        summary: String,
    },

    SupplementalPolicyChanged {
        mode: crate::supplemental::Mode,
        revision: u64,
        actor: String,
    },
    SupplementalReview {
        task: TaskId,
        state: String,
        provider: String,
        summary: String,
    },
    MergeRequested {
        task: TaskId,
        branch: String,
    },
    MergeCompleted {
        task: TaskId,
        branch: String,
        ok: bool,
        #[serde(default)]
        error: Option<String>,
    },

    /// A worktree was cut or removed. Worth an event of its own because it is
    /// the farm touching the user's disk.
    WorkspaceChanged {
        task: TaskId,
        path: String,
        created: bool,
    },

    /// The agent proposed work Spagitty has not scheduled. The human decides.
    TaskProposed {
        from: TaskId,
        title: String,
    },

    /// Something went wrong that is not attached to one task.
    Failed {
        message: String,
    },
}

impl FarmEvent {
    /// The task this is about, when it is about one.
    ///
    /// The Task detail screen filters the activity list by this rather than by
    /// matching on the variant, so a new variant is shown in the right place
    /// without the screen being edited.
    pub fn task(&self) -> Option<&TaskId> {
        match self {
            FarmEvent::TaskCreated { task, .. }
            | FarmEvent::TaskStatusChanged { task, .. }
            | FarmEvent::TaskAssigned { task, .. }
            | FarmEvent::AgentStarted { task, .. }
            | FarmEvent::AgentOutput { task, .. }
            | FarmEvent::AgentStopped { task, .. }
            | FarmEvent::VerificationStarted { task, .. }
            | FarmEvent::VerificationFinished { task, .. }
            | FarmEvent::ReviewRequested { task, .. }
            | FarmEvent::ReviewCompleted { task, .. }
            | FarmEvent::SupplementalReview { task, .. }
            | FarmEvent::MergeRequested { task, .. }
            | FarmEvent::MergeCompleted { task, .. }
            | FarmEvent::WorkspaceChanged { task, .. }
            | FarmEvent::TaskProposed { from: task, .. } => Some(task),
            FarmEvent::FarmStatusChanged { .. }
            | FarmEvent::Failed { .. }
            | FarmEvent::SupplementalPolicyChanged { .. } => None,
        }
    }

    /// True for the flood: agent transcript lines.
    ///
    /// The activity list keeps a bounded history of everything else and drops
    /// these, because a single agent run can produce thousands and they already
    /// have a home in the run's own log pane.
    pub fn is_transcript(&self) -> bool {
        matches!(self, FarmEvent::AgentOutput { .. })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_carry_their_kind_on_the_wire() {
        let json = serde_json::to_value(FarmEvent::TaskCreated {
            task: TaskId::new("TASK-0001"),
            title: "Do it".into(),
        })
        .unwrap();
        assert_eq!(json["kind"], "taskCreated");
        assert_eq!(json["task"], "TASK-0001");
    }

    #[test]
    fn every_task_shaped_event_reports_its_task() {
        let task = TaskId::new("TASK-0007");
        let events = [
            FarmEvent::TaskCreated {
                task: task.clone(),
                title: String::new(),
            },
            FarmEvent::TaskStatusChanged {
                task: task.clone(),
                status: TaskStatus::Done,
                note: None,
            },
            FarmEvent::TaskAssigned {
                task: task.clone(),
                agent: AgentId::new("a"),
            },
            FarmEvent::AgentOutput {
                run: RunId::new("r"),
                task: task.clone(),
                line: String::new(),
            },
            FarmEvent::VerificationStarted {
                task: task.clone(),
                command: String::new(),
            },
            FarmEvent::ReviewRequested {
                task: task.clone(),
                reviewer: AgentId::new("a"),
            },
            FarmEvent::MergeRequested {
                task: task.clone(),
                branch: String::new(),
            },
            FarmEvent::WorkspaceChanged {
                task: task.clone(),
                path: String::new(),
                created: true,
            },
            FarmEvent::TaskProposed {
                from: task.clone(),
                title: String::new(),
            },
        ];
        for event in events {
            assert_eq!(event.task(), Some(&task), "{event:?} lost its task");
        }
    }

    #[test]
    fn farm_wide_events_belong_to_no_task() {
        assert_eq!(
            FarmEvent::FarmStatusChanged {
                status: FarmStatus::Running
            }
            .task(),
            None
        );
        assert_eq!(
            FarmEvent::Failed {
                message: "x".into()
            }
            .task(),
            None
        );
    }

    #[test]
    fn only_agent_output_counts_as_transcript() {
        assert!(FarmEvent::AgentOutput {
            run: RunId::new("r"),
            task: TaskId::new("t"),
            line: "hello".into()
        }
        .is_transcript());
        assert!(!FarmEvent::Failed {
            message: "x".into()
        }
        .is_transcript());
    }

    #[test]
    fn an_event_round_trips_through_json() {
        let event = FarmEvent::VerificationFinished {
            task: TaskId::new("TASK-0001"),
            command: "cargo test".into(),
            passed: false,
            output: "2 failed".into(),
        };
        let json = serde_json::to_string(&event).unwrap();
        let back: FarmEvent = serde_json::from_str(&json).unwrap();
        assert_eq!(back, event);
    }
}
