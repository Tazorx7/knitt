//! External diff and merge tools.
//!
//! Knitt never asks a tool to touch the repository. It writes the versions Git
//! would show into `.git/knitt-tool/`, starts the configured program with
//! those paths, and returns immediately. For a merge the working-tree file is
//! the result path, so saving the conflict afterwards stages whatever the tool
//! wrote there.

use crate::config::settings::{IntegrationConfig, Settings};
use crate::git::conflicts;
use crate::integrations::{
    self, BASE_PLACEHOLDER, LOCAL_PLACEHOLDER, MERGED_PLACEHOLDER, REMOTE_PLACEHOLDER,
};
use git2::{IndexEntry, Repository};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU32, Ordering};

/// Scratch directory inside the Git directory, so the prepared files can never
/// appear as working-tree changes.
const SCRATCH: &str = "knitt-tool";

static COUNTER: AtomicU32 = AtomicU32::new(0);

/// Shows the committed and working-tree versions of a file in the configured
/// diff tool.
pub fn diff(repo_path: &str, file: &str) -> Result<(), String> {
    let settings = Settings::load().map_err(|_| "Could not load settings.".to_string())?;
    diff_with(repo_path, file, &settings.difftool)
}

/// Starts the configured merge tool for a conflicted file. The tool must write
/// the merged result to `%MERGED_PATH%`.
pub fn merge(repo_path: &str, file: &str) -> Result<(), String> {
    let settings = Settings::load().map_err(|_| "Could not load settings.".to_string())?;
    merge_with(repo_path, file, &settings.mergetool)
}

fn executable(config: &IntegrationConfig, kind: &str) -> Result<String, String> {
    config
        .executable
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            format!("No {kind} tool is configured. Choose one in Settings → Integrations.")
        })
}

fn diff_with(repo_path: &str, file: &str, config: &IntegrationConfig) -> Result<(), String> {
    let executable = executable(config, "diff")?;
    let relative = conflicts::check_file(file)?;
    let repo =
        Repository::open(repo_path).map_err(|_| "Could not open the repository.".to_string())?;
    let workdir = repo
        .workdir()
        .ok_or("This repository has no working tree.")?;

    let original = blob_at_head(&repo, relative);
    let modified = std::fs::read(workdir.join(relative)).ok();
    if original.is_none() && modified.is_none() {
        return Err("That file has no content to compare.".to_string());
    }

    let dir = scratch_dir(&repo)?;
    let local = write_version(&dir.join("local"), relative, original.as_deref())?;
    let remote = write_version(&dir.join("remote"), relative, modified.as_deref())?;
    let local = local.to_string_lossy().to_string();
    let remote = remote.to_string_lossy().to_string();

    let arguments = integrations::build_tool_arguments(
        &config.arguments,
        &[
            (LOCAL_PLACEHOLDER, local.as_str()),
            (REMOTE_PLACEHOLDER, remote.as_str()),
        ],
    )?;
    integrations::launch_tool(&executable, &arguments)
}

fn merge_with(repo_path: &str, file: &str, config: &IntegrationConfig) -> Result<(), String> {
    let executable = executable(config, "merge")?;
    // Without the result path the tool would edit one of the read-only
    // versions, and Knitt would silently keep the unresolved file.
    if !config.arguments.contains(MERGED_PLACEHOLDER) {
        return Err("The merge tool arguments must include %MERGED_PATH%.".to_string());
    }
    let relative = conflicts::check_file(file)?;
    let repo =
        Repository::open(repo_path).map_err(|_| "Could not open the repository.".to_string())?;
    let workdir = repo
        .workdir()
        .ok_or("This repository has no working tree.")?;
    let merged = workdir.join(relative);
    if !merged.is_file() {
        return Err("That file is not in the working tree.".to_string());
    }

    let stages = conflict_stages(&repo, relative)?;

    let dir = scratch_dir(&repo)?;
    let local = write_version(&dir.join("local"), relative, stages.local.as_deref())?;
    let base = write_version(&dir.join("base"), relative, stages.base.as_deref())?;
    let remote = write_version(&dir.join("remote"), relative, stages.remote.as_deref())?;
    let local = local.to_string_lossy().to_string();
    let base = base.to_string_lossy().to_string();
    let remote = remote.to_string_lossy().to_string();
    let merged = merged.to_string_lossy().to_string();

    let arguments = integrations::build_tool_arguments(
        &config.arguments,
        &[
            (LOCAL_PLACEHOLDER, local.as_str()),
            (BASE_PLACEHOLDER, base.as_str()),
            (REMOTE_PLACEHOLDER, remote.as_str()),
            (MERGED_PLACEHOLDER, merged.as_str()),
        ],
    )?;
    integrations::launch_tool(&executable, &arguments)
}

/// The three index stages of a conflicted file.
struct Stages {
    base: Option<Vec<u8>>,
    local: Option<Vec<u8>>,
    remote: Option<Vec<u8>>,
}

fn conflict_stages(repo: &Repository, relative: &Path) -> Result<Stages, String> {
    let index = repo
        .index()
        .map_err(|_| "Could not read the index.".to_string())?;
    let wanted = relative.to_string_lossy().to_string();
    let mut found = None;
    for conflict in index
        .conflicts()
        .map_err(|_| "Could not read the conflicts.".to_string())?
    {
        let conflict = conflict.map_err(|_| "Could not read the conflicts.".to_string())?;
        let matches = [&conflict.ancestor, &conflict.our, &conflict.their]
            .into_iter()
            .flatten()
            .any(|entry| String::from_utf8_lossy(&entry.path) == wanted);
        if matches {
            found = Some(conflict);
            break;
        }
    }
    let conflict = found.ok_or("That file has no conflict to merge.")?;
    Ok(Stages {
        base: stage_content(repo, conflict.ancestor),
        local: stage_content(repo, conflict.our),
        remote: stage_content(repo, conflict.their),
    })
}

fn stage_content(repo: &Repository, entry: Option<IndexEntry>) -> Option<Vec<u8>> {
    let entry = entry?;
    repo.find_blob(entry.id)
        .ok()
        .map(|blob| blob.content().to_vec())
}

fn blob_at_head(repo: &Repository, relative: &Path) -> Option<Vec<u8>> {
    let tree = repo.head().ok()?.peel_to_tree().ok()?;
    let entry = tree.get_path(relative).ok()?;
    let object = entry.to_object(repo).ok()?;
    let blob = object.peel_to_blob().ok()?;
    Some(blob.content().to_vec())
}

/// A fresh directory per launch. Earlier launches are left alone: a tool may
/// still have them open.
fn scratch_dir(repo: &Repository) -> Result<PathBuf, String> {
    let id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let dir = repo
        .path()
        .join(SCRATCH)
        .join(format!("{}-{}", std::process::id(), id));
    std::fs::create_dir_all(&dir).map_err(|_| "Could not prepare the tool files.".to_string())?;
    Ok(dir)
}

/// Writes one version of the file under `root`, keeping the repository path so
/// both sides of a diff carry the same file name. A missing version becomes an
/// empty file, which is how Git shows an added or deleted file.
fn write_version(root: &Path, relative: &Path, contents: Option<&[u8]>) -> Result<PathBuf, String> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|_| "Could not prepare the tool files.".to_string())?;
    }
    std::fs::write(&path, contents.unwrap_or_default())
        .map_err(|_| "Could not prepare the tool files.".to_string())?;
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use git2::Signature;

    static TEST_COUNTER: AtomicU32 = AtomicU32::new(0);

    fn scratch_repo() -> PathBuf {
        let n = TEST_COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "knitt-tools-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let repo = Repository::init(&dir).unwrap();
        let sig = Signature::now("Test", "test@example.com").unwrap();
        let mut index = repo.index().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "initial commit", &tree, &[])
            .unwrap();
        dir
    }

    fn config(executable: &str, arguments: &str) -> IntegrationConfig {
        IntegrationConfig {
            executable: Some(executable.to_string()),
            arguments: arguments.to_string(),
        }
    }

    /// Every file under `.git/knitt-tool`, keyed by its path relative to the
    /// launch directory.
    fn scratch_contents(repo: &Path) -> Vec<(String, String)> {
        fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, String)>) {
            for entry in std::fs::read_dir(dir).unwrap() {
                let entry = entry.unwrap();
                let path = entry.path();
                if path.is_dir() {
                    walk(root, &path, out);
                } else {
                    out.push((
                        path.strip_prefix(root).unwrap().to_string_lossy().to_string(),
                        std::fs::read_to_string(&path).unwrap(),
                    ));
                }
            }
        }
        let root = repo.join(".git").join(SCRATCH);
        let mut out = Vec::new();
        for launch in std::fs::read_dir(&root).unwrap() {
            let launch = launch.unwrap().path();
            walk(&launch, &launch, &mut out);
        }
        out.sort();
        out
    }

    #[test]
    fn diff_writes_both_versions_and_launches() {
        let dir = scratch_repo();
        std::fs::write(dir.join("a.txt"), "committed\n").unwrap();
        let repo = Repository::open(&dir).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("a.txt")).unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        let sig = Signature::now("Test", "test@example.com").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "add a", &tree, &[&head])
            .unwrap();
        std::fs::write(dir.join("a.txt"), "working\n").unwrap();

        diff_with(
            dir.to_str().unwrap(),
            "a.txt",
            &config("/bin/true", "%LOCAL_PATH% %REMOTE_PATH%"),
        )
        .unwrap();

        assert_eq!(
            scratch_contents(&dir),
            vec![
                ("local/a.txt".to_string(), "committed\n".to_string()),
                ("remote/a.txt".to_string(), "working\n".to_string()),
            ]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn diff_of_a_new_file_uses_an_empty_original() {
        let dir = scratch_repo();
        std::fs::write(dir.join("new.txt"), "fresh\n").unwrap();

        diff_with(
            dir.to_str().unwrap(),
            "new.txt",
            &config("/bin/true", "%LOCAL_PATH% %REMOTE_PATH%"),
        )
        .unwrap();

        assert_eq!(
            scratch_contents(&dir),
            vec![
                ("local/new.txt".to_string(), String::new()),
                ("remote/new.txt".to_string(), "fresh\n".to_string()),
            ]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn commit_file(repo: &Repository, name: &str, content: &str, message: &str) -> git2::Oid {
        std::fs::write(repo.workdir().unwrap().join(name), content).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new(name)).unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let sig = Signature::now("Test", "test@example.com").unwrap();
        let parents: Vec<git2::Commit> = repo
            .head()
            .ok()
            .and_then(|head| head.peel_to_commit().ok())
            .into_iter()
            .collect();
        let refs: Vec<&git2::Commit> = parents.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &refs)
            .unwrap()
    }

    fn checkout(repo: &Repository, branch: &str) {
        let reference = repo.find_reference(&format!("refs/heads/{branch}")).unwrap();
        let object = reference.peel_to_commit().unwrap().into_object();
        repo.checkout_tree(&object, None).unwrap();
        repo.set_head(&format!("refs/heads/{branch}")).unwrap();
    }

    /// A repository where `a.txt` is conflicted: base, ours and theirs differ.
    fn conflicted_repo() -> PathBuf {
        let dir = scratch_repo();
        let repo = Repository::open(&dir).unwrap();
        let branch = repo.head().unwrap().shorthand().unwrap().to_string();
        commit_file(&repo, "a.txt", "base\n", "base");

        let base = repo.head().unwrap().peel_to_commit().unwrap();
        repo.branch("other", &base, false).unwrap();
        checkout(&repo, "other");
        commit_file(&repo, "a.txt", "theirs\n", "theirs");
        checkout(&repo, &branch);
        commit_file(&repo, "a.txt", "ours\n", "ours");

        let other = repo
            .find_reference("refs/heads/other")
            .unwrap()
            .peel_to_commit()
            .unwrap();
        let annotated = repo.find_annotated_commit(other.id()).unwrap();
        repo.merge(&[&annotated], None, None).unwrap();
        std::fs::write(
            dir.join("a.txt"),
            "<<<<<<< HEAD\nours\n=======\ntheirs\n>>>>>>> other\n",
        )
        .unwrap();
        dir
    }

    #[test]
    fn merge_writes_the_three_stages_next_to_the_working_file() {
        let dir = conflicted_repo();
        let repo = Repository::open(&dir).unwrap();
        assert!(repo.index().unwrap().has_conflicts());
        drop(repo);

        merge_with(
            dir.to_str().unwrap(),
            "a.txt",
            &config(
                "/bin/true",
                "%LOCAL_PATH% %BASE_PATH% %REMOTE_PATH% %MERGED_PATH%",
            ),
        )
        .unwrap();

        assert_eq!(
            scratch_contents(&dir),
            vec![
                ("base/a.txt".to_string(), "base\n".to_string()),
                ("local/a.txt".to_string(), "ours\n".to_string()),
                ("remote/a.txt".to_string(), "theirs\n".to_string()),
            ]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn merge_requires_the_result_path() {
        let dir = scratch_repo();
        std::fs::write(dir.join("a.txt"), "ours\n").unwrap();
        assert_eq!(
            merge_with(
                dir.to_str().unwrap(),
                "a.txt",
                &config("/bin/true", "%LOCAL_PATH% %REMOTE_PATH%")
            )
            .unwrap_err(),
            "The merge tool arguments must include %MERGED_PATH%."
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unconfigured_and_missing_files_are_reported() {
        let dir = scratch_repo();
        std::fs::write(dir.join("a.txt"), "a\n").unwrap();
        let empty = config("", "%LOCAL_PATH% %REMOTE_PATH%");
        assert_eq!(
            diff_with(dir.to_str().unwrap(), "a.txt", &empty).unwrap_err(),
            "No diff tool is configured. Choose one in Settings → Integrations."
        );
        assert_eq!(
            merge_with(dir.to_str().unwrap(), "a.txt", &empty).unwrap_err(),
            "No merge tool is configured. Choose one in Settings → Integrations."
        );

        let configured = config("/bin/true", "%MERGED_PATH%");
        assert_eq!(
            merge_with(dir.to_str().unwrap(), "a.txt", &configured).unwrap_err(),
            "That file has no conflict to merge."
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn paths_outside_the_repository_are_refused() {
        let dir = scratch_repo();
        for file in ["../secret", "/etc/passwd", "", "  "] {
            assert_eq!(
                diff_with(
                    dir.to_str().unwrap(),
                    file,
                    &config("/bin/true", "%LOCAL_PATH% %REMOTE_PATH%")
                )
                .unwrap_err(),
                "Only repository files can be opened.",
                "diffing {file:?} should be refused"
            );
            assert_eq!(
                merge_with(dir.to_str().unwrap(), file, &config("/bin/true", "%MERGED_PATH%"))
                    .unwrap_err(),
                "Only repository files can be opened.",
                "merging {file:?} should be refused"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_missing_executable_is_reported_plainly() {
        let dir = scratch_repo();
        std::fs::write(dir.join("a.txt"), "a\n").unwrap();
        assert_eq!(
            diff_with(
                dir.to_str().unwrap(),
                "a.txt",
                &config("/nonexistent/knitt-tool", "%LOCAL_PATH% %REMOTE_PATH%")
            )
            .unwrap_err(),
            "The selected application could not be found. Choose it again in Settings."
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
