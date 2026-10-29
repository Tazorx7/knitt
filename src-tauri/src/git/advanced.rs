use git2::{Repository, Oid};

pub fn cherry_pick(path: &str, oid_str: &str) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let oid = Oid::from_str(oid_str).map_err(|e| e.to_string())?;
    let commit = repo.find_commit(oid).map_err(|e| e.to_string())?;
    repo.cherrypick(&commit, None).map_err(|e| e.to_string())?;
    let mut index = repo.index().map_err(|e| e.to_string())?;
    if index.has_conflicts() {
        return Err("conflicts".to_string());
    }
    let sig = repo.signature().map_err(|e| e.to_string())?;
    let tree_id = index.write_tree_to(&repo).map_err(|e| e.to_string())?;
    let tree = repo.find_tree(tree_id).map_err(|e| e.to_string())?;
    let head = repo.head().map_err(|e| e.to_string())?;
    let head_commit = repo.find_commit(head.peel_to_commit().unwrap().id()).map_err(|e| e.to_string())?;
    repo.commit(Some("HEAD"), &sig, &sig, commit.message().unwrap_or("Cherry-pick"), &tree, &[&head_commit]).map_err(|e| e.to_string())?;
    repo.cleanup_state().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn revert_commit(path: &str, oid_str: &str) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let oid = Oid::from_str(oid_str).map_err(|e| e.to_string())?;
    let commit = repo.find_commit(oid).map_err(|e| e.to_string())?;
    repo.revert(&commit, None).map_err(|e| e.to_string())?;
    let mut index = repo.index().map_err(|e| e.to_string())?;
    if index.has_conflicts() {
        return Err("conflicts".to_string());
    }
    let sig = repo.signature().map_err(|e| e.to_string())?;
    let tree_id = index.write_tree_to(&repo).map_err(|e| e.to_string())?;
    let tree = repo.find_tree(tree_id).map_err(|e| e.to_string())?;
    let head = repo.head().map_err(|e| e.to_string())?;
    let head_commit = head.peel_to_commit().map_err(|e| e.to_string())?;
    repo.commit(Some("HEAD"), &sig, &sig, &format!("Revert \"{}\"", commit.message().unwrap_or("")), &tree, &[&head_commit]).map_err(|e| e.to_string())?;
    repo.cleanup_state().map_err(|e| e.to_string())?;
    Ok(())
}
