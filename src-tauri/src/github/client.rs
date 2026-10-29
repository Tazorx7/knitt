use octocrab::Octocrab;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct PrInfo {
    pub number: u64,
    pub title: String,
    pub state: String,
}

pub async fn list_prs(token: &str, owner: &str, repo: &str) -> Result<Vec<PrInfo>, String> {
    let crab = Octocrab::builder()
        .personal_token(token.to_string())
        .build()
        .map_err(|e| e.to_string())?;
    let pulls = crab
        .pulls(owner, repo)
        .list()
        .state(octocrab::params::State::Open)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for pr in pulls.items {
        result.push(PrInfo {
            number: pr.number,
            title: pr.title.unwrap_or_default(),
            state: format!("{:?}", pr.state),
        });
    }
    Ok(result)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct IssueInfo {
    pub number: u64,
    pub title: String,
    pub state: String,
}

pub async fn list_issues(token: &str, owner: &str, repo: &str) -> Result<Vec<IssueInfo>, String> {
    let crab = Octocrab::builder()
        .personal_token(token.to_string())
        .build()
        .map_err(|e| e.to_string())?;
    let issues = crab
        .issues(owner, repo)
        .list()
        .state(octocrab::params::State::Open)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for issue in issues.items {
        result.push(IssueInfo {
            number: issue.number,
            title: issue.title,
            state: format!("{:?}", issue.state),
        });
    }
    Ok(result)
}
