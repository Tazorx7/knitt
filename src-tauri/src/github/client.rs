use octocrab::Octocrab;
use serde::{Deserialize, Serialize};

/// API base for a GitHub host: the public API, or `<host>/api/v3` on an
/// Enterprise instance.
pub fn api_base(host: &str) -> String {
    crate::auth::device_flow::api_base(host)
}

fn client(token: &str, host: &str) -> Result<Octocrab, String> {
    Octocrab::builder()
        .base_uri(api_base(host))
        .map_err(|_| "Could not reach that GitHub host.".to_string())?
        .personal_token(token.to_string())
        .build()
        .map_err(|_| "Could not reach that GitHub host.".to_string())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PrInfo {
    pub number: u64,
    pub title: String,
    pub state: String,
}

pub async fn list_prs(
    token: &str,
    host: &str,
    owner: &str,
    repo: &str,
) -> Result<Vec<PrInfo>, String> {
    let crab = client(token, host)?;
    let pulls = crab
        .pulls(owner, repo)
        .list()
        .state(octocrab::params::State::Open)
        .send()
        .await
        .map_err(|_| "Could not load the pull requests.".to_string())?;
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

pub async fn list_issues(
    token: &str,
    host: &str,
    owner: &str,
    repo: &str,
) -> Result<Vec<IssueInfo>, String> {
    let crab = client(token, host)?;
    let issues = crab
        .issues(owner, repo)
        .list()
        .state(octocrab::params::State::Open)
        .send()
        .await
        .map_err(|_| "Could not load the issues.".to_string())?;
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

pub async fn create_pr(
    token: &str,
    host: &str,
    owner: &str,
    repo: &str,
    title: &str,
    head: &str,
    base: &str,
) -> Result<u64, String> {
    let crab = client(token, host)?;
    let pr = crab
        .pulls(owner, repo)
        .create(title, head, base)
        .send()
        .await
        .map_err(|_| "Could not create the pull request.".to_string())?;
    Ok(pr.number)
}

pub async fn checkout_pr(
    _token: &str,
    _owner: &str,
    _repo: &str,
    number: u64,
    path: &str,
) -> Result<(), String> {
    let repo = git2::Repository::open(path)
        .map_err(|_| "Could not open the repository.".to_string())?;
    let mut remote = repo
        .find_remote("origin")
        .map_err(|_| "This repository has no origin remote.".to_string())?;
    remote
        .fetch(&[&format!("pull/{number}/head:pr-{number}")], None, None)
        .map_err(|_| "Could not fetch that pull request.".to_string())?;
    let refname = format!("refs/heads/pr-{number}");
    let obj = repo
        .revparse_single(&refname)
        .map_err(|_| "Could not read that pull request.".to_string())?;
    repo.checkout_tree(&obj, None)
        .map_err(|_| "Could not check out that pull request.".to_string())?;
    repo.set_head(&refname)
        .map_err(|_| "Could not check out that pull request.".to_string())?;
    Ok(())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowRun {
    pub id: u64,
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
}

pub async fn list_workflow_runs(
    token: &str,
    host: &str,
    owner: &str,
    repo: &str,
) -> Result<Vec<WorkflowRun>, String> {
    let crab = client(token, host)?;
    let route = format!("/repos/{owner}/{repo}/actions/runs");
    let runs: serde_json::Value = crab
        .get(route, None::<&()>)
        .await
        .map_err(|_| "Could not load the workflow runs.".to_string())?;
    let mut result = Vec::new();
    if let Some(items) = runs.get("workflow_runs").and_then(|v| v.as_array()) {
        for run in items {
            result.push(WorkflowRun {
                id: run.get("id").and_then(|v| v.as_u64()).unwrap_or(0),
                name: run.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                status: run.get("status").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                conclusion: run.get("conclusion").and_then(|v| v.as_str()).map(|s| s.to_string()),
            });
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::api_base;

    #[test]
    fn enterprise_hosts_use_their_own_api_base() {
        assert_eq!(api_base("github.com"), "https://api.github.com");
        assert_eq!(
            api_base("ghe.example.com"),
            "https://ghe.example.com/api/v3"
        );
    }
}
