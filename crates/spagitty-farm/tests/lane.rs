// SPDX-License-Identifier: GPL-3.0-or-later

//! The landing lane (FEAT-144), end to end against a real repository.
//!
//! The farm's own merges no longer touch the person's working copy: the lane
//! builds the merge commit, checks it when the destination moved, and moves
//! the branch by compare-and-swap. The agents are `/bin/sh` scripts, as in
//! `pipeline.rs`, so this suite runs on Linux and macOS.

use std::path::PathBuf;
use std::sync::Arc;

use spagitty_core::fixture::Fixture;
use spagitty_farm::agent::adapters::custom::PROMPT_TOKEN;
use spagitty_farm::model::*;
use spagitty_farm::service::{FarmService, Recorder};

fn agent(name: &str, body: &str) -> AgentDefinition {
    AgentDefinition {
        id: AgentId::new(name),
        provider: AgentProvider::Custom,
        display_name: name.to_string(),
        executable: PathBuf::from("/bin/sh"),
        capabilities: [AgentCapability::Coding, AgentCapability::Review]
            .into_iter()
            .collect(),
        input_mode: AgentInputMode::CliPrompt,
        traits: AgentTraits {
            resumable_sessions: false,
            streaming: true,
            structured_output: true,
            tool_use: true,
            headless: true,
        },
        role: AgentRole::General,
        extra_args: vec![
            "-c".to_string(),
            body.to_string(),
            name.to_string(),
            PROMPT_TOKEN.to_string(),
        ],
        enabled: true,
    }
}

/// An agent that writes `file` with `text` and commits it.
fn writer(name: &str, file: &str, text: &str) -> AgentDefinition {
    agent(
        name,
        &format!(
            r#"printf '%s\n' "{text}" > {file}
git add -A >/dev/null 2>&1
git -c user.name=Agent -c user.email=agent@example.com commit -q -m "TASK work by {name}" >/dev/null 2>&1
echo '```spagitty-handoff'
echo '{{"status":"completed","summary":"wrote {file}","filesChanged":["{file}"]}}'
echo '```'"#
        ),
    )
}

struct Lane {
    repo: Fixture,
    service: FarmService,
}

impl Lane {
    fn new(verification: &[&str]) -> Self {
        let repo = Fixture::woven();
        let service = FarmService::open(repo.path(), Arc::new(Recorder::default()));
        service.create("Ship it", "").unwrap();
        let commands: Vec<String> = verification.iter().map(|c| c.to_string()).collect();
        service
            .configure(|farm| farm.verification = commands.clone())
            .unwrap();
        Lane { repo, service }
    }

    /// A task done by an agent writing `file`, run to review.
    fn done(&self, name: &str, file: &str, text: &str) -> TaskId {
        let definition = writer(name, file, text);
        let id = definition.id.clone();
        self.service.save_agent(definition).unwrap();
        let mut task = Task::new(TaskId::new("unset"), "Do it", 0);
        task.status = TaskStatus::Ready;
        let task = self.service.add_task(task).unwrap().id;
        self.service.run_task(&task, Some(id)).unwrap();
        self.service.await_task(&task).unwrap();
        task
    }

    fn status(&self, task: &TaskId) -> TaskStatus {
        self.service.farm().unwrap().task(task).unwrap().status
    }

    fn note(&self, task: &TaskId) -> String {
        self.service
            .farm()
            .unwrap()
            .task(task)
            .unwrap()
            .note
            .clone()
            .unwrap_or_default()
    }
}

#[test]
fn a_task_lands_while_the_person_is_on_another_branch() {
    let lane = Lane::new(&[]);
    let task = lane.done("lander", "agent-output.txt", "made");
    lane.repo.git(&["switch", "-q", "feature/split-view"]);

    lane.service.merge(&task).unwrap();

    assert_eq!(lane.status(&task), TaskStatus::Done);
    // main has the work; the working copy, on another branch, does not.
    let files = lane.repo.git(&["ls-tree", "--name-only", "main"]);
    assert!(files.lines().any(|f| f == "agent-output.txt"));
    assert!(!lane.repo.path().join("agent-output.txt").exists());
    assert_eq!(
        lane.repo.git(&["branch", "--show-current"]).trim(),
        "feature/split-view"
    );
    // A merge commit, never a fast-forward.
    let parents = lane.repo.git(&["log", "-1", "--pretty=%P", "main"]);
    assert_eq!(parents.split_whitespace().count(), 2);

    let ledger = lane.service.ledger();
    assert_eq!(ledger.len(), 1);
    assert_eq!(ledger[0].kind, LedgerKind::Land);
    assert_eq!(ledger[0].reference, "refs/heads/main");
    assert_eq!(ledger[0].tasks, vec![task.clone()]);
    assert_eq!(ledger[0].by[0].agent, Some(AgentId::new("lander")));
    assert_eq!(ledger[0].checks, "before:skipped");
    assert_eq!(ledger[0].new, lane.repo.rev("main"));
}

#[test]
fn a_destination_that_moved_is_merged_again_and_checked_before_it_moves() {
    let lane = Lane::new(&["/bin/sh -c 'test -f agent-output.txt'"]);
    let task = lane.done("mover", "agent-output.txt", "made");
    assert_eq!(lane.status(&task), TaskStatus::Review);
    // The person moves main on after the task started.
    lane.repo.write("person.txt", "person\n");
    lane.repo.git(&["add", "person.txt"]);
    lane.repo.commit("The person's own work");
    let before = lane.repo.rev("main");

    lane.service.merge(&task).unwrap();

    assert_eq!(lane.status(&task), TaskStatus::Done);
    let ledger = lane.service.ledger();
    let last = ledger.last().unwrap();
    assert_eq!(last.checks, "before:pass");
    assert_eq!(last.old.as_deref(), Some(before.as_str()));
    // main is checked out here, so it was brought forward with --ff-only and
    // the person's files are the merge result.
    assert!(lane.repo.path().join("agent-output.txt").exists());
    assert!(lane.repo.path().join("person.txt").exists());
}

#[test]
fn a_conflict_blocks_the_task_and_touches_no_file() {
    let lane = Lane::new(&[]);
    let task = lane.done("clasher", "notes.md", "the agent's notes");
    lane.repo.write("notes.md", "the person's notes\n");
    lane.repo.commit_all("The person rewrites the notes");
    let before = lane.repo.rev("main");

    lane.service.merge(&task).unwrap();

    assert_eq!(lane.status(&task), TaskStatus::Blocked);
    assert!(
        lane.note(&task).contains("Conflicts with main"),
        "{}",
        lane.note(&task)
    );
    assert_eq!(lane.repo.rev("main"), before, "main moved");
    assert_eq!(lane.repo.read("notes.md"), "the person's notes\n");
    assert!(!lane.repo.path().join(".git/MERGE_HEAD").exists());
    assert!(lane.service.ledger().is_empty());
}

#[test]
fn a_candidate_that_fails_its_checks_goes_back_and_nothing_moves() {
    // Green on the task's own commit, red once the person's file is merged in.
    let lane = Lane::new(&["/bin/sh -c 'test ! -f person.txt'"]);
    let task = lane.done("red", "agent-output.txt", "made");
    assert_eq!(lane.status(&task), TaskStatus::Review);
    lane.repo.write("person.txt", "person\n");
    lane.repo.git(&["add", "person.txt"]);
    lane.repo.commit("The person's own work");
    let before = lane.repo.rev("main");

    lane.service.merge(&task).unwrap();

    assert_ne!(lane.status(&task), TaskStatus::Done);
    assert!(
        lane.note(&task)
            .contains("Merged with main, the checks fail"),
        "{}",
        lane.note(&task)
    );
    assert_eq!(lane.repo.rev("main"), before, "main moved");
    assert!(
        !lane.repo.path().join(".spagitty/farm/lane/main").exists(),
        "the scratch worktree was left"
    );
}

#[test]
fn a_task_merged_in_merger_is_in_the_ledger_too() {
    let lane = Lane::new(&[]);
    let task = lane.done("merged-by-hand", "agent-output.txt", "made");
    let branch = lane
        .service
        .farm()
        .unwrap()
        .task(&task)
        .unwrap()
        .branch
        .clone()
        .unwrap();
    lane.repo.git(&["merge", "--no-edit", "--no-ff", &branch]);

    lane.service.mark_merged(&task).unwrap();

    let ledger = lane.service.ledger();
    assert_eq!(ledger.len(), 1);
    assert_eq!(ledger[0].kind, LedgerKind::Merger);
    assert_eq!(ledger[0].old, None);
}
