#[cfg(test)]
mod integration {
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    pub(super) fn temp_repo() -> PathBuf {
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

    pub(super) fn checkout(repo: &git2::Repository, name: &str) {
        let refname = format!("refs/heads/{name}");
        let obj = repo.revparse_single(&refname).unwrap();
        repo.checkout_tree(&obj, None).unwrap();
        repo.set_head(&refname).unwrap();
    }

    pub(super) fn current_branch(repo: &git2::Repository) -> String {
        repo.head().unwrap().shorthand().unwrap().to_string()
    }

    pub(super) fn commit_file(dir: &PathBuf, name: &str, content: &str, message: &str) -> git2::Oid {
        let repo = git2::Repository::open(dir).unwrap();
        std::fs::write(dir.join(name), content).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(std::path::Path::new(name)).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();
        let parents: Vec<git2::Commit> = repo
            .head()
            .and_then(|h| h.peel_to_commit())
            .into_iter()
            .collect();
        let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)
            .unwrap()
    }

    #[test]
    fn log_commits_returns_history() {        let dir = temp_repo();
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
    fn file_contents_reports_original_and_modified() {
        let dir = temp_repo();
        let path = dir.to_str().unwrap();
        std::fs::write(dir.join("new.txt"), "hello world").unwrap();

        let contents = crate::git::status::get_file_contents(path, "new.txt").unwrap();
        assert_eq!(contents.original, "");
        assert_eq!(contents.modified, "hello world");

        let missing = crate::git::status::get_file_contents(path, "does-not-exist.txt").unwrap();
        assert_eq!(missing.original, "");
        assert_eq!(missing.modified, "");

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

#[cfg(test)]
mod advanced {
    use super::integration::{checkout, commit_file, current_branch, temp_repo};

    #[test]
    fn merge_fast_forward() {
        let dir = temp_repo();
        let path = dir.to_str().unwrap();
        let repo = git2::Repository::open(&dir).unwrap();
        let main = current_branch(&repo);
        repo.branch("feature", &repo.head().unwrap().peel_to_commit().unwrap(), false)
            .unwrap();
        checkout(&repo, "feature");
        let b = commit_file(&dir, "a.txt", "hello", "feature work");
        checkout(&repo, &main);

        crate::git::merge::merge_branch(path, "feature").unwrap();
        let head = git2::Repository::open(&dir)
            .unwrap()
            .head()
            .unwrap()
            .peel_to_commit()
            .unwrap()
            .id();
        assert_eq!(head, b);
        assert!(dir.join("a.txt").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn merge_conflict_is_reported_and_resolvable() {
        let dir = temp_repo();
        let path = dir.to_str().unwrap();
        let repo = git2::Repository::open(&dir).unwrap();
        let main = current_branch(&repo);
        commit_file(&dir, "x.txt", "base", "base");
        repo.branch("feature", &repo.head().unwrap().peel_to_commit().unwrap(), false)
            .unwrap();

        checkout(&repo, "feature");
        commit_file(&dir, "x.txt", "feature", "feature side");
        checkout(&repo, &main);
        commit_file(&dir, "x.txt", "main", "main side");

        let err = crate::git::merge::merge_branch(path, "feature").unwrap_err();
        assert!(err.contains("conflict"));

        let conflicts = crate::git::conflicts::list_conflicts(path).unwrap();
        assert_eq!(conflicts, vec!["x.txt".to_string()]);

        let content = crate::git::conflicts::get_conflict_content(path, "x.txt").unwrap();
        assert!(content.contains("<<<<<<<"));

        crate::git::conflicts::resolve_conflict(path, "x.txt", "resolved\n").unwrap();
        assert!(crate::git::conflicts::list_conflicts(path).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn branch_rename_and_delete() {
        let dir = temp_repo();
        let path = dir.to_str().unwrap();
        let repo = git2::Repository::open(&dir).unwrap();
        let main = current_branch(&repo);

        crate::git::branch::rename_branch(path, &main, "renamed").unwrap();
        let names = crate::git::branch::list_branches(path).unwrap();
        assert!(names.contains(&"renamed".to_string()));
        assert!(!names.contains(&main));

        crate::git::branch::create_branch(path, "temp").unwrap();
        crate::git::branch::delete_branch(path, "temp").unwrap();
        let names = crate::git::branch::list_branches(path).unwrap();
        assert!(!names.contains(&"temp".to_string()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn reflog_lists_entries() {
        let dir = temp_repo();
        let path = dir.to_str().unwrap();
        let entries = crate::git::reflog::list_reflog(path).unwrap();
        assert!(!entries.is_empty());
        assert!(entries.iter().any(|e| e.message.contains("initial commit")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cherry_pick_and_revert() {
        let dir = temp_repo();
        let path = dir.to_str().unwrap();
        let repo = git2::Repository::open(&dir).unwrap();
        let main = current_branch(&repo);
        repo.branch("feature", &repo.head().unwrap().peel_to_commit().unwrap(), false)
            .unwrap();
        checkout(&repo, "feature");
        let picked = commit_file(&dir, "c.txt", "picked", "add c");
        checkout(&repo, &main);

        crate::git::advanced::cherry_pick(path, &picked.to_string()).unwrap();
        assert!(dir.join("c.txt").exists());

        let picked_head = git2::Repository::open(&dir)
            .unwrap()
            .head()
            .unwrap()
            .target()
            .unwrap();
        crate::git::advanced::revert_commit(path, &picked_head.to_string()).unwrap();
        assert!(!dir.join("c.txt").exists());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
