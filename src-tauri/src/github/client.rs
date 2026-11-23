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

/// One job of a workflow run. Steps belong to the job but their names are only
/// shown in the log, so the list keeps the job's own state.
#[derive(Debug, Serialize, Deserialize)]
pub struct WorkflowJob {
    pub id: u64,
    pub name: String,
    pub status: String,
    pub conclusion: Option<String>,
}

pub async fn list_workflow_jobs(
    token: &str,
    host: &str,
    owner: &str,
    repo: &str,
    run_id: u64,
) -> Result<Vec<WorkflowJob>, String> {
    let crab = client(token, host)?;
    let route = format!("/repos/{owner}/{repo}/actions/runs/{run_id}/jobs");
    let payload: serde_json::Value = crab
        .get(route, None::<&()>)
        .await
        .map_err(|_| "Could not load the workflow jobs.".to_string())?;
    Ok(parse_jobs(&payload))
}

fn parse_jobs(payload: &serde_json::Value) -> Vec<WorkflowJob> {
    let mut result = Vec::new();
    if let Some(items) = payload.get("jobs").and_then(|v| v.as_array()) {
        for job in items {
            result.push(WorkflowJob {
                id: job.get("id").and_then(|v| v.as_u64()).unwrap_or(0),
                name: job.get("name").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                status: job.get("status").and_then(|v| v.as_str()).unwrap_or("").to_string(),
                conclusion: job
                    .get("conclusion")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
            });
        }
    }
    result
}

/// Reads a job's log as plain text. GitHub answers with a redirect to a signed
/// download; the request follows it, and the token is not needed there.
pub async fn workflow_job_log(
    token: &str,
    host: &str,
    owner: &str,
    repo: &str,
    job_id: u64,
) -> Result<String, String> {
    let url = format!(
        "{}/repos/{owner}/{repo}/actions/jobs/{job_id}/logs",
        api_base(host)
    );
    let response = reqwest::Client::new()
        .get(url)
        .bearer_auth(token)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|_| "Could not download the job log.".to_string())?;
    if !response.status().is_success() {
        return Err("GitHub did not return that job log.".to_string());
    }
    let text = response
        .text()
        .await
        .map_err(|_| "Could not read the job log.".to_string())?;
    Ok(tail(&text, LOG_LIMIT))
}

/// Job logs can be tens of megabytes; the end is where a failure is, so only
/// the last `limit` bytes are kept.
const LOG_LIMIT: usize = 256 * 1024;

fn tail(text: &str, limit: usize) -> String {
    if text.len() <= limit {
        return text.to_string();
    }
    let mut start = text.len() - limit;
    while !text.is_char_boundary(start) {
        start += 1;
    }
    let start = text[start..]
        .find('\n')
        .map(|newline| start + newline + 1)
        .unwrap_or(start);
    format!("[earlier output omitted]\n{}", &text[start..])
}

#[cfg(test)]
mod tests {
    use super::{api_base, parse_jobs, tail};

    #[test]
    fn enterprise_hosts_use_their_own_api_base() {
        assert_eq!(api_base("github.com"), "https://api.github.com");
        assert_eq!(
            api_base("ghe.example.com"),
            "https://ghe.example.com/api/v3"
        );
    }

    #[test]
    fn jobs_are_read_from_the_jobs_array() {
        let payload = serde_json::json!({
            "total_count": 2,
            "jobs": [
                { "id": 11, "name": "test", "status": "completed", "conclusion": "success" },
                { "id": 12, "name": "build", "status": "in_progress" }
            ]
        });
        let jobs = parse_jobs(&payload);
        assert_eq!(jobs.len(), 2);
        assert_eq!(jobs[0].id, 11);
        assert_eq!(jobs[0].conclusion.as_deref(), Some("success"));
        assert_eq!(jobs[1].status, "in_progress");
        assert_eq!(jobs[1].conclusion, None);
        assert!(parse_jobs(&serde_json::json!({})).is_empty());
    }

    #[test]
    fn long_logs_keep_their_ending() {
        assert_eq!(tail("short log", 100), "short log");
        let long = format!("{}\nlast line", "x".repeat(200));
        let kept = tail(&long, 20);
        assert!(kept.starts_with("[earlier output omitted]\n"));
        assert!(kept.ends_with("last line"));
        // A cut inside a multi-byte character must not panic.
        let unicode = format!("{}\néchec", "é".repeat(50));
        assert!(tail(&unicode, 7).ends_with("échec"));
    }
}
