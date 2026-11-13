use git2::{DiffOptions, Repository};
use serde::Serialize;
use std::path::Path;

/// A single hunk of a file's diff, identified by its header.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct HunkSummary {
    pub header: String,
    pub old_start: u32,
    pub old_lines: u32,
    pub new_start: u32,
    pub new_lines: u32,
    /// Changed lines, for the `+n −m` counter in the UI.
    pub added: u32,
    pub removed: u32,
}

#[derive(Debug, Clone)]
struct Hunk {
    summary: HunkSummary,
    /// Raw patch lines, each prefixed with ' ', '+' or '-'.
    lines: Vec<String>,
    old_no_newline: bool,
    new_no_newline: bool,
}

fn open(path: &str) -> Result<Repository, String> {
    Repository::open(path).map_err(|_| "Could not open the repository.".to_string())
}

fn check_file(file: &str) -> Result<(), String> {
    let candidate = Path::new(file);
    if file.trim().is_empty()
        || candidate.is_absolute()
        || candidate
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return Err("Only repository files can be changed.".to_string());
    }
    Ok(())
}

/// Reads `@@ -a,b +c,d @@` from a hunk header. A missing count means one line.
fn parse_header(header: &str) -> (u32, u32, u32, u32) {
    fn range(text: &str) -> (u32, u32) {
        let (start, count) = match text.split_once(',') {
            Some((start, count)) => (start, count),
            None => (text, "1"),
        };
        (
            start.parse::<u32>().unwrap_or(0),
            count.parse::<u32>().unwrap_or(1),
        )
    }

    let mut ranges = header
        .trim_start_matches("@@")
        .split("@@")
        .next()
        .unwrap_or("")
        .split_whitespace()
        .map(|part| part.trim_start_matches(['-', '+']))
        .map(range);
    let old = ranges.next().unwrap_or((0, 1));
    let new = ranges.next().unwrap_or((0, 1));
    (old.0, old.1, new.0, new.1)
}

/// Parses one file's diff into hunks. Only the hunk bodies are kept; file
/// headers are ignored.
fn parse_hunks(repo: &Repository, file: &str, staged: bool) -> Result<Vec<Hunk>, String> {
    let mut options = DiffOptions::new();
    options.pathspec(file);
    options.context_lines(3);
    options.include_untracked(true);
    options.recurse_untracked_dirs(true);
    // Untracked files have no hunks unless their content is included.
    options.show_untracked_content(true);

    let diff = if staged {
        let tree = repo.head().ok().and_then(|head| head.peel_to_tree().ok());
        repo.diff_tree_to_index(tree.as_ref(), None, Some(&mut options))
    } else {
        repo.diff_index_to_workdir(None, Some(&mut options))
    }
    .map_err(|_| "Could not read the changes.".to_string())?;

    let mut hunks: Vec<Hunk> = Vec::new();
    let mut file_cb = |_delta: git2::DiffDelta<'_>, _progress: f32| true;
    let mut binary_cb =
        |_delta: git2::DiffDelta<'_>, _binary: git2::DiffBinary<'_>| true;
    let mut hunk_cb = |_delta: git2::DiffDelta<'_>, _hunk: git2::DiffHunk<'_>| true;
    let mut line_cb = |_delta: git2::DiffDelta<'_>,
                       hunk: Option<git2::DiffHunk<'_>>,
                       line: git2::DiffLine<'_>| {
        let origin = line.origin();
        // '>', '<' and '=' mark "no newline at end of file" on the added,
        // removed and unchanged side.
        if !matches!(origin, ' ' | '+' | '-' | '\\' | '>' | '<' | '=') {
            return true;
        }
        let Some(hunk) = hunk else {
            return true;
        };
        let header = String::from_utf8_lossy(hunk.header()).trim_end().to_string();
        let starts = hunks
            .last()
            .map(|existing| existing.summary.header != header)
            .unwrap_or(true);
        if starts {
            let (old_start, old_lines, new_start, new_lines) = parse_header(&header);
            hunks.push(Hunk {
                summary: HunkSummary {
                    header,
                    old_start,
                    old_lines,
                    new_start,
                    new_lines,
                    added: 0,
                    removed: 0,
                },
                lines: Vec::new(),
                old_no_newline: false,
                new_no_newline: false,
            });
        }
        let Some(current) = hunks.last_mut() else {
            return true;
        };
        let content = String::from_utf8_lossy(line.content())
            .trim_end_matches('\n')
            .to_string();
        match origin {
            '>' => current.new_no_newline = true,
            '<' => current.old_no_newline = true,
            '\\' | '=' => {
                // The marker belongs to the line before it, so its prefix says
                // which side lacks the newline.
                match current.lines.last().and_then(|last| last.chars().next()) {
                    Some('-') => current.old_no_newline = true,
                    Some('+') => current.new_no_newline = true,
                    _ => {
                        current.old_no_newline = true;
                        current.new_no_newline = true;
                    }
                }
            }
            '+' => {
                current.summary.added += 1;
                current.lines.push(format!("+{content}"));
            }
            '-' => {
                current.summary.removed += 1;
                current.lines.push(format!("-{content}"));
            }
            _ => current.lines.push(format!(" {content}")),
        }
        true
    };

    diff.foreach(
        &mut file_cb,
        Some(&mut binary_cb),
        Some(&mut hunk_cb),
        Some(&mut line_cb),
    )
    .map_err(|_| "Could not read the changes.".to_string())?;

    // Drop hunks without a body, which happens for mode-only changes.
    hunks.retain(|hunk| !hunk.lines.is_empty());
    Ok(hunks)
}

fn old_side(hunk: &Hunk) -> Vec<String> {
    hunk.lines
        .iter()
        .filter_map(|line| match line.chars().next() {
            Some(' ') | Some('-') => Some(line.chars().skip(1).collect()),
            _ => None,
        })
        .collect()
}

fn new_side(hunk: &Hunk) -> Vec<String> {
    hunk.lines
        .iter()
        .filter_map(|line| match line.chars().next() {
            Some(' ') | Some('+') => Some(line.chars().skip(1).collect()),
            _ => None,
        })
        .collect()
}

fn split_content(text: &str) -> (Vec<String>, bool) {
    if text.is_empty() {
        return (Vec::new(), false);
    }
    match text.strip_suffix('\n') {
        Some(body) => (body.split('\n').map(str::to_string).collect(), true),
        None => (text.split('\n').map(str::to_string).collect(), false),
    }
}

fn join_content(lines: &[String], newline_at_end: bool) -> String {
    if lines.is_empty() {
        return String::new();
    }
    let mut text = lines.join("\n");
    if newline_at_end {
        text.push('\n');
    }
    text
}

/// Rewrites `content` with one hunk applied forwards or backwards. The target
/// side is checked first, so a file that changed after the diff was read is
/// refused instead of being corrupted.
fn apply_hunk(content: &str, hunk: &Hunk, forward: bool) -> Result<String, String> {
    let (target_start, target_count, expected, replacement) = if forward {
        (
            hunk.summary.old_start,
            hunk.summary.old_lines,
            old_side(hunk),
            new_side(hunk),
        )
    } else {
        (
            hunk.summary.new_start,
            hunk.summary.new_lines,
            new_side(hunk),
            old_side(hunk),
        )
    };

    let (mut lines, newline_at_end) = split_content(content);
    let index = if target_count == 0 {
        target_start as usize
    } else {
        target_start.saturating_sub(1) as usize
    };
    let end = index + target_count as usize;
    if end > lines.len() || lines[index..end] != expected[..] {
        return Err("The file changed since the diff was read. Reload and try again.".to_string());
    }

    let result_no_newline = if forward {
        hunk.new_no_newline
    } else {
        hunk.old_no_newline
    };
    let touches_end = end == lines.len();
    lines.splice(index..end, replacement);
    let mut newline = newline_at_end;
    if touches_end {
        // The hunk covers the last line, so its marker decides whether the
        // result ends with a newline.
        newline = !result_no_newline;
    }
    Ok(join_content(&lines, newline))
}

/// Index content for `file`, or an empty string when it has never been staged.
fn index_content(repo: &Repository, index: &git2::Index, file: &str) -> Result<String, String> {
    let Some(entry) = index.get_path(Path::new(file), 0) else {
        return Ok(String::new());
    };
    let blob = repo
        .find_blob(entry.id)
        .map_err(|_| "Could not read the staged file.".to_string())?;
    String::from_utf8(blob.content().to_vec())
        .map_err(|_| "That file is not text, so it cannot be staged by line.".to_string())
}

fn write_index_content(
    repo: &Repository,
    index: &mut git2::Index,
    file: &str,
    content: &str,
) -> Result<(), String> {
    let tracked = index.get_path(Path::new(file), 0).map(|entry| entry.mode);
    if content.is_empty() && tracked.is_none() {
        // Every line of an untracked file was unstaged; there is nothing to
        // record.
        return Ok(());
    }
    let oid = repo
        .blob(content.as_bytes())
        .map_err(|_| "Could not save the file.".to_string())?;
    let entry = git2::IndexEntry {
        ctime: git2::IndexTime::new(0, 0),
        mtime: git2::IndexTime::new(0, 0),
        dev: 0,
        ino: 0,
        mode: tracked.unwrap_or(0o100644),
        uid: 0,
        gid: 0,
        file_size: content.len() as u32,
        id: oid,
        flags: 0,
        flags_extended: 0,
        path: file.as_bytes().to_vec(),
    };
    // A file whose index content became empty while it was tracked is staged
    // as a deletion, which is what git does.
    if content.is_empty() && tracked.is_some() {
        let _ = index.remove_path(Path::new(file));
    } else {
        index
            .add(&entry)
            .map_err(|_| "Could not stage that file.".to_string())?;
    }
    index
        .write()
        .map_err(|_| "Could not save the staged changes.".to_string())?;
    Ok(())
}

fn find_hunk<'a>(hunks: &'a [Hunk], header: &str) -> Result<&'a Hunk, String> {
    hunks
        .iter()
        .find(|hunk| hunk.summary.header == header)
        .ok_or_else(|| "That change is no longer there. Reload and try again.".to_string())
}

/// Hunks of the unstaged changes (`staged = false`) or of the staged changes
/// (`staged = true`) for one file.
pub fn list_hunks(path: &str, file: &str, staged: bool) -> Result<Vec<HunkSummary>, String> {
    check_file(file)?;
    let repo = open(path)?;
    let hunks = parse_hunks(&repo, file, staged)?;
    Ok(hunks.into_iter().map(|hunk| hunk.summary).collect())
}

/// Stages a single hunk from the working tree.
pub fn stage_hunk(path: &str, file: &str, header: &str) -> Result<(), String> {
    check_file(file)?;
    let repo = open(path)?;
    let mut index = repo
        .index()
        .map_err(|_| "Could not read the index.".to_string())?;
    let hunks = parse_hunks(&repo, file, false)?;
    let hunk = find_hunk(&hunks, header)?;
    let content = index_content(&repo, &index, file)?;
    let updated = apply_hunk(&content, hunk, true)?;
    write_index_content(&repo, &mut index, file, &updated)
}

/// Removes a single hunk from the index, leaving the working tree alone.
pub fn unstage_hunk(path: &str, file: &str, header: &str) -> Result<(), String> {
    check_file(file)?;
    let repo = open(path)?;
    let mut index = repo
        .index()
        .map_err(|_| "Could not read the index.".to_string())?;
    let hunks = parse_hunks(&repo, file, true)?;
    let hunk = find_hunk(&hunks, header)?;
    let content = index_content(&repo, &index, file)?;
    let updated = apply_hunk(&content, hunk, false)?;
    write_index_content(&repo, &mut index, file, &updated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn scratch(name: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "knitt-hunks-{name}-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn committed(dir: &Path, file: &str, content: &str) -> Repository {
        let repo = Repository::init(dir).unwrap();
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();
        }
        std::fs::write(dir.join(file), content).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new(file)).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        {
            let tree = repo.find_tree(tree_id).unwrap();
            let sig = repo.signature().unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "initial", &tree, &[])
                .unwrap();
        }
        repo
    }

    fn index_text(repo: &Repository, file: &str) -> String {
        let index = repo.index().unwrap();
        index_content(repo, &index, file).unwrap()
    }

    /// Twenty numbered lines, so two edits can sit far enough apart to stay in
    /// separate hunks with the default three lines of context.
    const TEN_LINES: &str = "01\n02\n03\n04\n05\n06\n07\n08\n09\n10\n11\n12\n13\n14\n15\n16\n17\n18\n19\n20\n";

    fn two_hunks() -> String {
        TEN_LINES
            .replace("02\n", "TWO\n")
            .replace("18\n", "EIGHTEEN\n")
    }

    #[test]
    fn stages_only_the_chosen_hunk() {
        let dir = scratch("stage-one");
        let repo = committed(&dir, "a.txt", TEN_LINES);
        drop(repo);
        std::fs::write(dir.join("a.txt"), two_hunks()).unwrap();

        let hunks = list_hunks(dir.to_str().unwrap(), "a.txt", false).unwrap();
        assert_eq!(hunks.len(), 2, "the edits are far enough apart: {hunks:?}");

        stage_hunk(dir.to_str().unwrap(), "a.txt", &hunks[0].header).unwrap();

        let repo = Repository::open(&dir).unwrap();
        let staged = index_text(&repo, "a.txt");
        assert!(staged.contains("TWO"));
        assert!(staged.contains("18"), "the second hunk stays unstaged");
        assert_eq!(
            std::fs::read_to_string(dir.join("a.txt")).unwrap(),
            two_hunks(),
            "the working tree is untouched"
        );

        // The second hunk is still available, now alone.
        let remaining = list_hunks(dir.to_str().unwrap(), "a.txt", false).unwrap();
        assert_eq!(remaining.len(), 1);
        assert_eq!(remaining[0].added, 1);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unstages_only_the_chosen_hunk() {
        let dir = scratch("unstage-one");
        let repo = committed(&dir, "a.txt", TEN_LINES);
        drop(repo);
        std::fs::write(dir.join("a.txt"), two_hunks()).unwrap();
        crate::git::staging::stage_files(dir.to_str().unwrap(), &["a.txt".to_string()]).unwrap();

        let hunks = list_hunks(dir.to_str().unwrap(), "a.txt", true).unwrap();
        assert_eq!(hunks.len(), 2);

        unstage_hunk(dir.to_str().unwrap(), "a.txt", &hunks[0].header).unwrap();

        let repo = Repository::open(&dir).unwrap();
        let staged = index_text(&repo, "a.txt");
        assert!(
            staged.contains("\n02\n"),
            "the first hunk is back to HEAD: {staged:?}"
        );
        assert!(staged.contains("EIGHTEEN"), "the second hunk stays staged");
        assert_eq!(
            std::fs::read_to_string(dir.join("a.txt")).unwrap(),
            two_hunks(),
            "the working tree is untouched"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn refuses_a_header_that_is_not_there() {
        let dir = scratch("stale");
        let repo = committed(&dir, "a.txt", TEN_LINES);
        drop(repo);
        std::fs::write(dir.join("a.txt"), TEN_LINES.replace("02\n", "TWO\n")).unwrap();

        assert_eq!(
            stage_hunk(dir.to_str().unwrap(), "a.txt", "@@ -1,1 +1,1 @@").unwrap_err(),
            "That change is no longer there. Reload and try again."
        );
        assert_eq!(
            stage_hunk(dir.to_str().unwrap(), "../outside", "@@ -1 +1 @@").unwrap_err(),
            "Only repository files can be changed."
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stages_a_file_without_a_trailing_newline() {
        let dir = scratch("no-newline");
        let repo = committed(&dir, "a.txt", "one\ntwo");
        drop(repo);
        std::fs::write(dir.join("a.txt"), "one\nTWO").unwrap();

        let hunks = list_hunks(dir.to_str().unwrap(), "a.txt", false).unwrap();
        assert_eq!(hunks.len(), 1);
        stage_hunk(dir.to_str().unwrap(), "a.txt", &hunks[0].header).unwrap();

        let repo = Repository::open(&dir).unwrap();
        assert_eq!(index_text(&repo, "a.txt"), "one\nTWO");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn stages_a_new_file_hunk() {
        let dir = scratch("new-file");
        let repo = committed(&dir, "a.txt", "one\n");
        drop(repo);
        std::fs::write(dir.join("b.txt"), "hello\nworld\n").unwrap();

        let hunks = list_hunks(dir.to_str().unwrap(), "b.txt", false).unwrap();
        assert_eq!(hunks.len(), 1);
        assert_eq!(hunks[0].added, 2);
        stage_hunk(dir.to_str().unwrap(), "b.txt", &hunks[0].header).unwrap();

        let repo = Repository::open(&dir).unwrap();
        assert_eq!(index_text(&repo, "b.txt"), "hello\nworld\n");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_header_reads_the_counts() {
        assert_eq!(parse_header("@@ -1,4 +1,5 @@"), (1, 4, 1, 5));
        assert_eq!(parse_header("@@ -1 +1 @@ fn main()"), (1, 1, 1, 1));
        assert_eq!(parse_header("@@ -3,0 +4,2 @@"), (3, 0, 4, 2));
    }
}
