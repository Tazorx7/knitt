use git2::Repository;

pub fn list_tags(path: &str) -> Result<Vec<String>, String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let mut tags = Vec::new();
    repo.tag_foreach(|_oid, name| {
        if let Ok(n) = std::str::from_utf8(name)
            && let Some(t) = n.strip_prefix("refs/tags/")
        {
            tags.push(t.to_string());
        }
        true
    })
    .map_err(|e| e.to_string())?;
    Ok(tags)
}

pub fn create_tag(path: &str, name: &str) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let head = repo.head().map_err(|e| e.to_string())?;
    let commit = head.peel_to_commit().map_err(|e| e.to_string())?;
    repo.tag_lightweight(name, commit.as_object(), false)
        .map_err(|e| e.to_string())?;
    Ok(())
}
