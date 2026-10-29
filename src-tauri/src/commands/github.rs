use tauri::command;

#[command]
pub async fn gh_list_prs(token: String, owner: String, repo: String) -> Result<serde_json::Value, String> {
    use crate::github::client;
    let prs = client::list_prs(&token, &owner, &repo).await?;
    serde_json::to_value(prs).map_err(|e| e.to_string())
}
