use std::collections::HashSet;
use std::path::PathBuf;

use git2::{Oid, Repository, Sort};
use serde::{Deserialize, Serialize};

use super::commit;
use super::signing::SigningConfig;

/// A rebase is expressed here instead of going through libgit2's rebase API:
/// that API replays a fixed list of operations and offers no hook for
/// reordering, squashing or rewording. This sequencer resets to the target
/// commit and writes each group itself, which also lets signed commits use the
/// same signing path as the rest of the app.
const STATE_FILE: &str = "knitt-interactive-rebase.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RebaseAction {
    Pick,
    Reword,
    Squash,
    Fixup,
    Drop,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RebaseStep {
    pub oid: String,
    pub action: RebaseAction,
    #[serde(default)]
    pub message: Option<String>,
}

/// One row shown in the interactive rebase dialog.
#[derive(Debug, Clone, Serialize)]
pub struct PlanCommit {
    pub id: String,
    pub summary: String,
    pub author: String,
    pub time: i64,
    pub message: String,
}

/// Commits that become a single commit. A group always starts with a pick or
/// reword; squashes and fixups are folded into it.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct StoredGroup {
    commits: Vec<String>,
    message: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct RebaseState {
    original_head: String,
    onto: String,
    groups: Vec<StoredGroup>,
    group_index: usize,
    /// Message of the group whose changes are sitting in the index after a
    /// conflict stopped the rebase. `None` means no step is half-applied.
    #[serde(default)]
    pending: Option<String>,
}

enum Applied {
    Committed,
    Skipped,
    Conflicts,
}

fn open(path: &str) -> Result<Repository, String> {
    Repository::open(path).map_err(|_| "Could not open the repository.".to_string())
}

fn state_path(repo: &Repository) -> PathBuf {
    repo.path().join(STATE_FILE)
}

/// Whether an interactive rebase is in progress for this repository.
pub fn has_state(path: &str) -> bool {
    open(path)
        .map(|repo| state_path(&repo).is_file())
        .unwrap_or(false)
}

fn load_state(repo: &Repository) -> Result<Option<RebaseState>, String> {
    let path = state_path(repo);
    if !path.is_file() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(&path)
        .map_err(|_| "Could not read the rebase state.".to_string())?;
    serde_json::from_str(&text)
        .map(Some)
        .map_err(|_| "Could not read the rebase state.".to_string())
}

fn save_state(repo: &Repository, state: &RebaseState) -> Result<(), String> {
    let text = serde_json::to_string_pretty(state)
        .map_err(|_| "Could not save the rebase state.".to_string())?;
    std::fs::write(state_path(repo), text)
        .map_err(|_| "Could not save the rebase state.".to_string())
}

fn clear_state(repo: &Repository) -> Result<(), String> {
    let path = state_path(repo);
    if path.is_file() {
        std::fs::remove_file(&path)
            .map_err(|_| "Could not clear the rebase state.".to_string())?;
    }
    Ok(())
}

fn head_commit(repo: &Repository) -> Result<git2::Commit<'_>, String> {
    repo.head()
        .and_then(|head| head.peel_to_commit())
        .map_err(|_| "No commits on the current branch.".to_string())
}

fn branch_commit<'a>(repo: &'a Repository, name: &str) -> Result<git2::Commit<'a>, String> {
    repo.find_branch(name, git2::BranchType::Local)
        .and_then(|branch| branch.into_reference().peel_to_commit())
        .map_err(|_| "Branch not found.".to_string())
}

fn empty_tree(repo: &Repository) -> Result<git2::Tree<'_>, String> {
    let oid = repo
        .treebuilder(None)
        .and_then(|builder| builder.write())
        .map_err(|_| "Could not read the first commit.".to_string())?;
    repo.find_tree(oid)
        .map_err(|_| "Could not read the first commit.".to_string())
}

/// Commits reachable from HEAD but not from `onto`, oldest first.
fn plan_oids(repo: &Repository, onto: Oid) -> Result<Vec<Oid>, String> {
    let head = head_commit(repo)?;
    let mut walk = repo
        .revwalk()
        .map_err(|_| "Could not read the history.".to_string())?;
    walk.set_sorting(Sort::TOPOLOGICAL | Sort::REVERSE)
        .map_err(|_| "Could not read the history.".to_string())?;
    walk.push(head.id())
        .map_err(|_| "Could not read the history.".to_string())?;
    walk.hide(onto)
        .map_err(|_| "Could not read the history.".to_string())?;

    let mut oids = Vec::new();
    for oid in walk {
        oids.push(oid.map_err(|_| "Could not read the history.".to_string())?);
    }
    Ok(oids)
}

/// The commits an interactive rebase onto `onto` would replay, oldest first.
pub fn plan(path: &str, onto: &str) -> Result<Vec<PlanCommit>, String> {
    let repo = open(path)?;
    let target = branch_commit(&repo, onto)?;
    let mut commits = Vec::new();
    for oid in plan_oids(&repo, target.id())? {
        let commit = repo
            .find_commit(oid)
            .map_err(|_| "Could not read a commit.".to_string())?;
        let author = commit.author();
        let name = author.name().unwrap_or("");
        let email = author.email().unwrap_or("");
        commits.push(PlanCommit {
            id: oid.to_string(),
            summary: commit.summary().unwrap_or("").to_string(),
            author: format!("{name} <{email}>"),
            time: author.when().seconds(),
            message: commit.message().unwrap_or("").to_string(),
        });
    }
    Ok(commits)
}

/// Turns the requested steps into groups, refusing anything that would produce
/// a commit without a message or replay a commit outside the plan.
fn build_groups(
    repo: &Repository,
    steps: &[RebaseStep],
    allowed: &HashSet<Oid>,
) -> Result<Vec<StoredGroup>, String> {
    if steps.is_empty() {
        return Err("Choose at least one commit to rebase.".to_string());
    }

    let mut seen = HashSet::new();
    let mut groups: Vec<StoredGroup> = Vec::new();
    for step in steps {
        let oid = Oid::from_str(&step.oid)
            .map_err(|_| "That commit is not part of this rebase.".to_string())?;
        if !allowed.contains(&oid) {
            return Err("That commit is not part of this rebase.".to_string());
        }
        if !seen.insert(oid) {
            return Err("Each commit can only appear once in the rebase.".to_string());
        }
        let commit = repo
            .find_commit(oid)
            .map_err(|_| "That commit is not part of this rebase.".to_string())?;

        match step.action {
            RebaseAction::Drop => {}
            RebaseAction::Pick | RebaseAction::Reword => {
                let message = if step.action == RebaseAction::Reword {
                    step.message
                        .as_deref()
                        .map(str::trim)
                        .filter(|text| !text.is_empty())
                        .ok_or_else(|| "Every reworded commit needs a message.".to_string())?
                        .to_string()
                } else {
                    commit.message().unwrap_or("").trim().to_string()
                };
                if message.is_empty() {
                    return Err("A commit in the rebase has no message.".to_string());
                }
                groups.push(StoredGroup {
                    commits: vec![step.oid.clone()],
                    message,
                });
            }
            RebaseAction::Squash | RebaseAction::Fixup => {
                let Some(group) = groups.last_mut() else {
                    return Err(
                        "The first kept commit cannot be squashed or fixed up.".to_string()
                    );
                };
                if step.action == RebaseAction::Squash {
                    let original = commit.message().unwrap_or("").trim();
                    if !original.is_empty() {
                        group.message.push_str("\n\n");
                        group.message.push_str(original);
                    }
                }
                group.commits.push(step.oid.clone());
            }
        }
    }
    Ok(groups)
}

fn ensure_clean(repo: &Repository) -> Result<(), String> {
    let mut options = git2::StatusOptions::new();
    options.include_untracked(false);
    options.include_ignored(false);
    let statuses = repo
        .statuses(Some(&mut options))
        .map_err(|_| "Could not read the working tree.".to_string())?;
    if !statuses.is_empty() {
        return Err("Commit or stash your changes before rebasing.".to_string());
    }
    Ok(())
}

/// Copies a merge index (detached from the repository) into the real index,
/// then checks it out with conflict markers so the resolver can edit it.
fn stage_conflicts(repo: &Repository, index: &git2::Index) -> Result<(), String> {
    let mut repo_index = repo
        .index()
        .map_err(|_| "Could not read the index.".to_string())?;
    repo_index
        .clear()
        .map_err(|_| "Could not stage the conflicts.".to_string())?;
    for entry in index.iter() {
        // Conflict stages live in the entry flags, so adding every entry
        // rebuilds the conflicted index.
        repo_index
            .add(&entry)
            .map_err(|_| "Could not stage the conflicts.".to_string())?;
    }
    repo_index
        .write()
        .map_err(|_| "Could not stage the conflicts.".to_string())?;

    let mut checkout = git2::build::CheckoutBuilder::new();
    checkout.allow_conflicts(true).conflict_style_merge(true);
    repo.checkout_index(Some(&mut repo_index), Some(&mut checkout))
        .map_err(|_| "Could not write the conflict markers.".to_string())?;
    Ok(())
}

/// Applies the group's commits in order on top of HEAD as one commit.
///
/// Each commit's own diff is merged into an in-memory tree, which keeps a
/// reordered group faithful: squashing a commit into a later one applies that
/// commit's change, not the reverse of it.
fn apply_group(
    repo: &Repository,
    group: &StoredGroup,
    signing: Option<&SigningConfig>,
) -> Result<Applied, String> {
    let parse = |text: &str| {
        Oid::from_str(text).map_err(|_| "Could not read the rebase plan.".to_string())
    };
    let head = repo
        .head()
        .and_then(|head| head.peel_to_tree())
        .map_err(|_| "No commits on the current branch.".to_string())?;
    let head_id = head.id();
    let mut current = head;

    for oid in &group.commits {
        let commit = repo
            .find_commit(parse(oid)?)
            .map_err(|_| "Could not read the rebase plan.".to_string())?;
        let ancestor = match commit.parent(0) {
            Ok(parent) => parent
                .tree()
                .map_err(|_| "Could not read the rebase plan.".to_string())?,
            Err(_) => empty_tree(repo)?,
        };
        let their = commit
            .tree()
            .map_err(|_| "Could not read the rebase plan.".to_string())?;

        let mut index = repo
            .merge_trees(&ancestor, &current, &their, None)
            .map_err(|_| "Could not apply a commit during the rebase.".to_string())?;
        if index.has_conflicts() {
            stage_conflicts(repo, &index)?;
            return Ok(Applied::Conflicts);
        }
        let tree_oid = index
            .write_tree_to(repo)
            .map_err(|_| "Could not read the rebased changes.".to_string())?;
        current = repo
            .find_tree(tree_oid)
            .map_err(|_| "Could not read the rebased changes.".to_string())?;
    }

    // The change is already present on the target, so there is nothing left to
    // commit. Git drops these too during a rebase.
    if current.id() == head_id {
        return Ok(Applied::Skipped);
    }

    let parent = head_commit(repo)?;
    let oid = commit::commit_at_head(repo, &group.message, &current, &[&parent], signing)?;

    // commit_at_head only moves the branch, so bring the index and working
    // tree along before the next group (or a conflict) uses them.
    let object = repo
        .find_object(oid, None)
        .map_err(|_| "Could not update the working tree.".to_string())?;
    repo.reset(&object, git2::ResetType::Hard, None)
        .map_err(|_| "Could not update the working tree.".to_string())?;
    Ok(Applied::Committed)
}

/// Applies groups from `state.group_index` onwards. Returns `true` when every
/// group was applied and `false` when a conflict stopped the rebase.
fn apply_remaining(
    repo: &Repository,
    state: &mut RebaseState,
    signing: Option<&SigningConfig>,
) -> Result<bool, String> {
    while state.group_index < state.groups.len() {
        let group = state.groups[state.group_index].clone();
        match apply_group(repo, &group, signing)? {
            Applied::Conflicts => {
                state.pending = Some(group.message);
                save_state(repo, state)?;
                return Ok(false);
            }
            Applied::Committed | Applied::Skipped => {
                state.group_index += 1;
                state.pending = None;
                save_state(repo, state)?;
            }
        }
    }
    Ok(true)
}

fn restore(repo: &Repository, state: &RebaseState) -> Result<(), String> {
    let oid = Oid::from_str(&state.original_head)
        .map_err(|_| "Could not restore the branch.".to_string())?;
    let object = repo
        .find_object(oid, None)
        .map_err(|_| "Could not restore the branch.".to_string())?;
    repo.reset(&object, git2::ResetType::Hard, None)
        .map_err(|_| "Could not restore the branch.".to_string())?;
    Ok(())
}

/// Replays the current branch onto `onto` following the requested steps.
///
/// Returns `"completed"`, or `"conflicts"` when the rebase stopped and the
/// operation panel should take over. The original head, target and remaining
/// groups are written to the state file so a later continue or abort can
/// resume or undo the work.
pub fn run(
    path: &str,
    onto: &str,
    steps: &[RebaseStep],
    signing: Option<&SigningConfig>,
) -> Result<&'static str, String> {
    let repo = open(path)?;
    if has_state(path) {
        return Err("Finish or abandon the current rebase first.".to_string());
    }
    if repo.state() != git2::RepositoryState::Clean {
        return Err("Finish or abandon the current operation first.".to_string());
    }
    let head = repo
        .find_reference("HEAD")
        .map_err(|_| "Could not read HEAD.".to_string())?;
    if head.symbolic_target().is_none() {
        return Err("Check out a branch before rebasing.".to_string());
    }
    ensure_clean(&repo)?;

    let target = branch_commit(&repo, onto)?;
    let allowed: HashSet<Oid> = plan_oids(&repo, target.id())?.into_iter().collect();
    let groups = build_groups(&repo, steps, &allowed)?;

    let original_head = head_commit(&repo)?.id();
    let mut state = RebaseState {
        original_head: original_head.to_string(),
        onto: target.id().to_string(),
        groups,
        group_index: 0,
        pending: None,
    };
    // Save before moving anything so abort works even if the first group
    // fails unexpectedly.
    save_state(&repo, &state)?;

    let object = repo
        .find_object(target.id(), None)
        .map_err(|_| "Could not start the rebase.".to_string())?;
    if repo
        .reset(&object, git2::ResetType::Hard, None)
        .is_err()
    {
        let _ = clear_state(&repo);
        return Err("Could not start the rebase.".to_string());
    }

    match apply_remaining(&repo, &mut state, signing) {
        Ok(true) => {
            clear_state(&repo)?;
            Ok("completed")
        }
        Ok(false) => Ok("conflicts"),
        Err(error) => {
            let _ = restore(&repo, &state);
            let _ = clear_state(&repo);
            Err(error)
        }
    }
}

/// Continues an interactive rebase that stopped at a conflict, finishing the
/// current group from the resolved index first.
pub fn continue_rebase(
    path: &str,
    signing: Option<&SigningConfig>,
) -> Result<&'static str, String> {
    let repo = open(path)?;
    let Some(mut state) = load_state(&repo)? else {
        return Err("There is no rebase in progress.".to_string());
    };

    if let Some(message) = state.pending.take() {
        let mut index = repo
            .index()
            .map_err(|_| "Could not read the index.".to_string())?;
        if index.has_conflicts() {
            return Err("Resolve every conflict before continuing.".to_string());
        }
        let tree_oid = index
            .write_tree_to(&repo)
            .map_err(|_| "Could not read the resolved changes.".to_string())?;
        let tree = repo
            .find_tree(tree_oid)
            .map_err(|_| "Could not read the resolved changes.".to_string())?;
        let parent = head_commit(&repo)?;
        commit::commit_at_head(&repo, &message, &tree, &[&parent], signing)?;
        state.group_index += 1;
        save_state(&repo, &state)?;
    }

    match apply_remaining(&repo, &mut state, signing) {
        Ok(true) => {
            clear_state(&repo)?;
            Ok("completed")
        }
        Ok(false) => Ok("conflicts"),
        // Leave the state in place: the user can resolve and retry in the
        // dialog or abandon the rebase from the panel.
        Err(error) => Err(error),
    }
}

/// Throws away an interactive rebase and returns the branch to where it was.
pub fn abort_rebase(path: &str) -> Result<(), String> {
    let repo = open(path)?;
    let Some(state) = load_state(&repo)? else {
        return Err("There is no rebase in progress.".to_string());
    };
    restore(&repo, &state)?;
    clear_state(&repo)?;
    Ok(())
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
            "knitt-interactive-{name}-{}-{}",
            std::process::id(),
            n
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn init(dir: &Path) -> Repository {
        let repo = Repository::init(dir).unwrap();
        {
            let mut config = repo.config().unwrap();
            config.set_str("user.name", "Test").unwrap();
            config.set_str("user.email", "test@example.com").unwrap();
        }
        repo
    }

    fn commit(repo: &Repository, message: &str, files: &[(&str, &str)]) -> git2::Oid {
        for (name, content) in files {
            std::fs::write(repo.workdir().unwrap().join(name), content).unwrap();
            let mut index = repo.index().unwrap();
            index.add_path(Path::new(name)).unwrap();
            index.write().unwrap();
        }
        let mut index = repo.index().unwrap();
        let tree_id = index.write_tree().unwrap();
        let tree = repo.find_tree(tree_id).unwrap();
        let sig = repo.signature().unwrap();
        let parent = repo.head().ok().and_then(|head| head.peel_to_commit().ok());
        let parents: Vec<&git2::Commit> = parent.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &parents)
            .unwrap()
    }

    fn switch(repo: &Repository, refname: &str) {
        let object = repo.revparse_single(refname).unwrap();
        let mut checkout = git2::build::CheckoutBuilder::new();
        checkout.force();
        repo.checkout_tree(&object, Some(&mut checkout)).unwrap();
        repo.set_head(refname).unwrap();
    }

    /// Builds a repository with `topic` (three commits) diverged from the
    /// branch that is checked out before this returns.
    struct Fixture {
        dir: PathBuf,
        onto: String,
        topic: Vec<git2::Oid>,
        onto_tip: git2::Oid,
    }

    impl Fixture {
        fn new(name: &str) -> Fixture {
            let dir = scratch(name);
            let repo = init(&dir);
            commit(&repo, "base", &[("base.txt", "base\n")]);
            let onto = repo.head().unwrap().shorthand().unwrap().to_string();
            {
                let head = repo.head().unwrap().peel_to_commit().unwrap();
                repo.branch("topic", &head, false).unwrap();
            }
            switch(&repo, "refs/heads/topic");
            let topic = vec![
                commit(&repo, "first", &[("one.txt", "1\n")]),
                commit(&repo, "second", &[("two.txt", "2\n")]),
                commit(&repo, "third", &[("three.txt", "3\n")]),
            ];
            switch(&repo, &format!("refs/heads/{onto}"));
            let onto_tip = commit(&repo, "onto work", &[("onto.txt", "o\n")]);
            switch(&repo, "refs/heads/topic");
            drop(repo);
            Fixture {
                dir,
                onto,
                topic,
                onto_tip,
            }
        }

        fn path(&self) -> &str {
            self.dir.to_str().unwrap()
        }

        fn step(&self, index: usize, action: RebaseAction) -> RebaseStep {
            RebaseStep {
                oid: self.topic[index].to_string(),
                action,
                message: None,
            }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    fn messages(repo: &Repository) -> Vec<String> {
        let mut walk = repo.revwalk().unwrap();
        walk.push_head().unwrap();
        walk.map(|oid| {
            repo.find_commit(oid.unwrap())
                .unwrap()
                .message()
                .unwrap_or_default()
                .trim()
                .to_string()
        })
        .collect()
    }

    #[test]
    fn plan_lists_commits_oldest_first() {
        let fixture = Fixture::new("plan");
        let plan = plan(fixture.path(), &fixture.onto).unwrap();
        let summaries: Vec<&str> = plan.iter().map(|c| c.summary.as_str()).collect();
        assert_eq!(summaries, vec!["first", "second", "third"]);
        assert_eq!(plan[0].author, "Test <test@example.com>");
        assert!(plan.iter().all(|c| c.time > 0));
        assert_eq!(plan[0].message.trim(), "first");
    }

    #[test]
    fn reorders_and_drops_commits() {
        let fixture = Fixture::new("reorder");
        let steps = vec![
            fixture.step(2, RebaseAction::Pick),
            fixture.step(0, RebaseAction::Pick),
            fixture.step(1, RebaseAction::Drop),
        ];
        assert_eq!(
            run(fixture.path(), &fixture.onto, &steps, None).unwrap(),
            "completed"
        );

        let repo = Repository::open(&fixture.dir).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(head.message().unwrap_or_default().trim(), "first");
        assert_eq!(
            head.parent(0).unwrap().message().unwrap_or_default().trim(),
            "third"
        );
        assert_eq!(head.parent(0).unwrap().parent(0).unwrap().id(), fixture.onto_tip);
        assert!(fixture.dir.join("three.txt").is_file());
        assert!(fixture.dir.join("one.txt").is_file());
        assert!(!fixture.dir.join("two.txt").exists(), "unpicked commit is gone");
        assert!(!has_state(fixture.path()), "state is cleared on completion");
    }

    #[test]
    fn squash_folds_the_messages_into_one_commit() {
        let fixture = Fixture::new("squash");
        let steps = vec![
            fixture.step(0, RebaseAction::Pick),
            fixture.step(1, RebaseAction::Squash),
            fixture.step(2, RebaseAction::Squash),
        ];
        assert_eq!(
            run(fixture.path(), &fixture.onto, &steps, None).unwrap(),
            "completed"
        );

        let repo = Repository::open(&fixture.dir).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(head.message().unwrap_or_default().trim(), "first\n\nsecond\n\nthird");
        assert_eq!(head.parent_count(), 1);
        assert_eq!(head.parent(0).unwrap().id(), fixture.onto_tip);
        for file in ["one.txt", "two.txt", "three.txt"] {
            assert!(fixture.dir.join(file).is_file(), "{file} is kept");
        }
    }

    #[test]
    fn fixup_keeps_only_the_first_message() {
        let fixture = Fixture::new("fixup");
        let steps = vec![
            fixture.step(0, RebaseAction::Pick),
            fixture.step(1, RebaseAction::Fixup),
        ];
        assert_eq!(
            run(fixture.path(), &fixture.onto, &steps, None).unwrap(),
            "completed"
        );

        let repo = Repository::open(&fixture.dir).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(head.message().unwrap_or_default().trim(), "first");
        assert!(fixture.dir.join("two.txt").is_file());
        assert!(!fixture.dir.join("three.txt").exists(), "third was dropped");
    }

    #[test]
    fn squash_after_reorder_applies_each_change() {
        let fixture = Fixture::new("squash-reordered");
        // Third comes first in the rebase, then the older first commit is
        // folded into it. Both commits' changes must survive.
        let steps = vec![
            fixture.step(2, RebaseAction::Pick),
            fixture.step(0, RebaseAction::Squash),
        ];
        assert_eq!(
            run(fixture.path(), &fixture.onto, &steps, None).unwrap(),
            "completed"
        );

        let repo = Repository::open(&fixture.dir).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(head.message().unwrap_or_default().trim(), "third\n\nfirst");
        assert_eq!(head.parent_count(), 1);
        assert!(fixture.dir.join("three.txt").is_file());
        assert!(fixture.dir.join("one.txt").is_file());
        assert!(!fixture.dir.join("two.txt").exists(), "second was dropped");
    }

    #[test]
    fn reword_replaces_the_message() {
        let fixture = Fixture::new("reword");
        let mut reworded = fixture.step(1, RebaseAction::Reword);
        reworded.message = Some("a better message".to_string());
        let steps = vec![
            fixture.step(0, RebaseAction::Pick),
            reworded,
            fixture.step(2, RebaseAction::Pick),
        ];
        assert_eq!(
            run(fixture.path(), &fixture.onto, &steps, None).unwrap(),
            "completed"
        );

        let repo = Repository::open(&fixture.dir).unwrap();
        assert_eq!(
            messages(&repo),
            vec!["third", "a better message", "first", "onto work", "base"]
        );
        assert!(fixture.dir.join("two.txt").is_file());
    }

    #[test]
    fn dropping_everything_lands_on_the_target() {
        let fixture = Fixture::new("drop-all");
        let steps = vec![
            fixture.step(0, RebaseAction::Drop),
            fixture.step(1, RebaseAction::Drop),
            fixture.step(2, RebaseAction::Drop),
        ];
        assert_eq!(
            run(fixture.path(), &fixture.onto, &steps, None).unwrap(),
            "completed"
        );

        let repo = Repository::open(&fixture.dir).unwrap();
        assert_eq!(repo.head().unwrap().target().unwrap(), fixture.onto_tip);
        assert!(!fixture.dir.join("one.txt").exists());
        assert!(!has_state(fixture.path()));
    }

    #[test]
    fn an_empty_pick_is_skipped() {
        let dir = scratch("empty-pick");
        let repo = init(&dir);
        commit(&repo, "base", &[("shared.txt", "base\n")]);
        let onto = repo.head().unwrap().shorthand().unwrap().to_string();
        {
            let head = repo.head().unwrap().peel_to_commit().unwrap();
            repo.branch("topic", &head, false).unwrap();
        }
        switch(&repo, "refs/heads/topic");
        let duplicate = commit(&repo, "same change", &[("shared.txt", "same\n")]);
        switch(&repo, &format!("refs/heads/{onto}"));
        let onto_tip = commit(&repo, "already applied", &[("shared.txt", "same\n")]);
        switch(&repo, "refs/heads/topic");
        drop(repo);

        let steps = vec![RebaseStep {
            oid: duplicate.to_string(),
            action: RebaseAction::Pick,
            message: None,
        }];
        assert_eq!(run(dir.to_str().unwrap(), &onto, &steps, None).unwrap(), "completed");

        let repo = Repository::open(&dir).unwrap();
        assert_eq!(
            repo.head().unwrap().target().unwrap(),
            onto_tip,
            "nothing new was committed"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_conflict_can_be_resolved_and_continued() {
        let dir = scratch("conflict-continue");
        let repo = init(&dir);
        commit(&repo, "base", &[("shared.txt", "base\n")]);
        let onto = repo.head().unwrap().shorthand().unwrap().to_string();
        {
            let head = repo.head().unwrap().peel_to_commit().unwrap();
            repo.branch("topic", &head, false).unwrap();
        }
        switch(&repo, "refs/heads/topic");
        let picked = commit(&repo, "topic side", &[("shared.txt", "topic\n")]);
        switch(&repo, &format!("refs/heads/{onto}"));
        let onto_tip = commit(&repo, "onto side", &[("shared.txt", "onto\n")]);
        switch(&repo, "refs/heads/topic");
        drop(repo);

        let steps = vec![RebaseStep {
            oid: picked.to_string(),
            action: RebaseAction::Pick,
            message: None,
        }];
        assert_eq!(
            run(dir.to_str().unwrap(), &onto, &steps, None).unwrap(),
            "conflicts"
        );
        assert!(has_state(dir.to_str().unwrap()));
        assert_eq!(
            crate::git::merge::operation_state(dir.to_str().unwrap()).unwrap(),
            "rebase"
        );
        assert_eq!(
            crate::git::conflicts::list_conflicts(dir.to_str().unwrap()).unwrap(),
            vec!["shared.txt".to_string()]
        );

        crate::git::conflicts::resolve_conflict(dir.to_str().unwrap(), "shared.txt", "both\n")
            .unwrap();
        assert_eq!(
            continue_rebase(dir.to_str().unwrap(), None).unwrap(),
            "completed"
        );

        let repo = Repository::open(&dir).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        assert_eq!(head.message().unwrap_or_default().trim(), "topic side");
        assert_eq!(head.parent(0).unwrap().id(), onto_tip);
        assert_eq!(
            std::fs::read_to_string(dir.join("shared.txt")).unwrap(),
            "both\n"
        );
        assert!(!has_state(dir.to_str().unwrap()));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_conflict_can_be_aborted() {
        let dir = scratch("conflict-abort");
        let repo = init(&dir);
        commit(&repo, "base", &[("shared.txt", "base\n")]);
        let onto = repo.head().unwrap().shorthand().unwrap().to_string();
        {
            let head = repo.head().unwrap().peel_to_commit().unwrap();
            repo.branch("topic", &head, false).unwrap();
        }
        switch(&repo, "refs/heads/topic");
        let picked = commit(&repo, "topic side", &[("shared.txt", "topic\n")]);
        let topic_tip = repo.head().unwrap().target().unwrap();
        switch(&repo, &format!("refs/heads/{onto}"));
        commit(&repo, "onto side", &[("shared.txt", "onto\n")]);
        switch(&repo, "refs/heads/topic");
        drop(repo);

        let steps = vec![RebaseStep {
            oid: picked.to_string(),
            action: RebaseAction::Pick,
            message: None,
        }];
        assert_eq!(
            run(dir.to_str().unwrap(), &onto, &steps, None).unwrap(),
            "conflicts"
        );
        assert_eq!(
            crate::git::merge::operation_state(dir.to_str().unwrap()).unwrap(),
            "rebase"
        );
        abort_rebase(dir.to_str().unwrap()).unwrap();

        assert_eq!(
            crate::git::merge::operation_state(dir.to_str().unwrap()).unwrap(),
            "none"
        );
        let repo = Repository::open(&dir).unwrap();
        assert_eq!(repo.head().unwrap().target().unwrap(), topic_tip);
        assert_eq!(
            std::fs::read_to_string(dir.join("shared.txt")).unwrap(),
            "topic\n"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn bad_plans_are_refused_without_moving_the_branch() {
        let fixture = Fixture::new("validation");
        let before = Repository::open(&fixture.dir)
            .unwrap()
            .head()
            .unwrap()
            .target()
            .unwrap();

        let out_of_range = RebaseStep {
            oid: fixture.onto_tip.to_string(),
            action: RebaseAction::Pick,
            message: None,
        };
        let duplicate = vec![
            fixture.step(0, RebaseAction::Pick),
            fixture.step(0, RebaseAction::Pick),
        ];
        let mut empty_reword = fixture.step(1, RebaseAction::Reword);
        empty_reword.message = Some("   ".to_string());

        let cases: Vec<(Vec<RebaseStep>, &str)> = vec![
            (Vec::new(), "Choose at least one commit to rebase."),
            (
                vec![fixture.step(0, RebaseAction::Squash)],
                "The first kept commit cannot be squashed or fixed up.",
            ),
            (
                duplicate,
                "Each commit can only appear once in the rebase.",
            ),
            (
                vec![out_of_range],
                "That commit is not part of this rebase.",
            ),
            (
                vec![empty_reword],
                "Every reworded commit needs a message.",
            ),
        ];
        for (steps, expected) in cases {
            assert_eq!(
                run(fixture.path(), &fixture.onto, &steps, None).unwrap_err(),
                expected
            );
            assert!(
                !has_state(fixture.path()),
                "a refused plan leaves no state"
            );
        }

        let repo = Repository::open(&fixture.dir).unwrap();
        assert_eq!(repo.head().unwrap().target().unwrap(), before);
    }

    #[test]
    fn dirty_worktree_is_refused() {
        let fixture = Fixture::new("dirty");
        std::fs::write(fixture.dir.join("base.txt"), "changed\n").unwrap();

        let steps = vec![fixture.step(0, RebaseAction::Pick)];
        assert_eq!(
            run(fixture.path(), &fixture.onto, &steps, None).unwrap_err(),
            "Commit or stash your changes before rebasing."
        );
        let _ = std::fs::remove_file(fixture.dir.join("base.txt"));
    }

    #[test]
    fn detached_head_is_refused() {
        let dir = scratch("detached");
        let repo = init(&dir);
        let base = commit(&repo, "base", &[("a.txt", "a\n")]);
        repo.set_head_detached(base).unwrap();
        drop(repo);

        let steps = vec![RebaseStep {
            oid: base.to_string(),
            action: RebaseAction::Pick,
            message: None,
        }];
        assert_eq!(
            run(dir.to_str().unwrap(), "main", &steps, None).unwrap_err(),
            "Check out a branch before rebasing."
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn continue_and_abort_need_a_rebase() {
        let dir = scratch("no-state");
        let repo = init(&dir);
        commit(&repo, "base", &[("a.txt", "a\n")]);
        drop(repo);

        assert_eq!(
            continue_rebase(dir.to_str().unwrap(), None).unwrap_err(),
            "There is no rebase in progress."
        );
        assert_eq!(
            abort_rebase(dir.to_str().unwrap()).unwrap_err(),
            "There is no rebase in progress."
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn ssh_keygen_available() -> bool {
        std::process::Command::new("ssh-keygen")
            .arg("-?")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .map(|status| status.success() || status.code().is_some())
            .unwrap_or(false)
    }

    #[test]
    fn rewritten_commits_can_be_signed() {
        if !ssh_keygen_available() {
            return;
        }
        let fixture = Fixture::new("signed");
        let key = fixture.dir.join("signing_key");
        let generated = std::process::Command::new("ssh-keygen")
            .args(["-q", "-t", "ed25519", "-N", "", "-f"])
            .arg(&key)
            .status()
            .unwrap();
        assert!(generated.success());

        let config = SigningConfig {
            mode: crate::git::signing::SigningMode::Ssh,
            key: key.display().to_string(),
        };
        let steps = vec![fixture.step(0, RebaseAction::Pick)];
        assert_eq!(
            run(fixture.path(), &fixture.onto, &steps, Some(&config)).unwrap(),
            "completed"
        );

        let repo = Repository::open(&fixture.dir).unwrap();
        let head = repo.head().unwrap().peel_to_commit().unwrap();
        let (signature, _) = repo.extract_signature(&head.id(), None).unwrap();
        assert!(
            std::str::from_utf8(&signature)
                .unwrap_or_default()
                .starts_with("-----BEGIN SSH SIGNATURE-----"),
            "the rewritten commit is signed"
        );
    }
}
