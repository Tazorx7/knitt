use git2::{AnnotatedCommit, BranchType, FetchOptions, FetchPrune, Repository};
use serde::{Deserialize, Serialize};

use super::credentials;
use super::proxy::ProxyConfig;

/// How incoming remote commits are integrated into the local branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PullStrategy {
    Merge,
    Rebase,
    /// Move the branch only when it can be fast-forwarded; never create a
    /// merge commit or rewrite local commits.
    FastForwardOnly,
}

impl PullStrategy {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "merge" => Some(Self::Merge),
            "rebase" => Some(Self::Rebase),
            "ff-only" | "fast-forward-only" => Some(Self::FastForwardOnly),
            _ => None,
        }
    }
}

/// Where a push sends the current branch.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum PushMode {
    /// Push the checked-out branch to a same-named branch on `origin`.
    Current,
    /// Push to the branch name this branch already tracks.
    Upstream,
}

impl PushMode {
    pub fn parse(value: &str) -> Option<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "current" => Some(Self::Current),
            "upstream" => Some(Self::Upstream),
            _ => None,
        }
    }
}

fn open(path: &str) -> Result<Repository, String> {
    Repository::open(path).map_err(|_| "Could not open the repository.".to_string())
}

fn current_branch_name(repo: &Repository) -> Result<String, String> {
    repo.head()
        .ok()
        .and_then(|head| head.shorthand().map(str::to_string))
        .ok_or_else(|| "No branch is checked out.".to_string())
}

/// The remote-tracking reference for the current branch's upstream.
fn upstream_reference<'a>(
    repo: &'a Repository,
    branch_name: &str,
) -> Result<git2::Reference<'a>, String> {
    let branch = repo
        .find_branch(branch_name, BranchType::Local)
        .map_err(|_| "Branch not found.".to_string())?;
    let upstream = branch
        .upstream()
        .map_err(|_| "This branch has no upstream branch.".to_string())?;
    Ok(upstream.into_reference())
}

fn fetch_upstream(
    repo: &Repository,
    branch_name: &str,
    prune: bool,
    proxy: &ProxyConfig,
) -> Result<(), String> {
    let upstream = upstream_reference(repo, branch_name)?;
    let remote_name = upstream
        .name()
        .and_then(|name| name.strip_prefix("refs/remotes/"))
        .and_then(|rest| rest.split_once('/'))
        .map(|(remote, _)| remote.to_string())
        .ok_or_else(|| "Could not resolve the upstream remote.".to_string())?;

    let mut remote = repo
        .find_remote(&remote_name)
        .map_err(|_| "The upstream remote is not configured.".to_string())?;

    let mut options = FetchOptions::new();
    options.remote_callbacks(credentials::make_callbacks());
    options.proxy_options(proxy.options()?);
    options.prune(if prune {
        FetchPrune::On
    } else {
        FetchPrune::Unspecified
    });
    remote
        .fetch(&[] as &[&str], Some(&mut options), None)
        .map_err(|_| "Could not fetch from the remote.".to_string())
}

fn fast_forward(repo: &Repository, target: git2::Oid) -> Result<(), String> {
    let head = repo.head().map_err(|_| "No branch is checked out.".to_string())?;
    let refname = head
        .name()
        .ok_or_else(|| "Cannot read HEAD.".to_string())?
        .to_string();
    let mut reference = repo
        .find_reference(&refname)
        .map_err(|_| "Cannot read the current branch.".to_string())?;
    reference
        .set_target(target, "pull: fast-forward")
        .map_err(|_| "Could not update the branch.".to_string())?;
    repo.set_head(&refname)
        .map_err(|_| "Could not update HEAD.".to_string())?;
    repo.checkout_head(Some(git2::build::CheckoutBuilder::default().force()))
        .map_err(|_| "Could not update the working tree.".to_string())
}

fn merge_annotated(
    repo: &Repository,
    annotated: &AnnotatedCommit<'_>,
    label: &str,
) -> Result<(), String> {
    let (analysis, _) = repo
        .merge_analysis(&[annotated])
        .map_err(|_| "Could not analyse the local branch.".to_string())?;

    if analysis.is_up_to_date() {
        return Ok(());
    }
    if analysis.is_fast_forward() {
        return fast_forward(repo, annotated.id());
    }

    let head_commit = repo
        .head()
        .and_then(|head| head.peel_to_commit())
        .map_err(|_| "No commits on the current branch.".to_string())?;
    let incoming = repo
        .find_commit(annotated.id())
        .map_err(|_| "Could not read the incoming commit.".to_string())?;

    let mut checkout = git2::build::CheckoutBuilder::new();
    checkout.safe();
    repo.merge(&[annotated], None, Some(&mut checkout))
        .map_err(|_| "Could not merge the incoming changes.".to_string())?;

    let mut index = repo.index().map_err(|_| "Could not read the index.".to_string())?;
    if index.has_conflicts() {
        return Err("Pull produced conflicts. Resolve them, then commit.".to_string());
    }

    let sig = repo
        .signature()
        .map_err(|_| "Git author identity is not configured.".to_string())?;
    let tree_id = index
        .write_tree_to(repo)
        .map_err(|_| "Could not write the merge result.".to_string())?;
    let tree = repo
        .find_tree(tree_id)
        .map_err(|_| "Could not read the merge result.".to_string())?;
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        &format!("Merge branch '{label}'"),
        &tree,
        &[&head_commit, &incoming],
    )
    .map_err(|_| "Could not create the merge commit.".to_string())?;
    repo.cleanup_state()
        .map_err(|_| "Could not finish the merge.".to_string())
}

fn rebase_onto(repo: &Repository, annotated: &AnnotatedCommit<'_>) -> Result<(), String> {
    let sig = repo
        .signature()
        .map_err(|_| "Git author identity is not configured.".to_string())?;
    let mut rebase = repo
        .rebase(None, Some(annotated), None, None)
        .map_err(|_| "Could not start the rebase.".to_string())?;

    while let Some(op) = rebase.next() {
        op.map_err(|_| "Could not apply the next commit.".to_string())?;
        let index = repo.index().map_err(|_| "Could not read the index.".to_string())?;
        if index.has_conflicts() {
            return Err("Pull stopped due to conflicts. Resolve them, then continue.".to_string());
        }
        match rebase.commit(None, &sig, None) {
            Ok(_) => {}
            Err(error)
                if matches!(
                    error.code(),
                    git2::ErrorCode::Applied | git2::ErrorCode::Exists
                ) => {}
            Err(_) => return Err("Could not apply a commit during the rebase.".to_string()),
        }
    }

    rebase
        .finish(Some(&sig))
        .map_err(|_| "Could not finish the rebase.".to_string())
}

/// Fetch the current branch's upstream and integrate it using `strategy`.
///
/// Fetching always happens, so `ff-only` still updates remote-tracking refs and
/// can report whether the branch is behind.
pub fn pull(
    path: &str,
    strategy: PullStrategy,
    prune: bool,
    proxy: &ProxyConfig,
) -> Result<(), String> {
    let repo = open(path)?;
    let branch_name = current_branch_name(&repo)?;
    fetch_upstream(&repo, &branch_name, prune, proxy)?;

    let upstream = upstream_reference(&repo, &branch_name)?;
    let annotated = repo
        .reference_to_annotated_commit(&upstream)
        .map_err(|_| "Could not read the upstream branch.".to_string())?;

    let (analysis, _) = repo
        .merge_analysis(&[&annotated])
        .map_err(|_| "Could not analyse the local branch.".to_string())?;

    if analysis.is_up_to_date() {
        return Ok(());
    }

    if analysis.is_fast_forward() {
        return fast_forward(&repo, annotated.id());
    }

    let label = upstream
        .shorthand()
        .unwrap_or("upstream")
        .to_string();

    match strategy {
        PullStrategy::FastForwardOnly => {
            Err("This branch has diverged from the remote. Fast-forward only is enabled.".to_string())
        }
        PullStrategy::Merge => merge_annotated(&repo, &annotated, &label),
        PullStrategy::Rebase => rebase_onto(&repo, &annotated),
    }
}

/// Fetch changes from a remote, optionally pruning stale remote-tracking refs.
pub fn fetch(
    path: &str,
    remote_name: &str,
    prune: bool,
    proxy: &ProxyConfig,
) -> Result<(), String> {
    let repo = open(path)?;
    let mut remote = repo
        .find_remote(remote_name)
        .map_err(|_| "That remote is not configured.".to_string())?;
    let mut options = FetchOptions::new();
    options.remote_callbacks(credentials::make_callbacks());
    options.proxy_options(proxy.options()?);
    options.prune(if prune {
        FetchPrune::On
    } else {
        FetchPrune::Unspecified
    });
    remote
        .fetch(&[] as &[&str], Some(&mut options), None)
        .map_err(|_| "Could not fetch from the remote.".to_string())
}

/// Push the current branch according to `mode`.
pub fn push(path: &str, mode: PushMode, proxy: &ProxyConfig) -> Result<String, String> {
    let repo = open(path)?;
    let branch_name = current_branch_name(&repo)?;

    let (remote_name, remote_branch) = match mode {
        PushMode::Current => ("origin".to_string(), branch_name.clone()),
        PushMode::Upstream => {
            let branch = repo
                .find_branch(&branch_name, BranchType::Local)
                .map_err(|_| "Branch not found.".to_string())?;
            let upstream = branch
                .upstream()
                .map_err(|_| "This branch has no upstream branch. Publish it first.".to_string())?;
            let name = upstream
                .name()
                .ok()
                .flatten()
                .and_then(|name| name.strip_prefix("refs/remotes/"))
                .ok_or_else(|| "Could not resolve the tracked branch.".to_string())?;
            let (remote, tracked) = name
                .split_once('/')
                .ok_or_else(|| "Could not resolve the tracked branch.".to_string())?;
            (remote.to_string(), tracked.to_string())
        }
    };

    let mut remote = repo
        .find_remote(&remote_name)
        .map_err(|_| "That remote is not configured.".to_string())?;
    let mut options = git2::PushOptions::new();
    options.remote_callbacks(credentials::make_callbacks());
    options.proxy_options(proxy.options()?);
    let refspec = format!("refs/heads/{branch_name}:refs/heads/{remote_branch}");
    remote
        .push(&[refspec.as_str()], Some(&mut options))
        .map_err(|_| "Push failed.".to_string())?;
    Ok("pushed".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use std::sync::atomic::{AtomicU32, Ordering};

    static COUNTER: AtomicU32 = AtomicU32::new(0);

    fn scratch(name: &str) -> PathBuf {
        let n = COUNTER.fetch_add(1, Ordering::SeqCst);
        let dir = std::env::temp_dir().join(format!(
            "knitt-sync-{name}-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn init_repo(dir: &Path) -> Repository {
        let repo = Repository::init(dir).unwrap();
        let mut config = repo.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
        repo
    }

    fn commit_file(repo: &Repository, name: &str, content: &str, message: &str) -> git2::Oid {
        std::fs::write(repo.workdir().unwrap().join(name), content).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new(name)).unwrap();
        index.write().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();
        let parents: Vec<git2::Commit> = repo
            .head()
            .ok()
            .and_then(|head| head.peel_to_commit().ok())
            .into_iter()
            .collect();
        let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parent_refs)
            .unwrap()
    }

    /// Bare "origin" containing one commit on `master`, plus a clone that
    /// tracks `origin/master`.
    fn clone_with_remote(name: &str) -> (PathBuf, PathBuf, Repository) {
        let root = scratch(name);
        let remote_dir = root.join("remote.git");

        Repository::init_opts(
            &remote_dir,
            git2::RepositoryInitOptions::new().bare(true),
        )
        .unwrap();

        let seed = init_repo(&root.join("seed"));
        commit_file(&seed, "seed.txt", "seed\n", "seed");
        seed.remote("origin", remote_dir.to_str().unwrap()).unwrap();
        seed.find_remote("origin")
            .unwrap()
            .push(&["refs/heads/master:refs/heads/master"], None)
            .unwrap();
        // Point the bare repository's HEAD at the branch it now has, so clones
        // check out a real branch.
        Repository::open(&remote_dir)
            .unwrap()
            .set_head("refs/heads/master")
            .unwrap();

        let work_dir = root.join("work");
        let work = Repository::clone(remote_dir.to_str().unwrap(), &work_dir).unwrap();
        let mut config = work.config().unwrap();
        config.set_str("user.name", "Test").unwrap();
        config.set_str("user.email", "test@example.com").unwrap();
        drop(config);

        (root, remote_dir, work)
    }

    fn advance_remote(root: &Path, remote_dir: &Path, file: &str, content: &str) -> git2::Oid {
        let other_dir = root.join(format!("other-{}", COUNTER.fetch_add(1, Ordering::SeqCst)));
        let other = Repository::clone(remote_dir.to_str().unwrap(), &other_dir).unwrap();
        let mut config = other.config().unwrap();
        config.set_str("user.name", "Other").unwrap();
        config.set_str("user.email", "other@example.com").unwrap();
        drop(config);
        let oid = commit_file(&other, file, content, "remote change");
        other
            .find_remote("origin")
            .unwrap()
            .push(&["refs/heads/master:refs/heads/master"], None)
            .unwrap();
        oid
    }

    #[test]
    fn pull_fast_forwards_when_behind() {
        let (root, remote_dir, repo) = clone_with_remote("ff");
        advance_remote(&root, &remote_dir, "remote.txt", "remote\n");

        pull(repo.workdir().unwrap().to_str().unwrap(), PullStrategy::Merge, false, &ProxyConfig::default()).unwrap();

        let head = repository_head(&repo);
        assert_eq!(std::fs::read_to_string(repo.workdir().unwrap().join("remote.txt")).unwrap(), "remote\n");
        assert_eq!(
            head.message().unwrap_or_default().trim(),
            "remote change"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn pull_ff_only_refuses_divergence() {
        let (root, remote_dir, repo) = clone_with_remote("ff-only");
        commit_file(&repo, "local.txt", "local\n", "local change");
        advance_remote(&root, &remote_dir, "remote.txt", "remote\n");

        let result = pull(
            repo.workdir().unwrap().to_str().unwrap(),
            PullStrategy::FastForwardOnly,
            false,
            &ProxyConfig::default(),
        );
        assert_eq!(
            result.unwrap_err(),
            "This branch has diverged from the remote. Fast-forward only is enabled."
        );
        // The local commit is untouched.
        assert_eq!(
            repository_head(&repo).message().unwrap_or_default().trim(),
            "local change"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn pull_merge_creates_merge_commit_when_diverged() {
        let (root, remote_dir, repo) = clone_with_remote("merge");
        commit_file(&repo, "local.txt", "local\n", "local change");
        advance_remote(&root, &remote_dir, "remote.txt", "remote\n");

        pull(repo.workdir().unwrap().to_str().unwrap(), PullStrategy::Merge, false, &ProxyConfig::default()).unwrap();

        let head = repository_head(&repo);
        assert_eq!(head.parent_count(), 2);
        assert!(repo.workdir().unwrap().join("local.txt").exists());
        assert!(repo.workdir().unwrap().join("remote.txt").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn pull_rebase_replays_local_commits() {
        let (root, remote_dir, repo) = clone_with_remote("rebase");
        commit_file(&repo, "local.txt", "local\n", "local change");
        advance_remote(&root, &remote_dir, "remote.txt", "remote\n");

        pull(repo.workdir().unwrap().to_str().unwrap(), PullStrategy::Rebase, false, &ProxyConfig::default()).unwrap();

        let head = repository_head(&repo);
        assert_eq!(head.parent_count(), 1);
        assert_eq!(head.message().unwrap_or_default().trim(), "local change");
        let parent = head.parent(0).unwrap();
        assert_eq!(parent.message().unwrap_or_default().trim(), "remote change");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn pull_requires_an_upstream() {
        let dir = scratch("no-upstream");
        let repo = init_repo(&dir);
        commit_file(&repo, "a.txt", "a\n", "initial");

        let result = pull(dir.to_str().unwrap(), PullStrategy::Merge, false, &ProxyConfig::default());
        assert_eq!(result.unwrap_err(), "This branch has no upstream branch.");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn strategy_parsing_is_strict() {
        assert_eq!(PullStrategy::parse("merge"), Some(PullStrategy::Merge));
        assert_eq!(PullStrategy::parse("REBASE"), Some(PullStrategy::Rebase));
        assert_eq!(
            PullStrategy::parse("ff-only"),
            Some(PullStrategy::FastForwardOnly)
        );
        assert_eq!(PullStrategy::parse("octopus"), None);
        assert_eq!(PushMode::parse("current"), Some(PushMode::Current));
        assert_eq!(PushMode::parse("upstream"), Some(PushMode::Upstream));
        assert_eq!(PushMode::parse("matching"), None);
    }

    #[test]
    fn push_current_updates_the_origin_branch() {
        let (root, remote_dir, repo) = clone_with_remote("push");
        commit_file(&repo, "second.txt", "second\n", "second");

        push(repo.workdir().unwrap().to_str().unwrap(), PushMode::Current, &ProxyConfig::default()).unwrap();

        let bare = Repository::open(&remote_dir).unwrap();
        let tip = bare.refname_to_id("refs/heads/master").unwrap();
        assert_eq!(repository_head(&repo).id(), tip);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn push_upstream_requires_tracking_branch() {
        let dir = scratch("push-no-upstream");
        let repo = init_repo(&dir);
        commit_file(&repo, "a.txt", "a\n", "initial");
        repo.remote("origin", "https://example.invalid/owner/repo.git")
            .unwrap();

        let result = push(dir.to_str().unwrap(), PushMode::Upstream, &ProxyConfig::default());
        assert_eq!(
            result.unwrap_err(),
            "This branch has no upstream branch. Publish it first."
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn repository_head(repo: &Repository) -> git2::Commit<'_> {
        repo.head().unwrap().peel_to_commit().unwrap()
    }
}
