use git2::{Repository, StashFlags};

pub fn stash_save(path: &str, message: Option<&str>) -> Result<(), String> {
    let mut repo =
        Repository::open(path).map_err(|_| "Could not open the repository.".to_string())?;
    let sig = repo
        .signature()
        .map_err(|_| "Set your Git name and email before stashing changes.".to_string())?;
    repo.stash_save(&sig, message.unwrap_or("WIP"), Some(StashFlags::DEFAULT))
        .map_err(|_| "Could not stash your changes.".to_string())?;
    Ok(())
}

pub fn stash_list(path: &str) -> Result<Vec<String>, String> {
    let mut repo = Repository::open(path).map_err(|e| e.to_string())?;
    let mut result = Vec::new();
    repo.stash_foreach(|_index, msg, _oid| {
        result.push(msg.to_string());
        true
    }).map_err(|e| e.to_string())?;
    Ok(result)
}

pub fn stash_pop(path: &str) -> Result<(), String> {
    let mut repo = Repository::open(path).map_err(|e| e.to_string())?;
    repo.stash_pop(0, None).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn stash_drop(path: &str) -> Result<(), String> {
    let mut repo = Repository::open(path).map_err(|e| e.to_string())?;
    repo.stash_drop(0).map_err(|e| e.to_string())?;
    Ok(())
}
