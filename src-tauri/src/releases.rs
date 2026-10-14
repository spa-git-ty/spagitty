// SPDX-License-Identifier: GPL-3.0-or-later

use crate::{commands::AppState, forge_views as views};
use serde::{Deserialize, Serialize};
use spagitty_core::{
    forge::{releases, Kind},
    Error, Result,
};
use std::path::PathBuf;
use tauri::{ipc::Channel, AppHandle, Runtime, State};

static PREFERENCES_WRITES: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn save_preferences(path: &std::path::Path, preferences: &Preferences) -> Result<()> {
    let bytes =
        serde_json::to_vec(preferences).map_err(|error| Error::Config(error.to_string()))?;
    if bytes.len() > 8 * 1024 * 1024 {
        return Err(Error::Config(
            "Local release drafts exceed 8 MiB. Remove an old draft before saving more.".into(),
        ));
    }
    views::save(path, preferences)
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Preferences {
    pub hidden: Vec<String>,
    pub drafts: Vec<releases::Release>,
}

#[tauri::command(async)]
pub fn releases_preferences<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    remote: String,
) -> Result<Preferences> {
    let entry = views::selected(&app, &views::path(&state, "releases_preferences")?, &remote)?;
    Ok(views::cached(&views::cache_file(&app, &entry, "releases-local", "")?).unwrap_or_default())
}

#[tauri::command(async)]
pub fn releases_hide<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    remote: String,
    id: String,
    hidden: bool,
) -> Result<Preferences> {
    let entry = views::selected(&app, &views::path(&state, "releases_hide")?, &remote)?;
    let file = views::cache_file(&app, &entry, "releases-local", "")?;
    let _guard = PREFERENCES_WRITES
        .lock()
        .map_err(|error| Error::Config(error.to_string()))?;
    let mut preferences: Preferences = views::cached(&file).unwrap_or_default();
    preferences.hidden.retain(|old| old != &id);
    if hidden {
        preferences.hidden.push(id);
    }
    save_preferences(&file, &preferences)?;
    Ok(preferences)
}

#[tauri::command(async)]
pub fn releases_cached<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    remote: String,
    page: u32,
) -> Result<Option<releases::Page>> {
    let entry = views::selected(&app, &views::path(&state, "releases_cached")?, &remote)?;
    Ok(views::cached(&views::cache_file(
        &app,
        &entry,
        "releases",
        &page.to_string(),
    )?))
}

#[tauri::command(async)]
pub fn releases_page<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    remote: String,
    page: u32,
) -> Result<views::Read<releases::Page>> {
    let entry = views::selected(&app, &views::path(&state, "releases_page")?, &remote)?;
    let file = views::cache_file(&app, &entry, "releases", &page.to_string())?;
    let data = releases::page(
        &entry.repo,
        &views::token(&entry)?,
        page,
        views::cached::<releases::Page>(&file).as_ref(),
    )?;
    Ok(views::keep(&file, data))
}

#[tauri::command(async)]
pub fn releases_write<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    remote: String,
    action: releases::Write,
    confirmed: bool,
) -> Result<serde_json::Value> {
    if !confirmed {
        return Err(Error::NotStageable(
            "Confirm this release action first.".into(),
        ));
    }
    let entry = views::selected(&app, &views::path(&state, "releases_write")?, &remote)?;
    let file = views::cache_file(&app, &entry, "releases-local", "")?;
    if entry.repo.kind == Kind::GitLab {
        let _guard = PREFERENCES_WRITES
            .lock()
            .map_err(|error| Error::Config(error.to_string()))?;
        let mut preferences: Preferences = views::cached(&file).unwrap_or_default();
        match &action {
            releases::Write::Save {
                release,
                publish: false,
            } => {
                if !release.local && !release.id.is_empty() {
                    return Err(Error::NotStageable(
                        "Published GitLab releases are saved on the host.".into(),
                    ));
                }
                if release.tag.trim().is_empty()
                    || release.tag.len() > 256
                    || release.title.len() > 256
                    || release.body.len() > 125000
                {
                    return Err(Error::NotStageable(
                        "Choose a draft tag, title and notes within the host's size limits.".into(),
                    ));
                }
                let mut release = release.clone();
                release.local = true;
                release.draft = true;
                if release.id.is_empty() {
                    release.id = format!(
                        "local:{:016x}",
                        spagitty_core::hooks::generate::hash(release.tag.as_bytes())
                    );
                }
                preferences.drafts.retain(|old| old.id != release.id);
                preferences.drafts.insert(0, release.clone());
                save_preferences(&file, &preferences)?;
                return serde_json::to_value(release).map_err(|e| Error::Config(e.to_string()));
            }
            releases::Write::Delete { id, .. } if id.starts_with("local:") => {
                preferences.drafts.retain(|release| release.id != *id);
                save_preferences(&file, &preferences)?;
                return Ok(serde_json::Value::Null);
            }
            _ => {}
        }
    }
    let token = views::token(&entry)?;
    let mut action = action;
    if let releases::Write::Save { release, publish } = &mut action {
        release.target = releases::target(&entry.repo, &token, &release.target)?;
        if *publish {
            if let Some(tag_target) = releases::tag_target(&entry.repo, &token, &release.tag)? {
                if release.draft && tag_target != release.target {
                    return Err(Error::NotStageable(format!("The tag already exists at {tag_target}. Choose that target explicitly or choose a new tag. Existing tags are never moved.")));
                }
                release.target = tag_target;
            }
        }
    }
    let answer = releases::write(&entry.repo, &token, &action, true)?;
    if let releases::Write::Save {
        release,
        publish: true,
    } = &action
    {
        if release.local {
            let _guard = PREFERENCES_WRITES
                .lock()
                .map_err(|error| Error::Config(error.to_string()))?;
            let mut preferences: Preferences = views::cached(&file).unwrap_or_default();
            preferences.drafts.retain(|old| old.id != release.id);
            // Publishing succeeded even if local cleanup fails; report that distinct partial success.
            save_preferences(&file, &preferences).map_err(|error| Error::Config(format!("Release published, but its local draft could not be removed: {error}. Refresh the host before retrying.")))?;
        }
    }
    Ok(answer)
}

#[tauri::command(async)]
pub fn releases_notes(state: State<'_, AppState>, since: String, target: String) -> Result<String> {
    let path = views::path(&state, "releases_notes")?;
    let target = spagitty_core::shell::commit_id(&path, &target)?;
    let range = if since.is_empty() {
        target
    } else {
        format!(
            "{}..{target}",
            spagitty_core::shell::commit_id(&path, &since)?
        )
    };
    Ok(releases::notes(&spagitty_core::shell::commit_records(
        &path, &range, 10000,
    )?))
}

#[tauri::command(async)]
pub fn releases_target<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    remote: String,
    revision: String,
) -> Result<String> {
    let entry = views::selected(&app, &views::path(&state, "releases_target")?, &remote)?;
    releases::target(&entry.repo, &views::token(&entry)?, &revision)
}

#[tauri::command(async)]
#[allow(clippy::too_many_arguments)]
pub fn releases_changelog<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    remote: String,
    branch: String,
    expected: String,
    tag: String,
    body: String,
    confirmed: bool,
) -> Result<String> {
    if !confirmed {
        return Err(Error::NotStageable(
            "Confirm the changelog commit first.".into(),
        ));
    }
    let path = views::path(&state, "releases_changelog")?;
    let entry = views::selected(&app, &path, &remote)?;
    let token = views::token(&entry)?;
    if releases::tag_target(&entry.repo, &token, &tag)?.is_some() {
        return Err(Error::NotStageable("Choose a new tag before committing the release changelog. Existing tags are never moved.".into()));
    }
    if releases::target(&entry.repo, &token, &branch)? != expected {
        return Err(Error::NotStageable(
            "The target branch moved on the host. Refresh and review the target before publishing."
                .into(),
        ));
    }
    releases::commit_changelog(&path, &branch, &expected, &tag, &body)
}

#[tauri::command(async)]
#[allow(clippy::too_many_arguments)]
pub fn releases_push_target<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    remote: String,
    branch: String,
    expected: String,
    commit: String,
    confirmed: bool,
) -> Result<()> {
    if !confirmed {
        return Err(Error::NotStageable(
            "Confirm the changelog push first.".into(),
        ));
    }
    let path = views::path(&state, "releases_push_target")?;
    let entry = views::selected(&app, &path, &remote)?;
    if releases::target(&entry.repo, &views::token(&entry)?, &branch)? != expected
        || spagitty_core::shell::commit_id(&path, "HEAD")? != commit
    {
        return Err(Error::NotStageable(
            "The target moved. The local changelog commit remains; refresh before publishing."
                .into(),
        ));
    }
    if !spagitty_core::shell::valid_branch_name(&path, &branch)? {
        return Err(Error::NotStageable("Choose a valid branch.".into()));
    }
    spagitty_core::shell::push(
        &path,
        &remote,
        &format!("{commit}:refs/heads/{branch}"),
        false,
    )?;
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    pub sent: u64,
    pub total: u64,
}

#[tauri::command(async)]
#[allow(clippy::too_many_arguments)]
pub fn releases_upload<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    remote: String,
    release: releases::Release,
    path: PathBuf,
    confirmed: bool,
    progress: Channel<Progress>,
) -> Result<releases::Asset> {
    if !confirmed {
        return Err(Error::NotStageable("Confirm the file upload first.".into()));
    }
    let entry = views::selected(&app, &views::path(&state, "releases_upload")?, &remote)?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| Error::NotStageable("Choose a file with a valid name.".into()))?;
    let token = views::token(&entry)?;
    let (url, put) = releases::upload_url(&entry.repo, &release, name)?;
    let total = std::fs::metadata(&path)?.len();
    let response = spagitty_core::forge::http::upload_file(
        &url,
        &token,
        &entry.repo.host,
        &path,
        put,
        |sent| {
            let _ = progress.send(Progress { sent, total });
        },
    )?;
    let answer = releases::checked(&entry.repo, response)?;
    if entry.repo.kind == Kind::GitHub {
        Ok(releases::Asset {
            id: answer["id"].to_string(),
            name: name.into(),
            url: answer["browser_download_url"].as_str().unwrap_or("").into(),
            size: Some(total),
            downloads: Some(0),
            removable: true,
        })
    } else {
        // Linking a published release is a second host write, confirmed in the UI.
        Ok(releases::Asset {
            id: format!("package:{name}"),
            name: name.into(),
            url,
            size: Some(total),
            downloads: None,
            removable: true,
        })
    }
}

#[tauri::command(async)]
pub fn releases_artifacts<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    remote: String,
    commit: String,
) -> Result<Vec<releases::Artifact>> {
    let entry = views::selected(&app, &views::path(&state, "releases_artifacts")?, &remote)?;
    releases::artifacts(&entry.repo, &views::token(&entry)?, &commit)
}

#[tauri::command(async)]
pub fn releases_download_artifact<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    remote: String,
    artifact: releases::Artifact,
    progress: Channel<Progress>,
) -> Result<PathBuf> {
    let entry = views::selected(
        &app,
        &views::path(&state, "releases_download_artifact")?,
        &remote,
    )?;
    let token = views::token(&entry)?;
    let url = releases::artifact_url(&entry.repo, &artifact.id)?;
    let key = views::cache_file(&app, &entry, "release-artifacts", &artifact.id)?;
    let directory = key.with_extension("");
    std::fs::create_dir_all(&directory)?;
    let name: String = artifact
        .name
        .chars()
        .take(120)
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let name = if name.is_empty() || name == "." || name == ".." {
        "artifact.zip"
    } else {
        &name
    };
    let path = directory.join(name);
    let mut file = std::fs::File::create(&path)?;
    let result = spagitty_core::forge::http::download_file(
        &url,
        &token,
        &entry.repo.host,
        &mut file,
        |sent, total| {
            let _ = progress.send(Progress { sent, total });
        },
    );
    drop(file);
    if let Err(error) = result {
        let _ = std::fs::remove_file(&path);
        return Err(error);
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tauri::Manager;

    #[test]
    fn writes_refuse_without_confirmation_before_opening_a_repository() {
        let app = crate::testing::app();
        assert!(releases_write(
            app.handle().clone(),
            app.state(),
            "origin".into(),
            releases::Write::Delete {
                id: "12".into(),
                tag: "v1".into()
            },
            false
        )
        .unwrap_err()
        .to_string()
        .contains("Confirm"));
    }

    #[test]
    fn gitlab_drafts_and_hidden_ids_are_local_and_scoped_to_the_remote_repository() {
        let fixture = spagitty_core::fixture::Fixture::linear(1);
        fixture.git(&[
            "remote",
            "add",
            "origin",
            "https://gitlab.com/release-fixture/app.git",
        ]);
        fixture.git(&[
            "remote",
            "add",
            "other",
            "https://gitlab.com/release-fixture/other.git",
        ]);
        let app = crate::testing::app();
        crate::commands::open_repo(
            app.handle().clone(),
            app.state(),
            fixture.path().to_path_buf(),
            None,
        )
        .unwrap();
        let hidden = releases_hide(
            app.handle().clone(),
            app.state(),
            "origin".into(),
            "v1".into(),
            true,
        )
        .unwrap();
        assert_eq!(hidden.hidden, ["v1"]);
        let draft = releases::Release {
            tag: "v2".into(),
            target: "main".into(),
            title: "Next".into(),
            ..Default::default()
        };
        let result = releases_write(
            app.handle().clone(),
            app.state(),
            "origin".into(),
            releases::Write::Save {
                release: draft,
                publish: false,
            },
            true,
        )
        .unwrap();
        assert_eq!(result["local"], true);
        assert_eq!(result["draft"], true);
        let saved =
            releases_preferences(app.handle().clone(), app.state(), "origin".into()).unwrap();
        assert_eq!(saved.drafts[0].tag, "v2");
        let other =
            releases_preferences(app.handle().clone(), app.state(), "other".into()).unwrap();
        assert!(other.drafts.is_empty());
        assert!(other.hidden.is_empty());
        releases_write(
            app.handle().clone(),
            app.state(),
            "origin".into(),
            releases::Write::Delete {
                id: saved.drafts[0].id.clone(),
                tag: "v2".into(),
            },
            true,
        )
        .unwrap();
        assert!(
            releases_preferences(app.handle().clone(), app.state(), "origin".into())
                .unwrap()
                .drafts
                .is_empty()
        );
        crate::commands::close_repo(app.state());
    }
}
