use git2::{Repository, Signature};

pub fn commit_all(path: &str, message: &str) -> Result<String, String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let mut index = repo.index().map_err(|e| e.to_string())?;
    index.add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None).map_err(|e| e.to_string())?;
    index.write().map_err(|e| e.to_string())?;
    let oid = index.write_tree().map_err(|e| e.to_string())?;
    let tree = repo.find_tree(oid).map_err(|e| e.to_string())?;
    
    let sig = repo.signature().or_else(|_| {
        Signature::now("User", "user@example.com")
    }).map_err(|e| e.to_string())?;
    
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents = if let Some(ref p) = parent { vec![p] } else { vec![] };
    
    let oid = repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents).map_err(|e| e.to_string())?;
    Ok(oid.to_string())
}
