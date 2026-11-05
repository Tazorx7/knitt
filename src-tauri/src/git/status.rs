use git2::{DiffOptions, Repository};
use serde::Serialize;
use std::path::Path;

/// File content at HEAD (original) and in the working tree (modified).
/// Both are empty strings when the file is absent on that side
/// (e.g. a newly added or a deleted file).
#[derive(Debug, Serialize)]
pub struct FileContents {
    pub original: String,
    pub modified: String,
}

pub fn get_file_contents(path: &str, file: &str) -> Result<FileContents, String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;

    let original = repo
        .head()
        .ok()
        .and_then(|h| h.peel_to_tree().ok())
        .and_then(|tree| tree.get_path(Path::new(file)).ok())
        .and_then(|entry| entry.to_object(&repo).ok())
        .and_then(|obj| obj.peel_to_blob().ok())
        .and_then(|blob| String::from_utf8(blob.content().to_vec()).ok())
        .unwrap_or_default();

    let modified = repo
        .workdir()
        .map(|w| w.join(file))
        .and_then(|full| std::fs::read_to_string(full).ok())
        .unwrap_or_default();

    Ok(FileContents {
        original,
        modified,
    })
}

pub fn get_diff(path: &str, file: &str) -> Result<String, String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let mut opts = DiffOptions::new();
    opts.pathspec(file);
    let diff = repo
        .diff_index_to_workdir(None, Some(&mut opts))
        .map_err(|e| e.to_string())?;
    let mut output = String::new();
    diff.print(git2::DiffFormat::Patch, |_delta, _hunk, line| {
        let prefix = match line.origin() {
            '+' => "+",
            '-' => "-",
            ' ' => " ",
            _ => "",
        };
        output.push_str(prefix);
        output.push_str(std::str::from_utf8(line.content()).unwrap_or(""));
        true
    })
    .map_err(|e| e.to_string())?;
    Ok(output)
}
