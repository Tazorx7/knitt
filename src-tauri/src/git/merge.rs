use git2::Repository;

pub fn merge_branch(path: &str, branch_name: &str) -> Result<(), String> {
    let repo = Repository::open(path).map_err(|e| e.to_string())?;
    let branch_ref = repo
        .find_reference(&format!("refs/heads/{}", branch_name))
        .map_err(|e| e.to_string())?;
    let annotated = repo
        .reference_to_annotated_commit(&branch_ref)
        .map_err(|e| e.to_string())?;
    let (analysis, _) = repo.merge_analysis(&[&annotated]).map_err(|e| e.to_string())?;
    if analysis.is_up_to_date() {
        return Ok(());
    }
    if analysis.is_fast_forward() {
        let head = repo.head().map_err(|e| e.to_string())?;
        let refname = head.name().unwrap_or("HEAD").to_string();
        let mut reference = repo.find_reference(&refname).map_err(|e| e.to_string())?;
        reference
            .set_target(annotated.id(), "merge: fast-forward")
            .map_err(|e| e.to_string())?;
        repo.checkout_head(Some(
            git2::build::CheckoutBuilder::default().force(),
        ))
        .map_err(|e| e.to_string())?;
        return Ok(());
    }
    if analysis.is_normal() {
        let head_ref = repo.head().map_err(|e| e.to_string())?;
        let head_annotated = repo
            .reference_to_annotated_commit(&head_ref)
            .map_err(|e| e.to_string())?;
        let mut merge_opts = git2::MergeOptions::new();
        let mut checkout_opts = git2::build::CheckoutBuilder::new();
        repo.merge(&[&annotated], Some(&mut merge_opts), Some(&mut checkout_opts))
            .map_err(|e| e.to_string())?;
        let mut index = repo.index().map_err(|e| e.to_string())?;
        if index.has_conflicts() {
            return Err("conflicts".to_string());
        }
        let sig = repo.signature().map_err(|e| e.to_string())?;
        let tree_id = index.write_tree_to(&repo).map_err(|e| e.to_string())?;
        let tree = repo.find_tree(tree_id).map_err(|e| e.to_string())?;
        let head_commit_obj = repo
            .find_commit(head_annotated.id())
            .map_err(|e| e.to_string())?;
        let merge_commit_obj = repo.find_commit(annotated.id()).map_err(|e| e.to_string())?;
        repo.commit(
            Some("HEAD"),
            &sig,
            &sig,
            "Merge commit",
            &tree,
            &[&head_commit_obj, &merge_commit_obj],
        )
        .map_err(|e| e.to_string())?;
        repo.cleanup_state().map_err(|e| e.to_string())?;
        return Ok(());
    }
    Ok(())
}
