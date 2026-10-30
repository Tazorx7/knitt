use crate::git::repo;
use tauri::command;

#[command]
pub fn repo_open(path: String) -> Result<serde_json::Value, String> {
    let info = repo::open_repo(&path)?;
    serde_json::to_value(info).map_err(|e| e.to_string())
}

#[command]
pub fn repo_status(path: String) -> Result<serde_json::Value, String> {
    let status = repo::get_status(&path)?;
    serde_json::to_value(status).map_err(|e| e.to_string())
}

#[command]
pub fn repo_push(path: String) -> Result<String, String> {
    use crate::git::credentials;
    let repo = git2::Repository::open(&path).map_err(|e| e.to_string())?;
    let mut remote = repo.find_remote("origin").map_err(|e| e.to_string())?;
    let callbacks = credentials::make_callbacks();
    let mut push_options = git2::PushOptions::new();
    push_options.remote_callbacks(callbacks);
    remote
        .push(&["refs/heads/master:refs/heads/master"], Some(&mut push_options))
        .map_err(|e| e.to_string())?;
    Ok("pushed".to_string())
}

#[command]
pub fn repo_commit(path: String, message: String) -> Result<String, String> {
    use crate::git::commit;
    commit::commit_all(&path, &message)
}

#[command]
pub fn repo_branches(path: String) -> Result<serde_json::Value, String> {
    use crate::git::branch;
    let branches = branch::list_branches(&path)?;
    serde_json::to_value(branches).map_err(|e| e.to_string())
}

#[command]
pub fn repo_create_branch(path: String, name: String) -> Result<(), String> {
    use crate::git::branch;
    branch::create_branch(&path, &name)
}

#[command]
pub fn repo_checkout_branch(path: String, name: String) -> Result<(), String> {
    use crate::git::branch;
    branch::checkout_branch(&path, &name)
}

#[command]
pub fn repo_get_settings() -> Result<serde_json::Value, String> {
    use crate::config::settings::Settings;
    let s = Settings::load().map_err(|e| e.to_string())?;
    serde_json::to_value(s).map_err(|e| e.to_string())
}

#[command]
pub fn repo_save_settings(settings: serde_json::Value) -> Result<(), String> {
    use crate::config::settings::Settings;
    let s: Settings = serde_json::from_value(settings).map_err(|e| e.to_string())?;
    s.save().map_err(|e| e.to_string())
}

#[command]
pub fn repo_sync_fork(path: String) -> Result<(), String> {
    use crate::git::remotes;
    remotes::sync_fork(&path)
}

#[command]
pub fn repo_stage_hunk(path: String, file: String, hunk_header: String) -> Result<(), String> {
    use crate::git::staging;
    staging::stage_hunk(&path, &file, &hunk_header)
}

#[command]
pub fn repo_unstage_hunk(path: String, file: String, hunk_header: String) -> Result<(), String> {
    use crate::git::staging;
    staging::unstage_hunk(&path, &file, &hunk_header)
}

#[command]
pub fn repo_diff(path: String, file: String) -> Result<String, String> {
    use crate::git::status;
    status::get_diff(&path, &file)
}
