use tauri::command;

#[command]
pub async fn gh_list_prs(token: String, owner: String, repo: String) -> Result<serde_json::Value, String> {
    use crate::github::client;
    let prs = client::list_prs(&token, &owner, &repo).await?;
    serde_json::to_value(prs).map_err(|e| e.to_string())
}

#[command]
pub async fn gh_list_issues(token: String, owner: String, repo: String) -> Result<serde_json::Value, String> {
    use crate::github::client;
    let issues = client::list_issues(&token, &owner, &repo).await?;
    serde_json::to_value(issues).map_err(|e| e.to_string())
}

#[command]
pub async fn gh_create_pr(token: String, owner: String, repo: String, title: String, head: String, base: String) -> Result<u64, String> {
    use crate::github::client;
    client::create_pr(&token, &owner, &repo, &title, &head, &base).await
}

#[command]
pub async fn gh_checkout_pr(token: String, owner: String, repo: String, number: u64, path: String) -> Result<(), String> {
    use crate::github::client;
    client::checkout_pr(&token, &owner, &repo, number, &path).await
}

#[command]
pub async fn gh_list_workflow_runs(token: String, owner: String, repo: String) -> Result<serde_json::Value, String> {
    use crate::github::client;
    let runs = client::list_workflow_runs(&token, &owner, &repo).await?;
    serde_json::to_value(runs).map_err(|e| e.to_string())
}
