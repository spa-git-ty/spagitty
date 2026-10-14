// SPDX-License-Identifier: GPL-3.0-or-later

//! Thin issue commands. Cached reads need no credential; network calls and
//! writes resolve the selected remote and read its credential in Rust.

use crate::{commands::AppState, forge_views as views};
use spagitty_core::{
    forge::{issues, Kind},
    Error, Result,
};
use tauri::{AppHandle, Runtime, State};

fn page_key(query: &str, page: u32) -> String {
    format!("{query}\npage:{page}")
}

#[tauri::command(async)]
pub fn issues_remotes<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
) -> Result<Vec<views::Hosted>> {
    views::remotes(&app, &views::path(&state, "issues_remotes")?)
}
#[tauri::command(async)]
pub fn issues_cached<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    remote: String,
    query: String,
    page: u32,
) -> Result<Option<issues::Page>> {
    let entry = views::selected(&app, &views::path(&state, "issues_cached")?, &remote)?;
    Ok(views::cached(&views::cache_file(
        &app,
        &entry,
        "issues",
        &page_key(&query, page),
    )?))
}
#[tauri::command(async)]
pub fn issues_page<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    remote: String,
    query: String,
    page: u32,
) -> Result<views::Read<issues::Page>> {
    let entry = views::selected(&app, &views::path(&state, "issues_page")?, &remote)?;
    let file = views::cache_file(&app, &entry, "issues", &page_key(&query, page))?;
    let old = views::cached::<issues::Page>(&file);
    let mut filter = issues::filter(&query, entry.user.as_deref().unwrap_or(""))?;
    let metadata_file = views::cache_file(&app, &entry, "issues-meta", "")?;
    let metadata = views::cached::<issues::Metadata>(&metadata_file);
    if entry.repo.kind == Kind::GitHub && filter.search.is_empty() {
        if let Some(milestone) = metadata.as_ref().and_then(|metadata| {
            metadata
                .milestones
                .iter()
                .find(|m| m.title == filter.milestone)
        }) {
            filter.milestone = milestone.id.clone();
        }
    }
    let mut data = issues::page(
        &entry.repo,
        &views::token(&entry)?,
        &filter,
        page,
        old.as_ref(),
    )?;
    if let Some(metadata) = metadata {
        for issue in &mut data.issues {
            for label in &mut issue.labels {
                if let Some(found) = metadata.labels.iter().find(|l| l.name == label.name) {
                    label.color = found.color.clone();
                }
            }
        }
    }
    Ok(views::keep(&file, data))
}
#[tauri::command(async)]
pub fn issues_metadata<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    remote: String,
    refresh: bool,
) -> Result<views::Read<issues::Metadata>> {
    let entry = views::selected(&app, &views::path(&state, "issues_metadata")?, &remote)?;
    let file = views::cache_file(&app, &entry, "issues-meta", "")?;
    if !refresh {
        if let Some(data) = views::cached(&file) {
            return Ok(views::Read {
                data,
                cache_warning: None,
            });
        }
    }
    let data = issues::metadata(&entry.repo, &views::token(&entry)?)?;
    Ok(views::keep(&file, data))
}
#[tauri::command(async)]
pub fn issues_thread<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    remote: String,
    number: u64,
    comments_page: u32,
    events_page: u32,
) -> Result<issues::Thread> {
    let entry = views::selected(&app, &views::path(&state, "issues_thread")?, &remote)?;
    issues::thread(
        &entry.repo,
        &views::token(&entry)?,
        number,
        comments_page,
        events_page,
    )
}
#[tauri::command(async)]
pub fn issues_write<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    remote: String,
    action: issues::Write,
    confirmed: bool,
) -> Result<serde_json::Value> {
    if !confirmed {
        return Err(Error::NotStageable(
            "Confirm this issue action before sending it.".into(),
        ));
    }
    let entry = views::selected(&app, &views::path(&state, "issues_write")?, &remote)?;
    issues::write(&entry.repo, &views::token(&entry)?, &action, confirmed)
}

#[tauri::command(async)]
pub fn issues_branch<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    remote: String,
    name: String,
    confirmed: bool,
) -> Result<()> {
    if !confirmed {
        return Err(Error::NotStageable(
            "Confirm the branch before creating it.".into(),
        ));
    }
    let path = views::path(&state, "issues_branch")?;
    let entry = views::selected(&app, &path, &remote)?;
    let metadata: issues::Metadata =
        views::cached(&views::cache_file(&app, &entry, "issues-meta", "")?).ok_or_else(|| {
            Error::NotStageable("Read the repository's default branch first.".into())
        })?;
    if metadata.default_branch.is_empty() {
        return Err(Error::NotStageable(
            "The host has not named a default branch.".into(),
        ));
    }
    let start = format!("refs/remotes/{remote}/{}", metadata.default_branch);
    if spagitty_core::shell::commit_id(&path, &start).is_err() {
        let spec = format!("refs/heads/{}:{start}", metadata.default_branch);
        spagitty_core::shell::fetch_refspecs(&path, &remote, &[&spec])?;
    }
    let git = spagitty_core::repo::open(&path)?;
    spagitty_core::branches::create(&git, &name, &start, true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing;
    use tauri::Manager;
    #[test]
    fn reads_need_an_open_repository_and_writes_need_confirmation_first() {
        let app = testing::app();
        assert!(issues_remotes(app.handle().clone(), app.state()).is_err());
        assert!(issues_write(
            app.handle().clone(),
            app.state(),
            "origin".into(),
            issues::Write::Close { number: 1 },
            false
        )
        .unwrap_err()
        .to_string()
        .contains("Confirm"));
        assert!(issues_branch(
            app.handle().clone(),
            app.state(),
            "origin".into(),
            "fix/1".into(),
            false
        )
        .is_err());
    }
    #[test]
    fn a_remote_outside_the_repository_is_refused_before_credentials() {
        let fixture = spagitty_core::fixture::Fixture::linear(1);
        let app = testing::app();
        crate::commands::open_repo(
            app.handle().clone(),
            app.state(),
            fixture.path().to_path_buf(),
            None,
        )
        .unwrap();
        assert!(issues_cached(
            app.handle().clone(),
            app.state(),
            "https://other.example".into(),
            String::new(),
            1
        )
        .is_err());
        crate::commands::close_repo(app.state());
    }

    #[test]
    fn issue_branch_starts_at_default_tracking_tip_and_keeps_untracked_work() {
        let fixture = spagitty_core::fixture::Fixture::linear(2);
        fixture.git(&[
            "remote",
            "add",
            "origin",
            "https://github.com/example/app.git",
        ]);
        fixture.git(&["update-ref", "refs/remotes/origin/main", "HEAD"]);
        fixture.write("personal.txt", "keep this work\n");
        let tip = spagitty_core::shell::commit_id(fixture.path(), "HEAD").unwrap();
        let app = testing::app();
        crate::commands::open_repo(
            app.handle().clone(),
            app.state(),
            fixture.path().to_path_buf(),
            None,
        )
        .unwrap();
        let entry = views::selected(app.handle(), fixture.path(), "origin").unwrap();
        views::save(
            &views::cache_file(app.handle(), &entry, "issues-meta", "").unwrap(),
            &issues::Metadata {
                default_branch: "main".into(),
                open: 1,
                closed: 0,
                labels: vec![],
                milestones: vec![],
            },
        )
        .unwrap();
        issues_branch(
            app.handle().clone(),
            app.state(),
            "origin".into(),
            "fix/12-example".into(),
            true,
        )
        .unwrap();
        assert_eq!(
            spagitty_core::shell::commit_id(fixture.path(), "HEAD").unwrap(),
            tip
        );
        assert_eq!(
            fixture.git(&["branch", "--show-current"]).trim(),
            "fix/12-example"
        );
        assert_eq!(
            std::fs::read_to_string(fixture.path().join("personal.txt")).unwrap(),
            "keep this work\n"
        );
        crate::commands::close_repo(app.state());
    }
}
