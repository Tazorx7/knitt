use git2::{IndexAddOption, ObjectType, Repository};
use std::path::Path;

/// Rejects paths that would leave the repository. Raw libgit2 errors can name
/// absolute paths, so callers get a plain message instead.
fn repository_paths(files: &[String], action: &str) -> Result<(), String> {
    if files.is_empty() {
        return Err("Choose a file first.".to_string());
    }
    for file in files {
        let candidate = Path::new(file);
        if candidate.is_absolute()
            || candidate
                .components()
                .any(|part| matches!(part, std::path::Component::ParentDir))
        {
            return Err(format!("Only repository files can be {action}."));
        }
    }
    Ok(())
}

pub fn stage_files(path: &str, files: &[String]) -> Result<(), String> {
    repository_paths(files, "staged")?;
    let repo = Repository::open(path).map_err(|_| "Could not open the repository.".to_string())?;
    let mut index = repo.index().map_err(|_| "Could not read the index.".to_string())?;
    let workdir = repo
        .workdir()
        .ok_or_else(|| "This repository has no working tree.".to_string())?
        .to_path_buf();
    for file in files {
        let path = Path::new(file);
        // A deleted file is staged by dropping its index entry, not by adding
        // a path that no longer exists. The check must use the repository's
        // working directory, not the process directory.
        let result = if workdir.join(path).exists() {
            index.add_path(path)
        } else {
            index.remove_path(path)
        };
        result.map_err(|_| "Could not stage that file.".to_string())?;
    }
    index
        .write()
        .map_err(|_| "Could not save the staged changes.".to_string())?;
    Ok(())
}

pub fn unstage_files(path: &str, files: &[String]) -> Result<(), String> {
    repository_paths(files, "unstaged")?;
    let repo = Repository::open(path).map_err(|_| "Could not open the repository.".to_string())?;
    match repo.head() {
        Ok(head) => {
            let commit = head
                .peel_to_commit()
                .map_err(|_| "Could not read the last commit.".to_string())?;
            let object = commit.into_object();
            repo.reset_default(Some(&object), files.iter().map(|s| s.as_str()))
                .map_err(|_| "Could not unstage those changes.".to_string())?;
        }
        Err(_) => {
            // No commit yet: unstaging means dropping the entry from the index.
            let mut index = repo
                .index()
                .map_err(|_| "Could not read the index.".to_string())?;
            for file in files {
                let _ = index.remove_path(Path::new(file));
            }
            index
                .write()
                .map_err(|_| "Could not save the staged changes.".to_string())?;
        }
    }
    Ok(())
}

pub fn stage_all(path: &str) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|_| "Could not open the repository.".to_string())?;
    let mut index = repo.index().map_err(|_| "Could not read the index.".to_string())?;
    index
        .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
        .map_err(|_| "Could not stage the changes.".to_string())?;
    index
        .write()
        .map_err(|_| "Could not save the staged changes.".to_string())?;
    Ok(())
}

pub fn stage_hunk(path: &str, file: &str, hunk_header: &str) -> Result<(), String> {
    crate::git::hunks::stage_hunk(path, file, hunk_header)
}

pub fn unstage_hunk(path: &str, file: &str, hunk_header: &str) -> Result<(), String> {
    crate::git::hunks::unstage_hunk(path, file, hunk_header)
}

/// Restores tracked files to their committed state, discarding index and
/// working-tree changes. Untracked files are refused: deleting them is a
/// separate, riskier action that Knitt does not offer yet.
pub fn discard_files(path: &str, files: &[String]) -> Result<(), String> {
    if files.is_empty() {
        return Err("Choose a file to discard.".to_string());
    }
    for file in files {
        let candidate = Path::new(file);
        if candidate.is_absolute()
            || candidate
                .components()
                .any(|part| matches!(part, std::path::Component::ParentDir))
        {
            return Err("Only repository files can be discarded.".to_string());
        }
    }

    let repo = Repository::open(path).map_err(|_| "Could not open the repository.".to_string())?;
    let head = repo
        .head()
        .map_err(|_| "This repository has no commits yet.".to_string())?;
    let object = head
        .peel(ObjectType::Commit)
        .map_err(|_| "This repository has no commits yet.".to_string())?;
    let tree = object
        .peel_to_tree()
        .map_err(|_| "Could not read the committed files.".to_string())?;

    for file in files {
        if tree.get_path(Path::new(file)).is_err() {
            return Err("Only tracked files can be discarded.".to_string());
        }
    }

    repo.reset_default(Some(&object), files.iter().map(|file| file.as_str()))
        .map_err(|_| "Could not discard those changes.".to_string())?;

    let mut checkout = git2::build::CheckoutBuilder::new();
    checkout.force();
    for file in files {
        checkout.path(file.as_str());
    }
    repo.checkout_head(Some(&mut checkout))
        .map_err(|_| "Could not update the working tree.".to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_repo() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "knitt-discard-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let repo = Repository::init(&dir).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
        drop(config);

        std::fs::write(dir.join("tracked.txt"), "original\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("tracked.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();
        dir
    }

    #[test]
    fn discards_working_tree_changes() {
        let dir = temp_repo();
        std::fs::write(dir.join("tracked.txt"), "changed\n").unwrap();

        discard_files(dir.to_str().unwrap(), &["tracked.txt".to_string()]).unwrap();

        let content = std::fs::read_to_string(dir.join("tracked.txt")).unwrap();
        assert_eq!(content, "original\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuses_untracked_and_escaping_paths() {
        let dir = temp_repo();
        std::fs::write(dir.join("untracked.txt"), "new\n").unwrap();

        assert!(discard_files(dir.to_str().unwrap(), &["untracked.txt".to_string()]).is_err());
        assert!(discard_files(dir.to_str().unwrap(), &["../escape.txt".to_string()]).is_err());
        assert!(discard_files(dir.to_str().unwrap(), &[]).is_err());
        assert!(dir.join("untracked.txt").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn scratch(name: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "knitt-staging-{name}-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn committed_repo(dir: &Path) -> Repository {
        let repo = Repository::init(dir).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
        std::fs::write(dir.join("a.txt"), "one\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("a.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        {
            let tree = repo.find_tree(tree_id).unwrap();
            let sig = repo.signature().unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
                .unwrap();
        }
        repo
    }

    fn status_of(repo: &Repository, name: &str) -> git2::Status {
        let mut options = git2::StatusOptions::new();
        options.include_untracked(true);
        let statuses = repo.statuses(Some(&mut options)).unwrap();
        statuses
            .iter()
            .find(|entry| entry.path() == Some(name))
            .map(|entry| entry.status())
            .unwrap_or(git2::Status::CURRENT)
    }

    #[test]
    fn stage_then_unstage_round_trips() {
        let dir = scratch("round-trip");
        let repo = committed_repo(&dir);
        std::fs::write(dir.join("a.txt"), "two\n").unwrap();
        drop(repo);

        stage_files(dir.to_str().unwrap(), &["a.txt".to_string()]).unwrap();
        let repo = Repository::open(&dir).unwrap();
        assert!(status_of(&repo, "a.txt").is_index_modified());
        drop(repo);

        unstage_files(dir.to_str().unwrap(), &["a.txt".to_string()]).unwrap();
        let repo = Repository::open(&dir).unwrap();
        let status = status_of(&repo, "a.txt");
        assert!(status.is_wt_modified());
        assert!(!status.is_index_modified());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stages_a_deleted_file_as_a_deletion() {
        let dir = scratch("deleted");
        let repo = committed_repo(&dir);
        std::fs::remove_file(dir.join("a.txt")).unwrap();
        drop(repo);

        stage_files(dir.to_str().unwrap(), &["a.txt".to_string()]).unwrap();

        let repo = Repository::open(&dir).unwrap();
        assert!(status_of(&repo, "a.txt").is_index_deleted());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unstage_works_before_the_first_commit() {
        let dir = scratch("unborn");
        let repo = Repository::init(&dir).unwrap();
        std::fs::write(dir.join("new.txt"), "new\n").unwrap();
        drop(repo);

        stage_files(dir.to_str().unwrap(), &["new.txt".to_string()]).unwrap();
        unstage_files(dir.to_str().unwrap(), &["new.txt".to_string()]).unwrap();

        let repo = Repository::open(&dir).unwrap();
        let status = status_of(&repo, "new.txt");
        assert!(status.is_wt_new());
        assert!(!status.is_index_new());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuses_paths_outside_the_repository() {
        let dir = scratch("escape");
        let repo = Repository::init(&dir).unwrap();
        drop(repo);

        assert_eq!(
            stage_files(dir.to_str().unwrap(), &["../secret".to_string()]).unwrap_err(),
            "Only repository files can be staged."
        );
        assert_eq!(
            unstage_files(dir.to_str().unwrap(), &["/etc/passwd".to_string()]).unwrap_err(),
            "Only repository files can be unstaged."
        );
        assert_eq!(
            stage_files(dir.to_str().unwrap(), &[]).unwrap_err(),
            "Choose a file first."
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
