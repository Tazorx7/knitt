use git2::{Repository, StatusOptions};
use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct RepoInfo {
    pub path: String,
    pub name: String,
    pub branch: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct FileStatus {
    pub path: String,
    pub status: String,
    pub staged: bool,
    pub unstaged: bool,
}

pub fn open_repo(path: &str) -> Result<RepoInfo, String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let name = repo
        .workdir()
        .and_then(|p| p.file_name())
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_string();
    let branch = repo
        .head()
        .ok()
        .and_then(|h| h.shorthand().map(|s| s.to_string()));
    Ok(RepoInfo {
        path: path.to_string(),
        name,
        branch,
    })
}

pub fn get_status(path: &str) -> Result<Vec<FileStatus>, String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let mut opts = StatusOptions::new();
    opts.include_untracked(true);
    let statuses = repo.statuses(Some(&mut opts)).map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    for entry in statuses.iter() {
        let s = entry.status();
        let path_str = entry.path().unwrap_or("").to_string();
        let staged = s.is_index_new()
            || s.is_index_modified()
            || s.is_index_deleted()
            || s.is_index_renamed()
            || s.is_index_typechange();
        let unstaged = s.is_wt_new()
            || s.is_wt_modified()
            || s.is_wt_deleted()
            || s.is_wt_renamed()
            || s.is_wt_typechange();
        let status_str = format!("{:?}", s);
        result.push(FileStatus {
            path: path_str,
            status: status_str,
            staged,
            unstaged,
        });
    }
    Ok(result)
}
