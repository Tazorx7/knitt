use git2::Repository;

/// List files with unresolved conflicts in the index.
///
/// Uses the index conflict entries rather than scanning the working tree,
/// which avoids walking `.git`, `target`, `node_modules`, etc.
pub fn list_conflicts(path: &str) -> Result<Vec<String>, String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let index = repo.index().map_err(|e| e.to_string())?;
    if !index.has_conflicts() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for conflict in index.conflicts().map_err(|e| e.to_string())? {
        let conflict = conflict.map_err(|e| e.to_string())?;
        let entry = conflict.our.or(conflict.their).or(conflict.ancestor);
        if let Some(entry) = entry
            && let Ok(name) = std::str::from_utf8(&entry.path)
        {
            files.push(name.to_string());
        }
    }
    files.sort();
    files.dedup();
    Ok(files)
}

/// Return the current working-tree content of a conflicted file
/// (including conflict markers, when present).
pub fn get_conflict_content(path: &str, file: &str) -> Result<String, String> {
    let full = std::path::Path::new(path).join(file);
    std::fs::read_to_string(full).map_err(|_| "Could not read the conflicted file.".to_string())
}

/// Write resolved content to a file and stage it.
pub fn resolve_conflict(path: &str, file: &str, content: &str) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let full = std::path::Path::new(path).join(file);
    std::fs::write(&full, content).map_err(|_| "Could not write the resolved file.".to_string())?;
    let mut index = repo.index().map_err(|e| e.to_string())?;
    index
        .add_path(std::path::Path::new(file))
        .map_err(|e| e.to_string())?;
    index.write().map_err(|e| e.to_string())?;
    Ok(())
}
