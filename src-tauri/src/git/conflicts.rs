use git2::Repository;
use std::fs;

pub fn list_conflicts(path: &str) -> Result<Vec<String>, String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let workdir = repo.workdir().ok_or("no workdir")?;
    let mut files = Vec::new();
    for entry in walkdir::WalkDir::new(workdir).into_iter().flatten() {
        if entry.file_type().is_file()
            && let Ok(content) = fs::read_to_string(entry.path())
            && content.contains("<<<<<<<")
            && let Ok(rel) = entry.path().strip_prefix(workdir)
        {
            files.push(rel.to_string_lossy().to_string());
        }
    }
    Ok(files)
}

pub fn get_conflict_content(path: &str, file: &str) -> Result<String, String> {
    let full = std::path::Path::new(path).join(file);
    fs::read_to_string(full).map_err(|e| e.to_string())
}

pub fn resolve_conflict(path: &str, file: &str, content: &str) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let full = std::path::Path::new(path).join(file);
    fs::write(&full, content).map_err(|e| e.to_string())?;
    let mut index = repo.index().map_err(|e| e.to_string())?;
    index.add_path(std::path::Path::new(file)).map_err(|e| e.to_string())?;
    index.write().map_err(|e| e.to_string())?;
    Ok(())
}
