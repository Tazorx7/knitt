use git2::{Repository, Sort};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct CommitInfo {
    pub id: String,
    pub message: String,
    pub author: String,
    pub time: i64,
}

fn open(path: &str) -> Result<Repository, String> {
    Repository::open(path).map_err(|_| "Could not open the repository.".to_string())
}

/// Reads one page of the history of HEAD, newest first.
///
/// `skip` lets the history panel page through a long log without re-reading
/// the commits it already has, which is what keeps a virtualised list cheap.
pub fn log_commits(path: &str, limit: usize, skip: usize) -> Result<Vec<CommitInfo>, String> {
    let repo = open(path)?;
    let mut revwalk = repo
        .revwalk()
        .map_err(|_| "Could not read the history.".to_string())?;
    revwalk
        .push_head()
        .map_err(|_| "No commits on the current branch.".to_string())?;
    // Topological inside time: with several commits made in the same second
    // plain time sorting is unstable, which would make paging repeat or skip
    // commits.
    revwalk
        .set_sorting(Sort::TIME | Sort::TOPOLOGICAL)
        .map_err(|_| "Could not read the history.".to_string())?;

    let mut commits = Vec::new();
    for oid in revwalk.skip(skip).take(limit) {
        let oid = oid.map_err(|_| "Could not read the history.".to_string())?;
        let commit = repo
            .find_commit(oid)
            .map_err(|_| "Could not read a commit.".to_string())?;
        let message = commit.message().unwrap_or("").to_string();
        let author = commit.author();
        let name = author.name().unwrap_or("");
        let email = author.email().unwrap_or("");
        commits.push(CommitInfo {
            id: oid.to_string(),
            message,
            author: format!("{name} <{email}>"),
            time: author.when().seconds(),
        });
    }
    Ok(commits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn scratch(name: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "knitt-history-{name}-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn repo_with_commits(dir: &Path, count: usize) -> Repository {
        let repo = Repository::init(dir).unwrap();
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();
        }
        for index in 0..count {
            let file = format!("f{index}.txt");
            std::fs::write(dir.join(&file), format!("{index}\n")).unwrap();
            let mut index_file = repo.index().unwrap();
            index_file.add_path(Path::new(&file)).unwrap();
            index_file.write().unwrap();
            let tree_id = index_file.write_tree().unwrap();
            let tree = repo.find_tree(tree_id).unwrap();
            let sig = repo.signature().unwrap();
            let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
            let parents: Vec<&git2::Commit> = parent.iter().collect();
            repo.commit(Some("HEAD"), &sig, &sig, &format!("commit {index}"), &tree, &parents)
                .unwrap();
        }
        repo
    }

    #[test]
    fn pages_through_the_history() {
        let dir = scratch("pages");
        repo_with_commits(&dir, 5);

        let first = log_commits(dir.to_str().unwrap(), 2, 0).unwrap();
        assert_eq!(first.len(), 2);
        assert_eq!(first[0].message, "commit 4");
        assert_eq!(first[1].message, "commit 3");

        let second = log_commits(dir.to_str().unwrap(), 2, 2).unwrap();
        assert_eq!(
            second.iter().map(|c| c.message.as_str()).collect::<Vec<_>>(),
            ["commit 2", "commit 1"]
        );

        let last = log_commits(dir.to_str().unwrap(), 2, 4).unwrap();
        assert_eq!(last.len(), 1, "the last page is short");
        assert_eq!(last[0].message, "commit 0");
        assert_eq!(last[0].author, "Test <test@example.com>");

        assert!(
            log_commits(dir.to_str().unwrap(), 2, 10).unwrap().is_empty(),
            "paging past the end returns nothing"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn empty_history_is_not_an_error_to_report_as_missing() {
        let dir = scratch("empty");
        Repository::init(&dir).unwrap();
        assert_eq!(
            log_commits(dir.to_str().unwrap(), 10, 0).unwrap_err(),
            "No commits on the current branch."
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_repository_is_a_plain_message() {
        let dir = scratch("missing");
        let missing = dir.join("nope");
        assert_eq!(
            log_commits(missing.to_str().unwrap(), 10, 0).unwrap_err(),
            "Could not open the repository."
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
