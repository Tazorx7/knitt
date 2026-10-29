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

pub async fn create_pr(token: &str, owner: &str, repo: &str, title: &str, head: &str, base: &str) -> Result<u64, String> {
    let crab = Octocrab::builder()
        .personal_token(token.to_string())
        .build()
        .map_err(|e| e.to_string())?;
    let pr = crab
        .pulls(owner, repo)
        .create(title, head, base)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    Ok(pr.number)
}

pub async fn checkout_pr(_token: &str, _owner: &str, _repo: &str, number: u64, path: &str) -> Result<(), String> {
    let repo = git2::Repository::open(path).map_err(|e| e.to_string())?;
    let mut remote = repo.find_remote("origin").map_err(|e| e.to_string())?;
    remote.fetch(&[&format!("pull/{}/head:pr-{}", number, number)], None, None).map_err(|e| e.to_string())?;
    let refname = format!("refs/heads/pr-{}", number);
    let obj = repo.revparse_single(&refname).map_err(|e| e.to_string())?;
    repo.checkout_tree(&obj, None).map_err(|e| e.to_string())?;
    repo.set_head(&refname).map_err(|e| e.to_string())?;
    Ok(())
}
