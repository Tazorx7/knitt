use git2::{DiffOptions, Repository};
use serde::Serialize;
use std::path::Path;

use super::compare::{delta_path, status_label};

#[derive(Debug, Serialize)]
pub struct CommitFile {
    pub path: String,
    pub status: String,
}

#[derive(Debug, Serialize)]
pub struct CommitDetail {
    pub id: String,
    pub message: String,
    pub author: String,
    pub time: i64,
    pub files: Vec<CommitFile>,
}

#[derive(Debug, Serialize)]
pub struct CommitFileContents {
    pub original: String,
    pub modified: String,
}

fn find_commit<'a>(repo: &'a Repository, oid: &str) -> Result<git2::Commit<'a>, String> {
    let oid = git2::Oid::from_str(oid).map_err(|_| "Commit not found.".to_string())?;
    repo.find_commit(oid)
        .map_err(|_| "Commit not found.".to_string())
}

/// A commit's metadata and the files it changed against its first parent.
/// Root commits are compared against an empty tree.
pub fn commit_detail(path: &str, oid: &str) -> Result<CommitDetail, String> {
    let repo = Repository::open(path).map_err(|_| "Could not open the repository.".to_string())?;
    let commit = find_commit(&repo, oid)?;

    let tree = commit
        .tree()
        .map_err(|_| "Could not read the commit.".to_string())?;
    let parent_tree = commit.parent(0).ok().and_then(|parent| parent.tree().ok());

    let mut options = DiffOptions::new();
    let mut diff = repo
        .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), Some(&mut options))
        .map_err(|_| "Could not read the commit changes.".to_string())?;
    let _ = diff.find_similar(None);

    let files = diff
        .deltas()
        .filter_map(|delta| {
            delta_path(&delta).map(|path| CommitFile {
                path,
                status: status_label(delta.status()).to_string(),
            })
        })
        .collect();

    let author = commit.author();
    Ok(CommitDetail {
        id: commit.id().to_string(),
        message: commit.message().unwrap_or("").to_string(),
        author: format!(
            "{} <{}>",
            author.name().unwrap_or(""),
            author.email().unwrap_or("")
        ),
        time: author.when().seconds(),
        files,
    })
}

fn blob_at(repo: &Repository, tree: Option<&git2::Tree<'_>>, file: &str) -> String {
    tree.and_then(|tree| tree.get_path(Path::new(file)).ok())
        .and_then(|entry| entry.to_object(repo).ok())
        .and_then(|object| object.peel_to_blob().ok())
        .and_then(|blob| String::from_utf8(blob.content().to_vec()).ok())
        .unwrap_or_default()
}

/// Contents of one file before and after a commit, for the diff viewer.
pub fn commit_file_contents(
    path: &str,
    oid: &str,
    file: &str,
) -> Result<CommitFileContents, String> {
    let candidate = Path::new(file);
    if candidate.is_absolute()
        || candidate
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err("Only repository files can be viewed.".to_string());
    }
    let repo = Repository::open(path).map_err(|_| "Could not open the repository.".to_string())?;
    let commit = find_commit(&repo, oid)?;
    let tree = commit
        .tree()
        .map_err(|_| "Could not read the commit.".to_string())?;
    let parent_tree = commit.parent(0).ok().and_then(|parent| parent.tree().ok());

    Ok(CommitFileContents {
        original: blob_at(&repo, parent_tree.as_ref(), file),
        modified: blob_at(&repo, Some(&tree), file),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn scratch() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "knitt-commit-diff-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let repo = Repository::init(&dir).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
        dir
    }

    fn commit(repo: &Repository, message: &str, files: &[(&str, &str)]) -> git2::Oid {
        for (name, content) in files {
            std::fs::write(repo.workdir().unwrap().join(name), content).unwrap();
            let mut index = repo.index().unwrap();
            index.add_path(Path::new(name)).unwrap();
            index.write().unwrap();
        }
        let mut index = repo.index().unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();
        let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
        let parents: Vec<&git2::Commit> = parent.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
            .unwrap()
    }

    #[test]
    fn root_commit_shows_every_file_as_added() {
        let dir = scratch();
        let repo = Repository::open(&dir).unwrap();
        let oid = commit(&repo, "initial", &[("a.txt", "a\n"), ("b.txt", "b\n")]);
        drop(repo);

        let detail = commit_detail(dir.to_str().unwrap(), &oid.to_string()).unwrap();
        assert_eq!(detail.message.trim(), "initial");
        assert_eq!(detail.files.len(), 2);
        assert!(detail.files.iter().all(|file| file.status == "added"));
        let contents =
            commit_file_contents(dir.to_str().unwrap(), &oid.to_string(), "a.txt").unwrap();
        assert_eq!(contents.original, "");
        assert_eq!(contents.modified, "a\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn later_commit_shows_only_its_changes() {
        let dir = scratch();
        let repo = Repository::open(&dir).unwrap();
        commit(&repo, "initial", &[("a.txt", "one\n"), ("keep.txt", "k\n")]);
        let oid = commit(&repo, "update", &[("a.txt", "two\n")]);
        drop(repo);

        let detail = commit_detail(dir.to_str().unwrap(), &oid.to_string()).unwrap();
        assert_eq!(detail.files.len(), 1);
        assert_eq!(detail.files[0].path, "a.txt");
        assert_eq!(detail.files[0].status, "modified");

        let contents =
            commit_file_contents(dir.to_str().unwrap(), &oid.to_string(), "a.txt").unwrap();
        assert_eq!(contents.original, "one\n");
        assert_eq!(contents.modified, "two\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unknown_commit_is_reported() {
        let dir = scratch();
        let repo = Repository::open(&dir).unwrap();
        commit(&repo, "initial", &[("a.txt", "a\n")]);
        drop(repo);

        assert_eq!(
            commit_detail(dir.to_str().unwrap(), "not-an-oid").unwrap_err(),
            "Commit not found."
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn escaping_paths_are_refused() {
        let dir = scratch();
        let repo = Repository::open(&dir).unwrap();
        let oid = commit(&repo, "initial", &[("a.txt", "a\n")]);
        drop(repo);

        assert_eq!(
            commit_file_contents(dir.to_str().unwrap(), &oid.to_string(), "../secret").unwrap_err(),
            "Only repository files can be viewed."
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
