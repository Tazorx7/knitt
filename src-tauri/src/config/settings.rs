use crate::config::paths;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Current on-disk settings schema version.
///
/// Bump this whenever a field is added, removed or changes meaning, and add a
/// matching step to [`Settings::migrate`].
pub const CURRENT_SCHEMA_VERSION: u32 = 1;

const THEMES: [&str; 3] = ["light", "dark", "system"];

/// Application-wide preferences.
///
/// Invariants:
/// - Secrets never live here. Tokens are in the OS credential store.
/// - `#[serde(default)]` lets older files keep loading after fields are added,
///   instead of failing the whole file with a missing-field error.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub schema_version: u32,
    pub theme: String,
    pub git_user_name: Option<String>,
    pub git_user_email: Option<String>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            theme: "system".to_string(),
            git_user_name: None,
            git_user_email: None,
        }
    }
}

impl Settings {
    fn path() -> io::Result<PathBuf> {
        Ok(paths::config_dir()?.join("settings.json"))
    }

    pub fn load() -> io::Result<Self> {
        Self::load_from(&Self::path()?)
    }

    pub fn save(&self) -> io::Result<()> {
        self.save_to(&Self::path()?)
    }

    fn load_from(path: &Path) -> io::Result<Self> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let data = std::fs::read_to_string(path)?;
        let mut settings: Settings = serde_json::from_str(&data)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        settings.migrate();
        settings.normalize();
        Ok(settings)
    }

    /// Replace the file atomically: write a sibling temp file, flush it to
    /// disk, then rename over the target. A crash can leave a `.tmp` file but
    /// never a half-written `settings.json`.
    fn save_to(&self, path: &Path) -> io::Result<()> {
        let mut value = self.clone();
        value.normalize();

        let dir = path.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "settings path has no parent")
        })?;
        std::fs::create_dir_all(dir)?;

        let data = serde_json::to_vec_pretty(&value)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let tmp = dir.join(format!(".settings.json.{}.tmp", std::process::id()));
        {
            let mut file = std::fs::File::create(&tmp)?;
            restrict_permissions(&file)?;
            file.write_all(&data)?;
            file.sync_all()?;
        }

        if let Err(e) = std::fs::rename(&tmp, path) {
            let _ = std::fs::remove_file(&tmp);
            return Err(e);
        }
        Ok(())
    }

    /// Upgrades older schemas in place. A newer version is left untouched so a
    /// save never downgrades a file written by a later release.
    fn migrate(&mut self) {
        if self.schema_version < CURRENT_SCHEMA_VERSION {
            // No field-level migrations exist yet; v0 is v1 with defaults.
            self.schema_version = CURRENT_SCHEMA_VERSION;
        }
    }

    /// Coerces invalid values to safe defaults instead of rejecting the file.
    /// Saves run the same pass, so bad UI input cannot be persisted either.
    fn normalize(&mut self) {
        if !THEMES.contains(&self.theme.as_str()) {
            self.theme = "system".to_string();
        }
        self.git_user_name = clean(self.git_user_name.take());
        self.git_user_email = clean(self.git_user_email.take());
    }
}

fn clean(value: Option<String>) -> Option<String> {
    value
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

#[cfg(unix)]
fn restrict_permissions(file: &std::fs::File) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    file.set_permissions(std::fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn restrict_permissions(_file: &std::fs::File) -> io::Result<()> {
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn temp_path() -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir =
            std::env::temp_dir().join(format!("knitt-settings-{}-{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir.join("settings.json")
    }

    #[test]
    fn missing_file_returns_defaults() {
        let settings = Settings::load_from(&temp_path()).unwrap();
        assert_eq!(settings.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(settings.theme, "system");
        assert!(settings.git_user_name.is_none());
    }

    #[test]
    fn roundtrip_preserves_values() {
        let path = temp_path();
        let settings = Settings {
            theme: "dark".to_string(),
            git_user_name: Some("Ada".to_string()),
            git_user_email: Some("ada@example.com".to_string()),
            ..Settings::default()
        };
        settings.save_to(&path).unwrap();

        let loaded = Settings::load_from(&path).unwrap();
        assert_eq!(loaded.theme, "dark");
        assert_eq!(loaded.git_user_name.as_deref(), Some("Ada"));
        assert_eq!(loaded.git_user_email.as_deref(), Some("ada@example.com"));
    }

    #[test]
    fn missing_fields_adopt_defaults() {
        let path = temp_path();
        std::fs::write(&path, r#"{"theme":"dark"}"#).unwrap();

        let loaded = Settings::load_from(&path).unwrap();
        assert_eq!(loaded.theme, "dark");
        assert_eq!(loaded.schema_version, CURRENT_SCHEMA_VERSION);
        assert!(loaded.git_user_name.is_none());
    }

    #[test]
    fn unknown_theme_falls_back_to_system() {
        let path = temp_path();
        std::fs::write(&path, r#"{"theme":"neon"}"#).unwrap();
        assert_eq!(Settings::load_from(&path).unwrap().theme, "system");
    }

    #[test]
    fn old_schema_is_upgraded() {
        let path = temp_path();
        std::fs::write(&path, r#"{"schema_version":0,"theme":"light"}"#).unwrap();

        let loaded = Settings::load_from(&path).unwrap();
        assert_eq!(loaded.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(loaded.theme, "light");
    }

    #[test]
    fn save_leaves_no_temp_file_and_restricts_permissions() {
        let path = temp_path();
        Settings::default().save_to(&path).unwrap();

        let dir = path.parent().unwrap();
        let leftovers = std::fs::read_dir(dir)
            .unwrap()
            .filter_map(|entry| entry.ok())
            .filter(|entry| entry.file_name().to_string_lossy().ends_with(".tmp"))
            .count();
        assert_eq!(leftovers, 0);

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600);
        }
    }
}
