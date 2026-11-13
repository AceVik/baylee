//! Running other programs the way the gates need: from the checkout's root,
//! with every exit status read off the process itself and never off a
//! pipeline, and each step timed and reported on one `STEP` line.
use anyhow::{Context, Result, bail};
use std::{
    ffi::OsStr,
    path::PathBuf,
    process::{Command, ExitStatus, Output, Stdio},
    sync::OnceLock,
    time::Instant,
};

/// The checkout this runs in: git's top level of the current directory, so
/// a binary built in one worktree still works on the tree it is started in.
/// Every task changes into it first, as the scripts did with `cd`.
pub fn enter_root() -> Result<PathBuf> {
    let out = Command::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .context("running git")?;
    if !out.status.success() {
        bail!("not inside a git checkout");
    }
    let root = PathBuf::from(String::from_utf8(out.stdout)?.trim());
    std::env::set_current_dir(&root).with_context(|| format!("entering {}", root.display()))?;
    Ok(root)
}

/// `cargo`, as the caller's environment names it (`cargo dev` sets `CARGO`),
/// in the environment the caller's shell would give it.
///
/// `cargo dev` is `cargo run -p baylee-dev`, and cargo tells a program it
/// runs which package it is (`CARGO_PKG_NAME`, `CARGO_MANIFEST_DIR`, ...). A
/// cargo started from here inherited that, and a build script may ask to be
/// run again when such a variable changes: `ring`'s does. So a build under
/// `cargo dev gate` and the same build typed by hand each found the other's
/// `ring` stale and compiled it again with everything above it (rustls,
/// sqlx, the gateway, the client), in turn, for nothing (13.11.2026).
pub fn cargo() -> Command {
    let mut cargo = Command::new(std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into()));
    for (name, _) in std::env::vars_os() {
        if name.to_str().is_some_and(names_this_package) {
            cargo.env_remove(name);
        }
    }
    if SCCACHE.get().copied().unwrap_or(false) {
        cargo.env("RUSTC_WRAPPER", "sccache");
    }
    cargo
}

/// Whether this run's cargo children compile through sccache.
static SCCACHE: OnceLock<bool> = OnceLock::new();

/// Lets every cargo this run starts from here on compile through sccache,
/// where it is installed (owner, 13.11.2026: for the gates, without a
/// measurement first). What it can give back is a dependency compiled before
/// in another worktree or target directory: each has a `target` of its own,
/// and a cold one compiled every third-party crate again. A workspace crate
/// is built incrementally, which sccache passes through to rustc untouched.
///
/// A wrapper the caller set stays as it is, and `BAYLEE_SCCACHE=0` turns
/// this off. The cache is sccache's own (`sccache --show-stats` names its
/// place and size, `SCCACHE_CACHE_SIZE` bounds it).
pub fn cache_builds() {
    SCCACHE.get_or_init(|| {
        let on = std::env::var_os("RUSTC_WRAPPER").is_none()
            && std::env::var("BAYLEE_SCCACHE").as_deref() != Ok("0")
            && Command::new("sccache")
                .arg("--version")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .is_ok_and(|status| status.success());
        if on {
            println!("gate: compiling through sccache (BAYLEE_SCCACHE=0 turns it off)");
        }
        on
    });
}

/// Whether cargo set this variable to say which package a program it runs
/// belongs to. `CARGO` itself and a user's own settings (`CARGO_TARGET_DIR`,
/// `CARGO_PROFILE_*`, `CARGO_BUILD_*`, ...) are not of that kind.
fn names_this_package(name: &str) -> bool {
    name.starts_with("CARGO_PKG_")
        || matches!(
            name,
            "CARGO_MANIFEST_DIR"
                | "CARGO_MANIFEST_PATH"
                | "CARGO_MANIFEST_LINKS"
                | "CARGO_CRATE_NAME"
                | "CARGO_BIN_NAME"
                | "CARGO_PRIMARY_PACKAGE"
        )
}

/// A command from a program and its arguments.
pub fn command<I, S>(program: &str, args: I) -> Command
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = Command::new(program);
    command.args(args);
    command
}

/// A cargo command with these arguments.
pub fn cargo_with<I, S>(args: I) -> Command
where
    I: IntoIterator<Item = S>,
    S: AsRef<OsStr>,
{
    let mut command = cargo();
    command.args(args);
    command
}

/// Runs to completion with inherited output; a program that cannot start
/// counts as failed (status `None`) rather than aborting the whole gate.
pub fn status(command: &mut Command) -> Option<ExitStatus> {
    command.status().ok()
}

/// Whether the command ran and exited 0.
pub fn succeeds(command: &mut Command) -> bool {
    status(command).is_some_and(|s| s.success())
}

/// Runs with stdout and stderr captured together for a later tail.
pub fn captured(command: &mut Command) -> Option<Output> {
    command.stdin(Stdio::null()).output().ok()
}

/// A captured program's text, stdout then stderr.
pub fn text(output: &Output) -> String {
    let mut text = String::from_utf8_lossy(&output.stdout).into_owned();
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    text
}

/// The last `n` lines of a text, what a failed step shows of its log.
pub fn tail(text: &str, n: usize) -> String {
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(n)..].join("\n")
}

/// The exit code the gates print: the process's own, or 1 when it was
/// killed by a signal or could not start.
pub fn code(status: Option<ExitStatus>) -> i32 {
    status.and_then(|s| s.code()).unwrap_or(1)
}

/// A gate's steps, each timed and reported as `STEP <name> rc=<code> <n>s`.
/// Every step runs even when an earlier one failed, so one run reports every
/// red; [`Steps::finish`] says `done rc=<0|1>` and is the gate's exit code.
#[derive(Default)]
pub struct Steps {
    failed: bool,
}

impl Steps {
    /// Runs one step with inherited output.
    pub fn run(&mut self, name: &str, command: &mut Command) -> bool {
        let started = Instant::now();
        let rc = code(status(command));
        println!("STEP {name} rc={rc} {}s", started.elapsed().as_secs());
        self.record(rc == 0)
    }

    /// Records a step whose outcome the caller judged itself.
    pub fn record(&mut self, ok: bool) -> bool {
        self.failed |= !ok;
        ok
    }

    /// Prints `done rc=…` and returns the gate's exit code.
    pub fn finish(self) -> i32 {
        let rc = i32::from(self.failed);
        println!("done rc={rc}");
        rc
    }
}

/// The value of a required variable, or a sentence saying why it is needed.
pub fn require_env(name: &str, why: &str) -> Result<String> {
    match std::env::var(name) {
        Ok(value) if !value.is_empty() => Ok(value),
        _ => bail!("set {name} - {why}"),
    }
}
