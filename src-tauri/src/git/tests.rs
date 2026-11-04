#[cfg(test)]
mod integration {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_repo() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "knitt-test-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        let repo = git2::Repository::init(&dir).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
        let sig = git2::Signature::now("Test", "test@example.com").unwrap();
        let mut index = repo.index().unwrap();
        let tree_oid = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_oid).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[])
            .unwrap();

        dir
    }

    #[test]
    fn log_commits_returns_history() {
        let dir = temp_repo();
        let commits = crate::git::history::log_commits(dir.to_str().unwrap(), 10).unwrap();
        assert_eq!(commits.len(), 1);
        assert_eq!(commits[0].message, "initial commit");
        assert!(commits[0].author.contains("Test"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tags_roundtrip() {
        let dir = temp_repo();
        let path = dir.to_str().unwrap();
        crate::git::tags::create_tag(path, "v1.0.0").unwrap();
        let tags = crate::git::tags::list_tags(path).unwrap();
        assert_eq!(tags, vec!["v1.0.0".to_string()]);
        crate::git::tags::delete_tag(path, "v1.0.0").unwrap();
        assert!(crate::git::tags::list_tags(path).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stash_roundtrip() {
        let dir = temp_repo();
        let path = dir.to_str().unwrap();
        // Create an unstaged change to stash.
        let file = dir.join("a.txt");
        std::fs::write(&file, "hello").unwrap();
        let repo = git2::Repository::open(&dir).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new("a.txt")).unwrap();
        index.write().unwrap();

        crate::git::stash::stash_save(path, None).unwrap();
        let stashes = crate::git::stash::stash_list(path).unwrap();
        assert_eq!(stashes.len(), 1);
        crate::git::stash::stash_pop(path).unwrap();
        assert!(crate::git::stash::stash_list(path).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
