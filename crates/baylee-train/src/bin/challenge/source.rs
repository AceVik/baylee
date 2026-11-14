//! Live publication accepts a clean, identified release and its exact source checkout.
use anyhow::{Context, Result, ensure};
use std::{path::Path, process::Command};

pub(super) fn verify_live(root: &Path) -> Result<()> {
    ensure!(
        !baylee_build::DIRTY
            && baylee_build::PROFILE == "release"
            && baylee_build::COMMIT.len() == 40,
        "live publication requires an identified clean release build"
    );
    verify_checkout(root, baylee_build::COMMIT)
}

fn verify_checkout(root: &Path, commit: &str) -> Result<()> {
    let head = git(root, &["rev-parse", "HEAD"])?;
    ensure!(
        head.trim() == commit,
        "source checkout does not match the certified executor commit"
    );
    let status = git(root, &["status", "--porcelain", "--untracked-files=normal"])?;
    ensure!(
        status.trim().is_empty(),
        "source checkout has unpublished changes"
    );
    Ok(())
}

fn git(root: &Path, args: &[&str]) -> Result<String> {
    let result = Command::new("git")
        .arg("-C")
        .arg(root)
        .args(args)
        .output()
        .context("inspect source checkout")?;
    ensure!(
        result.status.success(),
        "source checkout cannot be verified"
    );
    String::from_utf8(result.stdout).context("source identity is not UTF-8")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn live_evidence_refuses_a_different_commit_or_dirty_checkout() {
        let root = tempfile::tempdir().unwrap();
        git(root.path(), &["init", "--quiet"]).unwrap();
        git(
            root.path(),
            &[
                "-c",
                "user.name=Fixture",
                "-c",
                "user.email=fixture@example.invalid",
                "commit",
                "--allow-empty",
                "--quiet",
                "-m",
                "fixture",
            ],
        )
        .unwrap();
        let head = git(root.path(), &["rev-parse", "HEAD"]).unwrap();
        verify_checkout(root.path(), head.trim()).unwrap();
        assert!(verify_checkout(root.path(), &"0".repeat(40)).is_err());
        std::fs::write(root.path().join("unpublished.rs"), "// changed evidence\n").unwrap();
        assert!(verify_checkout(root.path(), head.trim()).is_err());
    }
}
