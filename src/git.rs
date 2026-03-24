use anyhow::{Context, Result};
use git2::{Repository, Sort};
use std::path::{Path, PathBuf};

pub struct GitLog {
    pub hash: String,
    pub message: String,
}

pub fn open_repo(path: &Path) -> Result<Repository> {
    Repository::discover(path)
        .with_context(|| format!("Not a git repository: {}", path.display()))
}

pub fn get_repo_root(repo: &Repository) -> Result<PathBuf> {
    repo.workdir()
        .map(|p| p.to_path_buf())
        .ok_or_else(|| anyhow::anyhow!("Bare repositories are not supported"))
}

/// Get commits since the last tag matching `tag_prefix`, or all commits if none.
pub fn get_commits_since_last_tag(repo: &Repository, tag_prefix: &str) -> Result<Vec<GitLog>> {
    let last_tag_oid = find_last_tag_commit(repo, tag_prefix)?;

    let mut walk = repo.revwalk()?;
    walk.push_head()?;
    walk.set_sorting(Sort::TOPOLOGICAL | Sort::TIME)?;

    let mut commits = Vec::new();
    for oid in walk {
        let oid = oid?;
        if let Some(stop) = last_tag_oid {
            if oid == stop {
                break;
            }
        }
        if let Ok(commit) = repo.find_commit(oid) {
            let message = commit.message().unwrap_or("").to_string();
            if message.contains("[skip ci]") {
                continue;
            }
            commits.push(GitLog {
                hash: oid.to_string()[..8].to_string(),
                message,
            });
        }
    }

    Ok(commits)
}

fn find_last_tag_commit(repo: &Repository, prefix: &str) -> Result<Option<git2::Oid>> {
    let mut latest: Option<(i64, git2::Oid)> = None;

    repo.tag_foreach(|oid, name| {
        let name = String::from_utf8_lossy(name);
        let tag_name = name.trim_start_matches("refs/tags/");
        if tag_name.starts_with(prefix) {
            // Resolve tag to commit
            let commit_oid = if let Ok(tag_obj) = repo.find_tag(oid) {
                tag_obj.target_id()
            } else {
                oid
            };
            if let Ok(commit) = repo.find_commit(commit_oid) {
                let time = commit.time().seconds();
                if latest.is_none() || time > latest.unwrap().0 {
                    latest = Some((time, commit_oid));
                }
            }
        }
        true
    })?;

    Ok(latest.map(|(_, oid)| oid))
}

pub fn get_changed_files(repo: &Repository) -> Result<Vec<String>> {
    let head = match repo.head() {
        Ok(h) => h.peel_to_commit()?,
        Err(_) => return Ok(vec![]),
    };
    let head_tree = head.tree()?;

    let files = if let Ok(parent) = head.parent(0) {
        let parent_tree = parent.tree()?;
        let diff = repo.diff_tree_to_tree(Some(&parent_tree), Some(&head_tree), None)?;
        let mut files = Vec::new();
        diff.foreach(
            &mut |delta, _| {
                if let Some(path) = delta.new_file().path() {
                    files.push(path.to_string_lossy().to_string());
                }
                true
            },
            None,
            None,
            None,
        )?;
        files
    } else {
        // First commit
        let mut files = Vec::new();
        head_tree.walk(git2::TreeWalkMode::PreOrder, |_, entry| {
            if let Some(name) = entry.name() {
                files.push(name.to_string());
            }
            git2::TreeWalkResult::Ok
        })?;
        files
    };

    Ok(files)
}

pub fn create_tag(repo: &Repository, tag_name: &str, message: &str) -> Result<()> {
    let head = repo.head()?.peel_to_commit()?;
    let sig = repo.signature()?;
    repo.tag(tag_name, head.as_object(), &sig, message, false)?;
    Ok(())
}
