use git2::Repository;
use std::path::{Component, Path};

/// Conflicted paths come from the index, but they are also accepted from the
/// frontend, so reject anything that would leave the repository before it is
/// joined to the repository path.
fn check_file(file: &str) -> Result<&Path, String> {
    let candidate = Path::new(file);
    if file.trim().is_empty()
        || candidate.is_absolute()
        || candidate
            .components()
            .any(|part| matches!(part, Component::ParentDir))
    {
        return Err("Only repository files can be opened.".to_string());
    }
    Ok(candidate)
}

/// List files with unresolved conflicts in the index.
///
/// Uses the index conflict entries rather than scanning the working tree,
/// which avoids walking `.git`, `target`, `node_modules`, etc.
pub fn list_conflicts(path: &str) -> Result<Vec<String>, String> {
    let repo = Repository::open(path).map_err(|_| "Could not open the repository.".to_string())?;
    let index = repo.index().map_err(|_| "Could not read the index.".to_string())?;
    if !index.has_conflicts() {
        return Ok(Vec::new());
    }

    let mut files = Vec::new();
    for conflict in index
        .conflicts()
        .map_err(|_| "Could not read the conflicts.".to_string())?
    {
        let conflict = conflict.map_err(|_| "Could not read the conflicts.".to_string())?;
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
    let relative = check_file(file)?;
    let full = Path::new(path).join(relative);
    std::fs::read_to_string(full).map_err(|_| "Could not read the conflicted file.".to_string())
}

/// Write resolved content to a file and stage it.
pub fn resolve_conflict(path: &str, file: &str, content: &str) -> Result<(), String> {
    let relative = check_file(file)?;
    let repo = Repository::open(path).map_err(|_| "Could not open the repository.".to_string())?;
    let full = Path::new(path).join(relative);
    std::fs::write(&full, content).map_err(|_| "Could not write the resolved file.".to_string())?;
    let mut index = repo.index().map_err(|_| "Could not read the index.".to_string())?;
    index
        .add_path(relative)
        .map_err(|_| "Could not stage the resolved file.".to_string())?;
    index
        .write()
        .map_err(|_| "Could not save the resolved file.".to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn scratch() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "knitt-conflicts-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn refuses_paths_outside_the_repository() {
        let dir = scratch();
        let repo = Repository::init(&dir).unwrap();
        drop(repo);

        for file in ["../secret", "/etc/passwd", "", "  "] {
            assert_eq!(
                get_conflict_content(dir.to_str().unwrap(), file).unwrap_err(),
                "Only repository files can be opened.",
                "reading {file:?} should be refused"
            );
            assert_eq!(
                resolve_conflict(dir.to_str().unwrap(), file, "x").unwrap_err(),
                "Only repository files can be opened.",
                "writing {file:?} should be refused"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn resolves_and_stages_a_file() {
        let dir = scratch();
        Repository::init(&dir).unwrap();
        std::fs::write(dir.join("x.txt"), "a\n<<<<<<< HEAD\nb\n=======\nc\n>>>>>>> other\n")
            .unwrap();

        resolve_conflict(dir.to_str().unwrap(), "x.txt", "resolved\n").unwrap();

        assert_eq!(
            std::fs::read_to_string(dir.join("x.txt")).unwrap(),
            "resolved\n"
        );
        let repo = Repository::open(&dir).unwrap();
        let index = repo.index().unwrap();
        let entry = index.get_path(Path::new("x.txt"), 0).unwrap();
        let blob = repo.find_blob(entry.id).unwrap();
        assert_eq!(std::str::from_utf8(blob.content()).unwrap(), "resolved\n");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
