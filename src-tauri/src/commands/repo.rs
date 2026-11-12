use crate::git::repo;
use tauri::command;

#[command]
pub fn repo_open(path: String) -> Result<serde_json::Value, String> {
    let info = repo::open_repo(&path)?;
    // Remember the last good repository for restore-on-startup. Failure to
    // remember is not a reason to fail opening the repository.
    if let Ok(mut settings) = crate::config::settings::Settings::load() {
        settings.record_recent(&path);
        let _ = settings.save();
    }
    serde_json::to_value(info).map_err(|e| e.to_string())
}

#[command]
pub fn repo_status(path: String) -> Result<serde_json::Value, String> {
    let status = repo::get_status(&path)?;
    serde_json::to_value(status).map_err(|e| e.to_string())
}

#[command]
pub fn repo_push(path: String, mode: Option<String>) -> Result<String, String> {
    use crate::git::sync::{self, PushMode};
    let mode = mode.as_deref().unwrap_or("current");
    let mode = PushMode::parse(mode).ok_or_else(|| "Choose a valid push mode.".to_string())?;
    sync::push(&path, mode)
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
pub fn repo_discard(path: String, files: Vec<String>) -> Result<(), String> {
    use crate::git::staging;
    staging::discard_files(&path, &files)
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
pub fn repo_fetch(path: String, name: String, prune: Option<bool>) -> Result<(), String> {
    use crate::git::sync;
    sync::fetch(&path, &name, prune.unwrap_or(false))
}

#[command]
pub fn repo_pull(path: String, strategy: Option<String>) -> Result<(), String> {
    use crate::git::sync::{self, PullStrategy};
    let strategy = strategy.as_deref().unwrap_or("merge");
    let strategy = PullStrategy::parse(strategy)
        .ok_or_else(|| "Choose a valid pull strategy.".to_string())?;
    sync::pull(&path, strategy, false)
}

#[command]
pub fn repo_file_contents(path: String, file: String) -> Result<serde_json::Value, String> {
    use crate::git::status;
    let contents = status::get_file_contents(&path, &file)?;
    serde_json::to_value(contents).map_err(|e| e.to_string())
}

#[command]
pub fn repo_merge(path: String, branch: String) -> Result<(), String> {
    use crate::git::merge;
    merge::merge_branch(&path, &branch)
}

#[command]
pub fn repo_rebase(path: String, branch: String) -> Result<(), String> {
    use crate::git::merge;
    merge::rebase_branch(&path, &branch)
}

#[command]
pub fn repo_reflog(path: String) -> Result<serde_json::Value, String> {
    use crate::git::reflog;
    let entries = reflog::list_reflog(&path)?;
    serde_json::to_value(entries).map_err(|e| e.to_string())
}

#[command]
pub fn repo_list_conflicts(path: String) -> Result<serde_json::Value, String> {
    use crate::git::conflicts;
    let files = conflicts::list_conflicts(&path)?;
    serde_json::to_value(files).map_err(|e| e.to_string())
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
pub fn repo_cherry_pick(path: String, oid: String) -> Result<(), String> {
    use crate::git::advanced;
    advanced::cherry_pick(&path, &oid)
}

#[command]
pub fn repo_revert(path: String, oid: String) -> Result<(), String> {
    use crate::git::advanced;
    advanced::revert_commit(&path, &oid)
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
pub fn repo_set_upstream(path: String, branch: String, upstream: String) -> Result<(), String> {
    use crate::git::branch;
    branch::set_upstream(&path, &branch, &upstream)
}

#[command]
pub fn repo_publish_branch(path: String, branch: String) -> Result<(), String> {
    use crate::git::branch;
    branch::publish_branch(&path, &branch)
}
