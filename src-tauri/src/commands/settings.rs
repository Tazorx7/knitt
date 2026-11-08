use crate::config::settings::{self, Settings};
use crate::git::identity::{self, GitIdentity};
use tauri::command;

#[command]
pub fn settings_get() -> Result<Settings, String> {
    Settings::load().map_err(|_| "Could not load settings.".to_string())
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
    Settings::load().map_err(|_| "Could not load settings.".to_string())
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
