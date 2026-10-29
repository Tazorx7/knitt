use git2::Repository;

pub fn list_remotes(path: &str) -> Result<Vec<String>, String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let remotes = repo.remotes().map_err(|e| e.to_string())?;
    Ok(remotes.iter().flatten().map(|s| s.to_string()).collect())
}

pub fn add_remote(path: &str, name: &str, url: &str) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    repo.remote(name, url).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn remove_remote(path: &str, name: &str) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    repo.remote_delete(name).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn set_remote_url(path: &str, name: &str, url: &str) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    repo.remote_set_url(name, url).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn fetch_remote(path: &str, name: &str) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let mut remote = repo.find_remote(name).map_err(|e| e.to_string())?;
    remote.fetch(&[] as &[&str], None, None).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn pull_remote(path: &str) -> Result<(), String> {
    // Simple pull via fetch + merge is complex; for now just fetch origin
    fetch_remote(path, "origin")
}
