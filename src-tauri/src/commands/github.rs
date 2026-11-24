use tauri::command;

/// Resolves the repository's host, owner and repository, plus the token of the
/// account that belongs to that host. Enterprise repositories therefore use
/// their own API base and their own account.
fn target(path: &str) -> Result<(String, String, String, String), String> {
    let (owner, repo, host) = crate::git::remotes::get_github_owner_repo_host(path)?;
    let store = crate::auth::accounts::AccountsStore::load().map_err(|error| {
        tracing::warn!(%error, "could not read the saved accounts");
        "Could not read the saved GitHub accounts.".to_string()
    })?;
    let account = store.account_for_host(&host).ok_or_else(|| {
        format!("Sign in to {host} to see this repository's GitHub data.")
    })?;
    let id = account.id.clone();
    // A listed account without a token means the stored session is gone; say
    // so instead of asking the user to sign in again from scratch.
    let token = crate::auth::accounts::token_for_account(&id)?.ok_or_else(|| {
        format!("Your {host} session has expired. Sign in again in Settings → Accounts.")
    })?;
    Ok((host, owner, repo, token))
}

#[command]
pub async fn gh_list_repo_prs(path: String) -> Result<Vec<crate::github::client::PrInfo>, String> {
    use crate::github::client;
    let (host, owner, repo, token) = target(&path)?;
    let prs = client::list_prs(&token, &host, &owner, &repo).await?;
    Ok(prs)
}

#[command]
pub async fn gh_list_repo_issues(path: String) -> Result<Vec<crate::github::client::IssueInfo>, String> {
    use crate::github::client;
    let (host, owner, repo, token) = target(&path)?;
    let issues = client::list_issues(&token, &host, &owner, &repo).await?;
    Ok(issues)
}

#[command]
pub async fn gh_list_repo_workflow_runs(path: String) -> Result<Vec<crate::github::client::WorkflowRun>, String> {
    use crate::github::client;
    let (host, owner, repo, token) = target(&path)?;
    let runs = client::list_workflow_runs(&token, &host, &owner, &repo).await?;
    Ok(runs)
}

#[command]
pub async fn gh_list_workflow_jobs(
    path: String,
    run_id: u64,
) -> Result<Vec<crate::github::client::WorkflowJob>, String> {
    use crate::github::client;
    let (host, owner, repo, token) = target(&path)?;
    client::list_workflow_jobs(&token, &host, &owner, &repo, run_id).await
}

#[command]
pub async fn gh_workflow_job_log(path: String, job_id: u64) -> Result<String, String> {
    use crate::github::client;
    let (host, owner, repo, token) = target(&path)?;
    client::workflow_job_log(&token, &host, &owner, &repo, job_id).await
}
