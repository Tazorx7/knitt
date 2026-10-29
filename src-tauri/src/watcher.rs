use notify::{Config, Event, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::mpsc::channel;
use std::time::Duration;

pub fn watch_repo<F>(path: &str, mut callback: F) -> Result<(), String>
where
    F: FnMut(Event) + Send + 'static,
{
    let (tx, rx) = channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default().with_poll_interval(Duration::from_millis(200)))
        .map_err(|e| e.to_string())?;
    watcher.watch(Path::new(path), RecursiveMode::Recursive).map_err(|e| e.to_string())?;
    std::thread::spawn(move || {
        let _watcher = watcher;
        for res in rx {
            match res {
                Ok(event) => callback(event),
                Err(_) => {}
            }
        }
    });
    Ok(())
}
