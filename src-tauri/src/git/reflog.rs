use git2::Repository;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ReflogEntry {
    pub message: String,
    pub id: String,
    /// Seconds since the epoch, from the reflog entry's committer.
    pub time: i64,
}

/// Most recent reflog entries for HEAD (newest first). This is how a user can
/// find a commit that a reset, amend or rebase moved away.
pub fn list_reflog(path: &str) -> Result<Vec<ReflogEntry>, String> {
    let repo = Repository::open(path).map_err(|_| "Could not open the repository.".to_string())?;
    let reflog = repo
        .reflog("HEAD")
        .map_err(|_| "Could not read the reflog.".to_string())?;
    let mut entries = Vec::new();
    for i in 0..reflog.len().min(50) {
        if let Some(entry) = reflog.get(i) {
            entries.push(ReflogEntry {
                message: entry.message().unwrap_or("").trim().to_string(),
                id: entry.id_new().to_string(),
                time: entry.committer().when().seconds(),
            });
        }
    }
    Ok(entries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn scratch() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "knitt-reflog-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn commit(repo: &Repository, message: &str) -> git2::Oid {
        let sig = repo.signature().unwrap();
        let mut index = repo.index().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
        let parents: Vec<&git2::Commit> = parent.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
            .unwrap()
    }

    #[test]
    fn lists_newest_first_with_messages() {
        let dir = scratch();
        let repo = Repository::init(&dir).unwrap();
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();
        }
        std::fs::write(dir.join("a.txt"), "a\n").unwrap();
        {
            let mut index = repo.index().unwrap();
            index.add_path(Path::new("a.txt")).unwrap();
            index.write().unwrap();
        }
        commit(&repo, "first");
        let head = commit(&repo, "second");
        drop(repo);

        let entries = list_reflog(dir.to_str().unwrap()).unwrap();
        assert!(entries.len() >= 2);
        assert_eq!(entries[0].id, head.to_string());
        assert!(entries[0].message.contains("second"));
        assert!(entries[0].time > 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn an_empty_repository_has_no_entries() {
        let dir = scratch();
        let repo = Repository::init(&dir).unwrap();
        drop(repo);
        assert!(list_reflog(dir.to_str().unwrap()).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
