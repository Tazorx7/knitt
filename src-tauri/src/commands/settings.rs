use crate::config::autostart;
use crate::config::settings::{self, Settings};
use crate::git::identity::{self, GitIdentity, RepositoryIdentity};
use tauri::command;

#[command]
pub fn settings_get() -> Result<Settings, String> {
    let mut settings =
        Settings::load().map_err(|_| "Could not load settings.".to_string())?;
    // The desktop entry is the source of truth for autostart: it may have been
    // removed outside Knitt.
    settings.autostart = autostart::is_enabled();
    Ok(settings)
}

#[command]
pub fn settings_save(mut settings: Settings) -> Result<Settings, String> {
    // `last_repository_path` is tracked by the backend when a repository opens;
    // it is not a UI control, so a stale copy from the frontend must not clear
    // it.
    if let Ok(current) = Settings::load() {
        settings.last_repository_path = current.last_repository_path;
    }
    settings
        .save()
        .map_err(|_| "Could not save settings.".to_string())?;
    // Re-read so the UI receives the normalized values (clamped tab width,
    // invalid branch replaced, blank strings dropped) rather than its draft.
    let mut stored =
        Settings::load().map_err(|_| "Could not load settings.".to_string())?;
    stored.autostart = autostart::is_enabled();
    Ok(stored)
}

#[command]
pub fn settings_git_identity() -> GitIdentity {
    identity::read_global()
}

/// Write the global commit identity. `None` and blank values are ignored, so
/// this can never erase an existing global value.
#[command]
pub fn settings_set_git_identity(
    name: Option<String>,
    email: Option<String>,
) -> Result<GitIdentity, String> {
    if let Some(email) = email.as_deref() {
        let email = email.trim();
        if !email.is_empty() && (!email.contains('@') || email.contains(char::is_whitespace)) {
            return Err("Enter a valid email address.".to_string());
        }
    }
    if let Some(name) = name.as_deref()
        && name.trim().is_empty()
    {
        return Err("Enter a name.".to_string());
    }
    identity::write_global(name.as_deref(), email.as_deref())
}

#[command]
pub fn settings_validate_branch(name: String) -> Result<(), String> {
    if settings::is_valid_branch_name(&name) {
        Ok(())
    } else {
        Err("Enter a valid branch name.".to_string())
    }
}

#[command]
pub fn settings_repository_identity(path: String) -> Result<RepositoryIdentity, String> {
    identity::read_repository(&path)
}

/// Writes only this repository's local config; global identity is untouched.
#[command]
pub fn settings_set_repository_identity(
    path: String,
    name: String,
    email: String,
) -> Result<RepositoryIdentity, String> {
    let email = email.trim();
    if email.is_empty() || !email.contains('@') || email.contains(char::is_whitespace) {
        return Err("Enter a valid email address.".to_string());
    }
    if name.trim().is_empty() {
        return Err("Enter a name.".to_string());
    }
    identity::write_repository(&path, &name, email)
}

#[command]
pub fn settings_reset_repository_identity(path: String) -> Result<RepositoryIdentity, String> {
    identity::clear_repository(&path)
}

/// Restores defaults. Accounts and their keyring tokens are stored elsewhere
/// and are deliberately left alone; the autostart entry is removed because it
/// is one of the reset preferences.
#[command]
pub fn settings_reset() -> Result<Settings, String> {
    let _ = autostart::set_enabled(false);
    Settings::default()
        .save()
        .map_err(|_| "Could not reset settings.".to_string())?;
    settings_get()
}

/// Writes or removes the desktop autostart entry and returns its actual state.
#[command]
pub fn settings_set_autostart(enabled: bool) -> Result<bool, String> {
    autostart::set_enabled(enabled)
}

/// Removes one repository from the recent list. Never touches the filesystem.
#[command]
pub fn settings_remove_recent(path: String) -> Result<Settings, String> {
    let path = path.trim().to_string();
    let mut settings =
        Settings::load().map_err(|_| "Could not load settings.".to_string())?;
    settings.recent_repositories.retain(|known| known != &path);
    if settings.last_repository_path.as_deref() == Some(path.as_str()) {
        settings.last_repository_path = None;
    }
    settings
        .save()
        .map_err(|_| "Could not save settings.".to_string())?;
    settings_get()
}

/// Clears the recent list and the restore target. Never deletes repositories.
#[command]
pub fn settings_forget_recent() -> Result<Settings, String> {
    let mut settings =
        Settings::load().map_err(|_| "Could not load settings.".to_string())?;
    settings.recent_repositories.clear();
    settings.last_repository_path = None;
    settings
        .save()
        .map_err(|_| "Could not save settings.".to_string())?;
    settings_get()
}
