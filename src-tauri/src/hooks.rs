// SPDX-License-Identifier: GPL-3.0-or-later

//! Hooks' builder and run records (FEAT-137). The commands hold no session
//! lock while a hook runs; the repository watcher carries new output.

use crate::commands::AppState;
use serde::Serialize;
use spagitty_core::hooks::{
    self,
    blocks::{Action, Definition, Stage},
    builder,
};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use tauri::{AppHandle, Manager, Runtime, State};

type Result<T> = std::result::Result<T, String>;

fn repository(state: &AppState, command: &'static str) -> Result<PathBuf> {
    state.open_path(command).map_err(|e| e.to_string())
}

fn personal<R: Runtime>(app: &AppHandle<R>, repo: &Path) -> Result<PathBuf> {
    let name = format!(
        "{:016x}",
        hooks::generate::hash(repo.to_string_lossy().as_bytes())
    );
    Ok(app
        .path()
        .app_data_dir()
        .map_err(|e| e.to_string())?
        .join("hooks")
        .join(name)
        .join("hooks.toml"))
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Builder {
    definition: Definition,
    info: hooks::Hooks,
    runs: Vec<builder::Run>,
    staged: Vec<String>,
    installed: bool,
}

#[tauri::command(async)]
pub fn hooks_builder<R: Runtime>(app: AppHandle<R>, state: State<'_, AppState>) -> Result<Builder> {
    let repo = repository(&state, "hooks_builder")?;
    let definition = builder::load(&repo, &personal(&app, &repo)?)?;
    let info = hooks::list(&repo).map_err(|e| e.to_string())?;
    let installed = builder::installed(&repo, &definition)?;
    Ok(Builder {
        definition,
        info,
        runs: builder::runs(&repo)?,
        staged: spagitty_core::shell::hook_matches(&repo, &[]).map_err(|e| e.to_string())?,
        installed,
    })
}

#[tauri::command(async)]
pub fn hooks_save<R: Runtime>(
    app: AppHandle<R>,
    state: State<'_, AppState>,
    definition: Definition,
    install: bool,
    imported: bool,
) -> Result<()> {
    let repo = repository(&state, "hooks_save")?;
    static SAVES: std::sync::Mutex<()> = std::sync::Mutex::new(());
    let _guard = SAVES.lock().map_err(|e| e.to_string())?;
    // Preflight ownership before saving: a refused installation must not
    // quietly save a different definition behind the screen's error.
    if install {
        builder::preflight(&repo, &definition, true, imported)?;
    }
    builder::save(&repo, &personal(&app, &repo)?, &definition)?;
    if install {
        builder::scripts(&repo, &definition, true, imported)
            .map_err(|e| format!("The definition was saved, but hooks were not installed: {e}"))?;
    }
    Ok(())
}

#[tauri::command(async)]
pub fn hooks_import(state: State<'_, AppState>) -> Result<Definition> {
    builder::import(&repository(&state, "hooks_import")?)
}

#[tauri::command(async)]
pub fn hooks_matches(state: State<'_, AppState>, paths: Vec<String>) -> Result<Vec<String>> {
    spagitty_core::shell::hook_matches(&repository(&state, "hooks_matches")?, &paths)
        .map_err(|e| e.to_string())
}

#[tauri::command(async)]
pub fn hooks_runs(state: State<'_, AppState>) -> Result<Vec<builder::Run>> {
    builder::runs(&repository(&state, "hooks_runs")?)
}

#[tauri::command(async)]
pub fn hooks_run(state: State<'_, AppState>, id: String) -> Result<builder::Run> {
    builder::run(&repository(&state, "hooks_run")?, &id)
}

#[tauri::command(async)]
pub fn hooks_log(state: State<'_, AppState>, id: String, block: String) -> Result<String> {
    builder::log(&repository(&state, "hooks_log")?, &id, &block)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Tried {
    passed: bool,
    output: String,
}

#[tauri::command(async)]
pub fn hooks_dry_run(
    state: State<'_, AppState>,
    definition: Definition,
    stage: Stage,
) -> Result<Tried> {
    if stage == Stage::CommitMsg {
        return Err("Use Try a message for commit-msg.".into());
    }
    let repo = repository(&state, "hooks_dry_run")?;
    let directory = builder::scripts(&repo, &definition, false, false)?;
    Ok(tried(spagitty_core::shell::hook_run(
        &repo,
        &directory,
        stage.name(),
        None,
    )))
}

fn tried(result: spagitty_core::Result<String>) -> Tried {
    match result {
        Ok(output) => Tried {
            passed: true,
            output,
        },
        Err(e) => Tried {
            passed: false,
            output: e.to_string(),
        },
    }
}

#[tauri::command(async)]
pub fn hooks_try_message(
    state: State<'_, AppState>,
    definition: Definition,
    message: String,
) -> Result<Tried> {
    if message.len() > 65536 || message.contains('\0') {
        return Err("The message is too long or contains a NUL character.".into());
    }
    let repo = repository(&state, "hooks_try_message")?;
    let mut definition = definition;
    definition.blocks.retain(|block| {
        block.stage == Stage::CommitMsg && matches!(block.action, Action::Message { .. })
    });
    if definition.blocks.is_empty() {
        return Err("This definition has no commit-message rule to try.".into());
    }
    let directory = builder::scripts(&repo, &definition, false, false)?;
    static NEXT: AtomicU64 = AtomicU64::new(1);
    let temporary = builder::folder(&repo)?.join(format!(
        "message-{}-{}",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::create_dir_all(&temporary).map_err(|e| e.to_string())?;
    let file = temporary.join("try-message");
    std::fs::write(&file, message).map_err(|e| e.to_string())?;
    let result = tried(spagitty_core::shell::hook_run(
        &repo,
        &directory,
        "commit-msg",
        Some(&file),
    ));
    let _ = std::fs::remove_file(&file);
    let _ = std::fs::remove_dir(&temporary);
    Ok(result)
}

#[tauri::command(async)]
pub fn hooks_stop(state: State<'_, AppState>, id: String) -> Result<()> {
    builder::stop(&repository(&state, "hooks_stop")?, &id)
}

#[tauri::command(async)]
pub fn hooks_undo_restage(state: State<'_, AppState>, id: String) -> Result<()> {
    let repo = repository(&state, "hooks_undo_restage")?;
    let run = builder::run(&repo, &id)?;
    if run.exit.is_none() {
        return Err("Wait until the hook run ends before undoing its restage.".into());
    }
    for event in run
        .events
        .iter()
        .rev()
        .filter(|event| event["t"] == "restaged")
    {
        let field = |name: &str| {
            event[name]
                .as_str()
                .ok_or_else(|| "This record has no restage backup.".to_string())
        };
        let file = event["files"][0]
            .as_str()
            .ok_or("This record has no restaged path.")?;
        spagitty_core::shell::hook_restore_index(
            &repo,
            file,
            field("before")?,
            field("after")?,
            field("mode")?,
        )
        .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{commands, testing};
    use spagitty_core::{
        fixture::Fixture,
        hooks::blocks::{Action, Block},
    };

    #[test]
    fn every_read_and_write_requires_an_open_repository() {
        let app = testing::app();
        assert!(hooks_builder(app.handle().clone(), app.state()).is_err());
        assert!(hooks_runs(app.state()).is_err());
        assert!(hooks_import(app.state()).is_err());
        assert!(hooks_matches(app.state(), Vec::new()).is_err());
    }

    #[test]
    fn the_commands_install_dry_run_and_try_the_generated_message_rules() {
        let fixture = Fixture::linear(1);
        let app = testing::app();
        commands::open_repo(
            app.handle().clone(),
            app.state(),
            fixture.path().to_path_buf(),
            None,
        )
        .expect("open");
        let mut definition = Definition::default();
        definition.blocks.push(Block {
            id: "message".into(),
            label: "Message".into(),
            stage: Stage::CommitMsg,
            enabled: true,
            warn: false,
            folder: ".".into(),
            timeout_seconds: 30,
            skip_markdown: false,
            skip_wip: false,
            action: Action::Message {
                style: "conventional".into(),
                types: vec!["feat".into(), "fix".into()],
                scope_required: false,
                subject_max: 72,
                pattern: String::new(),
                ticket_pattern: String::new(),
            },
        });
        hooks_save(
            app.handle().clone(),
            app.state(),
            definition.clone(),
            true,
            false,
        )
        .expect("save");
        assert!(
            hooks_builder(app.handle().clone(), app.state())
                .expect("builder")
                .installed
        );
        assert!(
            hooks_try_message(app.state(), definition.clone(), "feat: add a thing".into())
                .expect("try")
                .passed
        );
        assert!(
            !hooks_try_message(app.state(), definition.clone(), "wrong".into())
                .expect("try")
                .passed
        );
        let mut strict = definition.clone();
        if let Action::Message {
            scope_required,
            ticket_pattern,
            ..
        } = &mut strict.blocks[0].action
        {
            *scope_required = true;
            *ticket_pattern = "^Refs: APP-[0-9]+$".into();
        }
        for (text, passed) in [
            ("feat(api): add a thing\n\nRefs: APP-12", true),
            ("feat: add a thing\n\nRefs: APP-12", false),
            (
                "feat(api): add a thing\n\nRefs: APP-12\n\nFurther explanation",
                false,
            ),
        ] {
            assert_eq!(
                hooks_try_message(app.state(), strict.clone(), text.into())
                    .expect("strict message")
                    .passed,
                passed,
                "{text}"
            );
        }
        assert!(
            hooks_dry_run(app.state(), definition, Stage::PreCommit)
                .expect("dry run")
                .passed
        );
        commands::close_repo(app.state());
    }
}
