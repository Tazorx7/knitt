use git2::{FetchOptions, Repository};
use serde::Serialize;

use super::{credentials, proxy::ProxyConfig};

/// One configured remote, as shown in the Remotes panel.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RemoteInfo {
    pub name: String,
    /// The fetch URL, when the remote has one.
    pub url: Option<String>,
}

fn open(path: &str) -> Result<Repository, String> {
    Repository::open(path).map_err(|_| "Could not open the repository.".to_string())
}

/// Remote names become ref and config keys, so they must be a single path-like
/// word. Anything that could address a file, an option or another ref is
/// rejected before libgit2 sees it.
fn check_name(name: &str) -> Result<(), String> {
    let trimmed = name.trim();
    let invalid = trimmed.is_empty()
        || trimmed != name
        || trimmed.starts_with('-')
        || trimmed.starts_with('/')
        || trimmed.ends_with('/')
        || trimmed.contains("..")
        || trimmed.contains(char::is_whitespace)
        || trimmed.contains(|c: char| c.is_control())
        || trimmed.contains(['~', '^', ':', '?', '*', '[', '\\']);
    if invalid {
        return Err("That remote name is not valid.".to_string());
    }
    Ok(())
}

/// Refuse URLs that carry a password, the same rule the clone dialog applies:
/// credentials belong to the credential store, not to a file in `.git/config`.
fn check_url(url: &str) -> Result<(), String> {
    let trimmed = url.trim();
    if trimmed.is_empty() {
        return Err("Enter a remote URL.".to_string());
    }
    if trimmed.contains(char::is_whitespace) || trimmed.contains(|c: char| c.is_control()) {
        return Err("That remote URL is not valid.".to_string());
    }
    // scheme://user:password@host/... — a colon in the user info means a secret.
    if let Some(rest) = trimmed.split_once("://").map(|(_, rest)| rest)
        && let Some((authority, _)) = rest.split_once('@')
        && authority.contains(':')
    {
        return Err("Remove the password from the URL and sign in instead.".to_string());
    }
    Ok(())
}

/// Parse a git remote URL into a GitHub (owner, repo) pair.
fn parse_github_owner_repo(url: &str) -> Result<(String, String), String> {
    // Normalize to an "owner/repo" path, stripping scheme and host.
    let path = if let Some(stripped) = url
        .strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
    {
        // https://host/owner/repo(.git)
        stripped.split_once('/').map(|x| x.1).unwrap_or("")
    } else if let Some(stripped) = url.strip_prefix("ssh://") {
        // ssh://git@host/owner/repo(.git)
        stripped.split_once('/').map(|x| x.1).unwrap_or("")
    } else if let Some(stripped) = url.strip_prefix("git@") {
        // git@host:owner/repo(.git)
        stripped.split_once(':').map(|x| x.1).unwrap_or("")
    } else {
        url
    };
    let path = path.trim_end_matches(".git");
    let parts: Vec<&str> = path.split('/').filter(|s| !s.is_empty()).collect();
    if parts.len() < 2 {
        return Err(format!("could not parse owner/repo from remote URL: {url}"));
    }
    let repo = parts[parts.len() - 1].to_string();
    let owner = parts[parts.len() - 2].to_string();
    Ok((owner, repo))
}

/// Derive the GitHub (owner, repo) pair from the repo's `origin` remote.
pub fn get_github_owner_repo(path: &str) -> Result<(String, String), String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let remote = repo.find_remote("origin").map_err(|e| e.to_string())?;
    let url = remote
        .url()
        .ok_or_else(|| "origin remote has no URL".to_string())?
        .to_string();
    parse_github_owner_repo(&url)
}

/// Every configured remote with its fetch URL, sorted by name.
pub fn list_remotes(path: &str) -> Result<Vec<RemoteInfo>, String> {
    let repo = open(path)?;
    let names = repo
        .remotes()
        .map_err(|_| "Could not read the remotes.".to_string())?;
    let mut remotes: Vec<RemoteInfo> = names
        .iter()
        .flatten()
        .map(|name| {
            let url = repo
                .find_remote(name)
                .ok()
                .and_then(|remote| remote.url().map(str::to_string));
            RemoteInfo {
                name: name.to_string(),
                url,
            }
        })
        .collect();
    remotes.sort_by(|a, b| a.name.cmp(&b.name));
    Ok(remotes)
}

pub fn add_remote(path: &str, name: &str, url: &str) -> Result<(), String> {
    check_name(name)?;
    check_url(url)?;
    let repo = open(path)?;
    if repo.find_remote(name).is_ok() {
        return Err("A remote with that name already exists.".to_string());
    }
    repo.remote(name, url)
        .map_err(|_| "Could not add that remote.".to_string())?;
    Ok(())
}

pub fn remove_remote(path: &str, name: &str) -> Result<(), String> {
    check_name(name)?;
    let repo = open(path)?;
    repo.find_remote(name)
        .map_err(|_| "That remote does not exist.".to_string())?;
    repo.remote_delete(name)
        .map_err(|_| "Could not remove that remote.".to_string())?;
    Ok(())
}

pub fn set_remote_url(path: &str, name: &str, url: &str) -> Result<(), String> {
    check_name(name)?;
    check_url(url)?;
    let repo = open(path)?;
    repo.find_remote(name)
        .map_err(|_| "That remote does not exist.".to_string())?;
    repo.remote_set_url(name, url)
        .map_err(|_| "Could not change that remote's URL.".to_string())?;
    Ok(())
}

/// Fetches the `upstream` remote of a fork.
pub fn sync_fork(path: &str, proxy: &ProxyConfig) -> Result<(), String> {
    let repo = open(path)?;
    let mut remote = repo
        .find_remote("upstream")
        .map_err(|_| "This repository has no upstream remote.".to_string())?;
    let mut options = FetchOptions::new();
    options.remote_callbacks(credentials::make_callbacks());
    options.proxy_options(proxy.options()?);
    remote
        .fetch(&[] as &[&str], Some(&mut options), None)
        .map_err(|_| "Could not fetch from the upstream remote.".to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn scratch(name: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "knitt-remotes-{name}-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Repository::init(&dir).unwrap();
        dir
    }

    fn path_of(dir: &Path) -> &str {
        dir.to_str().unwrap()
    }

    #[test]
    fn adds_lists_changes_and_removes_a_remote() {
        let dir = scratch("roundtrip");
        add_remote(path_of(&dir), "origin", "https://example.com/a/b.git").unwrap();
        add_remote(path_of(&dir), "upstream", "git@example.com:a/b.git").unwrap();

        let remotes = list_remotes(path_of(&dir)).unwrap();
        assert_eq!(
            remotes,
            vec![
                RemoteInfo {
                    name: "origin".to_string(),
                    url: Some("https://example.com/a/b.git".to_string()),
                },
                RemoteInfo {
                    name: "upstream".to_string(),
                    url: Some("git@example.com:a/b.git".to_string()),
                },
            ]
        );

        set_remote_url(path_of(&dir), "origin", "https://example.com/c/d.git").unwrap();
        assert_eq!(
            list_remotes(path_of(&dir)).unwrap()[0].url.as_deref(),
            Some("https://example.com/c/d.git")
        );

        remove_remote(path_of(&dir), "upstream").unwrap();
        assert_eq!(list_remotes(path_of(&dir)).unwrap().len(), 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuses_duplicate_and_missing_remotes() {
        let dir = scratch("duplicate");
        add_remote(path_of(&dir), "origin", "https://example.com/a/b.git").unwrap();

        assert_eq!(
            add_remote(path_of(&dir), "origin", "https://example.com/a/b.git").unwrap_err(),
            "A remote with that name already exists."
        );
        assert_eq!(
            set_remote_url(path_of(&dir), "nope", "https://example.com/a/b.git").unwrap_err(),
            "That remote does not exist."
        );
        assert_eq!(
            remove_remote(path_of(&dir), "nope").unwrap_err(),
            "That remote does not exist."
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuses_bad_names_and_urls() {
        let dir = scratch("validation");
        for name in ["", " ", "-x", "a b", "../x", "a:b", "a?b"] {
            assert_eq!(
                add_remote(path_of(&dir), name, "https://example.com/a/b.git").unwrap_err(),
                "That remote name is not valid.",
                "{name:?} should be refused"
            );
        }
        for url in [
            "",
            "  ",
            "https://user:secret@example.com/a/b.git",
            "https://user@example.com/a/b.git", // no secret, allowed below
        ] {
            let result = add_remote(path_of(&dir), "origin", url);
            if url.contains("secret") || url.trim().is_empty() {
                assert!(result.is_err(), "{url:?} should be refused");
            } else {
                assert!(result.is_ok(), "{url:?} should be accepted");
                remove_remote(path_of(&dir), "origin").unwrap();
            }
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn list_is_empty_for_a_repository_without_remotes() {
        let dir = scratch("empty");
        assert!(list_remotes(path_of(&dir)).unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parses_https_url() {
        assert_eq!(
            parse_github_owner_repo("https://github.com/Tazorx7/knitt.git").unwrap(),
            ("Tazorx7".to_string(), "knitt".to_string())
        );
    }

    #[test]
    fn parses_https_url_without_dot_git() {
        assert_eq!(
            parse_github_owner_repo("https://github.com/Tazorx7/knitt").unwrap(),
            ("Tazorx7".to_string(), "knitt".to_string())
        );
    }

    #[test]
    fn parses_scp_like_ssh_url() {
        assert_eq!(
            parse_github_owner_repo("git@github.com:Tazorx7/knitt.git").unwrap(),
            ("Tazorx7".to_string(), "knitt".to_string())
        );
    }

    #[test]
    fn parses_ssh_url() {
        assert_eq!(
            parse_github_owner_repo("ssh://git@github.com/Tazorx7/knitt.git").unwrap(),
            ("Tazorx7".to_string(), "knitt".to_string())
        );
    }

    #[test]
    fn parses_enterprise_https_url() {
        assert_eq!(
            parse_github_owner_repo("https://git.example.com/org/repo.git").unwrap(),
            ("org".to_string(), "repo".to_string())
        );
    }

    #[test]
    fn rejects_unparseable_url() {
        assert!(parse_github_owner_repo("https://github.com/onlyowner").is_err());
    }
}
