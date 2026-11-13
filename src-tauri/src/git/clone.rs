use git2::build::RepoBuilder;
use git2::FetchOptions;
use std::path::Path;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use super::credentials;
use super::proxy::ProxyConfig;
use super::repo::{open_repo, RepoInfo};

/// Reports clone progress as `(received, total)` objects/bytes. The callback is
/// optional so tests can clone without a frontend.
pub type Progress = Arc<dyn Fn(usize, usize) + Send + Sync>;

/// Accepts the transports the app can authenticate: HTTPS (GitHub token),
/// SSH (agent key) and `file://` for local repositories.
fn validate_url(url: &str) -> Result<(), String> {
    let lower = url.to_ascii_lowercase();
    let supported = ["https://", "http://", "ssh://", "file://"]
        .iter()
        .any(|scheme| lower.starts_with(scheme));
    // `git@host:owner/repo` has no scheme but is still SSH.
    let scp_like = !url.contains("://")
        && url
            .split_once('@')
            .is_some_and(|(_, rest)| rest.contains(':'));

    if !supported && !scp_like {
        return Err("Enter an HTTPS or SSH repository URL.".to_string());
    }
    if url.chars().any(char::is_whitespace) {
        return Err("Repository URLs cannot contain spaces.".to_string());
    }
    if let Some((scheme, rest)) = url.split_once("://") {
        let is_http = matches!(scheme.to_ascii_lowercase().as_str(), "http" | "https");
        let authority = rest.split('/').next().unwrap_or("");
        if is_http && authority.contains('@') {
            return Err(
                "Remove the username or password from the URL; Knitt uses your signed-in account."
                    .to_string(),
            );
        }
    }
    Ok(())
}

fn validate_destination(destination: &str) -> Result<(), String> {
    let path = Path::new(destination);
    if path.exists() {
        if !path.is_dir() {
            return Err("The destination is a file, not a folder.".to_string());
        }
        let mut entries =
            std::fs::read_dir(path).map_err(|_| "Could not read the destination.".to_string())?;
        if entries.next().is_some() {
            return Err("Choose an empty folder or a new one.".to_string());
        }
    }
    Ok(())
}

/// Clones `url` into `destination`. `branch` selects a single branch; when it
/// is `None` the remote's default branch is used.
pub fn clone_repo(
    url: &str,
    destination: &str,
    branch: Option<&str>,
    proxy: &ProxyConfig,
    progress: Option<Progress>,
) -> Result<RepoInfo, String> {
    let url = url.trim();
    let destination = destination.trim();
    if url.is_empty() {
        return Err("Enter a repository URL.".to_string());
    }
    if destination.is_empty() {
        return Err("Choose a destination folder.".to_string());
    }
    validate_url(url)?;
    let branch = branch.map(str::trim).filter(|name| !name.is_empty());
    if let Some(name) = branch
        && !crate::config::settings::is_valid_branch_name(name)
    {
        return Err("Enter a valid branch name.".to_string());
    }

    let target = Path::new(destination);
    validate_destination(destination)?;
    let created_target = !target.exists();
    if let Some(parent) = target.parent()
        && !parent.as_os_str().is_empty()
    {
        std::fs::create_dir_all(parent)
            .map_err(|_| "Could not create the destination folder.".to_string())?;
    }

    let mut callbacks = credentials::make_callbacks();
    if let Some(progress) = progress {
        let last_received = AtomicUsize::new(usize::MAX);
        callbacks.transfer_progress(move |stats| {
            let received = stats.received_objects();
            if last_received.swap(received, Ordering::Relaxed) != received {
                progress(received, stats.total_objects());
            }
            true
        });
    }

    let mut options = FetchOptions::new();
    options.remote_callbacks(callbacks);
    options.proxy_options(proxy.options()?);

    let mut builder = RepoBuilder::new();
    builder.fetch_options(options);
    if let Some(name) = branch {
        builder.branch(name);
    }

    match builder.clone(url, target) {
        Ok(repo) => {
            drop(repo);
            open_repo(destination)
        }
        Err(error) => {
            // The destination was created for this clone, so a failed clone
            // must not leave an empty or partial folder behind.
            if created_target {
                let _ = std::fs::remove_dir_all(target);
            }
            Err(error_message(&error))
        }
    }
}

/// Turns libgit2's error into something safe to show. Raw errors can contain
/// the URL, filesystem paths or transport details.
fn error_message(error: &git2::Error) -> String {
    match error.code() {
        git2::ErrorCode::Auth => {
            "Authentication failed. Sign in to GitHub or check your SSH key.".to_string()
        }
        git2::ErrorCode::NotFound => "Repository not found.".to_string(),
        _ => match error.class() {
            git2::ErrorClass::Net | git2::ErrorClass::Http => {
                "Could not reach the repository. Check the URL, network and proxy.".to_string()
            }
            git2::ErrorClass::Ssh => {
                "SSH authentication failed. Check that your key is loaded in the agent.".to_string()
            }
            git2::ErrorClass::Reference => {
                "That branch does not exist in the remote.".to_string()
            }
            _ => "Could not clone the repository.".to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Repository;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn scratch(label: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "knitt-clone-{}-{}-{}",
            label,
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    fn source_repo(dir: &Path) -> git2::Repository {
        std::fs::create_dir_all(dir).unwrap();
        let repo = Repository::init(dir).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
        std::fs::write(dir.join("readme.txt"), "hello\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("readme.txt")).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        {
            let tree = repo.find_tree(tree_id).unwrap();
            let sig = repo.signature().unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
                .unwrap();
        }
        repo
    }

    fn file_url(path: &Path) -> String {
        format!("file://{}", path.to_str().unwrap())
    }

    #[test]
    fn clones_a_local_repository() {
        let source = scratch("source");
        source_repo(&source);
        let destination = scratch("dest");

        let info = clone_repo(
            &file_url(&source),
            destination.to_str().unwrap(),
            None,
            &ProxyConfig::default(),
            None,
        )
        .unwrap();

        assert_eq!(info.name, destination.file_name().unwrap().to_str().unwrap());
        assert!(
            destination.join("readme.txt").is_file(),
            "worktree should be checked out"
        );
        let _ = std::fs::remove_dir_all(&source);
        let _ = std::fs::remove_dir_all(&destination);
    }

    #[test]
    fn clones_a_specific_branch() {
        let source = scratch("branch-source");
        let repo = source_repo(&source);
        {
            let head = repo.head().unwrap().peel_to_commit().unwrap();
            repo.branch("release", &head, false).unwrap();
        }
        drop(repo);
        let destination = scratch("branch-dest");

        let info = clone_repo(
            &file_url(&source),
            destination.to_str().unwrap(),
            Some("release"),
            &ProxyConfig::default(),
            None,
        )
        .unwrap();

        assert_eq!(info.branch.as_deref(), Some("release"));
        let _ = std::fs::remove_dir_all(&source);
        let _ = std::fs::remove_dir_all(&destination);
    }

    #[test]
    fn refuses_a_non_empty_destination() {
        let source = scratch("occupied-source");
        source_repo(&source);
        let destination = scratch("occupied-dest");
        std::fs::create_dir_all(&destination).unwrap();
        std::fs::write(destination.join("existing.txt"), "keep me").unwrap();

        assert_eq!(
            clone_repo(
                &file_url(&source),
                destination.to_str().unwrap(),
                None,
                &ProxyConfig::default(),
                None,
            )
            .unwrap_err(),
            "Choose an empty folder or a new one."
        );
        assert!(destination.join("existing.txt").is_file());
        let _ = std::fs::remove_dir_all(&source);
        let _ = std::fs::remove_dir_all(&destination);
    }

    #[test]
    fn rejects_unsupported_urls_and_schemes() {
        let destination = scratch("bad-url");
        for url in ["", "ftp://example.com/repo.git", "example.com/repo.git"] {
            assert!(clone_repo(
                url,
                destination.to_str().unwrap(),
                None,
                &ProxyConfig::default(),
                None,
            )
            .is_err());
        }
        assert_eq!(
            clone_repo(
                "https://user:pass@example.com/repo.git",
                destination.to_str().unwrap(),
                None,
                &ProxyConfig::default(),
                None,
            )
            .unwrap_err(),
            "Remove the username or password from the URL; Knitt uses your signed-in account."
        );
        assert!(!destination.exists(), "validation must not create the folder");
    }

    #[test]
    fn rejects_invalid_branch_names() {
        let destination = scratch("bad-branch");
        assert_eq!(
            clone_repo(
                "https://example.com/repo.git",
                destination.to_str().unwrap(),
                Some("bad name"),
                &ProxyConfig::default(),
                None,
            )
            .unwrap_err(),
            "Enter a valid branch name."
        );
        assert!(!destination.exists());
    }

    #[test]
    fn failed_clone_removes_the_folder_it_created() {
        let destination = scratch("missing-remote");
        let result = clone_repo(
            &file_url(Path::new("/nonexistent/knitt-clone-source")),
            destination.to_str().unwrap(),
            None,
            &ProxyConfig::default(),
            None,
        );
        assert!(result.is_err());
        assert!(!destination.exists(), "a failed clone should clean up");
    }
}
