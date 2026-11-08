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
        .map_err(|_| "Could not update your global Git identity.".to_string())?;
    Ok(true)
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
}
