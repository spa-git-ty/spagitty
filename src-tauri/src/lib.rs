// SPDX-License-Identifier: GPL-3.0-or-later

//! Spagitty's Tauri shell.
//!
//! This crate owns the window, the commands, and the background workers
//! (history walking, log searching and filesystem watching). All git logic
//! lives in `spagitty-core`.

mod about;
mod accounts;
mod clone_worker;
mod command_log;
mod commands;
mod farm;
mod graph_worker;
mod network_worker;
mod platform;
mod profiles;
mod rebase_worker;
mod recents;
mod search_worker;
mod settings;
#[cfg(test)]
mod testing;
mod watch;

use tauri::Manager;

pub fn run() {
    // Before the builder, because the webview reads its environment as it
    // starts and this process is still single-threaded here.
    platform::prepare_webview();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .manage(commands::AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::open_repo,
            commands::close_repo,
            commands::graph_request,
            commands::graph_restart,
            commands::snapshot,
            commands::commit_detail,
            commands::commit_diff,
            commands::file_diff,
            commands::binary_file_diff,
            commands::working_copy,
            commands::working_diff,
            commands::binary_working_diff,
            commands::stage,
            commands::unstage,
            commands::stage_hunk,
            commands::unstage_hunk,
            commands::discard,
            commands::discard_hunk,
            commands::commit,
            commands::head_message,
            commands::branches,
            commands::checkout,
            commands::create_branch,
            commands::rebase_todo,
            commands::rebase_preview,
            commands::rebase_run,
            commands::rebase_progress,
            commands::rebase_continue,
            commands::rebase_skip,
            commands::rebase_abort,
            commands::graph_visibility,
            commands::reset,
            commands::revert,
            commands::cherry_pick,
            commands::integrate,
            commands::rebase_onto,
            commands::checkout_detached,
            commands::rename_branch,
            commands::delete_branch,
            commands::create_tag,
            commands::delete_tag,
            commands::stash_action,
            commands::pull,
            commands::fetch,
            commands::push,
            commands::search_start,
            commands::search_stop,
            commands::blame,
            commands::file_history,
            commands::conflicts,
            commands::conflict_sides,
            commands::conflict_regions,
            commands::conflict_take,
            commands::conflict_resolve_region,
            commands::conflict_write,
            commands::conflict_resolve,
            commands::conflict_continue,
            commands::conflict_abort,
            commands::remotes,
            commands::remote_add,
            commands::remote_rename,
            commands::remote_remove,
            commands::remote_set_url,
            commands::reflog,
            commands::reflog_refs,
            commands::tags,
            commands::tag_create,
            commands::tag_delete,
            commands::tag_retag,
            commands::worktrees,
            commands::worktree_add,
            commands::worktree_remove,
            commands::worktree_lock,
            commands::worktree_unlock,
            commands::worktree_prune,
            commands::submodules,
            commands::submodule_update,
            commands::submodule_sync,
            commands::submodule_deinit,
            commands::external_tools_config,
            commands::set_external_tool,
            commands::launch_external_diff,
            commands::launch_external_merge,
            commands::network_release,
            commands::stashes,
            commands::stash_push,
            commands::recent_repos,
            commands::forget_repo,
            commands::clone_plan,
            commands::clone_start,
            commands::clone_release,
            commands::metrics,
            commands::about,
            commands::licenses,
            commands::identity,
            commands::set_identity,
            commands::identity_profiles,
            commands::save_identity_profile,
            commands::delete_identity_profile,
            commands::apply_identity_profile,
            commands::forge_repo,
            commands::forge_accounts,
            commands::forge_connect,
            commands::forge_disconnect,
            commands::pull_requests,
            commands::create_pull_request,
            commands::pull_request_files,
            commands::pull_request_commits,
            commands::commit_files,
            commands::pull_request_comments,
            commands::submit_review,
            commands::reply_comment,
            commands::merge_pull_request,
            commands::close_pull_request,
            commands::set_pr_draft,
            commands::check_update,
            commands::signing,
            commands::set_signing,
            commands::clear_signing,
            commands::settings,
            commands::set_settings,
            commands::avatar,
            commands::launch_path,
            commands::git_commands,
            commands::clear_git_commands,
            // The agent farm (FEAT-073). A separate module rather than more of
            // `commands.rs`: see its header.
            farm::farm_open,
            farm::farm_close,
            farm::farm_snapshot,
            farm::farm_events,
            farm::farm_stale,
            farm::farm_detect_agents,
            farm::farm_save_agent,
            farm::farm_remove_agent,
            farm::farm_create,
            farm::farm_configure,
            farm::farm_start,
            farm::farm_pause,
            farm::farm_cancel,
            farm::farm_write_policy,
            farm::farm_add_task,
            farm::farm_edit_task,
            farm::farm_delete_task,
            farm::farm_ready_task,
            farm::farm_ready_tasks,
            farm::farm_discard_tasks,
            farm::farm_assign_task,
            farm::farm_cancel_task,
            farm::farm_retry_task,
            farm::farm_run_task,
            farm::farm_task_detail,
            farm::farm_transcript,
            farm::farm_merge_task,
            farm::farm_review_task,
            farm::farm_verify_task,
            farm::farm_plan,
            farm::farm_cancel_plan,
            farm::farm_decompose,
            farm::farm_sweep,
        ])
        .setup(|app| {
            if let (Some(window), Some(icon)) =
                (app.get_webview_window("main"), app.default_window_icon())
            {
                window.set_icon(icon.clone())?;
            }

            // Registered before any command can run, so the first execution of
            // the session is already being forwarded.
            command_log::forward_to(app.handle().clone());

            // The farm's state, empty until a repository is opened.
            farm::manage(app.handle());
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("starting Spagitty");
}
