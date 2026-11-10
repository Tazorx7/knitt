use crate::config::diagnostics::{self, EnvironmentInfo};
use crate::integrations;
use crate::utils::logging;
use tauri::command;

#[command]
pub fn diagnostics_environment() -> EnvironmentInfo {
    diagnostics::environment()
}

/// The report text, so the UI can show it for review before the user saves it.
#[command]
pub fn diagnostics_report() -> String {
    diagnostics::report()
}

#[command]
pub fn diagnostics_export(path: String) -> Result<(), String> {
    diagnostics::export(&path)
}

#[command]
pub fn diagnostics_open_logs() -> Result<(), String> {
    let dir = logging::log_dir().map_err(|_| "Could not find the log folder.".to_string())?;
    integrations::open_file_manager(&dir.display().to_string())
}

#[command]
pub fn diagnostics_clear_logs() -> Result<(), String> {
    diagnostics::clear_logs()
}
