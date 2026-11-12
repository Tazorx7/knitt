use git2::{DiffOptions, Repository};
use serde::Serialize;
use std::path::Path;

/// One file that differs between two branches.
#[derive(Debug, Serialize)]
pub struct ComparedFile {
    pub path: String,
    /// `added`, `deleted`, `modified`, `renamed`, `copied`, `typechange` or
    /// `unreadable`.
    pub status: String,
}

/// Commits and files that differ between `base` and `head`.
#[derive(Debug, Serialize)]
pub struct BranchComparison {
    /// Commits reachable from `head` but not from `base`.
    pub ahead: usize,
    /// Commits reachable from `base` but not from `head`.
    pub behind: usize,
    pub files: Vec<ComparedFile>,
}

/// File content on both sides of a comparison.
#[derive(Debug, Serialize)]
pub struct ComparedContents {
    pub original: String,
    pub modified: String,
}

fn branch_commit<'a>(
    repo: &'a Repository,
    name: &str,
) -> Result<git2::Commit<'a>, String> {
    let branch = repo
        .find_branch(name, git2::BranchType::Local)
        .map_err(|_| "Branch not found.".to_string())?;
    branch
        .get()
        .peel_to_commit()
        .map_err(|_| "Branch has no commits.".to_string())
}

fn status_label(delta: git2::Delta) -> &'static str {
    match delta {
        git2::Delta::Added => "added",
        git2::Delta::Deleted => "deleted",
        git2::Delta::Modified => "modified",
        git2::Delta::Renamed => "renamed",
        git2::Delta::Copied => "copied",
        git2::Delta::Typechange => "typechange",
        _ => "unreadable",
    }
}

fn delta_path(delta: &git2::DiffDelta<'_>) -> Option<String> {
    delta
        .new_file()
        .path()
        .or_else(|| delta.old_file().path())
        .and_then(|path| path.to_str())
        .map(str::to_string)
}

/// Compares two local branches. `base` is the older side and `head` the newer
/// side, so `files` shows what `head` adds to `base`.
pub fn compare_branches(path: &str, base: &str, head: &str) -> Result<BranchComparison, String> {
    if base == head {
        return Ok(BranchComparison {
            ahead: 0,
            behind: 0,
            files: Vec::new(),
        });
    }
    let repo = Repository::open(path).map_err(|_| "Could not open the repository.".to_string())?;
    let base_commit = branch_commit(&repo, base)?;
    let head_commit = branch_commit(&repo, head)?;

    let (ahead, behind) = repo
        .graph_ahead_behind(head_commit.id(), base_commit.id())
        .map_err(|_| "Could not compare those branches.".to_string())?;

    let base_tree = base_commit
        .tree()
        .map_err(|_| "Could not read the base branch.".to_string())?;
    let head_tree = head_commit
        .tree()
        .map_err(|_| "Could not read the compared branch.".to_string())?;

    let mut options = DiffOptions::new();
    let mut diff = repo
        .diff_tree_to_tree(Some(&base_tree), Some(&head_tree), Some(&mut options))
        .map_err(|_| "Could not compare those branches.".to_string())?;
    // Rename detection keeps the file list readable; it never changes content.
    let _ = diff.find_similar(None);

    let files = diff
        .deltas()
        .filter_map(|delta| {
            delta_path(&delta).map(|path| ComparedFile {
                path,
                status: status_label(delta.status()).to_string(),
            })
        })
        .collect();

    Ok(BranchComparison {
        ahead,
        behind,
        files,
    })
}

fn blob_at(repo: &Repository, tree: &git2::Tree<'_>, file: &str) -> String {
    tree.get_path(Path::new(file))
        .ok()
        .and_then(|entry| entry.to_object(repo).ok())
        .and_then(|object| object.peel_to_blob().ok())
        .and_then(|blob| String::from_utf8(blob.content().to_vec()).ok())
        .unwrap_or_default()
}

/// Contents of `file` on both branches, for the diff viewer.
pub fn compare_file_contents(
    path: &str,
    base: &str,
    head: &str,
    file: &str,
) -> Result<ComparedContents, String> {
    let candidate = Path::new(file);
    if candidate.is_absolute()
        || candidate
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err("Only repository files can be compared.".to_string());
    }
    let repo = Repository::open(path).map_err(|_| "Could not open the repository.".to_string())?;
    let base_tree = branch_commit(&repo, base)?
        .tree()
        .map_err(|_| "Could not read the base branch.".to_string())?;
    let head_tree = branch_commit(&repo, head)?
        .tree()
        .map_err(|_| "Could not read the compared branch.".to_string())?;

    Ok(ComparedContents {
        original: blob_at(&repo, &base_tree, file),
        modified: blob_at(&repo, &head_tree, file),
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
            "knitt-compare-{}-{}",
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

    fn switch(repo: &Repository, oid: git2::Oid) {
        let object = repo.find_commit(oid).unwrap().into_object();
        let mut checkout = git2::build::CheckoutBuilder::new();
        checkout.force();
        repo.checkout_tree(&object, Some(&mut checkout)).unwrap();
        repo.set_head_detached(oid).unwrap();
    }

    #[test]
    fn reports_divergence_and_changed_files() {
        let dir = scratch();
        let repo = Repository::open(&dir).unwrap();
        let base = commit(&repo, "base", &[("shared.txt", "one\n")]);

        switch(&repo, base);
        let feature = commit(&repo, "feature", &[("feature.txt", "new\n")]);
        switch(&repo, base);
        let main = commit(&repo, "main", &[("shared.txt", "two\n")]);

        repo.branch("feature", &repo.find_commit(feature).unwrap(), true)
            .unwrap();
        repo.branch("main", &repo.find_commit(main).unwrap(), true)
            .unwrap();
        drop(repo);

        let comparison = compare_branches(dir.to_str().unwrap(), "main", "feature").unwrap();
        assert_eq!(comparison.ahead, 1);
        assert_eq!(comparison.behind, 1);
        let feature_file = comparison
            .files
            .iter()
            .find(|file| file.path == "feature.txt")
            .expect("feature.txt should differ");
        assert_eq!(feature_file.status, "added");
        let shared = comparison
            .files
            .iter()
            .find(|file| file.path == "shared.txt")
            .expect("shared.txt should differ");
        assert_eq!(shared.status, "modified");

        let contents = compare_file_contents(
            dir.to_str().unwrap(),
            "main",
            "feature",
            "shared.txt",
        )
        .unwrap();
        assert_eq!(contents.original, "two\n");
        assert_eq!(contents.modified, "one\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn same_branch_has_nothing_to_compare() {
        let dir = scratch();
        let repo = Repository::open(&dir).unwrap();
        let oid = commit(&repo, "base", &[("a.txt", "a\n")]);
        repo.branch("main", &repo.find_commit(oid).unwrap(), true)
            .unwrap();
        drop(repo);

        let comparison = compare_branches(dir.to_str().unwrap(), "main", "main").unwrap();
        assert_eq!(comparison.ahead, 0);
        assert_eq!(comparison.behind, 0);
        assert!(comparison.files.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn missing_branch_is_reported() {
        let dir = scratch();
        let repo = Repository::open(&dir).unwrap();
        let oid = commit(&repo, "base", &[("a.txt", "a\n")]);
        repo.branch("main", &repo.find_commit(oid).unwrap(), true)
            .unwrap();
        drop(repo);

        assert_eq!(
            compare_branches(dir.to_str().unwrap(), "main", "nope").unwrap_err(),
            "Branch not found."
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn escaping_paths_are_refused() {
        let dir = scratch();
        let repo = Repository::open(&dir).unwrap();
        commit(&repo, "base", &[("a.txt", "a\n")]);
        drop(repo);

        let result = compare_file_contents(dir.to_str().unwrap(), "main", "main", "../secret");
        assert_eq!(
            result.unwrap_err(),
            "Only repository files can be compared."
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
