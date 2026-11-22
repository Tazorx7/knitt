use tauri::command;

/// Resolves the repository's host, owner and repository, plus the token of the
/// account that belongs to that host. Enterprise repositories therefore use
/// their own API base and their own account.
fn target(path: &str) -> Result<(String, String, String, String), String> {
    let (owner, repo, host) = crate::git::remotes::get_github_owner_repo_host(path)?;
    let token = crate::auth::accounts::token_for_host(&host)?
        .ok_or_else(|| format!("Sign in to {host} to see this repository's GitHub data."))?;
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
