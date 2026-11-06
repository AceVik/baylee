//! Finding `train_ops` execution directories below a root.
//!
//! The scan is shallow and bounded: the root itself, its children and its
//! grandchildren (`<root>/<experiment>/<execution>`), at most `MAX_CHILDREN`
//! entries per directory, symlinks never followed. An execution directory is
//! one with a `status.json` file and a `jobs` directory. Nothing is read
//! here; a candidate is only looked at by name and type. It runs on the
//! training thread every refresh, so a new run appears without a restart.

use super::safe::valid_name;
use std::{
    fs,
    path::{Path, PathBuf},
};

/// Directory entries read per level; a root with more is cut, not slow.
const MAX_CHILDREN: usize = 512;
/// Candidates kept from all roots together.
const MAX_FOUND: usize = 512;

pub(super) fn find(roots: &[PathBuf]) -> Vec<PathBuf> {
    let mut found = Vec::new();
    for root in roots {
        if is_execution(root) {
            found.push(root.clone());
        }
        for child in subdirectories(root) {
            if is_execution(&child) {
                found.push(child.clone());
            }
            for grandchild in subdirectories(&child) {
                if is_execution(&grandchild) {
                    found.push(grandchild);
                }
            }
        }
        if found.len() >= MAX_FOUND {
            break;
        }
    }
    found.truncate(MAX_FOUND);
    found
}

/// The real directories (not symlinks) directly below `dir`.
fn subdirectories(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .take(MAX_CHILDREN)
        .flatten()
        // `file_type` does not follow symlinks.
        .filter(|entry| entry.file_type().is_ok_and(|t| t.is_dir()))
        .filter(|entry| entry.file_name().to_str().is_some_and(valid_name))
        .map(|entry| entry.path())
        .collect()
}

fn is_execution(dir: &Path) -> bool {
    fs::symlink_metadata(dir.join("status.json")).is_ok_and(|m| m.is_file())
        && fs::symlink_metadata(dir.join("jobs")).is_ok_and(|m| m.is_dir())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::training::safe::Scratch;

    fn execution(scratch: &Scratch, at: &str) {
        scratch.write(&format!("{at}/status.json"), "{}");
        fs::create_dir_all(scratch.0.join(at).join("jobs")).unwrap();
    }

    #[test]
    fn finds_executions_one_and_two_levels_down_and_ignores_the_rest() {
        let scratch = Scratch::new("discover");
        execution(&scratch, "v6x/execution-a");
        execution(&scratch, "v6x/execution-b");
        execution(&scratch, "solo");
        execution(&scratch, "v6y/deeper/execution-c");
        scratch.write("v6x/plan.json", "{}");
        scratch.write("v6z/execution-no-jobs/status.json", "{}");
        fs::create_dir_all(scratch.0.join("v6z/jobs-only/jobs")).unwrap();
        let mut found = find(std::slice::from_ref(&scratch.0));
        found.sort();
        let names: Vec<_> = found
            .iter()
            .map(|p| p.strip_prefix(&scratch.0).unwrap().to_str().unwrap())
            .collect();
        assert_eq!(names, ["solo", "v6x/execution-a", "v6x/execution-b"]);
    }

    #[test]
    fn a_root_that_is_itself_an_execution_counts_and_symlinks_do_not() {
        let scratch = Scratch::new("discover-links");
        execution(&scratch, "real/execution");
        execution(&scratch, "outside/execution");
        std::os::unix::fs::symlink(scratch.0.join("outside"), scratch.0.join("real/linked"))
            .unwrap();
        let found = find(&[scratch.0.join("real")]);
        assert_eq!(found, [scratch.0.join("real/execution")]);
        let own = find(&[scratch.0.join("real/execution")]);
        assert_eq!(own, [scratch.0.join("real/execution")]);
        assert!(find(&[scratch.0.join("missing")]).is_empty());
    }
}
