use git2::Repository;

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

pub fn list_remotes(path: &str) -> Result<Vec<String>, String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let remotes = repo.remotes().map_err(|e| e.to_string())?;
    Ok(remotes.iter().flatten().map(|s| s.to_string()).collect())
}

pub fn add_remote(path: &str, name: &str, url: &str) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    repo.remote(name, url).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn remove_remote(path: &str, name: &str) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    repo.remote_delete(name).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn set_remote_url(path: &str, name: &str, url: &str) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    repo.remote_set_url(name, url).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn fetch_remote(path: &str, name: &str) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let mut remote = repo.find_remote(name).map_err(|e| e.to_string())?;
    remote.fetch(&[] as &[&str], None, None).map_err(|e| e.to_string())?;
    Ok(())
}

pub fn pull_remote(path: &str) -> Result<(), String> {
    fetch_remote(path, "origin")
}

pub fn sync_fork(path: &str) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let mut remote = repo.find_remote("upstream").map_err(|e| e.to_string())?;
    remote.fetch(&[] as &[&str], None, None).map_err(|e| e.to_string())?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::parse_github_owner_repo;

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
