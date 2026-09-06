// SPDX-License-Identifier: GPL-3.0-or-later

//! The gate between "the agent says it is done" and "it is done".
//!
//! The plan states the rule plainly: *agent success is not task success*. An
//! agent that reports completion has reported its own opinion of its own work,
//! and every model will occasionally be wrong about it in the same confident
//! tone it is right in. So the farm runs the repository's own commands against
//! the agent's worktree, and the result of *those* is what moves a task.
//!
//! # Where the commands come from
//!
//! Three places, in this order:
//!
//! 1. The farm's own list — what the user typed once, for every task.
//! 2. The task's list — extra checks for this piece of work.
//! 3. Nothing, if both are empty.
//!
//! The third case is the interesting one. A farm with no verification commands
//! cannot verify, and the honest thing is to say so rather than to pass a task
//! by default. [`Verification::unverified`] is a distinct outcome from
//! [`Verification::passed`], and the interface shows it as a warning rather than
//! a tick — a green tick for "we checked nothing" is a lie the whole feature
//! rests on not telling.

use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};

use crate::model::Task;
use crate::verification::command::{self, CommandResult};

/// What a whole verification run produced.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Verification {
    pub results: Vec<CommandResult>,
    /// Every command passed. False when any failed, and false when there were
    /// none — see [`Verification::unverified`].
    pub passed: bool,
    /// There was nothing to run.
    pub unverified: bool,
}

impl Verification {
    /// The commands that failed, for the event and the interface.
    pub fn failures(&self) -> Vec<&CommandResult> {
        self.results
            .iter()
            .filter(|result| !result.passed)
            .collect()
    }

    /// One line saying what happened, for the task's note.
    ///
    /// Written here rather than in the UI because the same sentence goes into
    /// the task note, the activity log and the prompt of whoever is asked to
    /// fix it — and three renderings of the same fact drift.
    pub fn summary(&self) -> String {
        if self.unverified {
            return "No verification commands are configured, so nothing was checked.".into();
        }
        let failures = self.failures();
        if failures.is_empty() {
            return format!(
                "All {} verification {} passed.",
                self.results.len(),
                if self.results.len() == 1 {
                    "command"
                } else {
                    "commands"
                }
            );
        }
        let names: Vec<&str> = failures
            .iter()
            .map(|result| result.command.as_str())
            .collect();
        format!("Verification failed: {}", names.join(", "))
    }
}

/// The commands to run for one task.
///
/// A task's own list is *added* to the farm's unless it says otherwise, because
/// the common case is "the usual checks, plus this one". `verification_overrides`
/// exists for the case that is not — a task in a subdirectory with its own
/// toolchain, where the farm's `cargo test` would fail for reasons that have
/// nothing to do with the work.
pub fn commands_for(farm: &[String], task: &Task) -> Vec<String> {
    let mut out: Vec<String> = if task.verification_overrides {
        Vec::new()
    } else {
        farm.to_vec()
    };
    for command in &task.verification {
        if !out.contains(command) {
            out.push(command.clone());
        }
    }
    out.retain(|command| !command.trim().is_empty());
    out
}

/// Run every command against `workdir`, stopping at the first failure.
///
/// Stopping early is deliberate. The commands are ordered by the user, the
/// cheap ones come first by convention, and running a twenty-minute integration
/// suite after the type check has already failed spends time to learn nothing.
/// The interface shows the ones that did not run as pending rather than as
/// passed.
///
/// `report` is called as each command starts and finishes, so the screen fills
/// in as it goes rather than after several minutes of nothing.
pub fn run(
    workdir: &Path,
    commands: &[String],
    report: &mut dyn FnMut(&CommandResult),
) -> Verification {
    run_cancellable(
        workdir,
        commands,
        &AtomicBool::new(false),
        &mut |_| {},
        report,
    )
}

pub fn run_cancellable(
    workdir: &Path,
    commands: &[String],
    cancelled: &AtomicBool,
    starting: &mut dyn FnMut(&str),
    report: &mut dyn FnMut(&CommandResult),
) -> Verification {
    if commands.is_empty() {
        return Verification {
            results: Vec::new(),
            passed: false,
            unverified: true,
        };
    }

    let mut results = Vec::new();
    for line in commands {
        if cancelled.load(Ordering::Acquire) {
            break;
        }
        starting(line);
        let result = command::run_cancellable(workdir, line, cancelled, command::TIMEOUT);
        let failed = !result.passed;
        report(&result);
        results.push(result);
        if failed {
            break;
        }
    }

    Verification {
        passed: !cancelled.load(Ordering::Acquire)
            && results.len() == commands.len()
            && results.iter().all(|result| result.passed),
        unverified: false,
        results,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{task_id, Task};

    fn task() -> Task {
        Task::new(task_id(1), "Do the thing", 0)
    }

    fn silent() -> impl FnMut(&CommandResult) {
        |_: &CommandResult| {}
    }

    #[test]
    fn a_farm_with_no_commands_verifies_nothing_and_says_so() {
        let dir = tempfile::tempdir().unwrap();
        let verification = run(dir.path(), &[], &mut silent());
        assert!(verification.unverified);
        assert!(
            !verification.passed,
            "nothing checked must not read as passed"
        );
        assert!(verification.summary().contains("nothing was checked"));
    }

    #[test]
    fn every_command_passing_passes_the_task() {
        let dir = tempfile::tempdir().unwrap();
        let verification = run(
            dir.path(),
            &["/bin/sh -c 'exit 0'".into(), "/bin/sh -c 'exit 0'".into()],
            &mut silent(),
        );
        assert!(verification.passed);
        assert!(!verification.unverified);
        assert_eq!(verification.results.len(), 2);
        assert_eq!(
            verification.summary(),
            "All 2 verification commands passed."
        );
    }

    #[test]
    fn one_command_failing_fails_the_task() {
        let dir = tempfile::tempdir().unwrap();
        let verification = run(dir.path(), &["/bin/sh -c 'exit 1'".into()], &mut silent());
        assert!(!verification.passed);
        assert_eq!(verification.failures().len(), 1);
        assert!(verification.summary().starts_with("Verification failed:"));
    }

    #[test]
    fn nothing_runs_after_a_failure() {
        let dir = tempfile::tempdir().unwrap();
        let verification = run(
            dir.path(),
            &[
                "/bin/sh -c 'exit 1'".into(),
                "/bin/sh -c 'touch should-not-exist'".into(),
            ],
            &mut silent(),
        );
        assert_eq!(verification.results.len(), 1);
        assert!(!dir.path().join("should-not-exist").exists());
    }

    #[test]
    fn each_result_is_reported_as_it_finishes() {
        let dir = tempfile::tempdir().unwrap();
        let mut seen = Vec::new();
        run(
            dir.path(),
            &["/bin/sh -c 'exit 0'".into(), "/bin/sh -c 'exit 0'".into()],
            &mut |result| seen.push(result.command.clone()),
        );
        assert_eq!(seen.len(), 2);
    }

    #[test]
    fn a_task_adds_its_own_checks_to_the_farms() {
        let mut task = task();
        task.verification = vec!["cargo test -p auth".into()];
        assert_eq!(
            commands_for(&["cargo test".into()], &task),
            ["cargo test", "cargo test -p auth"]
        );
    }

    #[test]
    fn a_task_can_replace_the_farms_checks_entirely() {
        let mut task = task();
        task.verification = vec!["./mvnw test".into()];
        task.verification_overrides = true;
        assert_eq!(commands_for(&["cargo test".into()], &task), ["./mvnw test"]);
    }

    #[test]
    fn the_same_command_is_not_run_twice() {
        let mut task = task();
        task.verification = vec!["cargo test".into()];
        assert_eq!(commands_for(&["cargo test".into()], &task), ["cargo test"]);
    }

    #[test]
    fn blank_entries_are_dropped_rather_than_run() {
        let mut task = task();
        task.verification = vec!["  ".into()];
        assert_eq!(commands_for(&["".into()], &task), Vec::<String>::new());
    }

    #[test]
    fn a_single_command_is_described_in_the_singular() {
        let dir = tempfile::tempdir().unwrap();
        let verification = run(dir.path(), &["/bin/sh -c 'exit 0'".into()], &mut silent());
        assert_eq!(verification.summary(), "All 1 verification command passed.");
    }
}
