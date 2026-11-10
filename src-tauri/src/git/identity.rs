use git2::Config;
use serde::Serialize;
use std::path::PathBuf;

/// The global commit identity and the file it was read from.
#[derive(Debug, Clone, Serialize)]
pub struct GitIdentity {
    pub name: Option<String>,
    pub email: Option<String>,
    /// Global config file Knitt would write to, for display in the Git section.
    pub config_path: Option<String>,
}

/// The effective global identity (`user.name` / `user.email`).
///
/// Reads through `open_default`, so XDG and system values are included; local
/// repository values are not, because no repository is involved.
pub fn read_global() -> GitIdentity {
    let config_path = global_config_path().map(|p| p.display().to_string());
    let (name, email) = match Config::open_default() {
        Ok(cfg) => (
            cfg.get_string("user.name").ok(),
            cfg.get_string("user.email").ok(),
        ),
        Err(_) => (None, None),
    };
    GitIdentity {
        name,
        email,
        config_path,
    }
}

/// Update only the keys the caller changed.
///
/// `None` and blank values mean "leave this key alone", never "delete it", so
/// an empty form field can never erase a global value. Existing values that
/// already match are not rewritten, and unrelated keys are untouched.
pub fn write_global(name: Option<&str>, email: Option<&str>) -> Result<GitIdentity, String> {
    let path = global_config_path()
        .ok_or_else(|| "Could not locate your global Git configuration.".to_string())?;
    let mut cfg = Config::open(&path)
        .map_err(|_| "Could not open your global Git configuration.".to_string())?;

    apply(&mut cfg, name, email)?;
    Ok(read_global())
}

fn apply(cfg: &mut Config, name: Option<&str>, email: Option<&str>) -> Result<(), String> {
    if let Some(name) = name {
        set_if_changed(cfg, "user.name", name)?;
    }
    if let Some(email) = email {
        set_if_changed(cfg, "user.email", email)?;
    }
    Ok(())
}

fn set_if_changed(cfg: &mut Config, key: &str, value: &str) -> Result<bool, String> {
    let value = value.trim();
    if value.is_empty() {
        return Ok(false);
    }
    if cfg.get_string(key).ok().as_deref() == Some(value) {
        return Ok(false);
    }
    cfg.set_str(key, value)
        .map_err(|_| "Could not update the Git identity.".to_string())?;
    Ok(true)
}

/// The effective commit identity for one repository.
///
/// `name`/`email` are what Git will use (local values override global).
/// `local_*` are only the repository's own configuration and are `None` when
/// the value is inherited.
#[derive(Debug, Clone, Serialize)]
pub struct RepositoryIdentity {
    pub name: Option<String>,
    pub email: Option<String>,
    pub local_name: Option<String>,
    pub local_email: Option<String>,
    pub uses_global: bool,
}

/// Reads the identity Git would use for this repository.
pub fn read_repository(path: &str) -> Result<RepositoryIdentity, String> {
    let repo = open_repository(path)?;
    let effective = repo
        .config()
        .map_err(|_| "Could not read the repository configuration.".to_string())?;
    let local = local_config(&repo)?;

    let local_name = local.get_string("user.name").ok();
    let local_email = local.get_string("user.email").ok();

    Ok(RepositoryIdentity {
        name: effective.get_string("user.name").ok(),
        email: effective.get_string("user.email").ok(),
        uses_global: local_name.is_none() && local_email.is_none(),
        local_name,
        local_email,
    })
}

/// Writes `user.name` / `user.email` to the repository's own config only.
/// Global configuration and unrelated keys are untouched.
pub fn write_repository(path: &str, name: &str, email: &str) -> Result<RepositoryIdentity, String> {
    let name = name.trim();
    let email = email.trim();
    if name.is_empty() || email.is_empty() {
        return Err("Enter both a name and an email.".to_string());
    }

    let repo = open_repository(path)?;
    let mut config = local_config(&repo)?;
    set_if_changed(&mut config, "user.name", name)?;
    set_if_changed(&mut config, "user.email", email)?;
    drop(config);

    read_repository(path)
}

/// Removes repository-local identity keys so Git inherits the global identity
/// again. Other repository configuration is preserved.
pub fn clear_repository(path: &str) -> Result<RepositoryIdentity, String> {
    let repo = open_repository(path)?;
    let mut config = local_config(&repo)?;
    for key in ["user.name", "user.email"] {
        if config.get_string(key).is_ok() {
            config
                .remove(key)
                .map_err(|_| "Could not reset the repository identity.".to_string())?;
        }
    }
    drop(config);

    read_repository(path)
}

fn open_repository(path: &str) -> Result<git2::Repository, String> {
    git2::Repository::open(path).map_err(|_| "Could not open that repository.".to_string())
}

/// The repository's own config file. The path comes from the already-opened
/// repository, never from raw user input, so a stale path cannot target
/// another repository's configuration.
fn local_config(repo: &git2::Repository) -> Result<Config, String> {
    Config::open(&repo.path().join("config"))
        .map_err(|_| "Could not open the repository configuration.".to_string())
}

/// `Config::find_global` only returns a path when the file already exists.
/// Falling back to `$HOME/.gitconfig` lets a first-time user create one, which
/// is exactly what `git config --global` does.
fn global_config_path() -> Option<PathBuf> {
    if let Ok(path) = Config::find_global()
        && !path.as_os_str().is_empty()
    {
        return Some(path);
    }
    dirs::home_dir().map(|home| home.join(".gitconfig"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_repo() -> (PathBuf, git2::Repository) {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("knitt-repo-identity-{}-{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let repo = git2::Repository::init(&dir).unwrap();
        (dir, repo)
    }

    fn temp_config() -> (PathBuf, Config) {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!("knitt-identity-{}-{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config");
        let cfg = Config::open(&path).unwrap();
        (path, cfg)
    }

    #[test]
    fn writes_identity_to_an_empty_config() {
        let (path, mut cfg) = temp_config();
        apply(&mut cfg, Some("Ada"), Some("ada@example.com")).unwrap();

        let written = std::fs::read_to_string(&path).unwrap();
        assert!(written.contains("Ada"));
        assert!(written.contains("ada@example.com"));
    }

    #[test]
    fn leaves_unrelated_keys_untouched() {
        let (path, mut cfg) = temp_config();
        cfg.set_str("core.autocrlf", "input").unwrap();
        apply(&mut cfg, Some("Ada"), Some("ada@example.com")).unwrap();

        let written = std::fs::read_to_string(&path).unwrap();
        assert!(written.contains("autocrlf"));
        assert!(written.contains("input"));
        assert!(written.contains("Ada"));
    }

    #[test]
    fn blank_values_never_erase_existing_keys() {
        let (path, mut cfg) = temp_config();
        apply(&mut cfg, Some("Ada"), Some("ada@example.com")).unwrap();
        apply(&mut cfg, Some("   "), Some("")).unwrap();

        let written = std::fs::read_to_string(&path).unwrap();
        assert!(written.contains("Ada"));
        assert!(written.contains("ada@example.com"));
    }

    #[test]
    fn identical_values_do_not_rewrite_the_file() {
        let (path, mut cfg) = temp_config();
        apply(&mut cfg, Some("Ada"), Some("ada@example.com")).unwrap();
        let before = std::fs::read(&path).unwrap();

        apply(&mut cfg, Some("Ada"), Some("ada@example.com")).unwrap();
        assert_eq!(before, std::fs::read(&path).unwrap());
    }

    #[test]
    fn repository_identity_reads_and_writes_local_config() {
        let (dir, _repo) = temp_repo();
        let path = dir.to_str().unwrap();

        let initial = read_repository(path).unwrap();
        assert!(initial.uses_global);
        assert_eq!(initial.local_name, None);
        assert_eq!(initial.local_email, None);

        write_repository(path, "Repo User", "repo@example.com").unwrap();
        let written = read_repository(path).unwrap();
        assert!(!written.uses_global);
        assert_eq!(written.local_name.as_deref(), Some("Repo User"));
        assert_eq!(written.local_email.as_deref(), Some("repo@example.com"));
        assert_eq!(written.name.as_deref(), Some("Repo User"));
        assert_eq!(written.email.as_deref(), Some("repo@example.com"));

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn repository_identity_reset_preserves_unrelated_keys() {
        let (dir, repo) = temp_repo();
        let path = dir.to_str().unwrap();

        write_repository(path, "Repo User", "repo@example.com").unwrap();
        let mut config = local_config(&repo).unwrap();
        config.set_str("core.fsmonitor", "true").unwrap();
        drop(config);

        let reset = clear_repository(path).unwrap();
        assert!(reset.uses_global);
        assert_eq!(reset.local_name, None);
        assert_eq!(reset.local_email, None);

        let config = local_config(&git2::Repository::open(&dir).unwrap()).unwrap();
        assert!(config.get_string("core.fsmonitor").is_ok());
        assert!(config.get_string("user.name").is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn repository_identity_rejects_invalid_input() {
        let (dir, _repo) = temp_repo();
        let path = dir.to_str().unwrap();

        assert!(write_repository(path, " ", "repo@example.com").is_err());
        assert!(write_repository(path, "Repo User", "  ").is_err());
        assert!(read_repository("/nonexistent/knitt-repo").is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
