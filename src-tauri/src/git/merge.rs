use git2::{BranchType, Repository};

/// Merge `branch_name` into the current HEAD.
///
/// Up-to-date is a no-op; fast-forwards move the branch pointer; divergent
/// history produces a merge commit. On conflicts the merge is left in place
/// (index + worktree contain conflict markers) and a friendly error is
/// returned so the UI can offer conflict resolution.
pub fn merge_branch(path: &str, branch_name: &str) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let branch_ref = repo
        .find_reference(&format!("refs/heads/{branch_name}"))
        .map_err(|_| "Branch not found.".to_string())?;
    let annotated = repo
        .reference_to_annotated_commit(&branch_ref)
        .map_err(|e| e.to_string())?;
    let (analysis, _) = repo
        .merge_analysis(&[&annotated])
        .map_err(|e| e.to_string())?;

    if analysis.is_up_to_date() {
        return Ok(());
    }

    if analysis.is_fast_forward() {
        let head = repo.head().map_err(|e| e.to_string())?;
        let refname = head
            .name()
            .ok_or_else(|| "Cannot read HEAD.".to_string())?
            .to_string();
        let mut reference = repo.find_reference(&refname).map_err(|e| e.to_string())?;
        reference
            .set_target(annotated.id(), "merge: fast-forward")
            .map_err(|e| e.to_string())?;
        repo.set_head(&refname).map_err(|e| e.to_string())?;
        repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
            .map_err(|e| e.to_string())?;
        return Ok(());
    }

    // Normal (non-fast-forward) merge.
    let head_commit = repo
        .head()
        .and_then(|h| h.peel_to_commit())
        .map_err(|_| "No commits on the current branch.".to_string())?;
    let merge_commit = repo
        .find_commit(annotated.id())
        .map_err(|e| e.to_string())?;

    let mut checkout = git2::build::CheckoutBuilder::new();
    checkout.safe();
    repo.merge(&[&annotated], None, Some(&mut checkout))
        .map_err(|e| e.to_string())?;

    let mut index = repo.index().map_err(|e| e.to_string())?;
    if index.has_conflicts() {
        return Err("Merge produced conflicts. Resolve them, then commit.".to_string());
    }

    let sig = repo
        .signature()
        .map_err(|_| "Git author identity is not configured.".to_string())?;
    let tree_id = index.write_tree_to(&repo).map_err(|e| e.to_string())?;
    let tree = repo.find_tree(tree_id).map_err(|e| e.to_string())?;
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        &format!("Merge branch '{branch_name}'"),
        &tree,
        &[&head_commit, &merge_commit],
    )
    .map_err(|e| e.to_string())?;
    repo.cleanup_state().map_err(|e| e.to_string())?;
    Ok(())
}

/// Rebase the current branch onto `branch_name`.
///
/// Applies each commit in turn. On conflicts the rebase is left in progress so
/// the caller can resolve and continue; a friendly error is returned.
pub fn rebase_branch(path: &str, branch_name: &str) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let branch = repo
        .find_branch(branch_name, BranchType::Local)
        .map_err(|_| "Branch not found.".to_string())?;
    let upstream = repo
        .reference_to_annotated_commit(branch.get())
        .map_err(|e| e.to_string())?;

    let sig = repo
        .signature()
        .map_err(|_| "Git author identity is not configured.".to_string())?;
    let mut rebase = repo
        .rebase(None, Some(&upstream), None, None)
        .map_err(|e| e.to_string())?;

    while let Some(op) = rebase.next() {
        op.map_err(|e| e.to_string())?;
        let index = repo.index().map_err(|e| e.to_string())?;
        if index.has_conflicts() {
            return Err("Rebase stopped due to conflicts. Resolve them, then continue.".to_string());
        }
        match rebase.commit(None, &sig, None) {
            Ok(_) => {}
            // Nothing to commit for this operation (already applied/empty).
            Err(e)
                if matches!(
                    e.code(),
                    git2::ErrorCode::Applied | git2::ErrorCode::Exists
                ) => {}
            Err(e) => return Err(e.to_string()),
        }
    }

    rebase.finish(Some(&sig)).map_err(|e| e.to_string())?;
    Ok(())
}
