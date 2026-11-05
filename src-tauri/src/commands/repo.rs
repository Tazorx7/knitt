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

#[command]
pub fn repo_log(path: String, limit: usize) -> Result<serde_json::Value, String> {
    use crate::git::history;
    let commits = history::log_commits(&path, limit)?;
    serde_json::to_value(commits).map_err(|e| e.to_string())
}

#[command]
pub fn repo_list_tags(path: String) -> Result<serde_json::Value, String> {
    use crate::git::tags;
    let tags = tags::list_tags(&path)?;
    serde_json::to_value(tags).map_err(|e| e.to_string())
}

#[command]
pub fn repo_create_tag(path: String, name: String) -> Result<(), String> {
    use crate::git::tags;
    tags::create_tag(&path, &name)
}

#[command]
pub fn repo_delete_tag(path: String, name: String) -> Result<(), String> {
    use crate::git::tags;
    tags::delete_tag(&path, &name)
}

#[command]
pub fn repo_push_tag(path: String, name: String) -> Result<(), String> {
    use crate::git::tags;
    tags::push_tag(&path, &name)
}

#[command]
pub fn repo_stash_list(path: String) -> Result<serde_json::Value, String> {
    use crate::git::stash;
    let stashes = stash::stash_list(&path)?;
    serde_json::to_value(stashes).map_err(|e| e.to_string())
}

#[command]
pub fn repo_stash_save(path: String, message: Option<String>) -> Result<(), String> {
    use crate::git::stash;
    stash::stash_save(&path, message.as_deref())
}

#[command]
pub fn repo_stash_pop(path: String) -> Result<(), String> {
    use crate::git::stash;
    stash::stash_pop(&path)
}

#[command]
pub fn repo_stash_drop(path: String) -> Result<(), String> {
    use crate::git::stash;
    stash::stash_drop(&path)
}

#[command]
pub fn repo_stage_files(path: String, files: Vec<String>) -> Result<(), String> {
    use crate::git::staging;
    staging::stage_files(&path, &files)
}

#[command]
pub fn repo_stage_all(path: String) -> Result<(), String> {
    use crate::git::staging;
    staging::stage_all(&path)
}

#[command]
pub fn repo_list_remotes(path: String) -> Result<serde_json::Value, String> {
    use crate::git::remotes;
    let remotes = remotes::list_remotes(&path)?;
    serde_json::to_value(remotes).map_err(|e| e.to_string())
}

#[command]
pub fn repo_add_remote(path: String, name: String, url: String) -> Result<(), String> {
    use crate::git::remotes;
    remotes::add_remote(&path, &name, &url)
}

#[command]
pub fn repo_remove_remote(path: String, name: String) -> Result<(), String> {
    use crate::git::remotes;
    remotes::remove_remote(&path, &name)
}

#[command]
pub fn repo_set_remote_url(path: String, name: String, url: String) -> Result<(), String> {
    use crate::git::remotes;
    remotes::set_remote_url(&path, &name, &url)
}

#[command]
pub fn repo_fetch(path: String, name: String) -> Result<(), String> {
    use crate::git::remotes;
    remotes::fetch_remote(&path, &name)
}

#[command]
pub fn repo_pull(path: String) -> Result<(), String> {
    use crate::git::remotes;
    remotes::pull_remote(&path)
}

#[command]
pub fn repo_file_contents(path: String, file: String) -> Result<serde_json::Value, String> {
    use crate::git::status;
    let contents = status::get_file_contents(&path, &file)?;
    serde_json::to_value(contents).map_err(|e| e.to_string())
}
