use crate::config::paths;
use serde::{Deserialize, Serialize};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

/// Current on-disk settings schema version.
///
/// Bump this whenever a field is added, removed or changes meaning, and add a
/// matching step to [`Settings::migrate`].
pub const CURRENT_SCHEMA_VERSION: u32 = 5;

const THEMES: [&str; 3] = ["light", "dark", "system"];
const PULL_STRATEGIES: [&str; 3] = ["merge", "rebase", "ff-only"];
const PUSH_MODES: [&str; 2] = ["current", "upstream"];
const MIN_TAB_WIDTH: u8 = 1;
const MAX_TAB_WIDTH: u8 = 16;
const EDITOR_ARGUMENTS: &str = "%TARGET_PATH%";
const TERMINAL_ARGUMENTS: &str = "--working-directory=%TARGET_PATH%";

/// An external application Knitt may launch.
///
/// The executable is stored as a resolved absolute path so the same launcher
/// is used even if `PATH` changes later. `arguments` is an argument vector
/// template, never a shell string: it is split into individual arguments and
/// launched directly.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(default)]
pub struct IntegrationConfig {
    pub executable: Option<String>,
    pub arguments: String,
}

/// Application-wide preferences.
///
/// Invariants:
/// - Secrets never live here. Tokens are in the OS credential store, and Git
///   identity lives in the user's global Git configuration.
/// - `#[serde(default)]` lets older files keep loading after fields are added,
///   instead of failing the whole file with a missing-field error.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub schema_version: u32,
    pub theme: String,
    pub default_branch: String,
    pub restore_last_repository: bool,
    pub default_repository_directory: Option<String>,
    /// Last repository that opened successfully, used for restore-on-startup.
    /// Internal: not exposed as a control in the settings UI.
    pub last_repository_path: Option<String>,
    pub diff_tab_width: u8,
    /// Global shortcuts (Ctrl+O, Ctrl+Enter, ...). Typing always works.
    pub keyboard_shortcuts: bool,
    /// Force reduced motion regardless of the desktop preference.
    pub reduce_motion: bool,
    /// How pulled commits are integrated: `merge`, `rebase` or `ff-only`.
    pub pull_strategy: String,
    /// `current` pushes to a same-named branch; `upstream` uses the tracked branch.
    pub push_mode: String,
    /// Prune remote-tracking refs that no longer exist on the remote.
    pub fetch_prune: bool,
    pub external_editor: IntegrationConfig,
    pub terminal: IntegrationConfig,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            theme: "system".to_string(),
            default_branch: "main".to_string(),
            restore_last_repository: true,
            default_repository_directory: None,
            last_repository_path: None,
            diff_tab_width: 4,
            keyboard_shortcuts: true,
            reduce_motion: false,
            pull_strategy: "merge".to_string(),
            push_mode: "current".to_string(),
            fetch_prune: false,
            external_editor: IntegrationConfig {
                executable: None,
                arguments: EDITOR_ARGUMENTS.to_string(),
            },
            terminal: IntegrationConfig {
                executable: None,
                arguments: TERMINAL_ARGUMENTS.to_string(),
            },
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
            // v0/v1 carried `theme` and inert Git identity fields; the new
            // fields simply adopt their defaults, so no field rewrite is due.
            self.schema_version = CURRENT_SCHEMA_VERSION;
        }
    }

    /// Coerces invalid values to safe defaults instead of rejecting the file.
    /// Saves run the same pass, so bad UI input cannot be persisted either.
    fn normalize(&mut self) {
        if !THEMES.contains(&self.theme.as_str()) {
            self.theme = "system".to_string();
        }
        if !is_valid_branch_name(&self.default_branch) {
            self.default_branch = "main".to_string();
        }
        self.diff_tab_width = self.diff_tab_width.clamp(MIN_TAB_WIDTH, MAX_TAB_WIDTH);
        if !PULL_STRATEGIES.contains(&self.pull_strategy.as_str()) {
            self.pull_strategy = "merge".to_string();
        }
        if !PUSH_MODES.contains(&self.push_mode.as_str()) {
            self.push_mode = "current".to_string();
        }
        self.default_repository_directory = clean(self.default_repository_directory.take());
        self.last_repository_path = clean(self.last_repository_path.take());
        normalize_integration(&mut self.external_editor, EDITOR_ARGUMENTS);
        normalize_integration(&mut self.terminal, TERMINAL_ARGUMENTS);
    }
}

fn normalize_integration(config: &mut IntegrationConfig, default_arguments: &str) {
    config.executable = clean(config.executable.take());
    config.arguments = config.arguments.trim().to_string();
    if config.arguments.is_empty() {
        config.arguments = default_arguments.to_string();
    }
}

/// Validates a short branch name (`main`, `release/2.0`) using Git's own rules
/// rather than a hand-rolled pattern.
pub fn is_valid_branch_name(name: &str) -> bool {
    let name = name.trim();
    !name.is_empty()
        && !name.starts_with('-')
        && git2::Reference::is_valid_name(&format!("refs/heads/{name}"))
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
        assert_eq!(settings.default_branch, "main");
        assert!(settings.restore_last_repository);
        assert_eq!(settings.diff_tab_width, 4);
        assert_eq!(settings.pull_strategy, "merge");
        assert_eq!(settings.push_mode, "current");
        assert!(!settings.fetch_prune);
        assert!(settings.keyboard_shortcuts);
        assert!(!settings.reduce_motion);
        assert_eq!(settings.external_editor.executable, None);
        assert_eq!(settings.external_editor.arguments, EDITOR_ARGUMENTS);
        assert_eq!(settings.terminal.arguments, TERMINAL_ARGUMENTS);
    }

    #[test]
    fn roundtrip_preserves_values() {
        let path = temp_path();
        let settings = Settings {
            theme: "dark".to_string(),
            default_branch: "trunk".to_string(),
            restore_last_repository: false,
            diff_tab_width: 2,
            keyboard_shortcuts: false,
            reduce_motion: true,
            pull_strategy: "rebase".to_string(),
            push_mode: "upstream".to_string(),
            fetch_prune: true,
            ..Settings::default()
        };
        settings.save_to(&path).unwrap();

        let loaded = Settings::load_from(&path).unwrap();
        assert_eq!(loaded.theme, "dark");
        assert_eq!(loaded.default_branch, "trunk");
        assert!(!loaded.restore_last_repository);
        assert_eq!(loaded.diff_tab_width, 2);
        assert!(!loaded.keyboard_shortcuts);
        assert!(loaded.reduce_motion);
        assert_eq!(loaded.pull_strategy, "rebase");
        assert_eq!(loaded.push_mode, "upstream");
        assert!(loaded.fetch_prune);
    }

    #[test]
    fn missing_fields_adopt_defaults() {
        let path = temp_path();
        std::fs::write(&path, r#"{"theme":"dark"}"#).unwrap();

        let loaded = Settings::load_from(&path).unwrap();
        assert_eq!(loaded.theme, "dark");
        assert_eq!(loaded.schema_version, CURRENT_SCHEMA_VERSION);
        assert_eq!(loaded.default_branch, "main");
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
        // Accessibility fields added in v4 adopt their defaults.
        assert!(loaded.keyboard_shortcuts);
        assert!(!loaded.reduce_motion);
    }

    #[test]
    fn invalid_sync_strategy_falls_back_to_defaults() {
        let path = temp_path();
        std::fs::write(
            &path,
            r#"{"pull_strategy":"octopus","push_mode":"matching"}"#,
        )
        .unwrap();

        let loaded = Settings::load_from(&path).unwrap();
        assert_eq!(loaded.pull_strategy, "merge");
        assert_eq!(loaded.push_mode, "current");
    }

    #[test]
    fn invalid_branch_falls_back_to_main() {
        let path = temp_path();
        std::fs::write(&path, r#"{"default_branch":"bad name"}"#).unwrap();
        assert_eq!(Settings::load_from(&path).unwrap().default_branch, "main");
    }

    #[test]
    fn tab_width_is_clamped() {
        let path = temp_path();
        std::fs::write(&path, r#"{"diff_tab_width":0}"#).unwrap();
        assert_eq!(Settings::load_from(&path).unwrap().diff_tab_width, 1);

        std::fs::write(&path, r#"{"diff_tab_width":200}"#).unwrap();
        assert_eq!(Settings::load_from(&path).unwrap().diff_tab_width, 16);
    }

    #[test]
    fn integration_values_are_trimmed_and_defaulted() {
        let path = temp_path();
        std::fs::write(
            &path,
            r#"{"external_editor":{"executable":"  /usr/bin/code  ","arguments":"   "},"terminal":{"arguments":""}}"#,
        )
        .unwrap();

        let loaded = Settings::load_from(&path).unwrap();
        assert_eq!(
            loaded.external_editor.executable.as_deref(),
            Some("/usr/bin/code")
        );
        assert_eq!(loaded.external_editor.arguments, EDITOR_ARGUMENTS);
        assert_eq!(loaded.terminal.arguments, TERMINAL_ARGUMENTS);
        assert_eq!(loaded.terminal.executable, None);
    }

    #[test]
    fn known_branch_names_validate() {
        assert!(is_valid_branch_name("main"));
        assert!(is_valid_branch_name("release/2.0"));
        assert!(!is_valid_branch_name(""));
        assert!(!is_valid_branch_name("bad name"));
        assert!(!is_valid_branch_name("-x"));
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
