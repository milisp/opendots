use std::path::Path;
use std::process::Command;

/// FNV-1a. Hand-rolled because the key must stay byte-identical across Rust
/// versions — a changed hash would orphan the worktree a task has been reusing.
fn stable_hash(value: &str) -> String {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("{hash:016x}")
}

/// One stable worktree per task per project, so repeated runs land in the same
/// place for review instead of piling up a new directory every night.
pub(super) fn worktree_key(task_id: &str, project: &str) -> String {
    let name = Path::new(project)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repo");
    format!("{task_id}-{name}-{}", &stable_hash(project)[..8])
}

/// Returns the working directory the agent should run in.
///
/// A clean worktree is rebuilt so the run starts from the project's current HEAD.
/// One holding unreviewed changes is reused as-is: destroying output the user has
/// not looked at yet would be worse than working from a slightly stale tree.
pub(super) async fn prepare_worktree(task_id: &str, project: &str) -> Result<String, String> {
    let key = worktree_key(task_id, project);
    let project = project.to_string();
    tokio::task::spawn_blocking(move || {
        let root_output = Command::new("git")
            .args(["-C", &project, "rev-parse", "--show-toplevel"])
            .output()
            .map_err(|e| e.to_string())?;
        if !root_output.status.success() {
            return Err(String::from_utf8_lossy(&root_output.stderr).into_owned());
        }
        let root = Path::new(
            std::str::from_utf8(&root_output.stdout)
                .unwrap_or_default()
                .trim(),
        );
        let parent = root.parent().ok_or("repository has no parent directory")?;
        let destination = parent.join(format!(".opendots-worktree-{key}"));

        if destination.exists() {
            let status = Command::new("git")
                .args([
                    "-C",
                    destination.to_str().unwrap_or_default(),
                    "status",
                    "--porcelain",
                ])
                .output()
                .map_err(|e| e.to_string())?;
            if status.status.success() && !status.stdout.is_empty() {
                return Ok(destination.to_string_lossy().into_owned());
            }
            let _ = Command::new("git")
                .args([
                    "-C",
                    &project,
                    "worktree",
                    "remove",
                    "--force",
                    destination.to_str().unwrap_or_default(),
                ])
                .status();
        }

        let result = Command::new("git")
            .args([
                "-C",
                &project,
                "worktree",
                "add",
                "--detach",
                destination.to_str().unwrap_or_default(),
                "HEAD",
            ])
            .output()
            .map_err(|e| e.to_string())?;
        if !result.status.success() {
            return Err(String::from_utf8_lossy(&result.stderr).into_owned());
        }
        Ok(destination.to_string_lossy().into_owned())
    })
    .await
    .map_err(|err| err.to_string())?
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn worktree_key_is_stable_for_the_same_project() {
        let first = worktree_key("automation-1", "/Users/me/code/app");
        let second = worktree_key("automation-1", "/Users/me/code/app");
        assert_eq!(first, second);
        assert!(first.starts_with("automation-1-app-"));
    }

    #[test]
    fn worktree_key_differs_per_project_and_per_task() {
        // Same directory name in different paths must not collide.
        assert_ne!(
            worktree_key("automation-1", "/a/app"),
            worktree_key("automation-1", "/b/app")
        );
        assert_ne!(
            worktree_key("automation-1", "/a/app"),
            worktree_key("automation-2", "/a/app")
        );
    }
}
