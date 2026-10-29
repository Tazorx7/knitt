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
pub fn repo_stage_files(path: String, files: Vec<String>) -> Result<(), String> {
    use crate::git::staging;
    staging::stage_files(&path, &files)
}

#[command]
pub fn repo_unstage_files(path: String, files: Vec<String>) -> Result<(), String> {
    use crate::git::staging;
    staging::unstage_files(&path, &files)
}

#[command]
pub fn repo_stage_all(path: String) -> Result<(), String> {
    use crate::git::staging;
    staging::stage_all(&path)
}

#[command]
pub fn repo_list_remotes(path: String) -> Result<serde_json::Value, String> {
    use crate::git::remotes;
    let r = remotes::list_remotes(&path)?;
    serde_json::to_value(r).map_err(|e| e.to_string())
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
pub fn repo_fetch(path: String, name: Option<String>) -> Result<(), String> {
    use crate::git::remotes;
    remotes::fetch_remote(&path, name.as_deref().unwrap_or("origin"))
}

#[command]
pub fn repo_pull(path: String) -> Result<(), String> {
    use crate::git::remotes;
    remotes::pull_remote(&path)
}

#[command]
pub fn repo_stash_save(path: String, message: Option<String>) -> Result<(), String> {
    use crate::git::stash;
    stash::stash_save(&path, message.as_deref())
}

#[command]
pub fn repo_stash_list(path: String) -> Result<serde_json::Value, String> {
    use crate::git::stash;
    let s = stash::stash_list(&path)?;
    serde_json::to_value(s).map_err(|e| e.to_string())
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
pub fn repo_log(path: String, limit: Option<usize>) -> Result<serde_json::Value, String> {
    use crate::git::history;
    let c = history::log_commits(&path, limit.unwrap_or(100))?;
    serde_json::to_value(c).map_err(|e| e.to_string())
}

#[command]
pub fn repo_list_tags(path: String) -> Result<serde_json::Value, String> {
    use crate::git::tags;
    let t = tags::list_tags(&path)?;
    serde_json::to_value(t).map_err(|e| e.to_string())
}

#[command]
pub fn repo_create_tag(path: String, name: String) -> Result<(), String> {
    use crate::git::tags;
    tags::create_tag(&path, &name)
}

#[command]
pub fn repo_merge(path: String, branch: String) -> Result<(), String> {
    use crate::git::merge;
    merge::merge_branch(&path, &branch)
}

#[command]
pub fn repo_reflog(path: String) -> Result<serde_json::Value, String> {
    use crate::git::reflog;
    let r = reflog::list_reflog(&path)?;
    serde_json::to_value(r).map_err(|e| e.to_string())
}

#[command]
pub fn repo_list_conflicts(path: String) -> Result<serde_json::Value, String> {
    use crate::git::conflicts;
    let c = conflicts::list_conflicts(&path)?;
    serde_json::to_value(c).map_err(|e| e.to_string())
}

#[command]
pub fn repo_get_conflict(path: String, file: String) -> Result<String, String> {
    use crate::git::conflicts;
    conflicts::get_conflict_content(&path, &file)
}

#[command]
pub fn repo_resolve_conflict(path: String, file: String, content: String) -> Result<(), String> {
    use crate::git::conflicts;
    conflicts::resolve_conflict(&path, &file, &content)
}

#[command]
pub fn repo_rename_branch(path: String, old_name: String, new_name: String) -> Result<(), String> {
    use crate::git::branch;
    branch::rename_branch(&path, &old_name, &new_name)
}

#[command]
pub fn repo_delete_branch(path: String, name: String) -> Result<(), String> {
    use crate::git::branch;
    branch::delete_branch(&path, &name)
}

#[command]
pub fn repo_cherry_pick(path: String, oid: String) -> Result<(), String> {
    use crate::git::advanced;
    advanced::cherry_pick(&path, &oid)
}

#[command]
pub fn repo_revert(path: String, oid: String) -> Result<(), String> {
    use crate::git::advanced;
    advanced::revert_commit(&path, &oid)
}
