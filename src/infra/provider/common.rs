use anyhow::Result;
use std::path::Path;

pub fn copy_dir(src: &Path, dest: &Path) -> Result<()> {
    copy_dir_filtered(src, dest, |_| true)
}

/// Copy a directory tree with a per-entry filter. `should_copy` receives the
/// path relative to `src`. Returning `false` skips the entry (and its
/// children) entirely.
pub fn copy_dir_filtered<F>(src: &Path, dest: &Path, should_copy: F) -> Result<()>
where
    F: Fn(&std::path::Path) -> bool,
{
    if dest.exists() {
        let _ = std::fs::remove_dir_all(dest);
    }
    std::fs::create_dir_all(dest)?;
    for entry in walkdir::WalkDir::new(src).min_depth(1).follow_links(false) {
        let entry = entry?;
        if entry.file_type().is_symlink() {
            continue;
        }
        let rel = entry.path().strip_prefix(src)?;
        if !should_copy(rel) {
            continue;
        }
        let target = dest.join(rel);
        if entry.file_type().is_dir() {
            std::fs::create_dir_all(&target)?;
        } else {
            std::fs::copy(entry.path(), &target)?;
        }
    }
    Ok(())
}

/// Returns `true` if `rel` is **not** inside a top-level `evals/` directory.
/// Use with `copy_dir_filtered` to exclude evaluation sub-folders.
pub fn is_not_evals(rel: &std::path::Path) -> bool {
    !rel.components().next().map_or(
        false,
        |c| matches!(c, std::path::Component::Normal(s) if s == "evals"),
    )
}

/// Remove a directory and prune empty parent directories up to `max_parent_levels`
/// levels above the removed directory. A `max_parent_levels` of 1 removes the
/// asset directory and then prunes its immediate parent (e.g. `skills/` or
/// `instructions/`) if that becomes empty, but never goes further up the tree.
pub fn remove_dir_and_prune_empty_parents(dir: &Path, max_parent_levels: usize) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    std::fs::remove_dir_all(dir)?;
    let mut current = dir;
    for _ in 0..max_parent_levels {
        if let Some(parent) = current.parent() {
            if parent.exists() && is_dir_empty(parent)? {
                std::fs::remove_dir(parent)?;
            }
            current = parent;
        } else {
            break;
        }
    }
    Ok(())
}

fn is_dir_empty(path: &Path) -> Result<bool> {
    Ok(std::fs::read_dir(path)?.next().is_none())
}
