//! Lands the checked-out commit on main: the two desktop gates, then a
//! fast-forward push of exactly the commit they passed. The rule since
//! 08.10.2026 (`docs/releasing.md` §"Landing on main"): a change lands after
//! the local gates and does not wait for CI; CI runs on the main push, and a
//! release needs that run green.
//!
//! What it refuses before spending a gate's minutes: uncommitted changes to
//! tracked files (the gates would measure something else), and a HEAD that
//! cannot merge with `origin/main` without a conflict (`git merge-tree`,
//! nothing written).
//!
//! Landing is serial (09.10.2026). Everything from the merge to the push runs
//! under the machine's cargo lock (`BAYLEE_CARGO_LOCK`, default
//! `~/.baylee-locks/with-cargo-lock.sh`): once the lock is ours, `origin/main`
//! is fetched again and, if it moved while we waited, merged into HEAD (a
//! merge commit; a conflict aborts and stops). After a merge `cargo dev` is
//! rebuilt from the merged tree, so main's gate steps judge it, not the
//! branch's. The gates run on that commit and it is pushed, still under the
//! lock, so the next lander gates on top of this push instead of racing it. A push from outside moving main during the
//! gates makes git refuse ours (never `--force`). `--check` never merges.
use crate::{features, gate, run};
use anyhow::{Result, bail};
use std::{path::PathBuf, process::Command, time::Instant};

/// Runs the landing; `locked` is the part that runs under the lock.
pub fn run(check: bool, locked: bool) -> Result<i32> {
    run::require_env("DATABASE_URL", "the full and feature gates need Postgres")?;
    let push = !check;
    let branch = git_out(&["symbolic-ref", "--quiet", "--short", "HEAD"])
        .unwrap_or_else(|_| "(detached)".into());
    if locked {
        return under_lock(push, &branch);
    }
    clean()?;
    if !behind()? && rev("HEAD")? == rev("origin/main")? {
        bail!("nothing to land: HEAD is origin/main");
    }
    if push {
        mergeable()?;
    }
    let untracked = git_out(&[
        "ls-files",
        "--others",
        "--exclude-standard",
        "--",
        "crates",
        "xtask",
        "Cargo.toml",
        "Cargo.lock",
    ])?;
    if !untracked.is_empty() {
        let first: Vec<&str> = untracked.lines().take(5).collect();
        eprintln!(
            "land: note - untracked files beside the code (not part of the commit): {}",
            first.join(" ")
        );
    }

    let me = std::env::current_exe()?;
    let mut again = match lock_script() {
        Some(lock) => {
            let mut c = Command::new(lock);
            c.arg(&me);
            c
        }
        None => Command::new(&me),
    };
    again.args(["land", "--locked"]);
    if check {
        again.arg("--check");
    }
    if std::env::var_os("BAYLEE_SESSION").is_none() {
        again.env("BAYLEE_SESSION", format!("land {branch}"));
    }
    Ok(run::code(run::status(&mut again)))
}

fn under_lock(push: bool, branch: &str) -> Result<i32> {
    clean()?;
    if push && behind()? {
        mergeable()?;
        if !run::succeeds(Command::new("git").args([
            "merge",
            "--quiet",
            "--no-edit",
            "origin/main",
        ])) {
            let _ = Command::new("git").args(["merge", "--abort"]).status();
            bail!("merging origin/main failed; nothing pushed");
        }
        println!(
            "land: merged origin/main ({}) into {branch}",
            short("origin/main")?
        );
        // The gates are this binary, built from HEAD before the merge, so a
        // check main changed since (a reviewed pin, a new step) would judge
        // the merged tree in its old form: on 31.10.2026 a trainer land went
        // red on main's Central pin. Rebuild the tool from the merged tree
        // and gate from there; the lock stays ours.
        println!("land: rebuilding cargo dev from the merged tree");
        return Ok(run::code(run::status(&mut run::cargo_with([
            "dev", "land", "--locked",
        ]))));
    }
    let head = rev("HEAD")?;
    if head == rev("origin/main")? {
        bail!("nothing to land: {head} is origin/main");
    }
    println!("land: gating {head} ({branch})");
    let started = Instant::now();
    if !gates()? {
        bail!("a gate is red; nothing pushed");
    }
    println!("land: gates green in {}s", started.elapsed().as_secs());
    if rev("HEAD")? != head {
        bail!("HEAD moved during the gates; run again");
    }
    clean()?;
    if !push {
        println!("land: --check: {head} passed the gates; not pushed");
        return Ok(0);
    }
    // Never --force: a push that is no fast-forward is refused by git itself.
    if !run::succeeds(Command::new("git").args([
        "push",
        "origin",
        &format!("{head}:refs/heads/main"),
    ])) {
        bail!("main moved during the gates by a push outside `cargo dev land`; run again");
    }
    println!("land: main is {head}; CI runs on it (a release needs that run green)");
    Ok(0)
}

/// Both desktop gates run even when one fails, so one run reports every red.
fn gates() -> Result<bool> {
    let timed = |name: &str, rc: i32, started: Instant| {
        println!("LAND {name} rc={rc} {}s", started.elapsed().as_secs());
        rc == 0
    };
    let started = Instant::now();
    let full = timed("gate", gate::full()?, started);
    let started = Instant::now();
    let features = timed("gate features", features::run(), started);
    Ok(full && features)
}

fn lock_script() -> Option<PathBuf> {
    let path = std::env::var_os("BAYLEE_CARGO_LOCK")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .map(|home| PathBuf::from(home).join(".baylee-locks/with-cargo-lock.sh"))
        })?;
    executable(&path).then_some(path)
}

#[cfg(unix)]
fn executable(path: &std::path::Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.metadata()
        .is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
}

#[cfg(not(unix))]
fn executable(path: &std::path::Path) -> bool {
    path.is_file()
}

fn clean() -> Result<()> {
    if !git_out(&["status", "--porcelain", "--untracked-files=no"])?.is_empty() {
        bail!("uncommitted changes to tracked files; commit or set them aside first");
    }
    Ok(())
}

/// Fetches `origin/main`; true when HEAD does not already contain it.
fn behind() -> Result<bool> {
    if !run::succeeds(Command::new("git").args(["fetch", "--quiet", "origin", "main"])) {
        bail!("fetching origin/main failed; nothing gated or pushed");
    }
    Ok(!run::succeeds(Command::new("git").args([
        "merge-base",
        "--is-ancestor",
        "origin/main",
        "HEAD",
    ])))
}

fn mergeable() -> Result<()> {
    let ok = Command::new("git")
        .args(["merge-tree", "--write-tree", "origin/main", "HEAD"])
        .output()
        .is_ok_and(|o| o.status.success());
    if !ok {
        bail!(
            "HEAD conflicts with origin/main ({}): merge or rebase, resolve, and run again",
            short("origin/main")?
        );
    }
    Ok(())
}

fn rev(name: &str) -> Result<String> {
    git_out(&["rev-parse", name])
}

fn short(name: &str) -> Result<String> {
    git_out(&["rev-parse", "--short", name])
}

fn git_out(args: &[&str]) -> Result<String> {
    let out = Command::new("git").args(args).output()?;
    if !out.status.success() {
        bail!("git {} failed", args.join(" "));
    }
    Ok(String::from_utf8(out.stdout)?.trim().to_owned())
}
