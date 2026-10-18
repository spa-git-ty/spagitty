// SPDX-License-Identifier: GPL-3.0-or-later

//! Pipeline commands keep credentials and repository files on the Rust side.
use crate::{commands::AppState, forge_views as views};
use serde::{Deserialize, Serialize};
use spagitty_core::{forge::pipelines as host, pipelines as local, Error, Result};
use tauri::{AppHandle, Runtime, State};

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    pub branches: Vec<String>,
}
#[tauri::command(async)]
pub fn pipelines_preview(
    state: State<'_, AppState>,
    simulation: local::Simulation,
    mode: String,
    from: String,
    to: String,
) -> Result<local::Preview> {
    local::preview(
        &views::path(&state, "pipelines_preview")?,
        simulation,
        &mode,
        &from,
        &to,
    )
}
#[tauri::command(async)]
pub fn pipelines_source(state: State<'_, AppState>, file: String) -> Result<String> {
    local::source(&views::path(&state, "pipelines_source")?, &file)
}
#[tauri::command(async)]
pub fn pipelines_preferences<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    remote: String,
    branches: Option<Vec<String>>,
) -> Result<Preferences> {
    let path = views::path(&state, "pipelines_preferences")?;
    let entry = views::selected(&app, &path, &remote)?;
    let file = views::cache_file(&app, &entry, "pipeline-preferences", "")?;
    if let Some(mut branches) = branches {
        if branches.len() > 100
            || branches
                .iter()
                .any(|b| b.is_empty() || b.len() > 255 || b.contains(['\n', '\r', '\0']))
        {
            return Err(Error::Workflow(
                "Choose up to 100 branch names, each shorter than 256 characters.".into(),
            ));
        }
        branches.sort();
        branches.dedup();
        let result = Preferences { branches };
        views::save(&file, &result)?;
        Ok(result)
    } else {
        Ok(views::cached(&file).unwrap_or_default())
    }
}
#[tauri::command(async)]
pub fn pipelines_watches<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    remote: String,
) -> Result<views::Read<host::Watches>> {
    let entry = views::selected(&app, &views::path(&state, "pipelines_watches")?, &remote)?;
    let file = views::cache_file(&app, &entry, "pipeline-watches", "")?;
    let data = host::watches(
        &entry.repo,
        &views::token(&entry)?,
        entry.user.as_deref().unwrap_or(""),
    )?;
    Ok(views::keep(&file, data))
}
#[tauri::command(async)]
pub fn pipelines_cached<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    remote: String,
    branch: String,
) -> Result<Option<host::Page>> {
    let entry = views::selected(&app, &views::path(&state, "pipelines_cached")?, &remote)?;
    Ok(views::cached(&views::cache_file(
        &app,
        &entry,
        "pipelines",
        &branch,
    )?))
}
#[tauri::command(async)]
pub fn pipelines_runs<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    remote: String,
    branch: String,
) -> Result<views::Read<host::Page>> {
    let entry = views::selected(&app, &views::path(&state, "pipelines_runs")?, &remote)?;
    let file = views::cache_file(&app, &entry, "pipelines", &branch)?;
    let old = views::cached(&file);
    let data = host::page(&entry.repo, &views::token(&entry)?, &branch, old.as_ref())?;
    Ok(views::keep(&file, data))
}
#[tauri::command(async)]
pub fn pipelines_jobs<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    remote: String,
    id: u64,
    page: u32,
) -> Result<views::Read<host::Jobs>> {
    let entry = views::selected(&app, &views::path(&state, "pipelines_jobs")?, &remote)?;
    let file = views::cache_file(&app, &entry, "pipeline-jobs", &format!("{id}/{page}"))?;
    let old = views::cached(&file);
    let data = host::jobs(&entry.repo, &views::token(&entry)?, id, page, old.as_ref())?;
    Ok(views::keep(&file, data))
}
#[tauri::command(async)]
pub fn pipelines_graph<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    remote: String,
    id: u64,
) -> Result<views::Read<local::Workflow>> {
    let entry = views::selected(&app, &views::path(&state, "pipelines_graph")?, &remote)?;
    let file = views::cache_file(&app, &entry, "pipeline-graph", &id.to_string())?;
    if let Some(data) = views::cached(&file) {
        return Ok(views::Read {
            data,
            cache_warning: None,
        });
    }
    Ok(views::keep(
        &file,
        local::hosted(&entry.repo, &views::token(&entry)?, id)?,
    ))
}
#[tauri::command(async)]
pub fn pipelines_log<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    remote: String,
    id: u64,
) -> Result<String> {
    let entry = views::selected(&app, &views::path(&state, "pipelines_log")?, &remote)?;
    host::log(&entry.repo, &views::token(&entry)?, id)
}
#[tauri::command(async)]
pub fn pipelines_write<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    remote: String,
    action: host::Write,
    confirmed: bool,
) -> Result<serde_json::Value> {
    if !confirmed {
        return Err(Error::Workflow(
            "Confirm this pipeline action before sending it.".into(),
        ));
    }
    let path = views::path(&state, "pipelines_write")?;
    let entry = views::selected(&app, &path, &remote)?;
    let action = if matches!(action, host::Write::Lint { .. }) {
        host::Write::Lint {
            content: local::source(&path, ".gitlab-ci.yml")?,
        }
    } else {
        action
    };
    host::write(&entry.repo, &views::token(&entry)?, &action, confirmed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::Manager;
    #[test]
    fn writes_require_confirmation_before_any_account_read() {
        let app = crate::testing::app();
        assert!(pipelines_write(
            app.handle().clone(),
            app.state(),
            "origin".into(),
            host::Write::Cancel { id: 1 },
            false
        )
        .unwrap_err()
        .to_string()
        .contains("Confirm"));
        assert!(pipelines_preview(
            app.state(),
            local::Simulation::default(),
            "working".into(),
            String::new(),
            String::new()
        )
        .is_err());
    }
    #[test]
    fn watched_branches_are_local_and_an_unknown_remote_is_refused() {
        let f = spagitty_core::fixture::Fixture::linear(1);
        f.git(&[
            "remote",
            "add",
            "origin",
            "https://github.com/example/app.git",
        ]);
        let app = crate::testing::app();
        crate::commands::open_repo(
            app.handle().clone(),
            app.state(),
            f.path().to_path_buf(),
            None,
        )
        .unwrap();
        let p = pipelines_preferences(
            app.handle().clone(),
            app.state(),
            "origin".into(),
            Some(vec!["feature/test".into(), "feature/test".into()]),
        )
        .unwrap();
        assert_eq!(p.branches, vec!["feature/test"]);
        assert!(
            pipelines_preferences(app.handle().clone(), app.state(), "other".into(), None).is_err()
        );
        crate::commands::close_repo(app.state());
    }
}
