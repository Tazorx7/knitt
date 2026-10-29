use git2::{Repository, Sort};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitInfo {
    pub id: String,
    pub message: String,
    pub author: String,
    pub time: i64,
}

pub fn log_commits(path: &str, limit: usize) -> Result<Vec<CommitInfo>, String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let mut revwalk = repo.revwalk().map_err(|e| e.to_string())?;
    revwalk.push_head().map_err(|e| e.to_string())?;
    revwalk.set_sorting(Sort::TIME).map_err(|e| e.to_string())?;
    let mut commits = Vec::new();
    for oid in revwalk.take(limit) {
        let oid = oid.map_err(|e| e.to_string())?;
        let commit = repo.find_commit(oid).map_err(|e| e.to_string())?;
        let message = commit.message().unwrap_or("").to_string();
        let author = commit.author();
        let name = author.name().unwrap_or("").to_string();
        let email = author.email().unwrap_or("").to_string();
        commits.push(CommitInfo {
            id: oid.to_string(),
            message,
            author: format!("{} <{}>", name, email),
            time: author.when().seconds(),
        });
    }
    Ok(commits)
}
