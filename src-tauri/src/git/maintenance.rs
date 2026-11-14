use git2::{Repository, ResetType, Status, StatusOptions};
use std::path::Path;

/// What a reset does to the index and the working tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetMode {
    /// Move the branch only; the index and files keep their changes.
    Soft,
    /// Move the branch and reset the index; changes stay in the files.
    Mixed,
    /// Move the branch and throw the changes away.
    Hard,
}

impl ResetMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "soft" => Some(Self::Soft),
            "mixed" => Some(Self::Mixed),
            "hard" => Some(Self::Hard),
            _ => None,
        }
    }

    fn reset_type(self) -> ResetType {
        match self {
            Self::Soft => ResetType::Soft,
            Self::Mixed => ResetType::Mixed,
            Self::Hard => ResetType::Hard,
        }
    }
}

fn open(path: &str) -> Result<Repository, String> {
    Repository::open(path).map_err(|_| "Could not open the repository.".to_string())
}

/// Moves the checked-out branch to `target` (or HEAD when `None`).
///
/// A reset during a merge or rebase would leave the operation pointing at
/// commits that no longer exist, so it is refused until the operation is
/// finished or abandoned.
pub fn reset_branch(path: &str, mode: ResetMode, target: Option<&str>) -> Result<(), String> {
    let repo = open(path)?;
    if repo.state() != git2::RepositoryState::Clean {
        return Err("Finish or abandon the current operation first.".to_string());
    }
    let object = match target {
        Some(target) => repo
            .revparse_single(target)
            .map_err(|_| "That commit is not in this repository.".to_string())?,
        None => repo
            .head()
            .and_then(|head| head.peel(git2::ObjectType::Commit))
            .map_err(|_| "No commits on the current branch.".to_string())?,
    };
    repo.reset(&object, mode.reset_type(), None)
        .map_err(|_| "Could not reset the branch.".to_string())?;
    Ok(())
}

/// Deletes untracked files, and untracked directories when `directories` is
/// set. Ignored files are left alone, tracked files are never touched, and a
/// nested repository is kept because removing it would discard another
/// project's history.
///
/// Returns how many files were removed.
pub fn clean_untracked(path: &str, directories: bool) -> Result<usize, String> {
    let repo = open(path)?;
    let workdir = repo
        .workdir()
        .ok_or_else(|| "This repository has no working tree.".to_string())?
        .to_path_buf();

    let mut options = StatusOptions::new();
    options
        .include_untracked(true)
        // Without recursion an untracked directory is one entry, which is what
        // lets `directories == false` leave its contents alone.
        .recurse_untracked_dirs(directories)
        .include_ignored(false);
    let statuses = repo
        .statuses(Some(&mut options))
        .map_err(|_| "Could not read the working tree.".to_string())?;

    let mut targets: Vec<String> = Vec::new();
    for entry in statuses.iter() {
        let Some(relative) = entry.path() else {
            continue;
        };
        if entry.status() != Status::WT_NEW {
            continue;
        }
        if relative.split('/').any(|part| part == ".git") {
            continue;
        }
        if relative.ends_with('/') && !directories {
            continue;
        }
        targets.push(relative.to_string());
    }

    let mut removed = 0;
    for relative in targets {
        let full = workdir.join(&relative);
        // A path that resolved outside the working tree is not ours to delete.
        if !full.starts_with(&workdir) {
            continue;
        }
        if full.is_dir() {
            if contains_nested_repository(&full) {
                continue;
            }
            if std::fs::remove_dir_all(&full).is_ok() {
                removed += 1;
            }
        } else if std::fs::remove_file(&full).is_ok() {
            removed += 1;
        }
    }

    if directories {
        // Untracked files often live in directories that are now empty.
        remove_empty_directories(&workdir, &mut removed);
    }
    Ok(removed)
}

/// True when `dir` (or a subdirectory) holds a `.git` entry, so deleting it
/// would remove another repository.
fn contains_nested_repository(dir: &Path) -> bool {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return true;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.file_name().is_some_and(|name| name == ".git") {
            return true;
        }
        if path.is_dir() && contains_nested_repository(&path) {
            return true;
        }
    }
    false
}

/// Removes empty directories below the working tree, skipping `.git`.
fn remove_empty_directories(dir: &Path, removed: &mut usize) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() || path.file_name().is_some_and(|name| name == ".git") {
            continue;
        }
        remove_empty_directories(&path, removed);
        if std::fs::read_dir(&path).is_ok_and(|mut entries| entries.next().is_none())
            && std::fs::remove_dir(&path).is_ok()
        {
            *removed += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn scratch(name: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "knitt-maintenance-{name}-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn init(dir: &Path) -> Repository {
        let repo = Repository::init(dir).unwrap();
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();
        }
        repo
    }

    fn commit(repo: &Repository, message: &str, files: &[(&str, &str)]) -> git2::Oid {
        for (name, content) in files {
            let path = repo.workdir().unwrap().join(name);
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(path, content).unwrap();
            let mut index = repo.index().unwrap();
            index.add_path(Path::new(name)).unwrap();
            index.write().unwrap();
        }
        let mut index = repo.index().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();
        let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
        let parents: Vec<&git2::Commit> = parent.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
            .unwrap()
    }

    fn head_id(dir: &Path) -> git2::Oid {
        Repository::open(dir).unwrap().head().unwrap().target().unwrap()
    }

    #[test]
    fn soft_reset_keeps_changes_staged() {
        let dir = scratch("soft");
        let repo = init(&dir);
        let first = commit(&repo, "one", &[("a.txt", "one\n")]);
        commit(&repo, "two", &[("a.txt", "two\n")]);
        drop(repo);

        reset_branch(dir.to_str().unwrap(), ResetMode::Soft, Some(&first.to_string())).unwrap();

        let repo = Repository::open(&dir).unwrap();
        assert_eq!(repo.head().unwrap().target().unwrap(), first);
        let index = repo.index().unwrap();
        let entry = index.get_path(Path::new("a.txt"), 0).unwrap();
        let blob = repo.find_blob(entry.id).unwrap();
        assert_eq!(
            std::str::from_utf8(blob.content()).unwrap(),
            "two\n",
            "the new content is staged"
        );
        assert_eq!(std::fs::read_to_string(dir.join("a.txt")).unwrap(), "two\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn mixed_reset_unstages_but_keeps_files() {
        let dir = scratch("mixed");
        let repo = init(&dir);
        let first = commit(&repo, "one", &[("a.txt", "one\n")]);
        commit(&repo, "two", &[("a.txt", "two\n")]);
        drop(repo);

        reset_branch(dir.to_str().unwrap(), ResetMode::Mixed, Some(&first.to_string())).unwrap();

        let repo = Repository::open(&dir).unwrap();
        assert_eq!(repo.head().unwrap().target().unwrap(), first);
        let index = repo.index().unwrap();
        let entry = index.get_path(Path::new("a.txt"), 0).unwrap();
        let blob = repo.find_blob(entry.id).unwrap();
        assert_eq!(
            std::str::from_utf8(blob.content()).unwrap(),
            "one\n",
            "the index matches the new HEAD"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("a.txt")).unwrap(),
            "two\n",
            "the working tree still has the newer content"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn hard_reset_discards_the_working_tree() {
        let dir = scratch("hard");
        let repo = init(&dir);
        let first = commit(&repo, "one", &[("a.txt", "one\n")]);
        commit(&repo, "two", &[("a.txt", "two\n")]);
        std::fs::write(dir.join("b.txt"), "untracked\n").unwrap();
        drop(repo);

        reset_branch(dir.to_str().unwrap(), ResetMode::Hard, Some(&first.to_string())).unwrap();

        assert_eq!(head_id(&dir), first);
        assert_eq!(
            std::fs::read_to_string(dir.join("a.txt")).unwrap(),
            "one\n"
        );
        assert!(
            dir.join("b.txt").is_file(),
            "hard reset does not remove untracked files"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reset_refuses_an_in_progress_operation_and_a_bad_target() {
        let dir = scratch("refusals");
        let repo = init(&dir);
        commit(&repo, "one", &[("a.txt", "one\n")]);
        drop(repo);

        assert_eq!(
            reset_branch(dir.to_str().unwrap(), ResetMode::Hard, Some("not-a-rev")).unwrap_err(),
            "That commit is not in this repository."
        );
        assert!(ResetMode::parse("hard").is_some());
        assert!(ResetMode::parse("nope").is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn clean_removes_untracked_files_and_keeps_everything_else() {
        let dir = scratch("clean");
        let repo = init(&dir);
        commit(
            &repo,
            "base",
            &[("a.txt", "tracked\n"), (".gitignore", "ignored.log\n")],
        );
        drop(repo);

        std::fs::write(dir.join("loose.txt"), "loose\n").unwrap();
        std::fs::create_dir_all(dir.join("nested/deeper")).unwrap();
        std::fs::write(dir.join("nested/deeper/x.txt"), "x\n").unwrap();
        std::fs::write(dir.join("ignored.log"), "ignored\n").unwrap();
        std::fs::write(dir.join("a.txt"), "changed\n").unwrap();

        let removed = clean_untracked(dir.to_str().unwrap(), true).unwrap();
        assert!(removed >= 2, "the untracked files are gone: {removed}");
        assert!(!dir.join("loose.txt").exists());
        assert!(!dir.join("nested").exists(), "the empty directory is removed");
        assert!(dir.join("ignored.log").is_file(), "ignored files are kept");
        assert!(dir.join("a.txt").is_file(), "modified tracked files are kept");
        assert!(dir.join(".git").is_dir());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn clean_keeps_files_inside_a_nested_repository() {
        let dir = scratch("nested");
        let repo = init(&dir);
        commit(&repo, "base", &[("a.txt", "a\n")]);
        drop(repo);

        std::fs::create_dir_all(dir.join("vendor/other")).unwrap();
        Repository::init(dir.join("vendor/other")).unwrap();
        std::fs::write(dir.join("vendor/other/file.txt"), "keep\n").unwrap();

        clean_untracked(dir.to_str().unwrap(), true).unwrap();

        assert!(
            dir.join("vendor/other/file.txt").is_file(),
            "another repository is not deleted"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn clean_without_directories_keeps_untracked_folders() {
        let dir = scratch("clean-files-only");
        let repo = init(&dir);
        commit(&repo, "base", &[("a.txt", "a\n")]);
        drop(repo);

        std::fs::create_dir_all(dir.join("folder")).unwrap();
        std::fs::write(dir.join("folder/keep.txt"), "keep\n").unwrap();

        clean_untracked(dir.to_str().unwrap(), false).unwrap();
        assert!(dir.join("folder/keep.txt").is_file());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
