use crate::integrations::{self, DetectedIntegrations, Kind};
use tauri::command;

#[command]
pub fn integrations_detect() -> DetectedIntegrations {
    integrations::detect()
}

/// Returns the resolved absolute path so the UI can show what will run.
#[command]
pub fn integrations_validate(executable: String) -> Result<String, String> {
    integrations::resolve_executable(&executable)
        .map(|path| path.to_string_lossy().to_string())
        .ok_or_else(|| "Choose an application that exists and can be run.".to_string())
}

/// Starts a draft configuration without saving it, so the test reflects what
/// is on screen. Never runs automatically.
#[command]
pub fn integrations_test(
    kind: String,
    executable: String,
    arguments: String,
    target: String,
) -> Result<(), String> {
    let kind = Kind::parse(&kind).ok_or_else(|| "Unknown integration.".to_string())?;
    let target = integrations::existing_directory(&target)?;
    if executable.trim().is_empty() {
        return Err(match kind {
            Kind::Editor => "Choose an external editor first.".to_string(),
            Kind::Terminal => "Choose a terminal first.".to_string(),
        });
    }
    integrations::launch(&executable, &arguments, &target)
}

#[command]
pub fn integrations_open(kind: String, target: String) -> Result<(), String> {
    let kind = Kind::parse(&kind).ok_or_else(|| "Unknown integration.".to_string())?;
    integrations::open(kind, &target)
}

#[command]
pub fn integrations_open_file_manager(target: String) -> Result<(), String> {
    integrations::open_file_manager(&target)
}

/// A safe target for testing an integration before any repository is open.
#[command]
pub fn integrations_default_target() -> Result<String, String> {
    dirs::home_dir()
        .map(|path| path.to_string_lossy().to_string())
        .ok_or_else(|| "Could not find a folder to test with.".to_string())
}
