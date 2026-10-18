// SPDX-License-Identifier: GPL-3.0-or-later

//! What starting a farm one step at a time needs from the service (FEAT-141):
//! the farm it replaces kept, an earlier one put back, the crew and rules
//! remembered, and a goal that starts from something.
//!
//! No agent runs here, so these run on every platform.

use std::sync::Arc;

use spagitty_core::fixture::Fixture;
use spagitty_farm::model::*;
use spagitty_farm::persistence::store;
use spagitty_farm::service::{FarmService, Recorder};

fn service(repo: &Fixture) -> FarmService {
    FarmService::open(repo.path(), Arc::new(Recorder::default()))
}

fn add_task(service: &FarmService, title: &str) -> TaskId {
    let task = Task::new(TaskId::new("unset"), title, 0);
    service.add_task(task).unwrap().id
}

#[test]
fn a_farm_with_work_in_it_is_kept_when_a_new_one_starts() {
    let repo = Fixture::woven();
    let farm = service(&repo);
    farm.create("Hooks tab, step by step", "").unwrap();
    add_task(&farm, "Write the screen");

    farm.create_from("Gitea support", "", Vec::new()).unwrap();

    let past = farm.past();
    assert_eq!(past.len(), 1);
    assert_eq!(past[0].goal.title, "Hooks tab, step by step");
    assert_eq!((past[0].merged, past[0].total), (0, 1));
    assert_eq!(farm.farm().unwrap().goal.title, "Gitea support");
}

#[test]
fn a_farm_that_was_never_planned_is_not_kept() {
    let repo = Fixture::woven();
    let farm = service(&repo);
    farm.create("Abandoned before planning", "").unwrap();
    farm.create("The real one", "").unwrap();
    assert!(farm.past().is_empty());
}

#[test]
fn an_earlier_farm_is_put_back_and_the_current_one_kept_in_its_place() {
    let repo = Fixture::woven();
    let farm = service(&repo);
    let first = farm.create("First", "").unwrap();
    add_task(&farm, "one");
    farm.create("Second", "").unwrap();
    add_task(&farm, "two");

    let resumed = farm.resume(&first.id).unwrap();

    assert_eq!(resumed.goal.title, "First");
    assert_eq!(farm.farm().unwrap().goal.title, "First");
    // On disk too, so a restart opens the resumed farm.
    assert_eq!(store::load_farm(repo.path()).unwrap().goal.title, "First");
    let past: Vec<_> = farm.past().into_iter().map(|p| p.goal.title).collect();
    assert_eq!(past, vec!["Second".to_string()]);
    assert!(
        farm.resume(&first.id).is_err(),
        "it is no longer in the archive"
    );
}

#[test]
fn the_crew_and_rules_are_remembered_with_the_farm_they_came_from() {
    let repo = Fixture::woven();
    let farm = service(&repo);
    assert!(farm.remembered().is_none());
    farm.create("GitLab support for pull requests", "").unwrap();
    farm.configure(|f| {
        f.autonomy = Autonomy::SemiAuto;
        f.max_parallel = 3;
        f.verification = vec!["cargo test --workspace".into()];
        f.agents = vec![AgentId::new("claude")];
        f.workspace = WorkspaceMode::Slice;
    })
    .unwrap();

    farm.remember().unwrap();

    // Read back by a service opened afresh, as the next farm's wizard is.
    let again = service(&repo);
    let setup = again.remembered().expect("remembered");
    assert_eq!(setup.from_title, "GitLab support for pull requests");
    assert_eq!(setup.max_parallel, 3);
    assert_eq!(
        setup.verification,
        vec!["cargo test --workspace".to_string()]
    );
    assert_eq!(setup.agents, vec![AgentId::new("claude")]);
    assert_eq!(setup.workspace, WorkspaceMode::Slice);
}

#[test]
fn a_goal_keeps_what_it_starts_from_across_a_restart() {
    let repo = Fixture::woven();
    let farm = service(&repo);
    farm.create_from(
        "Gitea support",
        "Same Host trait as GitHub.",
        vec![
            GoalSource::Issue {
                number: 142,
                title: "Gitea: list and open pull requests".into(),
                body: String::new(),
                labels: vec!["feature".into()],
                url: String::new(),
            },
            GoalSource::Branch {
                name: "feature/split-view".into(),
            },
        ],
    )
    .unwrap();

    let goal = service(&repo).farm().unwrap().goal;
    assert_eq!(goal.sources.len(), 2);
    assert_eq!(goal.branch(), Some("feature/split-view"));
}

#[test]
fn a_farm_written_before_reads_as_worktrees_with_nothing_to_start_from() {
    let repo = Fixture::woven();
    let farm = service(&repo);
    farm.create("Old", "").unwrap();
    // Strip what this build added, as a file from 3.1 would have it.
    let path = store::farm_path(repo.path());
    let mut json: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
    json.as_object_mut().unwrap().remove("workspace");
    json["goal"].as_object_mut().unwrap().remove("sources");
    std::fs::write(&path, json.to_string()).unwrap();

    let read = service(&repo).farm().unwrap();
    assert_eq!(read.workspace, WorkspaceMode::Worktree);
    assert!(read.goal.sources.is_empty());
}
