//! Stamps what this binary is into it, at compile time.
//!
//! Everything here has to answer even when the answer is "unknown": the
//! crate is compiled from a tarball with no `.git`, on a runner with no
//! `git` binary, and inside `cargo publish`, and a build script that fails
//! in any of those is a build script that breaks the build for a string
//! nobody reads. So every probe falls back to a word rather than to an
//! error, and the one field that is always true — the version — comes from
//! Cargo rather than from a process.

use std::process::Command;

fn main() {
    // A commit is only news when `HEAD` moves. Naming both the file and the
    // ref it points at covers the two shapes: a checkout on a branch, where
    // `HEAD` is a symref and the branch file is what changes on a commit,
    // and a detached checkout, where `HEAD` itself holds the hash.
    for path in watched() {
        println!("cargo:rerun-if-changed={path}");
    }
    println!("cargo:rerun-if-env-changed=GITHUB_RUN_NUMBER");
    println!("cargo:rerun-if-env-changed=SOURCE_DATE_EPOCH");

    emit("BAYLEE_COMMIT", &git(&["rev-parse", "HEAD"]));
    emit(
        "BAYLEE_COMMIT_SHORT",
        &git(&["rev-parse", "--short=10", "HEAD"]),
    );
    emit(
        "BAYLEE_BRANCH",
        &git(&["rev-parse", "--abbrev-ref", "HEAD"]),
    );

    // Dirty is a fact about the binary, not about the repository: a build
    // made from an edited tree is not the commit it names, and saying so is
    // the difference between a version string and a guess.
    let dirty = Command::new("git")
        .args(["status", "--porcelain", "--untracked-files=no"])
        .output()
        .is_ok_and(|out| out.status.success() && !out.stdout.is_empty());
    emit("BAYLEE_DIRTY", if dirty { "1" } else { "0" });

    emit("BAYLEE_BUILD_NUMBER", &build_number());
    emit("BAYLEE_BUILT_AT", &built_at());
    emit(
        "BAYLEE_TARGET",
        &std::env::var("TARGET").unwrap_or_else(|_| "unknown".into()),
    );
    emit(
        "BAYLEE_PROFILE",
        &std::env::var("PROFILE").unwrap_or_else(|_| "unknown".into()),
    );
}

/// The build number, which is deliberately **not** the commit count.
///
/// A count of commits is the obvious answer and it is the wrong one here:
/// the history is squashed from time to time, and a build number that walks
/// backwards is worse than none. A CI run number survives a rewritten
/// history because it belongs to the repository rather than to the commits,
/// so that is the first answer; the commit count is the local fallback, for
/// a developer build where the number is only ever compared with itself.
fn build_number() -> String {
    if let Ok(run) = std::env::var("GITHUB_RUN_NUMBER")
        && !run.is_empty()
    {
        return run;
    }
    let count = git(&["rev-list", "--count", "HEAD"]);
    if count == "unknown" {
        "0".into()
    } else {
        count
    }
}

/// When this was built, as a UTC date.
///
/// `SOURCE_DATE_EPOCH` wins where it is set, because a reproducible build
/// sets it precisely so that a timestamp stops being the one field that
/// differs between two builds of the same source. Without it the wall clock
/// answers, through `date` rather than through a dependency — this crate has
/// none, and one whole crate of transitive dependencies to print a date in a
/// splash line is a poor trade.
fn built_at() -> String {
    let epoch = std::env::var("SOURCE_DATE_EPOCH")
        .ok()
        .filter(|s| !s.is_empty());
    let mut args = vec!["-u"];
    let stamp;
    if let Some(seconds) = &epoch {
        stamp = format!("-d@{seconds}");
        args.push(&stamp);
    }
    args.push("+%Y-%m-%dT%H:%M:%SZ");
    Command::new("date")
        .args(&args)
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map_or_else(|| "unknown".into(), |text| text.trim().to_owned())
}

/// The files whose change means `HEAD` may name another commit.
///
/// Asked of git rather than spelled `../../.git/HEAD`, because in a linked
/// worktree `.git` is a file pointing elsewhere: a path cargo watches that
/// does not exist is one it calls changed on every build, so this crate and
/// everything that links it (the gateway, the engine-server, the client)
/// were rebuilt by every `cargo build` in a worktree — 24 s for a tree with
/// no change at all (06.10.2026). `HEAD` lives in the worktree's own git
/// directory, a branch's ref and `packed-refs` in the common one; only the
/// ref `HEAD` names is watched, so another branch's commit is no news here.
///
/// Without git the old paths stand, which is right for a plain checkout and
/// at worst a rebuild for a tarball, never a wrong commit.
fn watched() -> Vec<String> {
    let dir = git(&["rev-parse", "--absolute-git-dir"]);
    let common = git(&["rev-parse", "--path-format=absolute", "--git-common-dir"]);
    if dir == "unknown" || common == "unknown" {
        return vec!["../../.git/HEAD".into(), "../../.git/refs".into()];
    }
    let mut paths = vec![format!("{dir}/HEAD"), format!("{common}/packed-refs")];
    let head = git(&["symbolic-ref", "-q", "HEAD"]);
    if head.starts_with("refs/") {
        paths.push(format!("{common}/{head}"));
    }
    // Only what is there: a missing path would be the bug this replaces.
    // A ref that is packed lives in `packed-refs`, and one packed later
    // disappears from where it was watched, which reads as a change once.
    paths.retain(|path| std::path::Path::new(path).exists());
    paths
}

/// One `git` question, answered with "unknown" rather than with a failure.
fn git(args: &[&str]) -> String {
    Command::new("git")
        .args(args)
        .output()
        .ok()
        .filter(|out| out.status.success())
        .and_then(|out| String::from_utf8(out.stdout).ok())
        .map(|text| text.trim().to_owned())
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| "unknown".into())
}

fn emit(key: &str, value: &str) {
    println!("cargo:rustc-env={key}={value}");
}
