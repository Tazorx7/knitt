use crate::system::uninstall::{self, UninstallOutcome, UninstallPlan};
use crate::utils::task::blocking;
use tauri::command;

/// What uninstalling would remove, for the confirmation dialog. Read-only.
#[command]
pub async fn app_uninstall_plan() -> Result<UninstallPlan, String> {
    blocking(|| Ok(uninstall::plan())).await
}

/// Removes the install and the app's data. The caller quits the app afterwards.
#[command]
pub async fn app_uninstall() -> Result<UninstallOutcome, String> {
    blocking(uninstall::run).await
}

#[command]
pub fn app_quit(app: tauri::AppHandle) {
    app.exit(0);
}
