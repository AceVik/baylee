//! Release provenance checks and Windows executable icon verification.
//!
//! `release resolve|verify-run|write|verify|installer-sums` replaces
//! `scripts/release/ci_artifacts.py`, with the same positional arguments.
//! `release pe-icon FILE...` replaces `scripts/release/pe_icon.py`.
//! API reads use the authenticated `gh` CLI; these commands never publish,
//! tag, sign, or execute a downloaded package.

use anyhow::{Result, bail};
use std::{env, path::Path};

mod artifacts;
mod check;
mod pe;
mod policy;
#[cfg(test)]
mod tests;

const HELP: &str = "Usage: baylee-tools release <COMMAND>

Commands:
  resolve                         Find a trusted CI run for this commit
  verify-run ID                   Check CI identity, jobs and six artifacts
  write DIR VERSION TARGET        Write package manifest and SHA-256 sidecars
  verify DIR VERSION TARGET RUN   Verify exact package contents and provenance
  installer-sums DIR VERSION      Print all eight verified installer checksums
  pe-icon FILE...                 Verify Windows executable icon resources
  check-installers [--install] TARGET VERSION
                                  Open the installers and check what a player gets

CI identity uses GITHUB_REPOSITORY and GITHUB_SHA; write also uses GITHUB_RUN_ID.
CI queries use the authenticated gh CLI. Relative paths use the repository root.";

/// Runs a release verification command relative to the repository root.
///
/// # Errors
/// Returns an error for invalid arguments, failed validation, or failed I/O.
pub fn run(root: &Path, args: &[String]) -> Result<()> {
    let words: Vec<_> = args.iter().map(String::as_str).collect();
    match words.as_slice() {
        ["--help" | "-h"] => println!("{HELP}"),
        ["resolve"] => println!(
            "{}",
            policy::resolve(&policy::Identity::environment()?, &mut policy::gh)?
        ),
        ["verify-run", run] => {
            policy::verify_run(run, &policy::Identity::environment()?, &mut policy::gh)?;
        }
        ["write", directory, version, target] => artifacts::write(
            &root.join(directory),
            version,
            target,
            &env::var("GITHUB_RUN_ID")?,
            &policy::Identity::environment()?,
        )?,
        ["verify", directory, version, target, run] => artifacts::verify(
            &root.join(directory),
            version,
            target,
            run,
            &policy::Identity::environment()?,
        )?,
        ["installer-sums", directory, version] => println!(
            "{}",
            artifacts::installer_sums(&root.join(directory), version)?
        ),
        ["check-installers", "--install", target, version] => {
            check::run(root, true, target, version)?;
        }
        ["check-installers", target, version] => check::run(root, false, target, version)?,
        ["pe-icon", paths @ ..] if !paths.is_empty() => {
            for path in paths {
                pe::check_file(&root.join(path))?;
                println!("{path}: icon group 1 with 16, 24, 32, 48, 64, 128, 256 px");
            }
        }
        _ => bail!(
            "usage: release resolve|verify-run ID|write DIR VERSION TARGET|verify DIR VERSION TARGET RUN|installer-sums DIR VERSION|pe-icon FILE..."
        ),
    }
    Ok(())
}
