//! Checking for and applying updates.
//!
//! Only the AppImage can replace itself: the `.deb` and `.rpm` are owned by the
//! package manager, which is where their updates come from. A source build is
//! updated by whoever built it. Every command therefore reports why no update
//! can be applied instead of failing silently.

use serde::Serialize;
use tauri::{AppHandle, Emitter};
use tauri_plugin_updater::UpdaterExt;
use tracing::warn;

/// What the UI needs to decide whether to show an update prompt.
#[derive(Debug, Clone, Serialize)]
pub struct UpdateStatus {
    pub current_version: String,
    /// True only when a newer version exists and this build can install it.
    pub available: bool,
    pub version: Option<String>,
    pub notes: Option<String>,
    /// Set when updates cannot be applied to this build at all.
    pub unavailable: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct UpdateProgress {
    downloaded: u64,
    total: Option<u64>,
}

const NOT_APPIMAGE: &str =
    "Automatic updates only apply to the AppImage. This build updates through its package manager.";

/// Whether this process is running from an AppImage, which is the only Linux
/// bundle that can be replaced while the app runs.
fn can_self_update() -> bool {
    std::env::var_os("APPIMAGE").is_some()
}

#[tauri::command]
pub async fn updates_check(app: AppHandle) -> Result<UpdateStatus, String> {
    let current_version = app.package_info().version.to_string();

    if !can_self_update() {
        return Ok(UpdateStatus {
            current_version,
            available: false,
            version: None,
            notes: None,
            unavailable: Some(NOT_APPIMAGE.to_string()),
        });
    }

    let updater = app
        .updater()
        .map_err(|error| {
            warn!(%error, "could not create the updater");
            "Could not check for updates.".to_string()
        })?;

    let update = updater.check().await.map_err(|error| {
        warn!(%error, "update check failed");
        "Could not check for updates.".to_string()
    })?;

    Ok(match update {
        Some(update) => UpdateStatus {
            current_version,
            available: true,
            version: Some(update.version.clone()),
            notes: update.body.as_deref().map(str::trim).filter(|body| !body.is_empty()).map(str::to_string),
            unavailable: None,
        },
        None => UpdateStatus {
            current_version,
            available: false,
            version: None,
            notes: None,
            unavailable: None,
        },
    })
}

/// Downloads and applies the pending update, then restarts into it.
///
/// The update is looked up again rather than remembered between calls: the
/// check and the install can be minutes apart, and a cached manifest could be
/// stale by then.
#[tauri::command]
pub async fn updates_install(app: AppHandle) -> Result<(), String> {
    if !can_self_update() {
        return Err(NOT_APPIMAGE.to_string());
    }

    let updater = app.updater().map_err(|error| {
        warn!(%error, "could not create the updater");
        "Could not install the update.".to_string()
    })?;

    let Some(update) = updater.check().await.map_err(|error| {
        warn!(%error, "update check failed before installing");
        "Could not install the update.".to_string()
    })? else {
        return Ok(());
    };

    let progress_app = app.clone();
    update
        .download_and_install(
            move |downloaded, total| {
                let _ = progress_app.emit(
                    "update-progress",
                    UpdateProgress {
                        downloaded: downloaded as u64,
                        total,
                    },
                );
            },
            || {},
        )
        .await
        .map_err(|error| {
            warn!(%error, "could not download or install the update");
            "Could not install the update.".to_string()
        })?;

    // The new binary is in place; restarting is the only way to run it.
    app.restart();
}
