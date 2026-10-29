pub mod auth;
pub mod commands;
pub mod config;
pub mod git;
pub mod github;
pub mod state;
pub mod updater;
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
            commands::repo::repo_status,
            commands::repo::repo_push,
            commands::repo::repo_commit,
            commands::repo::repo_branches,
            commands::repo::repo_create_branch,
            commands::repo::repo_checkout_branch,
            commands::repo::repo_get_settings,
            commands::repo::repo_save_settings,
            commands::repo::repo_stage_files,
            commands::repo::repo_unstage_files,
            commands::repo::repo_stage_all,
            commands::repo::repo_list_remotes,
            commands::repo::repo_add_remote,
            commands::repo::repo_remove_remote,
            commands::repo::repo_set_remote_url,
            commands::repo::repo_fetch,
            commands::repo::repo_pull,
            commands::repo::repo_stash_save,
            commands::repo::repo_stash_list,
            commands::repo::repo_stash_pop,
            commands::repo::repo_stash_drop,
            commands::repo::repo_log,
            commands::repo::repo_list_tags,
            commands::repo::repo_create_tag,
        ])
        .setup(|_app| {
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

