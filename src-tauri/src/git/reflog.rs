use git2::Repository;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct ReflogEntry {
    pub message: String,
    pub id: String,
}

pub fn list_reflog(path: &str) -> Result<Vec<ReflogEntry>, String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let reflog = repo.reflog("HEAD").map_err(|e| e.to_string())?;
    let mut entries = Vec::new();
    for i in 0..reflog.len().min(50) {
        if let Some(entry) = reflog.get(i) {
            entries.push(ReflogEntry {
                message: entry.message().unwrap_or("").to_string(),
                id: entry.id_new().to_string(),
            });
        }
    }
    Ok(entries)
}
