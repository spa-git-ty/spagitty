// SPDX-License-Identifier: GPL-3.0-or-later

//! Long-running work, each piece ending exactly once.
//!
//! An operation is minted by the host when it asks a worker to do something
//! that outlives a request — a command, a review. It ends when the worker sends
//! `operation.complete`, when it is cancelled and the grace period passes, when
//! it goes quiet for too long, when its absolute deadline passes, or when its
//! worker dies. Whichever comes **first** ends it; [`Operations::finish`]
//! hands the operation to exactly one caller and every later attempt gets
//! `None`. That is what makes a duplicate or late completion harmless: there is
//! nothing left for it to complete.

use std::collections::{HashMap, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::history::Requester;
use crate::review::{ReviewFinding, ReviewSnapshot};

/// Defaults from the protocol's Limits table.
pub const INACTIVITY: Duration = Duration::from_secs(120);
pub const DEADLINE: Duration = Duration::from_secs(45 * 60);
pub const CANCEL_GRACE: Duration = Duration::from_secs(5);
pub const MAX_PER_EXTENSION: usize = 4;
pub const MAX_FINDINGS: usize = 2000;

// A handful of operations exist at once; boxing the snapshot would buy nothing.
#[allow(clippy::large_enum_variant)]
#[derive(Debug, Clone)]
pub enum Kind {
    Command {
        command: String,
    },
    Review {
        provider: String,
        review_id: String,
        snapshot: ReviewSnapshot,
        /// The directory the review's tools run in.
        workdir: PathBuf,
        /// The main checkout, where its history is kept.
        main_workdir: PathBuf,
        requester: Requester,
    },
}

#[derive(Debug)]
pub struct Operation {
    pub id: String,
    pub extension: String,
    pub extension_version: String,
    pub session: String,
    pub kind: Kind,
    /// The repository handle the operation was started against, if any.
    pub repository: Option<String>,
    pub started: Instant,
    pub started_at: String,
    pub last_activity: Instant,
    pub deadline: Instant,
    pub cancel_requested: Option<Instant>,
    /// Shared with every tool run the operation owns: setting it ends them.
    pub cancel: Arc<AtomicBool>,
    pub progress: Option<String>,
    pub findings: Vec<ReviewFinding>,
    pub dropped_findings: usize,
    /// Callbacks waiting on host-owned tool work do not count as worker silence.
    pub host_work: usize,
}

impl Operation {
    pub fn review_id(&self) -> Option<&str> {
        match &self.kind {
            Kind::Review { review_id, .. } => Some(review_id),
            Kind::Command { .. } => None,
        }
    }

    pub fn is_cancelling(&self) -> bool {
        self.cancel_requested.is_some()
    }
}

/// Why the supervisor is ending an operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Expiry {
    Inactive,
    Deadline,
    CancelGrace,
}

#[derive(Default)]
pub struct Operations {
    running: Mutex<HashMap<String, Operation>>,
    /// Recently finished ids, so a late message can be told apart from a
    /// message about nothing.
    finished: Mutex<VecDeque<String>>,
}

/// A host callback holds this until its tool work has returned, including
/// detection. Dropping it resumes the inactivity clock, also on an error.
pub struct HostWork<'a> {
    operations: &'a Operations,
    id: String,
    extension: String,
    session: String,
}

impl Drop for HostWork<'_> {
    fn drop(&mut self) {
        self.operations
            .with(&self.id, &self.extension, &self.session, |op| {
                op.host_work -= 1;
                op.last_activity = Instant::now();
            });
    }
}

impl Operations {
    pub fn host_work(&self, id: &str, extension: &str, session: &str) -> Option<HostWork<'_>> {
        self.with(id, extension, session, |op| op.host_work += 1)?;
        Some(HostWork {
            operations: self,
            id: id.into(),
            extension: extension.into(),
            session: session.into(),
        })
    }

    pub fn insert(&self, operation: Operation) {
        self.running
            .lock()
            .expect("operations")
            .insert(operation.id.clone(), operation);
    }

    pub fn count_for(&self, extension: &str) -> usize {
        self.running
            .lock()
            .expect("operations")
            .values()
            .filter(|op| op.extension == extension)
            .count()
    }

    /// Run `f` on a running operation owned by `extension` in `session`.
    /// `None` when there is no such operation — finished, unknown, or someone
    /// else's.
    pub fn with<T>(
        &self,
        id: &str,
        extension: &str,
        session: &str,
        f: impl FnOnce(&mut Operation) -> T,
    ) -> Option<T> {
        let mut running = self.running.lock().expect("operations");
        let op = running.get_mut(id)?;
        if op.extension != extension || op.session != session {
            return None;
        }
        Some(f(op))
    }

    /// Run `f` on any running operation, by id.
    pub fn with_any<T>(&self, id: &str, f: impl FnOnce(&mut Operation) -> T) -> Option<T> {
        self.running.lock().expect("operations").get_mut(id).map(f)
    }

    /// End an operation. Returns it to exactly one caller.
    pub fn finish(&self, id: &str) -> Option<Operation> {
        let op = self.running.lock().expect("operations").remove(id)?;
        let mut finished = self.finished.lock().expect("finished");
        finished.push_back(id.to_string());
        while finished.len() > 1000 {
            finished.pop_front();
        }
        Some(op)
    }

    pub fn recently_finished(&self, id: &str) -> bool {
        self.finished
            .lock()
            .expect("finished")
            .iter()
            .any(|held| held == id)
    }

    /// Every running operation of `extension`, ended at once — its worker is
    /// gone.
    pub fn finish_all(&self, extension: &str) -> Vec<Operation> {
        let ids: Vec<String> = self
            .running
            .lock()
            .expect("operations")
            .values()
            .filter(|op| op.extension == extension)
            .map(|op| op.id.clone())
            .collect();
        ids.iter().filter_map(|id| self.finish(id)).collect()
    }

    /// Operations whose time is up, and why. Does not end them.
    pub fn expired(
        &self,
        now: Instant,
        inactivity: Duration,
        grace: Duration,
    ) -> Vec<(String, Expiry)> {
        self.running
            .lock()
            .expect("operations")
            .values()
            .filter_map(|op| {
                if let Some(asked) = op.cancel_requested {
                    return (now.duration_since(asked) >= grace)
                        .then(|| (op.id.clone(), Expiry::CancelGrace));
                }
                if now >= op.deadline {
                    return Some((op.id.clone(), Expiry::Deadline));
                }
                if op.host_work == 0 && now.duration_since(op.last_activity) >= inactivity {
                    return Some((op.id.clone(), Expiry::Inactive));
                }
                None
            })
            .collect()
    }

    pub fn ids(&self) -> Vec<String> {
        self.running
            .lock()
            .expect("operations")
            .keys()
            .cloned()
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn op(id: &str, extension: &str) -> Operation {
        let now = Instant::now();
        Operation {
            id: id.into(),
            extension: extension.into(),
            extension_version: "1".into(),
            session: "s".into(),
            kind: Kind::Command {
                command: "c".into(),
            },
            repository: None,
            started: now,
            started_at: String::new(),
            last_activity: now,
            deadline: now + DEADLINE,
            cancel_requested: None,
            cancel: Arc::new(AtomicBool::new(false)),
            progress: None,
            findings: Vec::new(),
            dropped_findings: 0,
            host_work: 0,
        }
    }

    #[test]
    fn an_operation_ends_exactly_once() {
        let ops = Operations::default();
        ops.insert(op("op-1", "a.b"));
        assert!(ops.finish("op-1").is_some());
        assert!(
            ops.finish("op-1").is_none(),
            "a second completion finds nothing"
        );
        assert!(ops.recently_finished("op-1"));
        assert!(!ops.recently_finished("op-2"));
    }

    #[test]
    fn an_operation_is_only_reachable_by_its_owner() {
        let ops = Operations::default();
        ops.insert(op("op-1", "a.b"));
        assert!(ops.with("op-1", "c.d", "s", |_| ()).is_none());
        assert!(ops.with("op-1", "a.b", "other-session", |_| ()).is_none());
        assert!(ops.with("op-1", "a.b", "s", |_| ()).is_some());
    }

    #[test]
    fn quiet_late_and_cancelled_operations_expire_for_their_own_reasons() {
        let ops = Operations::default();
        let now = Instant::now();
        let mut quiet = op("quiet", "a.b");
        quiet.last_activity = now - Duration::from_secs(200);
        let mut late = op("late", "a.b");
        late.deadline = now - Duration::from_secs(1);
        let mut cancelled = op("cancelled", "a.b");
        cancelled.cancel_requested = Some(now - Duration::from_secs(6));
        cancelled.last_activity = now - Duration::from_secs(500);
        let mut healthy = op("healthy", "a.b");
        healthy.last_activity = now;
        for o in [quiet, late, cancelled, healthy] {
            ops.insert(o);
        }
        let mut expired = ops.expired(now, INACTIVITY, CANCEL_GRACE);
        expired.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(
            expired,
            vec![
                ("cancelled".into(), Expiry::CancelGrace),
                ("late".into(), Expiry::Deadline),
                ("quiet".into(), Expiry::Inactive)
            ]
        );
    }

    #[test]
    fn a_dead_worker_ends_all_of_its_operations_and_only_its() {
        let ops = Operations::default();
        ops.insert(op("1", "a.b"));
        ops.insert(op("2", "a.b"));
        ops.insert(op("3", "c.d"));
        assert_eq!(ops.finish_all("a.b").len(), 2);
        assert_eq!(ops.count_for("c.d"), 1);
        assert_eq!(ops.ids(), vec!["3".to_string()]);
    }
    #[test]
    fn host_work_pauses_silence_until_every_callback_returns() {
        let ops = Operations::default();
        ops.insert(op("tool", "a.b"));
        assert!(ops.host_work("tool", "c.d", "s").is_none());
        assert!(ops.host_work("tool", "a.b", "old").is_none());
        let first = ops.host_work("tool", "a.b", "s").unwrap();
        let second = ops.host_work("tool", "a.b", "s").unwrap();
        let quiet = Instant::now() - INACTIVITY;
        ops.with_any("tool", |op| op.last_activity = quiet);
        assert!(ops
            .expired(Instant::now(), INACTIVITY, CANCEL_GRACE)
            .is_empty());
        drop(first);
        ops.with_any("tool", |op| op.last_activity = quiet);
        assert!(ops
            .expired(Instant::now(), INACTIVITY, CANCEL_GRACE)
            .is_empty());
        drop(second);
        assert!(ops
            .expired(Instant::now(), INACTIVITY, CANCEL_GRACE)
            .is_empty());
        ops.with_any("tool", |op| op.last_activity = quiet);
        assert_eq!(
            ops.expired(Instant::now(), INACTIVITY, CANCEL_GRACE),
            vec![("tool".into(), Expiry::Inactive)]
        );
    }

    #[test]
    fn host_work_does_not_suspend_deadlines_or_cancellation() {
        let ops = Operations::default();
        ops.insert(op("tool", "a.b"));
        let waiting = ops.host_work("tool", "a.b", "s").unwrap();
        let now = Instant::now();
        ops.with_any("tool", |op| op.deadline = now);
        assert_eq!(
            ops.expired(now, INACTIVITY, CANCEL_GRACE),
            vec![("tool".into(), Expiry::Deadline)]
        );
        ops.with_any("tool", |op| op.cancel_requested = Some(now - CANCEL_GRACE));
        assert_eq!(
            ops.expired(now, INACTIVITY, CANCEL_GRACE),
            vec![("tool".into(), Expiry::CancelGrace)]
        );
        ops.finish("tool").unwrap();
        drop(waiting); // A late return cannot resurrect a finished operation.
        assert!(ops.ids().is_empty());
    }
}
