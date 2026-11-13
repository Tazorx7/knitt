pub mod auth;
pub mod commands;
pub mod config;
pub mod git;
pub mod github;
pub mod integrations;
pub mod state;
pub mod utils;
pub mod watcher;

use state::app::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    utils::logging::init();
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState::new())
        .invoke_handler(tauri::generate_handler![
            commands::auth::auth_check_secret_service,
            commands::auth::auth_start_device_flow,
            commands::auth::auth_cancel_device_flow,
            commands::auth::auth_poll_device_flow,
            commands::auth::auth_list_accounts,
            commands::auth::auth_sign_out,
            commands::auth::auth_sign_out_all,
            commands::auth::auth_set_active_account,
            commands::repo::repo_open,
            commands::repo::repo_init,
            commands::repo::repo_status,
            commands::repo::repo_push,
            commands::repo::repo_commit,
            commands::repo::repo_branches,
            commands::repo::repo_create_branch,
            commands::repo::repo_checkout_branch,
            commands::settings::settings_get,
            commands::settings::settings_save,
            commands::settings::settings_git_identity,
            commands::settings::settings_set_git_identity,
            commands::settings::settings_validate_branch,
            commands::settings::settings_repository_identity,
            commands::settings::settings_set_repository_identity,
            commands::settings::settings_reset_repository_identity,
            commands::settings::settings_reset,
            commands::settings::settings_set_autostart,
            commands::settings::settings_remove_recent,
            commands::settings::settings_forget_recent,
            commands::settings::settings_ssh_diagnostics,
            commands::diagnostics::diagnostics_environment,
            commands::diagnostics::diagnostics_report,
            commands::diagnostics::diagnostics_export,
            commands::diagnostics::diagnostics_open_logs,
            commands::diagnostics::diagnostics_clear_logs,
            commands::integrations::integrations_detect,
            commands::integrations::integrations_validate,
            commands::integrations::integrations_test,
            commands::integrations::integrations_open,
            commands::integrations::integrations_open_file_manager,
            commands::integrations::integrations_default_target,
            commands::github::gh_list_prs,
            commands::github::gh_list_issues,
            commands::github::gh_create_pr,
            commands::github::gh_checkout_pr,
            commands::repo::repo_sync_fork,
            commands::github::gh_list_workflow_runs,
            commands::github::gh_list_repo_prs,
            commands::github::gh_list_repo_issues,
            commands::github::gh_list_repo_workflow_runs,
            commands::repo::repo_discard,
            commands::repo::repo_diff,
            commands::repo::repo_stage_hunk,
            commands::repo::repo_unstage_hunk,
            commands::repo::repo_commit_detail,
            commands::repo::repo_commit_file,
            commands::repo::repo_compare_branches,
            commands::repo::repo_compare_file,
            commands::repo::repo_log,
            commands::repo::repo_list_tags,
            commands::repo::repo_create_tag,
            commands::repo::repo_delete_tag,
            commands::repo::repo_push_tag,
            commands::repo::repo_stash_list,
            commands::repo::repo_stash_save,
            commands::repo::repo_stash_pop,
            commands::repo::repo_stash_drop,
            commands::repo::repo_stage_files,
            commands::repo::repo_stage_all,
            commands::repo::repo_list_remotes,
            commands::repo::repo_add_remote,
            commands::repo::repo_remove_remote,
            commands::repo::repo_set_remote_url,
            commands::repo::repo_fetch,
            commands::repo::repo_pull,
            commands::repo::repo_file_contents,
            commands::repo::repo_merge,
            commands::repo::repo_rebase,
            commands::repo::repo_reflog,
            commands::repo::repo_list_conflicts,
            commands::repo::repo_get_conflict,
            commands::repo::repo_resolve_conflict,
            commands::repo::repo_cherry_pick,
            commands::repo::repo_revert,
            commands::repo::repo_rename_branch,
            commands::repo::repo_delete_branch,
            commands::repo::repo_set_upstream,
            commands::repo::repo_publish_branch,
        ])
        .setup(|_app| {
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
