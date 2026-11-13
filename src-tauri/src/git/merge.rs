use git2::{BranchType, Repository};

fn open(path: &str) -> Result<Repository, String> {
    Repository::open(path).map_err(|_| "Could not open the repository.".to_string())
}

fn head_commit(repo: &Repository) -> Result<git2::Commit<'_>, String> {
    repo.head()
        .and_then(|head| head.peel_to_commit())
        .map_err(|_| "No commits on the current branch.".to_string())
}

fn signature(repo: &Repository) -> Result<git2::Signature<'_>, String> {
    repo.signature()
        .map_err(|_| "Set your name and email in Settings → Git first.".to_string())
}

/// Merge `branch_name` into the current branch.
///
/// Up-to-date is a no-op and fast-forwards move the branch pointer. Divergent
/// history creates a merge commit. When the merge conflicts, the conflict
/// state is left in place and `Ok(())` is returned; callers read
/// `list_conflicts` or the repository state to offer resolution.
pub fn merge_branch(path: &str, branch_name: &str) -> Result<(), String> {
    let repo = open(path)?;
    let branch_ref = repo
        .find_reference(&format!("refs/heads/{branch_name}"))
        .map_err(|_| "Branch not found.".to_string())?;
    if head_commit(&repo)?.id() == branch_ref.peel_to_commit().map_err(|_| "Branch not found.".to_string())?.id() {
        return Ok(());
    }
    let annotated = repo
        .reference_to_annotated_commit(&branch_ref)
        .map_err(|_| "Could not read that branch.".to_string())?;
    let (analysis, _) = repo
        .merge_analysis(&[&annotated])
        .map_err(|_| "Could not analyse the merge.".to_string())?;

    if analysis.is_up_to_date() {
        return Ok(());
    }

    if analysis.is_fast_forward() {
        let head = repo.head().map_err(|_| "No commits on the current branch.".to_string())?;
        let refname = head
            .name()
            .ok_or_else(|| "Cannot read HEAD.".to_string())?
            .to_string();
        let mut reference = repo
            .find_reference(&refname)
            .map_err(|_| "Cannot read the current branch.".to_string())?;
        reference
            .set_target(annotated.id(), "merge: fast-forward")
            .map_err(|_| "Could not fast-forward the branch.".to_string())?;
        repo.set_head(&refname)
            .map_err(|_| "Could not update HEAD.".to_string())?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
            .map_err(|_| "Could not update the working tree.".to_string())?;
        return Ok(());
    }

    let head_commit = head_commit(&repo)?;
    let merge_commit = repo
        .find_commit(annotated.id())
        .map_err(|_| "Could not read that branch.".to_string())?;

    let mut checkout = git2::build::CheckoutBuilder::new();
    checkout.safe();
    repo.merge(&[&annotated], None, Some(&mut checkout))
        .map_err(|_| "Could not merge that branch.".to_string())?;

    let mut index = repo.index().map_err(|_| "Could not read the index.".to_string())?;
    if index.has_conflicts() {
        // Leave MERGE_HEAD and the conflict markers for the resolver.
        return Ok(());
    }

    let sig = signature(&repo)?;
    let tree_id = index
        .write_tree_to(&repo)
        .map_err(|_| "Could not read the merged changes.".to_string())?;
    let tree = repo
        .find_tree(tree_id)
        .map_err(|_| "Could not read the merged changes.".to_string())?;
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        &format!("Merge branch '{branch_name}'"),
        &tree,
        &[&head_commit, &merge_commit],
    )
    .map_err(|_| "Could not create the merge commit.".to_string())?;
    repo.cleanup_state()
        .map_err(|_| "Could not finish the merge.".to_string())?;
    Ok(())
}

/// Completes a conflicted merge once every conflict is resolved and staged.
pub fn merge_continue(path: &str, message: Option<&str>) -> Result<(), String> {
    let repo = open(path)?;
    if repo.state() != git2::RepositoryState::Merge {
        return Err("There is no merge in progress.".to_string());
    }
    let mut index = repo.index().map_err(|_| "Could not read the index.".to_string())?;
    if index.has_conflicts() {
        return Err("Resolve every conflict before continuing.".to_string());
    }

    let head = head_commit(&repo)?;
    let merge_head = repo
        .find_reference("MERGE_HEAD")
        .and_then(|reference| reference.peel_to_commit())
        .map_err(|_| "Could not read the branch being merged.".to_string())?;

    let sig = signature(&repo)?;
    let tree_id = index
        .write_tree_to(&repo)
        .map_err(|_| "Could not read the resolved changes.".to_string())?;
    let tree = repo
        .find_tree(tree_id)
        .map_err(|_| "Could not read the resolved changes.".to_string())?;
    let message = message
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .unwrap_or("Merge commit");

    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        message,
        &tree,
        &[&head, &merge_head],
    )
    .map_err(|_| "Could not create the merge commit.".to_string())?;
    repo.cleanup_state()
        .map_err(|_| "Could not finish the merge.".to_string())?;
    Ok(())
}

/// Throws away an in-progress merge and restores the pre-merge state.
pub fn merge_abort(path: &str) -> Result<(), String> {
    let repo = open(path)?;
    if repo.state() != git2::RepositoryState::Merge {
        return Err("There is no merge in progress.".to_string());
    }
    let head = head_commit(&repo)?;
    repo.reset(
        head.as_object(),
        git2::ResetType::Hard,
        None,
    )
    .map_err(|_| "Could not restore the branch.".to_string())?;
    repo.cleanup_state()
        .map_err(|_| "Could not clear the merge state.".to_string())?;
    Ok(())
}

fn rebase_state(repo: &Repository) -> bool {
    matches!(
        repo.state(),
        git2::RepositoryState::Rebase
            | git2::RepositoryState::RebaseInteractive
            | git2::RepositoryState::RebaseMerge
    )
}

/// Rebase the current branch onto `branch_name`.
///
/// On a conflict the rebase is left in progress and `Ok(())` is returned.
pub fn rebase_branch(path: &str, branch_name: &str) -> Result<(), String> {
    let repo = open(path)?;
    let branch = repo
        .find_branch(branch_name, BranchType::Local)
        .map_err(|_| "Branch not found.".to_string())?;
    let upstream = repo
        .reference_to_annotated_commit(branch.get())
        .map_err(|_| "Could not read that branch.".to_string())?;

    let sig = signature(&repo)?;
    let mut rebase = repo
        .rebase(None, Some(&upstream), None, None)
        .map_err(|_| "Could not start the rebase.".to_string())?;

    while let Some(op) = rebase.next() {
        op.map_err(|_| "Could not apply a commit during the rebase.".to_string())?;
        let index = repo.index().map_err(|_| "Could not read the index.".to_string())?;
        if index.has_conflicts() {
            return Ok(());
        }
        match rebase.commit(None, &sig, None) {
            Ok(_) => {}
            // An empty commit can happen when a change was already applied.
            Err(error)
                if matches!(
                    error.code(),
                    git2::ErrorCode::Applied | git2::ErrorCode::Exists
                ) => {}
            Err(_) => return Err("Could not apply a commit during the rebase.".to_string()),
        }
    }

    rebase
        .finish(Some(&sig))
        .map_err(|_| "Could not finish the rebase.".to_string())?;
    Ok(())
}

/// Applies the remaining commits after conflicts were resolved, and repeats
/// until the rebase finishes or stops at the next conflict.
pub fn rebase_continue(path: &str) -> Result<(), String> {
    let repo = open(path)?;
    if !rebase_state(&repo) {
        return Err("There is no rebase in progress.".to_string());
    }
    {
        let index = repo.index().map_err(|_| "Could not read the index.".to_string())?;
        if index.has_conflicts() {
            return Err("Resolve every conflict before continuing.".to_string());
        }
    }

    let sig = signature(&repo)?;
    let mut rebase = repo
        .open_rebase(None)
        .map_err(|_| "Could not open the rebase.".to_string())?;

    // The operation that conflicted was already applied to the index before
    // the rebase stopped, so the resolution is committed first. An operation
    // with nothing staged is skipped, the same way git treats an empty step.
    match rebase.commit(None, &sig, None) {
        Ok(_) => {}
        Err(error)
            if matches!(
                error.code(),
                git2::ErrorCode::Applied | git2::ErrorCode::Exists
            ) => {}
        Err(_) => return Err("Could not apply a commit during the rebase.".to_string()),
    }

    while let Some(op) = rebase.next() {
        op.map_err(|_| "Could not apply a commit during the rebase.".to_string())?;
        let index = repo.index().map_err(|_| "Could not read the index.".to_string())?;
        if index.has_conflicts() {
            return Ok(());
        }
        match rebase.commit(None, &sig, None) {
            Ok(_) => {}
            Err(error)
                if matches!(
                    error.code(),
                    git2::ErrorCode::Applied | git2::ErrorCode::Exists
                ) => {}
            Err(_) => return Err("Could not apply a commit during the rebase.".to_string()),
        }
    }

    rebase
        .finish(Some(&sig))
        .map_err(|_| "Could not finish the rebase.".to_string())?;
    Ok(())
}

/// Abandons an in-progress rebase and returns to the original branch.
pub fn rebase_abort(path: &str) -> Result<(), String> {
    let repo = open(path)?;
    if !rebase_state(&repo) {
        return Err("There is no rebase in progress.".to_string());
    }
    let mut rebase = repo
        .open_rebase(None)
        .map_err(|_| "Could not open the rebase.".to_string())?;
    rebase
        .abort()
        .map_err(|_| "Could not abort the rebase.".to_string())?;
    Ok(())
}

/// Name of the operation currently in progress, for the UI.
pub fn operation_state(path: &str) -> Result<&'static str, String> {
    let repo = open(path)?;
    Ok(match repo.state() {
        git2::RepositoryState::Clean => "none",
        git2::RepositoryState::Merge => "merge",
        git2::RepositoryState::Rebase
        | git2::RepositoryState::RebaseInteractive
        | git2::RepositoryState::RebaseMerge => "rebase",
        git2::RepositoryState::CherryPick
        | git2::RepositoryState::CherryPickSequence => "cherry-pick",
        git2::RepositoryState::Revert | git2::RepositoryState::RevertSequence => "revert",
        _ => "other",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::conflicts;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn scratch(name: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "knitt-merge-{name}-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn init(dir: &Path) -> Repository {
        let repo = Repository::init(dir).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
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
    fn fast_forwards_a_branch() {
        let dir = scratch("ff");
        let repo = init(&dir);
        commit(&repo, "base", &[("a.txt", "a\n")]);
        let original = repo.head().unwrap().name().unwrap().to_string();
        let branch = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch("feature", &branch, false).unwrap();
        drop(branch);
        switch(&repo, "refs/heads/feature");
        let feature = commit(&repo, "feature", &[("b.txt", "b\n")]);
        switch(&repo, &original);
        drop(repo);

        merge_branch(dir.to_str().unwrap(), "feature").unwrap();
        let repo = Repository::open(&dir).unwrap();
        assert_eq!(repo.head().unwrap().target().unwrap(), feature);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn conflicted_merge_can_be_resolved_and_continued() {
        let dir = scratch("conflict");
        let repo = init(&dir);
        commit(&repo, "base", &[("shared.txt", "base\n")]);
        let original = repo.head().unwrap().name().unwrap().to_string();
        {
            let head = repo.head().unwrap().peel_to_commit().unwrap();
            repo.branch("feature", &head, false).unwrap();
        }

        switch(&repo, "refs/heads/feature");
        commit(&repo, "feature", &[("shared.txt", "feature\n")]);
        switch(&repo, &original);
        commit(&repo, "main", &[("shared.txt", "main\n")]);
        drop(repo);

        merge_branch(dir.to_str().unwrap(), "feature").unwrap();
        let files = conflicts::list_conflicts(dir.to_str().unwrap()).unwrap();
        assert_eq!(files, vec!["shared.txt".to_string()]);
        assert_eq!(
            operation_state(dir.to_str().unwrap()).unwrap(),
            "merge"
        );

        conflicts::resolve_conflict(dir.to_str().unwrap(), "shared.txt", "resolved\n").unwrap();
        merge_continue(dir.to_str().unwrap(), Some("Merge feature")).unwrap();

        let repo = Repository::open(&dir).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(head.message().unwrap_or_default().trim(), "Merge feature");
        assert_eq!(head.parent_count(), 2);
        assert_eq!(
            std::fs::read_to_string(dir.join("shared.txt")).unwrap(),
            "resolved\n"
        );
        assert_eq!(operation_state(dir.to_str().unwrap()).unwrap(), "none");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn merge_can_be_aborted() {
        let dir = scratch("abort");
        let repo = init(&dir);
        commit(&repo, "base", &[("shared.txt", "base\n")]);
        let original = repo.head().unwrap().name().unwrap().to_string();
        {
            let head = repo.head().unwrap().peel_to_commit().unwrap();
            repo.branch("feature", &head, false).unwrap();
        }
        switch(&repo, "refs/heads/feature");
        commit(&repo, "feature", &[("shared.txt", "feature\n")]);
        switch(&repo, &original);
        let main = commit(&repo, "main", &[("shared.txt", "main\n")]);
        drop(repo);

        merge_branch(dir.to_str().unwrap(), "feature").unwrap();
        assert_eq!(
            operation_state(dir.to_str().unwrap()).unwrap(),
            "merge",
            "a conflicting merge stays in progress"
        );
        merge_abort(dir.to_str().unwrap()).unwrap();

        assert_eq!(
            operation_state(dir.to_str().unwrap()).unwrap(),
            "none"
        );
        let repo = Repository::open(&dir).unwrap();
        assert_eq!(
            repo.head().unwrap().target().unwrap(),
            main,
            "abort leaves the branch where it was"
        );
        assert_eq!(
            std::fs::read_to_string(dir.join("shared.txt")).unwrap(),
            "main\n"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn continue_requires_a_merge_and_resolved_files() {
        let dir = scratch("continue");
        let repo = init(&dir);
        commit(&repo, "base", &[("a.txt", "a\n")]);
        drop(repo);

        assert_eq!(
            merge_continue(dir.to_str().unwrap(), None).unwrap_err(),
            "There is no merge in progress."
        );
        assert_eq!(
            merge_abort(dir.to_str().unwrap()).unwrap_err(),
            "There is no merge in progress."
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rebases_onto_another_branch() {
        let dir = scratch("rebase");
        let repo = init(&dir);
        commit(&repo, "base", &[("a.txt", "a\n")]);
        let original = repo.head().unwrap().shorthand().unwrap().to_string();
        {
            let head = repo.head().unwrap().peel_to_commit().unwrap();
            repo.branch("topic", &head, false).unwrap();
        }

        // main moves on, topic gets its own commit.
        commit(&repo, "main work", &[("main.txt", "m\n")]);
        let main_tip = repo.head().unwrap().target().unwrap();
        switch(&repo, "refs/heads/topic");
        commit(&repo, "topic work", &[("topic.txt", "t\n")]);
        // Stay on topic: rebase_branch replays the current branch onto the
        // branch it is given.
        drop(repo);

        rebase_branch(dir.to_str().unwrap(), &original).unwrap();
        let repo = Repository::open(&dir).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(head.message().unwrap_or_default().trim(), "topic work");
        assert_eq!(head.parent_count(), 1);
        assert_eq!(
            head.parent(0).unwrap().id(),
            main_tip,
            "topic sits on top of main"
        );
        assert_eq!(
            repo.head().unwrap().shorthand(),
            Some("topic"),
            "the rebase stays on the current branch"
        );
        assert_eq!(operation_state(dir.to_str().unwrap()).unwrap(), "none");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rebase_conflict_stops_and_can_be_continued() {
        let dir = scratch("rebase-conflict");
        let repo = init(&dir);
        commit(&repo, "base", &[("shared.txt", "base\n")]);
        let original = repo.head().unwrap().shorthand().unwrap().to_string();
        {
            let head = repo.head().unwrap().peel_to_commit().unwrap();
            repo.branch("topic", &head, false).unwrap();
        }
        commit(&repo, "main side", &[("shared.txt", "main\n")]);
        let main_tip = repo.head().unwrap().target().unwrap();
        switch(&repo, "refs/heads/topic");
        commit(&repo, "topic side", &[("shared.txt", "topic\n")]);
        drop(repo);

        rebase_branch(dir.to_str().unwrap(), &original).unwrap();
        assert_eq!(
            operation_state(dir.to_str().unwrap()).unwrap(),
            "rebase",
            "a conflicting rebase stays in progress"
        );
        assert_eq!(
            conflicts::list_conflicts(dir.to_str().unwrap()).unwrap(),
            vec!["shared.txt".to_string()]
        );

        conflicts::resolve_conflict(dir.to_str().unwrap(), "shared.txt", "both\n").unwrap();
        rebase_continue(dir.to_str().unwrap()).unwrap();

        let repo = Repository::open(&dir).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(head.message().unwrap_or_default().trim(), "topic side");
        assert_eq!(head.parent(0).unwrap().id(), main_tip);
        assert_eq!(
            std::fs::read_to_string(dir.join("shared.txt")).unwrap(),
            "both\n"
        );
        assert_eq!(operation_state(dir.to_str().unwrap()).unwrap(), "none");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rebase_can_be_aborted() {
        let dir = scratch("rebase-abort");
        let repo = init(&dir);
        commit(&repo, "base", &[("shared.txt", "base\n")]);
        let original = repo.head().unwrap().shorthand().unwrap().to_string();
        {
            let head = repo.head().unwrap().peel_to_commit().unwrap();
            repo.branch("topic", &head, false).unwrap();
        }
        commit(&repo, "main side", &[("shared.txt", "main\n")]);
        switch(&repo, "refs/heads/topic");
        let topic_tip = commit(&repo, "topic side", &[("shared.txt", "topic\n")]);
        drop(repo);

        rebase_branch(dir.to_str().unwrap(), &original).unwrap();
        rebase_abort(dir.to_str().unwrap()).unwrap();

        assert_eq!(operation_state(dir.to_str().unwrap()).unwrap(), "none");
        let repo = Repository::open(&dir).unwrap();
        assert_eq!(repo.head().unwrap().shorthand(), Some("topic"));
        assert_eq!(
            repo.head().unwrap().target().unwrap(),
            topic_tip,
            "abort puts the branch back"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rebase_actions_need_a_rebase() {
        let dir = scratch("rebase-none");
        let repo = init(&dir);
        commit(&repo, "base", &[("a.txt", "a\n")]);
        drop(repo);

        assert_eq!(
            rebase_continue(dir.to_str().unwrap()).unwrap_err(),
            "There is no rebase in progress."
        );
        assert_eq!(
            rebase_abort(dir.to_str().unwrap()).unwrap_err(),
            "There is no rebase in progress."
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
