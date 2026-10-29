use git2::{BranchType, Repository};

pub fn list_branches(path: &str) -> Result<Vec<String>, String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let mut branches = Vec::new();
    let branches_iter = repo.branches(Some(BranchType::Local)).map_err(|e| e.to_string())?;
    for branch in branches_iter {
        let (b, _) = branch.map_err(|e| e.to_string())?;
        if let Some(name) = b.name().map_err(|e| e.to_string())? {
            branches.push(name.to_string());
        }
    }
    Ok(branches)
}

pub fn create_branch(path: &str, name: &str) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let head = repo.head().map_err(|e| e.to_string())?;
    let commit = head.peel_to_commit().map_err(|e| e.to_string())?;
    repo.branch(name, &commit, false).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn checkout_branch(path: &str, name: &str) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let refname = format!("refs/heads/{}", name);
    let obj = repo.revparse_single(&refname).map_err(|e| e.to_string())?;
    repo.checkout_tree(&obj, None).map_err(|e| e.to_string())?;
    repo.set_head(&refname).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn rename_branch(path: &str, old_name: &str, new_name: &str) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let mut branch = repo.find_branch(old_name, BranchType::Local).map_err(|e| e.to_string())?;
    branch.rename(new_name, false).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn delete_branch(path: &str, name: &str) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let mut branch = repo.find_branch(name, BranchType::Local).map_err(|e| e.to_string())?;
    branch.delete().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn set_upstream(path: &str, branch_name: &str, upstream: &str) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let mut branch = repo.find_branch(branch_name, BranchType::Local).map_err(|e| e.to_string())?;
    branch.set_upstream(Some(upstream)).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn publish_branch(path: &str, branch_name: &str) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let mut remote = repo.find_remote("origin").map_err(|e| e.to_string())?;
    let refspec = format!("refs/heads/{}:refs/heads/{}", branch_name, branch_name);
    remote.push(&[refspec.as_str()], None).map_err(|e| e.to_string())?;
    set_upstream(path, branch_name, &format!("origin/{}", branch_name))?;
    Ok(())
}
