use crate::system::uninstall::{self, UninstallOutcome, UninstallPlan};
use crate::utils::task::blocking;
use tauri::command;

/// What uninstalling would remove, for the confirmation dialog. Read-only.
#[command]
pub async fn app_uninstall_plan() -> Result<UninstallPlan, String> {
    blocking(|| Ok(uninstall::plan())).await
}

/// Removes the install and the app's data. `password` is the sudo password
/// typed in the dialog; `None` means a cached credential should cover it. The
/// caller quits the app afterwards.
#[command]
pub async fn app_uninstall(password: Option<String>) -> Result<UninstallOutcome, String> {
    blocking(move || uninstall::run(password.as_deref())).await
}

#[command]
pub fn app_quit(app: tauri::AppHandle) {
    app.exit(0);
}
