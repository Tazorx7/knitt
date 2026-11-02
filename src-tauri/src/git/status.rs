
use git2::{Repository, DiffOptions};
pub fn get_diff(path: &str, file: &str) -> Result<String, String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let mut opts = DiffOptions::new();
    opts.pathspec(file);
    let diff = repo.diff_index_to_workdir(None, Some(&mut opts)).map_err(|e| e.to_string())?;
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
    }).map_err(|e| e.to_string())?;
    Ok(output)
}
