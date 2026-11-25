/// Runs slow work on a blocking thread so filesystem walks, process spawns and
/// remote operations never stall the UI thread or the async runtime.
pub async fn blocking<T, F>(work: F) -> Result<T, String>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, String> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|_| "The operation could not be completed.".to_string())?
}
