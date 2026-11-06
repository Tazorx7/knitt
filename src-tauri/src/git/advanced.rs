use git2::{Oid, Repository};

pub fn cherry_pick(path: &str, oid_str: &str) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let oid = Oid::from_str(oid_str).map_err(|_| "Invalid commit id.".to_string())?;
    let commit = repo.find_commit(oid).map_err(|_| "Commit not found.".to_string())?;

    repo.cherrypick(&commit, None).map_err(|e| e.to_string())?;

    let mut index = repo.index().map_err(|e| e.to_string())?;
    if index.has_conflicts() {
        return Err("Cherry-pick produced conflicts. Resolve them, then commit.".to_string());
    }

    let sig = repo
        .signature()
        .map_err(|_| "Git author identity is not configured.".to_string())?;
    let tree_id = index.write_tree_to(&repo).map_err(|e| e.to_string())?;
    let tree = repo.find_tree(tree_id).map_err(|e| e.to_string())?;
    let head = repo
        .head()
        .and_then(|h| h.peel_to_commit())
        .map_err(|_| "No commits on the current branch.".to_string())?;
    let message = commit.message().unwrap_or("Cherry-pick");
    repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&head])
        .map_err(|e| e.to_string())?;
    repo.cleanup_state().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn revert_commit(path: &str, oid_str: &str) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let oid = Oid::from_str(oid_str).map_err(|_| "Invalid commit id.".to_string())?;
    let commit = repo.find_commit(oid).map_err(|_| "Commit not found.".to_string())?;

    repo.revert(&commit, None).map_err(|e| e.to_string())?;

    let mut index = repo.index().map_err(|e| e.to_string())?;
    if index.has_conflicts() {
        return Err("Revert produced conflicts. Resolve them, then commit.".to_string());
    }

    let sig = repo
        .signature()
        .map_err(|_| "Git author identity is not configured.".to_string())?;
    let tree_id = index.write_tree_to(&repo).map_err(|e| e.to_string())?;
    let tree = repo.find_tree(tree_id).map_err(|e| e.to_string())?;
    let head = repo
        .head()
        .and_then(|h| h.peel_to_commit())
        .map_err(|_| "No commits on the current branch.".to_string())?;
    let summary = commit.summary().unwrap_or("commit");
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        &format!("Revert \"{summary}\""),
        &tree,
        &[&head],
    )
    .map_err(|e| e.to_string())?;
    repo.cleanup_state().map_err(|e| e.to_string())?;
    Ok(())
}
