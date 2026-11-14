use git2::{Oid, Repository, RepositoryState};

use super::commit;
use super::signing::SigningConfig;

fn open(path: &str) -> Result<Repository, String> {
    Repository::open(path).map_err(|_| "Could not open the repository.".to_string())
}

fn head_commit(repo: &Repository) -> Result<git2::Commit<'_>, String> {
    repo.head()
        .and_then(|head| head.peel_to_commit())
        .map_err(|_| "No commits on the current branch.".to_string())
}

fn picked_commit<'a>(repo: &'a Repository, name: &str) -> Result<git2::Commit<'a>, String> {
    repo.find_reference(name)
        .and_then(|reference| reference.peel_to_commit())
        .map_err(|_| "Could not read the commit being applied.".to_string())
}

/// Commits the current index on top of HEAD. Used to finish a cherry-pick or
/// revert that stopped at a conflict.
fn commit_index(
    repo: &Repository,
    message: &str,
    extra_parent: Option<&git2::Commit<'_>>,
    signing: Option<&SigningConfig>,
) -> Result<String, String> {
    let mut index = repo
        .index()
        .map_err(|_| "Could not read the index.".to_string())?;
    if index.has_conflicts() {
        return Err("Resolve every conflict before continuing.".to_string());
    }
    let tree_id = index
        .write_tree_to(repo)
        .map_err(|_| "Could not read the changes.".to_string())?;
    let tree = repo
        .find_tree(tree_id)
        .map_err(|_| "Could not read the changes.".to_string())?;
    let head = head_commit(repo)?;
    let mut parents: Vec<&git2::Commit> = vec![&head];
    if let Some(parent) = extra_parent {
        parents.push(parent);
    }
    let oid = commit::commit_at_head(repo, message, &tree, &parents, signing)?;
    repo.cleanup_state()
        .map_err(|_| "Could not clear the operation state.".to_string())?;
    Ok(oid.to_string())
}

fn cherry_pick_state(repo: &Repository) -> bool {
    matches!(
        repo.state(),
        RepositoryState::CherryPick | RepositoryState::CherryPickSequence
    )
}

fn revert_state(repo: &Repository) -> bool {
    matches!(
        repo.state(),
        RepositoryState::Revert | RepositoryState::RevertSequence
    )
}

/// Applies `commit` on top of the current branch. Conflicts leave the state in
/// progress and `Ok(())` is returned so the UI can resolve them.
pub fn cherry_pick(
    path: &str,
    oid_str: &str,
    signing: Option<&SigningConfig>,
) -> Result<(), String> {
    let repo = open(path)?;
    let oid = Oid::from_str(oid_str).map_err(|_| "That commit id is not valid.".to_string())?;
    let commit = repo
        .find_commit(oid)
        .map_err(|_| "Commit not found.".to_string())?;

    if repo.state() != RepositoryState::Clean {
        return Err("Finish or abandon the current operation first.".to_string());
    }
    repo.cherrypick(&commit, None)
        .map_err(|_| "Could not apply that commit.".to_string())?;

    {
        let index = repo
            .index()
            .map_err(|_| "Could not read the index.".to_string())?;
        if index.has_conflicts() {
            return Ok(());
        }
    }

    let message = commit.message().unwrap_or("Cherry-pick").to_string();
    commit_index(&repo, &message, None, signing)?;
    Ok(())
}

/// Finishes a cherry-pick that stopped at a conflict.
pub fn cherry_pick_continue(path: &str, signing: Option<&SigningConfig>) -> Result<(), String> {
    let repo = open(path)?;
    if !cherry_pick_state(&repo) {
        return Err("There is no cherry-pick in progress.".to_string());
    }
    let picked = picked_commit(&repo, "CHERRY_PICK_HEAD")?;
    let message = picked
        .message()
        .unwrap_or("Cherry-pick")
        .to_string();
    commit_index(&repo, &message, None, signing)?;
    Ok(())
}

/// Abandons a conflicted cherry-pick and restores the branch.
pub fn cherry_pick_abort(path: &str) -> Result<(), String> {
    let repo = open(path)?;
    if !cherry_pick_state(&repo) {
        return Err("There is no cherry-pick in progress.".to_string());
    }
    let head = head_commit(&repo)?;
    repo.reset(head.as_object(), git2::ResetType::Hard, None)
        .map_err(|_| "Could not restore the branch.".to_string())?;
    repo.cleanup_state()
        .map_err(|_| "Could not clear the cherry-pick state.".to_string())?;
    Ok(())
}

/// Reverts `commit` on top of the current branch. Conflicts leave the state in
/// progress and `Ok(())` is returned.
pub fn revert_commit(
    path: &str,
    oid_str: &str,
    signing: Option<&SigningConfig>,
) -> Result<(), String> {
    let repo = open(path)?;
    let oid = Oid::from_str(oid_str).map_err(|_| "That commit id is not valid.".to_string())?;
    let commit = repo
        .find_commit(oid)
        .map_err(|_| "Commit not found.".to_string())?;

    if repo.state() != RepositoryState::Clean {
        return Err("Finish or abandon the current operation first.".to_string());
    }
    repo.revert(&commit, None)
        .map_err(|_| "Could not revert that commit.".to_string())?;

    {
        let index = repo
            .index()
            .map_err(|_| "Could not read the index.".to_string())?;
        if index.has_conflicts() {
            return Ok(());
        }
    }

    let summary = commit.summary().unwrap_or("commit").to_string();
    commit_index(&repo, &format!("Revert \"{summary}\""), None, signing)?;
    Ok(())
}

/// Finishes a revert that stopped at a conflict.
pub fn revert_continue(path: &str, signing: Option<&SigningConfig>) -> Result<(), String> {
    let repo = open(path)?;
    if !revert_state(&repo) {
        return Err("There is no revert in progress.".to_string());
    }
    let reverted = picked_commit(&repo, "REVERT_HEAD")?;
    let summary = reverted.summary().unwrap_or("commit").to_string();
    commit_index(&repo, &format!("Revert \"{summary}\""), None, signing)?;
    Ok(())
}

/// Abandons a conflicted revert and restores the branch.
pub fn revert_abort(path: &str) -> Result<(), String> {
    let repo = open(path)?;
    if !revert_state(&repo) {
        return Err("There is no revert in progress.".to_string());
    }
    let head = head_commit(&repo)?;
    repo.reset(head.as_object(), git2::ResetType::Hard, None)
        .map_err(|_| "Could not restore the branch.".to_string())?;
    repo.cleanup_state()
        .map_err(|_| "Could not clear the revert state.".to_string())?;
    Ok(())
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
            "knitt-advanced-{name}-{}-{}",
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
            std::fs::write(repo.workdir().unwrap().join(name), content).unwrap();
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

    fn switch(repo: &Repository, refname: &str) {
        let object = repo.revparse_single(refname).unwrap();
        let mut checkout = git2::build::CheckoutBuilder::new();
        checkout.force();
        repo.checkout_tree(&object, Some(&mut checkout)).unwrap();
        repo.set_head(refname).unwrap();
    }

    #[test]
    fn cherry_picks_a_commit_from_another_branch() {
        let dir = scratch("pick");
        let repo = init(&dir);
        let base = commit(&repo, "base", &[("a.txt", "a\n")]);
        let original = repo.head().unwrap().shorthand().unwrap().to_string();
        {
            let head = repo.head().unwrap().peel_to_commit().unwrap();
            repo.branch("feature", &head, false).unwrap();
        }
        switch(&repo, "refs/heads/feature");
        let picked = commit(&repo, "feature work", &[("b.txt", "b\n")]);
        switch(&repo, &format!("refs/heads/{original}"));
        drop(repo);

        cherry_pick(dir.to_str().unwrap(), &picked.to_string(), None).unwrap();

        let repo = Repository::open(&dir).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(head.message().unwrap_or_default().trim(), "feature work");
        // The result carries the picked commit's tree on top of this branch.
        assert_eq!(
            head.tree().unwrap().id(),
            Repository::open(&dir)
                .unwrap()
                .find_commit(picked)
                .unwrap()
                .tree()
                .unwrap()
                .id()
        );
        assert_eq!(head.parent_count(), 1);
        assert_eq!(head.parent(0).unwrap().id(), base);
        assert!(head.id() != base);
        assert_eq!(
            std::fs::read_to_string(dir.join("b.txt")).unwrap(),
            "b\n"
        );
        assert_eq!(repo.state(), RepositoryState::Clean);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn conflicted_cherry_pick_can_be_continued_or_aborted() {
        let dir = scratch("pick-conflict");
        let repo = init(&dir);
        commit(&repo, "base", &[("shared.txt", "base\n")]);
        let original = repo.head().unwrap().shorthand().unwrap().to_string();
        {
            let head = repo.head().unwrap().peel_to_commit().unwrap();
            repo.branch("feature", &head, false).unwrap();
        }
        switch(&repo, "refs/heads/feature");
        let picked = commit(&repo, "feature side", &[("shared.txt", "feature\n")]);
        switch(&repo, &format!("refs/heads/{original}"));
        commit(&repo, "main side", &[("shared.txt", "main\n")]);
        let main_tip = repo.head().unwrap().target().unwrap();
        drop(repo);

        cherry_pick(dir.to_str().unwrap(), &picked.to_string(), None).unwrap();
        assert_eq!(
            crate::git::conflicts::list_conflicts(dir.to_str().unwrap()).unwrap(),
            vec!["shared.txt".to_string()]
        );
        assert_eq!(
            crate::git::merge::operation_state(dir.to_str().unwrap()).unwrap(),
            "cherry-pick"
        );

        cherry_pick_abort(dir.to_str().unwrap()).unwrap();
        {
            let repo = Repository::open(&dir).unwrap();
            assert_eq!(repo.head().unwrap().target().unwrap(), main_tip);
            assert_eq!(
                std::fs::read_to_string(dir.join("shared.txt")).unwrap(),
                "main\n"
            );
        }

        cherry_pick(dir.to_str().unwrap(), &picked.to_string(), None).unwrap();
        crate::git::conflicts::resolve_conflict(
            dir.to_str().unwrap(),
            "shared.txt",
            "both\n",
        )
        .unwrap();
        cherry_pick_continue(dir.to_str().unwrap(), None).unwrap();

        let repo = Repository::open(&dir).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(head.message().unwrap_or_default().trim(), "feature side");
        assert_eq!(head.parent(0).unwrap().id(), main_tip);
        assert_eq!(repo.state(), RepositoryState::Clean);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reverts_a_commit() {
        let dir = scratch("revert");
        let repo = init(&dir);
        commit(&repo, "base", &[("a.txt", "one\n")]);
        let target = commit(&repo, "adds a line", &[("a.txt", "one\ntwo\n")]);
        commit(&repo, "later", &[("b.txt", "b\n")]);
        drop(repo);

        revert_commit(dir.to_str().unwrap(), &target.to_string(), None).unwrap();

        let repo = Repository::open(&dir).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(head.message().unwrap_or_default().trim(), "Revert \"adds a line\"");
        assert_eq!(
            std::fs::read_to_string(dir.join("a.txt")).unwrap(),
            "one\n"
        );
        assert!(dir.join("b.txt").is_file(), "other commits are untouched");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn conflicted_revert_can_be_aborted() {
        let dir = scratch("revert-conflict");
        let repo = init(&dir);
        commit(&repo, "base", &[("a.txt", "one\n")]);
        let target = commit(&repo, "second", &[("a.txt", "two\n")]);
        let before = commit(&repo, "third", &[("a.txt", "three\n")]);
        drop(repo);

        revert_commit(dir.to_str().unwrap(), &target.to_string(), None).unwrap();
        assert_eq!(
            crate::git::merge::operation_state(dir.to_str().unwrap()).unwrap(),
            "revert"
        );

        revert_abort(dir.to_str().unwrap()).unwrap();

        let repo = Repository::open(&dir).unwrap();
        assert_eq!(repo.head().unwrap().target().unwrap(), before);
        assert_eq!(
            std::fs::read_to_string(dir.join("a.txt")).unwrap(),
            "three\n"
        );
        assert_eq!(repo.state(), RepositoryState::Clean);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuses_bad_ids_and_an_operation_already_in_progress() {
        let dir = scratch("guards");
        let repo = init(&dir);
        commit(&repo, "base", &[("a.txt", "a\n")]);
        drop(repo);

        assert_eq!(
            cherry_pick(dir.to_str().unwrap(), "not-an-id", None).unwrap_err(),
            "That commit id is not valid."
        );
        assert_eq!(
            cherry_pick_continue(dir.to_str().unwrap(), None).unwrap_err(),
            "There is no cherry-pick in progress."
        );
        assert_eq!(
            revert_abort(dir.to_str().unwrap()).unwrap_err(),
            "There is no revert in progress."
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn ssh_keygen_available() -> bool {
        std::process::Command::new("ssh-keygen")
            .arg("-?")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|status| status.success() || status.code().is_some())
            .unwrap_or(false)
    }

    #[test]
    fn a_signed_cherry_pick_carries_a_signature() {
        if !ssh_keygen_available() {
            return;
        }
        let dir = scratch("signed-pick");
        let key = dir.join("signing_key");
        let generated = std::process::Command::new("ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-f"])
            .arg(&key)
            .status()
            .unwrap();
        assert!(generated.success());

        let repo = init(&dir);
        commit(&repo, "base", &[("a.txt", "a\n")]);
        let original = repo.head().unwrap().shorthand().unwrap().to_string();
        {
            let head = repo.head().unwrap().peel_to_commit().unwrap();
            repo.branch("feature", &head, false).unwrap();
        }
        switch(&repo, "refs/heads/feature");
        let picked = commit(&repo, "feature work", &[("b.txt", "b\n")]);
        switch(&repo, &format!("refs/heads/{original}"));
        drop(repo);

        let config = SigningConfig {
            mode: crate::git::signing::SigningMode::Ssh,
            key: key.display().to_string(),
        };
        cherry_pick(dir.to_str().unwrap(), &picked.to_string(), Some(&config)).unwrap();

        let repo = Repository::open(&dir).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(head.message().unwrap_or_default().trim(), "feature work");
        let (signature, _) = repo.extract_signature(&head.id(), None).unwrap();
        assert!(
            std::str::from_utf8(&signature)
                .unwrap_or_default()
                .starts_with("-----BEGIN SSH SIGNATURE-----"),
            "the cherry-picked commit is signed"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
