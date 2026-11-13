use git2::Repository;
use std::path::Path;

use super::repo::{open_repo, RepoInfo};

/// Filesystem names that would make an unusable repository even though git2
/// would happily accept them.
fn reject_reason(path: &Path) -> Option<&'static str> {
    if path.exists() && !path.is_dir() {
        return Some("That path is a file, not a folder.");
    }
    if path.is_dir() && Repository::open(path).is_ok() {
        return Some("That folder is already a Git repository.");
    }
    None
}

/// Creates a repository at `path` with `branch` checked out, ready for the
/// first commit. Only the settings-level default branch or an explicit name is
/// used; nothing global is written.
pub fn init_repo(path: &str, branch: &str) -> Result<RepoInfo, String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("Choose a folder for the new repository.".to_string());
    }
    let branch = branch.trim();
    if !crate::config::settings::is_valid_branch_name(branch) {
        return Err("Enter a valid initial branch name.".to_string());
    }

    let target = Path::new(path);
    if let Some(reason) = reject_reason(target) {
        return Err(reason.to_string());
    }

    std::fs::create_dir_all(target).map_err(|_| "Could not create that folder.".to_string())?;
    let repo = Repository::init(target)
        .map_err(|_| "Could not create a repository in that folder.".to_string())?;
    // The branch does not exist yet; set HEAD so the first commit lands on it
    // instead of whatever the user's init.defaultBranch happens to be.
    repo.set_head(&format!("refs/heads/{branch}"))
        .map_err(|_| "Could not set the initial branch.".to_string())?;

    open_repo(path)
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
            "knitt-init-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn creates_a_repository_on_the_requested_branch() {
        let dir = scratch();
        let info = init_repo(dir.to_str().unwrap(), "trunk").unwrap();
        assert_eq!(info.branch.as_deref(), Some("trunk"));
        assert_eq!(info.name, dir.file_name().unwrap().to_str().unwrap());

        let repo = Repository::open(&dir).unwrap();
        let head = repo.find_reference("HEAD").unwrap();
        assert_eq!(head.symbolic_target(), Some("refs/heads/trunk"));
        // No commit yet, so HEAD cannot resolve to an object.
        assert!(repo.head().is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn creates_missing_parent_folders() {
        let dir = scratch();
        let nested = dir.join("work").join("project");
        let info = init_repo(nested.to_str().unwrap(), "main").unwrap();
        assert_eq!(info.branch.as_deref(), Some("main"));
        assert!(nested.join(".git").is_dir());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuses_an_existing_repository() {
        let dir = scratch();
        std::fs::create_dir_all(&dir).unwrap();
        Repository::init(&dir).unwrap();
        assert_eq!(
            init_repo(dir.to_str().unwrap(), "main").unwrap_err(),
            "That folder is already a Git repository."
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuses_invalid_branch_names() {
        let dir = scratch();
        for name in ["", "  ", "bad name", "-dash", "a..b"] {
            assert_eq!(
                init_repo(dir.to_str().unwrap(), name).unwrap_err(),
                "Enter a valid initial branch name.",
                "branch {name:?} should be refused"
            );
        }
        assert!(!dir.exists(), "invalid input must not create the folder");
    }

    #[test]
    fn refuses_a_file_path() {
        let dir = scratch();
        std::fs::write(&dir, "not a folder").unwrap();
        assert_eq!(
            init_repo(dir.to_str().unwrap(), "main").unwrap_err(),
            "That path is a file, not a folder."
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
