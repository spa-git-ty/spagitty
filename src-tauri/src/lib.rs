// SPDX-License-Identifier: GPL-3.0-or-later

//! Spagitty's Tauri shell.
//!
//! This crate owns the window, the commands, and the background workers
//! (history walking, log searching and filesystem watching). All git logic
//! lives in `spagitty-core`.

mod about;
mod accounts;
mod agents;
mod clone_worker;
mod command_log;
mod commands;
mod desktop;
mod extensions;
mod farm;
mod forge_bridge;
mod forge_views;
mod graph_worker;
mod hooks;
mod inbox;
mod issues;
mod merger_state;
mod network_worker;
mod notifications;
mod pipelines;
mod platform;
mod profiles;
mod rebase_worker;
mod recents;
mod recipes;
mod releases;
mod review_state;
mod search_worker;
mod settings;
mod supplemental;
mod terminal;
#[cfg(test)]
mod testing;
mod timing;
mod watch;

use tauri::Manager;

pub fn run() {
    // Before the builder, because the webview reads its environment as it
    // starts and this process is still single-threaded here.
    platform::prepare_webview();

    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .manage(commands::AppState::default())
        // Terminals belong to the window, not to the repository that is open.
        .manage(terminal::TerminalState::default())
        // The notification center's record, opened on first use (FEAT-142).
        .manage(inbox::InboxState::default())
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
            commands::hooks,
            commands::set_hooks_enabled,
            hooks::hooks_builder,
            hooks::hooks_save,
            hooks::hooks_import,
            hooks::hooks_matches,
            hooks::hooks_runs,
            hooks::hooks_run,
            hooks::hooks_log,
            hooks::hooks_dry_run,
            hooks::hooks_try_message,
            hooks::hooks_stop,
            hooks::hooks_undo_restage,
            issues::issues_remotes,
            issues::issues_cached,
            issues::issues_page,
            issues::issues_metadata,
            issues::issues_thread,
            issues::issues_write,
            issues::issues_branch,
            pipelines::pipelines_preview,
            pipelines::pipelines_source,
            pipelines::pipelines_preferences,
            pipelines::pipelines_watches,
            pipelines::pipelines_cached,
            pipelines::pipelines_runs,
            pipelines::pipelines_jobs,
            pipelines::pipelines_log,
            pipelines::pipelines_graph,
            pipelines::pipelines_write,
            releases::releases_preferences,
            releases::releases_hide,
            releases::releases_cached,
            releases::releases_page,
            releases::releases_write,
            releases::releases_notes,
            releases::releases_target,
            releases::releases_changelog,
            releases::releases_push_target,
            releases::releases_upload,
            releases::releases_artifacts,
            releases::releases_download_artifact,
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
            commands::graph_order,
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
            commands::pull_branch,
            commands::fetch,
            commands::push,
            commands::search_start,
            commands::search_stop,
            commands::blame,
            commands::files_resolve,
            commands::files_tree,
            commands::files_read,
            commands::files_find,
            commands::files_search,
            commands::files_spans,
            commands::files_blame,
            commands::files_before,
            commands::investigate_life,
            commands::investigate_today,
            commands::investigate_hunk,
            commands::investigate_compare,
            commands::investigate_branch,
            commands::terminal_links,
            terminal::terminal_info,
            terminal::terminal_open,
            terminal::terminal_write,
            terminal::terminal_resize,
            terminal::terminal_close,
            terminal::terminal_session_name,
            terminal::terminal_sessions,
            terminal::terminal_windows,
            terminal::terminal_act,
            recipes::recipes_list,
            recipes::recipes_save,
            recipes::recipes_delete,
            recipes::recipes_import,
            recipes::recipes_check,
            recipes::recipes_propose,
            recipes::recipe_run_start,
            recipes::recipe_run_answer,
            recipes::recipe_run_stop,
            recipes::recipe_run_undo,
            recipes::recipe_run_current,
            recipes::recipe_run_close,
            recipes::restore_points,
            recipes::restore_point_undo,
            recipes::restore_point_drop,
            commands::conflicts,
            commands::conflict_sides,
            commands::conflict_regions,
            commands::conflict_take,
            commands::conflict_resolve_region,
            commands::conflict_write,
            commands::conflict_resolve,
            commands::conflict_continue,
            commands::conflict_abort,
            commands::conflict_settle,
            commands::merger_forecast,
            commands::merger_land,
            commands::merger_conflicts,
            commands::merger_state,
            commands::set_merger_state,
            commands::merger_rebase_open,
            commands::merger_rebase_continue,
            commands::merger_rebase_skip,
            commands::merger_rebase_abort,
            commands::merger_rebase_finish,
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
            commands::home_overview,
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
            commands::involved_pull_requests,
            notifications::watch_pull_requests,
            notifications::notify_desktop,
            inbox::inbox_list,
            inbox::inbox_add,
            inbox::inbox_read,
            inbox::inbox_read_all,
            inbox::inbox_clear,
            commands::local_clone_of,
            commands::review_summaries,
            commands::review_checkout,
            commands::review_files,
            commands::review_conflicts,
            commands::review_comments,
            commands::resolve_thread,
            commands::review_file,
            commands::review_check_out,
            commands::review_state,
            commands::set_review_state,
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
            commands::command_timings,
            commands::clear_git_commands,
            // The desktop's own palette (FEAT-080). Its own module rather than
            // more of `commands.rs`: it is filesystem reading and a watcher,
            // and it touches no git at all.
            desktop::desktop_theme,
            desktop::desktop_theme_watch,
            desktop::desktop_theme_unwatch,
            // Agents in Review and Merger (2.0): the machine list, each
            // repository's rules, and the assignments. Its own module: see
            // its header.
            agents::agents_snapshot,
            agents::agents_set_jobs,
            agents::agents_set_codex_full_access,
            agents::agents_set_omp_options,
            agents::agents_save_custom,
            agents::agents_take_offer,
            agents::agents_save_remote,
            agents::agents_remove,
            agents::agents_set_defaults,
            agents::agents_set_rules,
            agents::agents_consent,
            agents::agents_test_local,
            agents::agents_models,
            agents::agents_test_remote,
            agents::assignment_start,
            agents::assignment_control,
            agents::assignment_list,
            agents::assignment_transcript,
            agents::assignment_forget,
            // The agent farm (FEAT-073). A separate module rather than more of
            // `commands.rs`: see its header.
            farm::farm_open,
            farm::farm_exists,
            farm::farm_close,
            farm::farm_snapshot,
            farm::farm_events,
            farm::farm_stale,
            farm::farm_detect_agents,
            farm::farm_save_agent,
            farm::farm_remove_agent,
            farm::farm_create,
            farm::farm_remembered,
            farm::farm_remember,
            farm::farm_past,
            farm::farm_ledger,
            farm::farm_resume,
            farm::farm_briefs,
            farm::farm_brief,
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
            farm::farm_merge_check,
            farm::farm_task_merged,
            farm::farm_review_supplemental,
            farm::farm_review_task,
            farm::farm_verify_task,
            farm::farm_plan,
            farm::farm_cancel_plan,
            farm::farm_decompose,
            farm::farm_sweep,
            // The extension host (FEAT-096). Its own module for the same reason
            // as the farm: a host with its own state and lifetime.
            extensions::extensions_list,
            extensions::extensions_inspect,
            extensions::extensions_install,
            extensions::extensions_rollback,
            extensions::extensions_uninstall,
            extensions::extensions_attach,
            extensions::extensions_restart,
            extensions::extensions_enable,
            extensions::extensions_disable,
            extensions::extensions_set_grant,
            extensions::extensions_set_setting,
            extensions::extensions_choose_executable,
            extensions::extensions_detect_tool,
            extensions::extensions_run_command,
            extensions::extensions_preview_review,
            extensions::extensions_start_review,
            extensions::extensions_cancel,
            extensions::extensions_reviews,
            extensions::extensions_set_disposition,
            extensions::extensions_delete_reviews,
            extensions::extensions_panel,
            extensions::extensions_suggested_bases,
            extensions::extensions_confirm,
            extensions::extensions_location,
            extensions::extensions_send_findings,
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

            // The desktop theme watcher's slot. Empty until somebody chooses
            // to follow the desktop.
            desktop::manage(app.handle());

            // The extension host. Nothing starts until an extension is used.
            extensions::manage(app.handle());

            // Agents' assignments. Nothing runs until one is assigned.
            agents::manage(app.handle());

            // Recipes' runs. Nothing runs until one is run.
            recipes::manage(app.handle());
            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("starting Spagitty")
        .run(|app, event| {
            // Workers are asked to stop rather than left to notice that
            // their stdin closed.
            if let tauri::RunEvent::Exit = event {
                agents::shutdown(app);
                extensions::shutdown(app);
            }
        });
}
