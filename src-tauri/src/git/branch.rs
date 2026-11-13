use git2::{BranchType, Repository};
use serde::Serialize;

use super::proxy::ProxyConfig;

/// A local branch and the remote branch it tracks, if any.
#[derive(Debug, Serialize)]
pub struct BranchOverview {
    pub name: String,
    pub upstream: Option<String>,
}

pub fn list_branches(path: &str) -> Result<Vec<String>, String> {
    let repo = Repository::open(path).map_err(|_| "Could not open the repository.".to_string())?;
    let mut branches = Vec::new();
    let branches_iter = repo
        .branches(Some(BranchType::Local))
        .map_err(|_| "Could not read the branches.".to_string())?;
    for branch in branches_iter {
        let (branch, _) = branch.map_err(|_| "Could not read the branches.".to_string())?;
        if let Some(name) = branch.name().ok().flatten() {
            branches.push(name.to_string());
        }
    }
    Ok(branches)
}

/// Branch names with their upstream, so the UI can offer Publish only when a
/// branch has never been pushed.
pub fn list_branch_overview(path: &str) -> Result<Vec<BranchOverview>, String> {
    let repo = Repository::open(path).map_err(|_| "Could not open the repository.".to_string())?;
    let mut branches = Vec::new();
    let branches_iter = repo
        .branches(Some(BranchType::Local))
        .map_err(|_| "Could not read the branches.".to_string())?;
    for branch in branches_iter {
        let (branch, _) = branch.map_err(|_| "Could not read the branches.".to_string())?;
        let Some(name) = branch.name().ok().flatten() else {
            continue;
        };
        let upstream = branch
            .upstream()
            .ok()
            .and_then(|upstream| upstream.name().ok().flatten().map(str::to_string));
        branches.push(BranchOverview {
            name: name.to_string(),
            upstream,
        });
    }
    Ok(branches)
}

pub fn create_branch(path: &str, name: &str) -> Result<(), String> {
    let name = name.trim();
    if !crate::config::settings::is_valid_branch_name(name) {
        return Err("Enter a valid branch name.".to_string());
    }
    let repo = Repository::open(path).map_err(|_| "Could not open the repository.".to_string())?;
    if repo.find_branch(name, BranchType::Local).is_ok() {
        return Err("A branch with that name already exists.".to_string());
    }
    let head = repo
        .head()
        .map_err(|_| "The repository has no commits yet.".to_string())?;
    let commit = head
        .peel_to_commit()
        .map_err(|_| "The repository has no commits yet.".to_string())?;
    repo.branch(name, &commit, false)
        .map_err(|_| "Could not create that branch.".to_string())?;
    Ok(())
}

pub fn checkout_branch(path: &str, name: &str) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|_| "Could not open the repository.".to_string())?;
    let refname = format!("refs/heads/{name}");
    let obj = repo
        .revparse_single(&refname)
        .map_err(|_| "Branch not found.".to_string())?;
    let mut options = git2::build::CheckoutBuilder::new();
    options.safe();
    repo.checkout_tree(&obj, Some(&mut options)).map_err(|e| {
        if e.code() == git2::ErrorCode::Conflict {
            "Commit or stash your changes before switching branches.".to_string()
        } else {
            "Could not switch branches.".to_string()
        }
    })?;
    repo.set_head(&refname)
        .map_err(|_| "Could not switch branches.".to_string())?;
    Ok(())
}

pub fn rename_branch(path: &str, old_name: &str, new_name: &str) -> Result<(), String> {
    let new_name = new_name.trim();
    if !crate::config::settings::is_valid_branch_name(new_name) {
        return Err("Enter a valid branch name.".to_string());
    }
    let repo = Repository::open(path).map_err(|_| "Could not open the repository.".to_string())?;
    if old_name == new_name {
        return Ok(());
    }
    if repo.find_branch(new_name, BranchType::Local).is_ok() {
        return Err("A branch with that name already exists.".to_string());
    }
    let mut branch = repo
        .find_branch(old_name, BranchType::Local)
        .map_err(|_| "Branch not found.".to_string())?;
    branch
        .rename(new_name, false)
        .map_err(|_| "Could not rename that branch.".to_string())?;
    Ok(())
}

pub fn delete_branch(path: &str, name: &str) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|_| "Could not open the repository.".to_string())?;
    let current = repo
        .head()
        .ok()
        .and_then(|head| head.shorthand().map(str::to_string));
    if current.as_deref() == Some(name) {
        return Err("Check out another branch before deleting this one.".to_string());
    }
    let mut branch = repo
        .find_branch(name, BranchType::Local)
        .map_err(|_| "Branch not found.".to_string())?;

    // Deleting an unmerged branch loses commits, so refuse it unless the
    // branch is reachable from the current branch.
    let target = branch
        .get()
        .peel_to_commit()
        .map_err(|_| "Branch not found.".to_string())?;
    let head = repo
        .head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok());
    let merged = head.is_some_and(|head| {
        head.id() == target.id()
            || repo
                .graph_descendant_of(head.id(), target.id())
                .unwrap_or(false)
    });
    if !merged {
        return Err("That branch has commits that are not merged.".to_string());
    }

    branch
        .delete()
        .map_err(|_| "Could not delete that branch.".to_string())?;
    Ok(())
}

pub fn set_upstream(path: &str, branch_name: &str, upstream: &str) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|_| "Could not open the repository.".to_string())?;
    let mut branch = repo
        .find_branch(branch_name, BranchType::Local)
        .map_err(|_| "Branch not found.".to_string())?;
    branch
        .set_upstream(Some(upstream))
        .map_err(|_| "Could not set the upstream branch.".to_string())?;
    Ok(())
}

/// Push a local branch to `origin` and set it as its upstream.
pub fn publish_branch(
    path: &str,
    branch_name: &str,
    proxy: &ProxyConfig,
) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|_| "Could not open the repository.".to_string())?;
    if repo.find_branch(branch_name, BranchType::Local).is_err() {
        return Err("Branch not found.".to_string());
    }
    let mut remote = repo
        .find_remote("origin")
        .map_err(|_| "Remote 'origin' not found.".to_string())?;

    let mut push_options = git2::PushOptions::new();
    push_options.remote_callbacks(crate::git::credentials::make_callbacks());
    push_options.proxy_options(proxy.options()?);

    let refspec = format!("refs/heads/{branch_name}:refs/heads/{branch_name}");
    remote
        .push(&[refspec.as_str()], Some(&mut push_options))
        .map_err(|e| match e.code() {
            git2::ErrorCode::Auth => {
                "Authentication failed. Sign in to GitHub or check your SSH key.".to_string()
            }
            _ => "Could not publish that branch.".to_string(),
        })?;

    set_upstream(path, branch_name, &format!("origin/{branch_name}"))
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
            "knitt-branch-{name}-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn repo_with_commit(dir: &Path) -> Repository {
        let repo = Repository::init(dir).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
        drop(config);
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

    fn commit_on(repo: &Repository, message: &str, file: &str) -> git2::Oid {
        std::fs::write(repo.workdir().unwrap().join(file), message).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new(file)).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();
        let parent = repo.head().unwrap().peel_to_commit().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent])
            .unwrap()
    }

    #[test]
    fn overview_reports_missing_upstream() {
        let dir = scratch("overview");
        let repo = repo_with_commit(&dir);
        repo.branch("feature", &repo.head().unwrap().peel_to_commit().unwrap(), false)
            .unwrap();
        drop(repo);

        let overview = list_branch_overview(dir.to_str().unwrap()).unwrap();
        let names: Vec<_> = overview.iter().map(|b| b.name.as_str()).collect();
        assert!(names.contains(&"feature"));
        assert!(overview.iter().all(|branch| branch.upstream.is_none()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn renames_a_branch_and_refuses_duplicates() {
        let dir = scratch("rename");
        let repo = repo_with_commit(&dir);
        repo.branch("old", &repo.head().unwrap().peel_to_commit().unwrap(), false)
            .unwrap();
        drop(repo);

        rename_branch(dir.to_str().unwrap(), "old", "new").unwrap();
        let repo = Repository::open(&dir).unwrap();
        assert!(repo.find_branch("new", BranchType::Local).is_ok());
        assert!(repo.find_branch("old", BranchType::Local).is_err());
        {
            let head = repo.head().unwrap().peel_to_commit().unwrap();
            repo.branch("taken", &head, false).unwrap();
        }
        drop(repo);

        assert_eq!(
            rename_branch(dir.to_str().unwrap(), "new", "taken").unwrap_err(),
            "A branch with that name already exists."
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuses_invalid_names() {
        let dir = scratch("invalid");
        let repo = repo_with_commit(&dir);
        drop(repo);

        assert_eq!(
            create_branch(dir.to_str().unwrap(), "bad name").unwrap_err(),
            "Enter a valid branch name."
        );
        assert_eq!(
            rename_branch(dir.to_str().unwrap(), "main", "bad name").unwrap_err(),
            "Enter a valid branch name."
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn deletes_a_merged_branch_but_not_the_current_one() {
        let dir = scratch("delete");
        let repo = repo_with_commit(&dir);
        {
            let head = repo.head().unwrap().peel_to_commit().unwrap();
            repo.branch("feature", &head, false).unwrap();
        }
        drop(repo);

        let current = Repository::open(&dir)
            .unwrap()
            .head()
            .unwrap()
            .shorthand()
            .unwrap()
            .to_string();
        assert!(delete_branch(dir.to_str().unwrap(), &current).is_err());

        delete_branch(dir.to_str().unwrap(), "feature").unwrap();
        let repo = Repository::open(&dir).unwrap();
        assert!(repo.find_branch("feature", BranchType::Local).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuses_to_delete_an_unmerged_branch() {
        let dir = scratch("unmerged");
        let repo = repo_with_commit(&dir);
        let original = repo.head().unwrap().shorthand().unwrap().to_string();
        repo.branch("feature", &repo.head().unwrap().peel_to_commit().unwrap(), false)
            .unwrap();
        repo.set_head("refs/heads/feature").unwrap();
        commit_on(&repo, "feature work", "b.txt");
        // Back to the original branch, which does not contain that commit.
        repo.set_head(&format!("refs/heads/{original}")).unwrap();
        drop(repo);

        assert_eq!(
            delete_branch(dir.to_str().unwrap(), "feature").unwrap_err(),
            "That branch has commits that are not merged."
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
