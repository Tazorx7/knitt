use crate::config::crash::{self, CrashReport};
use tauri::command;

/// Saved panic reports, newest first. Empty when crash reporting is off.
#[command]
pub fn crashes_list() -> Vec<CrashReport> {
    crash::list()
}

/// Report text for review or copying. `file` must name a listed report.
#[command]
pub fn crashes_read(file: String) -> Result<String, String> {
    crash::read(&file)
}

/// Saves one report to a path chosen in the native save dialog.
#[command]
pub fn crashes_export(path: String, file: String) -> Result<(), String> {
    crash::export(&path, &file)
}

#[command]
pub fn crashes_clear() -> Result<(), String> {
    crash::clear()
}
