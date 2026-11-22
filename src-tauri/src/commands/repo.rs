use crate::git::{proxy::ProxyConfig, repo};
use tauri::command;

/// Proxy choice for network commands. Unreadable settings fall back to the
/// system default rather than blocking the operation.
fn proxy_config() -> ProxyConfig {
    crate::config::settings::Settings::load()
        .map(|settings| ProxyConfig::from_settings(&settings.proxy_mode, &settings.proxy_url))
        .unwrap_or_default()
}

#[command]
pub fn repo_open(path: String) -> Result<crate::git::repo::RepoInfo, String> {
    let info = repo::open_repo(&path)?;
    // Remember the last good repository for restore-on-startup. Failure to
    // remember is not a reason to fail opening the repository.
    if let Ok(mut settings) = crate::config::settings::Settings::load() {
        settings.record_recent(&path);
        let _ = settings.save();
    }
    Ok(info)
}

#[command]
pub async fn repo_clone(
    app: tauri::AppHandle,
    url: String,
    destination: String,
    branch: Option<String>,
) -> Result<crate::git::repo::RepoInfo, String> {
    use crate::git::clone::{self, Progress};
    use tauri::Emitter;

    let proxy = proxy_config();
    // Throttling lives in the clone callback; this only forwards each change.
    let progress: Option<Progress> = Some(std::sync::Arc::new(move |received, total| {
        let _ = app.emit(
            "clone-progress",
            serde_json::json!({ "received": received, "total": total }),
        );
    }));
    let outcome = tauri::async_runtime::spawn_blocking(move || {
        clone::clone_repo(&url, &destination, branch.as_deref(), &proxy, progress)
    })
    .await
    .map_err(|_| "Clone failed.".to_string())?;
    let info = outcome?;

    if let Ok(mut settings) = crate::config::settings::Settings::load() {
        settings.record_recent(&info.path);
        let _ = settings.save();
    }
    Ok(info)
}

#[command]
pub fn repo_init(path: String, branch: String) -> Result<crate::git::repo::RepoInfo, String> {
    use crate::git::repo_init;
    let info = repo_init::init_repo(&path, &branch)?;
    // Remember the new repository the same way an opened one is remembered.
    if let Ok(mut settings) = crate::config::settings::Settings::load() {
        settings.record_recent(&info.path);
        let _ = settings.save();
    }
    Ok(info)
}

#[command]
pub fn repo_status(path: String) -> Result<Vec<crate::git::repo::FileStatus>, String> {
    let status = repo::get_status(&path)?;
    Ok(status)
}

#[command]
pub fn repo_push(
    path: String,
    mode: Option<String>,
    force: Option<bool>,
) -> Result<String, String> {
    use crate::git::sync::{self, PushMode};
    let mode = mode.as_deref().unwrap_or("current");
    let mode = PushMode::parse(mode).ok_or_else(|| "Choose a valid push mode.".to_string())?;
    sync::push(&path, mode, force.unwrap_or(false), &proxy_config())
}

/// Signing configuration for commits created by this app. Missing or invalid
/// settings simply mean an unsigned commit.
fn signing_config() -> Option<crate::git::signing::SigningConfig> {
    crate::config::settings::Settings::load()
        .ok()
        .and_then(|settings| {
            crate::git::signing::SigningConfig::from_settings(
                &settings.signing_mode,
                &settings.signing_key,
            )
        })
}

#[command]
pub fn repo_commit(path: String, message: String) -> Result<String, String> {
    use crate::git::commit;
    let signing = signing_config();
    commit::commit_staged(&path, &message, signing.as_ref())
}

#[command]
pub fn repo_amend_commit(path: String, message: String) -> Result<String, String> {
    use crate::git::commit;
    let signing = signing_config();
    commit::amend_last(&path, &message, signing.as_ref())
}

#[command]
pub fn repo_undo_last_commit(path: String) -> Result<(), String> {
    use crate::git::commit;
    commit::undo_last(&path)
}

#[command]
pub fn repo_add_coauthor(message: String, name: String, email: String) -> Result<String, String> {
    use crate::git::coauthors;
    coauthors::add_coauthor(&message, &name, &email)
}

#[command]
pub fn repo_branches(path: String) -> Result<Vec<String>, String> {
    use crate::git::branch;
    let branches = branch::list_branches(&path)?;
    Ok(branches)
}

#[command]
pub fn repo_create_branch(path: String, name: String) -> Result<(), String> {
    use crate::git::branch;
    branch::create_branch(&path, &name)
}

#[command]
pub fn repo_checkout_branch(
    path: String,
    name: String,
) -> Result<crate::git::branch::CheckoutOutcome, String> {
    use crate::git::branch;
    branch::checkout_branch(&path, &name)
}

/// Switches branches after stashing the working tree. The stash stays listed,
/// so the changes can be applied again from the Stash panel.
#[command]
pub fn repo_checkout_branch_stashing(path: String, name: String) -> Result<(), String> {
    use crate::git::branch;
    branch::checkout_branch_stashing(&path, &name)
}

#[command]
pub fn repo_sync_fork(path: String) -> Result<String, String> {
    use crate::git::remotes;
    let signing = signing_config();
    remotes::sync_fork(&path, &proxy_config(), signing.as_ref()).map(str::to_string)
}

#[command]
pub fn repo_list_hunks(
    path: String,
    file: String,
    staged: bool,
) -> Result<Vec<crate::git::hunks::HunkSummary>, String> {
    use crate::git::hunks;
    let hunks = hunks::list_hunks(&path, &file, staged)?;
    Ok(hunks)
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
pub fn repo_commit_detail(path: String, oid: String) -> Result<crate::git::commit_diff::CommitDetail, String> {
    use crate::git::commit_diff;
    let detail = commit_diff::commit_detail(&path, &oid)?;
    Ok(detail)
}

#[command]
pub fn repo_commit_file(
    path: String,
    oid: String,
    file: String,
) -> Result<crate::git::commit_diff::CommitFileContents, String> {
    use crate::git::commit_diff;
    let contents = commit_diff::commit_file_contents(&path, &oid, &file)?;
    Ok(contents)
}

#[command]
pub fn repo_compare_branches(
    path: String,
    base: String,
    head: String,
) -> Result<crate::git::compare::BranchComparison, String> {
    use crate::git::compare;
    let comparison = compare::compare_branches(&path, &base, &head)?;
    Ok(comparison)
}

#[command]
pub fn repo_compare_file(
    path: String,
    base: String,
    head: String,
    file: String,
) -> Result<crate::git::compare::ComparedContents, String> {
    use crate::git::compare;
    let contents = compare::compare_file_contents(&path, &base, &head, &file)?;
    Ok(contents)
}

#[command]
pub fn repo_log(
    path: String,
    limit: usize,
    skip: Option<usize>,
) -> Result<Vec<crate::git::history::CommitInfo>, String> {
    use crate::git::history;
    let commits = history::log_commits(&path, limit, skip.unwrap_or(0))?;
    Ok(commits)
}

#[command]
pub fn repo_list_tags(path: String) -> Result<Vec<String>, String> {
    use crate::git::tags;
    let tags = tags::list_tags(&path)?;
    Ok(tags)
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
    tags::push_tag(&path, &name, &proxy_config())
}

#[command]
pub fn repo_stash_list(path: String) -> Result<Vec<String>, String> {
    use crate::git::stash;
    let stashes = stash::stash_list(&path)?;
    Ok(stashes)
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
pub fn repo_unstage_files(path: String, files: Vec<String>) -> Result<(), String> {
    use crate::git::staging;
    staging::unstage_files(&path, &files)
}

#[command]
pub fn repo_list_remotes(path: String) -> Result<Vec<crate::git::remotes::RemoteInfo>, String> {
    use crate::git::remotes;
    let remotes = remotes::list_remotes(&path)?;
    Ok(remotes)
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
    sync::fetch(&path, &name, prune.unwrap_or(false), &proxy_config())
}

#[command]
pub fn repo_pull(path: String, strategy: Option<String>) -> Result<(), String> {
    use crate::git::sync::{self, PullStrategy};
    let strategy = strategy.as_deref().unwrap_or("merge");
    let strategy = PullStrategy::parse(strategy)
        .ok_or_else(|| "Choose a valid pull strategy.".to_string())?;
    sync::pull(&path, strategy, false, &proxy_config())
}

#[command]
pub fn repo_file_contents(path: String, file: String) -> Result<crate::git::status::FileContents, String> {
    use crate::git::status;
    let contents = status::get_file_contents(&path, &file)?;
    Ok(contents)
}

#[command]
pub fn repo_merge(path: String, branch: String) -> Result<(), String> {
    use crate::git::merge;
    let signing = signing_config();
    merge::merge_branch(&path, &branch, signing.as_ref())
}

#[command]
pub fn repo_rebase(path: String, branch: String) -> Result<(), String> {
    use crate::git::merge;
    merge::rebase_branch(&path, &branch)
}

#[command]
pub fn repo_rebase_plan(path: String, onto: String) -> Result<Vec<crate::git::interactive::PlanCommit>, String> {
    use crate::git::interactive;
    let commits = interactive::plan(&path, &onto)?;
    Ok(commits)
}

#[command]
pub fn repo_interactive_rebase(
    path: String,
    onto: String,
    steps: Vec<crate::git::interactive::RebaseStep>,
) -> Result<String, String> {
    use crate::git::interactive;
    let signing = signing_config();
    interactive::run(&path, &onto, &steps, signing.as_ref()).map(str::to_string)
}

#[command]
pub fn repo_merge_continue(path: String, message: Option<String>) -> Result<(), String> {
    use crate::git::merge;
    let signing = signing_config();
    merge::merge_continue(&path, message.as_deref(), signing.as_ref())
}

#[command]
pub fn repo_merge_abort(path: String) -> Result<(), String> {
    use crate::git::merge;
    merge::merge_abort(&path)
}

#[command]
pub fn repo_rebase_continue(path: String) -> Result<(), String> {
    use crate::git::interactive;
    use crate::git::merge;
    if interactive::has_state(&path) {
        let signing = signing_config();
        interactive::continue_rebase(&path, signing.as_ref()).map(|_| ())
    } else {
        merge::rebase_continue(&path)
    }
}

#[command]
pub fn repo_rebase_abort(path: String) -> Result<(), String> {
    use crate::git::{interactive, merge};
    if interactive::has_state(&path) {
        interactive::abort_rebase(&path)
    } else {
        merge::rebase_abort(&path)
    }
}

#[command]
pub fn repo_operation_state(path: String) -> Result<String, String> {
    use crate::git::merge;
    Ok(merge::operation_state(&path)?.to_string())
}

#[command]
pub fn repo_reset(path: String, mode: String, target: Option<String>) -> Result<(), String> {
    use crate::git::maintenance::{self, ResetMode};
    let mode = ResetMode::parse(&mode).ok_or_else(|| "Choose a valid reset mode.".to_string())?;
    maintenance::reset_branch(&path, mode, target.as_deref())
}

#[command]
pub fn repo_clean_untracked(path: String, directories: Option<bool>) -> Result<usize, String> {
    use crate::git::maintenance;
    maintenance::clean_untracked(&path, directories.unwrap_or(true))
}

#[command]
pub fn repo_reflog(path: String) -> Result<Vec<crate::git::reflog::ReflogEntry>, String> {
    use crate::git::reflog;
    let entries = reflog::list_reflog(&path)?;
    Ok(entries)
}

#[command]
pub fn repo_list_conflicts(path: String) -> Result<Vec<String>, String> {
    use crate::git::conflicts;
    let files = conflicts::list_conflicts(&path)?;
    Ok(files)
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

/// Opens a repository file (changed, staged or conflicted) in the configured
/// external editor. The path stays inside the repository.
#[command]
pub fn repo_open_in_editor(path: String, file: String) -> Result<(), String> {
    use crate::git::conflicts;
    use crate::integrations;
    let relative = conflicts::check_file(&file)?;
    let full = std::path::Path::new(&path).join(relative);
    integrations::open_file(&full.to_string_lossy())
}

/// Shows the committed and working-tree versions of a file in the configured
/// diff tool.
/// Git LFS availability. Read-only, so it is safe before any repository is open.
#[command]
pub fn repo_lfs_status() -> crate::git::lfs::LfsStatus {
    crate::git::lfs::status()
}

#[command]
pub fn repo_lfs_tracked(path: String) -> Result<Vec<crate::git::lfs::TrackedPattern>, String> {
    use crate::git::lfs;
    lfs::tracked(&path)
}

#[command]
pub fn repo_lfs_track(path: String, pattern: String) -> Result<(), String> {
    use crate::git::lfs;
    lfs::track(&path, &pattern)
}

#[command]
pub fn repo_lfs_untrack(path: String, pattern: String) -> Result<(), String> {
    use crate::git::lfs;
    lfs::untrack(&path, &pattern)
}

#[command]
pub fn repo_lfs_pull(path: String) -> Result<(), String> {
    use crate::git::lfs;
    lfs::pull(&path)
}

#[command]
pub fn repo_lfs_push(path: String) -> Result<(), String> {
    use crate::git::lfs;
    lfs::push(&path)
}

#[command]
pub fn repo_difftool(path: String, file: String) -> Result<(), String> {
    use crate::git::tools;
    tools::diff(&path, &file)
}

/// Starts the configured merge tool for a conflicted file. The tool writes the
/// working-tree file; saving the conflict afterwards stages the result.
#[command]
pub fn repo_mergetool(path: String, file: String) -> Result<(), String> {
    use crate::git::tools;
    tools::merge(&path, &file)
}

#[command]
pub fn repo_cherry_pick(path: String, oid: String) -> Result<(), String> {
    use crate::git::advanced;
    let signing = signing_config();
    advanced::cherry_pick(&path, &oid, signing.as_ref())
}

#[command]
pub fn repo_cherry_pick_continue(path: String) -> Result<(), String> {
    use crate::git::advanced;
    let signing = signing_config();
    advanced::cherry_pick_continue(&path, signing.as_ref())
}

#[command]
pub fn repo_cherry_pick_abort(path: String) -> Result<(), String> {
    use crate::git::advanced;
    advanced::cherry_pick_abort(&path)
}

#[command]
pub fn repo_revert(path: String, oid: String) -> Result<(), String> {
    use crate::git::advanced;
    let signing = signing_config();
    advanced::revert_commit(&path, &oid, signing.as_ref())
}

#[command]
pub fn repo_revert_continue(path: String) -> Result<(), String> {
    use crate::git::advanced;
    let signing = signing_config();
    advanced::revert_continue(&path, signing.as_ref())
}

#[command]
pub fn repo_revert_abort(path: String) -> Result<(), String> {
    use crate::git::advanced;
    advanced::revert_abort(&path)
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
pub fn repo_publish_branch(path: String, branch: String) -> Result<(), String> {
    use crate::git::branch;
    branch::publish_branch(&path, &branch, &proxy_config())
}

#[command]
pub fn repo_branch_overview(path: String) -> Result<Vec<crate::git::branch::BranchOverview>, String> {
    use crate::git::branch;
    let branches = branch::list_branch_overview(&path)?;
    Ok(branches)
}
