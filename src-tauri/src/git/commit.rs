use git2::Repository;

use super::signing::{self, SigningConfig};

/// Points HEAD (or the branch it names) at `oid` after writing a signed commit.
fn move_head_to(repo: &Repository, oid: git2::Oid) -> Result<(), String> {
    let head = repo
        .find_reference("HEAD")
        .map_err(|_| "Could not read HEAD.".to_string())?;
    // An unborn branch still has a symbolic HEAD, so create its target.
    let target = head
        .symbolic_target()
        .map(str::to_string)
        .unwrap_or_else(|| "HEAD".to_string());
    repo.reference(&target, oid, true, "commit: signed")
        .map_err(|_| "Could not update the branch after signing.".to_string())?;
    Ok(())
}

/// Commits exactly what is in the index, so the user decides what goes into
/// the commit. An unchanged index is refused rather than creating an empty
/// commit.
pub fn commit_staged(
    path: &str,
    message: &str,
    signing: Option<&SigningConfig>,
) -> Result<String, String> {
    let message = message.trim();
    if message.is_empty() {
        return Err("Write a commit message first.".to_string());
    }

    let repo = Repository::open(path).map_err(|_| "Could not open the repository.".to_string())?;
    let mut index = repo
        .index()
        .map_err(|_| "Could not read the index.".to_string())?;
    let tree_oid = index
        .write_tree()
        .map_err(|_| "Could not read the staged changes.".to_string())?;
    let tree = repo
        .find_tree(tree_oid)
        .map_err(|_| "Could not read the staged changes.".to_string())?;

    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let unchanged = match &parent {
        Some(commit) => commit
            .tree()
            .map(|parent_tree| parent_tree.id() == tree_oid)
            .unwrap_or(false),
        None => tree.is_empty(),
    };
    if unchanged {
        return Err("Stage the changes you want to commit first.".to_string());
    }

    let sig = repo.signature().map_err(|_| {
        "Set your name and email in Settings → Git before committing.".to_string()
    })?;

    let parents = if let Some(ref p) = parent { vec![p] } else { vec![] };
    let parent_refs: Vec<&git2::Commit> = parents.clone();

    let Some(config) = signing else {
        let oid = repo
            .commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)
            .map_err(|_| "Could not create the commit.".to_string())?;
        return Ok(oid.to_string());
    };

    // Build the exact commit bytes libgit2 would write, sign those, and let
    // libgit2 attach the signature so the object stays valid.
    let buffer = repo
        .commit_create_buffer(&sig, &sig, message, &tree, &parent_refs)
        .map_err(|_| "Could not prepare the commit for signing.".to_string())?;
    let content = String::from_utf8(buffer.to_vec())
        .map_err(|_| "Could not prepare the commit for signing.".to_string())?;
    let signature = signing::sign(&content, config)?;
    let oid = repo
        .commit_signed(&content, &signature, None)
        .map_err(|_| "Could not write the signed commit.".to_string())?;
    move_head_to(&repo, oid)?;
    Ok(oid.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn scratch(name: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "knitt-commit-{name}-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn init_repo(dir: &Path) -> Repository {
        let repo = Repository::init(dir).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
        repo
    }

    fn stage_all(dir: &Path) {
        crate::git::staging::stage_all(dir.to_str().unwrap()).unwrap();
    }

    fn ssh_keygen_available() -> bool {
        std::process::Command::new("ssh-keygen")
            .arg("-?")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|status| status.success() || status.code().is_some())
            .unwrap_or(false)
    }

    #[test]
    fn unsigned_commit_keeps_working() {
        let dir = scratch("unsigned");
        let repo = init_repo(&dir);
        std::fs::write(dir.join("a.txt"), "a\n").unwrap();
        drop(repo);
        stage_all(&dir);

        let oid = commit_staged(dir.to_str().unwrap(), "initial", None).unwrap();
        assert!(!oid.is_empty());

        let repo = Repository::open(&dir).unwrap();
        assert_eq!(repo.head().unwrap().target().unwrap().to_string(), oid);
        let commit = repo.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(commit.message().unwrap_or_default().trim(), "initial");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ssh_signed_commit_updates_the_branch_and_carries_a_signature() {
        if !ssh_keygen_available() {
            return;
        }
        let dir = scratch("signed");
        let key = dir.join("signing_key");
        let generated = std::process::Command::new("ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-f"])
            .arg(&key)
            .status()
            .unwrap();
        assert!(generated.success());

        let repo = init_repo(&dir);
        std::fs::write(dir.join("a.txt"), "a\n").unwrap();
        drop(repo);
        stage_all(&dir);

        let config = SigningConfig {
            mode: signing::SigningMode::Ssh,
            key: key.display().to_string(),
        };
        let oid = commit_staged(dir.to_str().unwrap(), "signed initial", Some(&config)).unwrap();

        let repo = Repository::open(&dir).unwrap();
        let head = repo.head().unwrap();
        assert_eq!(head.target().unwrap().to_string(), oid);
        let commit = head.peel_to_commit().unwrap();
        assert_eq!(commit.message().unwrap_or_default().trim(), "signed initial");

        let oid = git2::Oid::from_str(&oid).unwrap();
        let (signature, _) = repo.extract_signature(&oid, None).unwrap();
        let signature = std::str::from_utf8(&signature).unwrap_or_default();
        assert!(signature.starts_with("-----BEGIN SSH SIGNATURE-----"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn signing_failure_leaves_the_branch_untouched() {
        let dir = scratch("failure");
        let repo = init_repo(&dir);
        std::fs::write(dir.join("a.txt"), "a\n").unwrap();
        drop(repo);
        stage_all(&dir);
        commit_staged(dir.to_str().unwrap(), "first", None).unwrap();
        let before = Repository::open(&dir).unwrap().head().unwrap().target();

        std::fs::write(dir.join("a.txt"), "b\n").unwrap();
        stage_all(&dir);
        let config = SigningConfig {
            mode: signing::SigningMode::Ssh,
            key: String::new(),
        };
        assert!(commit_staged(dir.to_str().unwrap(), "second", Some(&config)).is_err());

        let repo = Repository::open(&dir).unwrap();
        assert_eq!(repo.head().unwrap().target(), before);
        let mut file = std::fs::File::create(dir.join("marker")).unwrap();
        file.write_all(b"reached").unwrap();
        drop(file);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unchanged_index_is_refused() {
        let dir = scratch("empty-index");
        let repo = init_repo(&dir);
        std::fs::write(dir.join("a.txt"), "a\n").unwrap();
        drop(repo);
        stage_all(&dir);
        commit_staged(dir.to_str().unwrap(), "initial", None).unwrap();

        let before = Repository::open(&dir).unwrap().head().unwrap().target();
        assert_eq!(
            commit_staged(dir.to_str().unwrap(), "nothing new", None).unwrap_err(),
            "Stage the changes you want to commit first."
        );
        assert_eq!(
            Repository::open(&dir).unwrap().head().unwrap().target(),
            before
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unstaged_worktree_changes_are_not_committed() {
        let dir = scratch("index-only");
        let repo = init_repo(&dir);
        std::fs::write(dir.join("a.txt"), "a\n").unwrap();
        drop(repo);
        stage_all(&dir);
        commit_staged(dir.to_str().unwrap(), "initial", None).unwrap();

        std::fs::write(dir.join("a.txt"), "changed\n").unwrap();
        assert_eq!(
            commit_staged(dir.to_str().unwrap(), "should fail", None).unwrap_err(),
            "Stage the changes you want to commit first."
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
