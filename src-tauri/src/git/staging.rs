use git2::{IndexAddOption, ObjectType, Repository};
use std::path::Path;

pub fn stage_files(path: &str, files: &[String]) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let mut index = repo.index().map_err(|e| e.to_string())?;
    for f in files {
        index.add_path(std::path::Path::new(f)).map_err(|e| e.to_string())?;
    }
    index.write().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn unstage_files(path: &str, files: &[String]) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let head = repo.head().map_err(|e| e.to_string())?;
    let commit = head.peel_to_commit().map_err(|e| e.to_string())?;
    let obj = commit.into_object();
    repo.reset_default(Some(&obj), files.iter().map(|s| s.as_str()))
        .map_err(|e| e.to_string())?;
    Ok(())
}

pub fn stage_all(path: &str) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let mut index = repo.index().map_err(|e| e.to_string())?;
    index
        .add_all(["*"].iter(), IndexAddOption::DEFAULT, None)
        .map_err(|e| e.to_string())?;
    index.write().map_err(|e| e.to_string())?;
    Ok(())
}

pub fn stage_hunk(path: &str, file: &str, _hunk_header: &str) -> Result<(), String> {
    stage_files(path, &[file.to_string()])
}

pub fn unstage_hunk(path: &str, file: &str, _hunk_header: &str) -> Result<(), String> {
    unstage_files(path, &[file.to_string()])
}

/// Restores tracked files to their committed state, discarding index and
/// working-tree changes. Untracked files are refused: deleting them is a
/// separate, riskier action that Knitt does not offer yet.
pub fn discard_files(path: &str, files: &[String]) -> Result<(), String> {
    if files.is_empty() {
        return Err("Choose a file to discard.".to_string());
    }
    for file in files {
        let candidate = Path::new(file);
        if candidate.is_absolute()
            || candidate
                .components()
                .any(|part| matches!(part, std::path::Component::ParentDir))
        {
            return Err("Only repository files can be discarded.".to_string());
        }
    }

    let repo = Repository::open(path).map_err(|_| "Could not open the repository.".to_string())?;
    let head = repo
        .head()
        .map_err(|_| "This repository has no commits yet.".to_string())?;
    let object = head
        .peel(ObjectType::Commit)
        .map_err(|_| "This repository has no commits yet.".to_string())?;
    let tree = object
        .peel_to_tree()
        .map_err(|_| "Could not read the committed files.".to_string())?;

    for file in files {
        if tree.get_path(Path::new(file)).is_err() {
            return Err("Only tracked files can be discarded.".to_string());
        }
    }

    repo.reset_default(Some(&object), files.iter().map(|file| file.as_str()))
        .map_err(|_| "Could not discard those changes.".to_string())?;

    let mut checkout = git2::build::CheckoutBuilder::new();
    checkout.force();
    for file in files {
        checkout.path(file.as_str());
    }
    repo.checkout_head(Some(&mut checkout))
        .map_err(|_| "Could not update the working tree.".to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_repo() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "knitt-discard-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let repo = Repository::init(&dir).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
        drop(config);

        std::fs::write(dir.join("tracked.txt"), "original\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("tracked.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
            .unwrap();
        dir
    }

    #[test]
    fn discards_working_tree_changes() {
        let dir = temp_repo();
        std::fs::write(dir.join("tracked.txt"), "changed\n").unwrap();

        discard_files(dir.to_str().unwrap(), &["tracked.txt".to_string()]).unwrap();

        let content = std::fs::read_to_string(dir.join("tracked.txt")).unwrap();
        assert_eq!(content, "original\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuses_untracked_and_escaping_paths() {
        let dir = temp_repo();
        std::fs::write(dir.join("untracked.txt"), "new\n").unwrap();

        assert!(discard_files(dir.to_str().unwrap(), &["untracked.txt".to_string()]).is_err());
        assert!(discard_files(dir.to_str().unwrap(), &["../escape.txt".to_string()]).is_err());
        assert!(discard_files(dir.to_str().unwrap(), &[]).is_err());
        assert!(dir.join("untracked.txt").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
