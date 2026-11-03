//! `cargo dev e2e`: the client's end-to-end tests (`docs/dev/e2e.md`).
//!
//! Builds `baylee-client` with `dev-control` once, copies it into the run's
//! own directory (a later build in a shared target would replace the binary
//! under a running test), then runs `baylee-client-e2e`'s ignored tests
//! against that copy. Each test starts, drives and closes its own client;
//! afterwards no process of the copy may be left, and one that is gets
//! killed and fails the run.
use crate::run::{cargo_with, code, command, status};
use anyhow::{Context, Result, bail};
use std::{
    fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

/// What `cargo dev e2e` was asked.
#[derive(clap::Args)]
pub struct E2e {
    /// Clients open at once (`BAYLEE_E2E_JOBS`).
    #[arg(long, default_value_t = 1)]
    pub jobs: usize,
    /// The screen they open on: `builtin`, `primary`, `default` or an index
    /// (default: the built-in display on macOS; `BAYLEE_E2E_MONITOR`).
    #[arg(long)]
    pub monitor: Option<String>,
    /// Test the client the last build left in `target/debug`, without
    /// building it.
    #[arg(long)]
    pub no_build: bool,
    /// Only the tests whose names contain one of these.
    pub filters: Vec<String>,
}

/// Runs the suite; the exit code is the tests'.
pub fn run(options: &E2e) -> Result<i32> {
    if !options.no_build {
        let built = status(&mut cargo_with([
            "build",
            "-p",
            "baylee-client",
            "--features",
            "dev-control",
        ]));
        if !built.is_some_and(|s| s.success()) {
            bail!("the dev-control client did not build");
        }
    }
    let name = format!(
        "run-{}",
        SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()
    );
    let dir = PathBuf::from("target/e2e").join(&name);
    let binary = copy_client(&dir)?;
    println!(
        "e2e: client {} — artifacts in {}",
        binary.display(),
        dir.display()
    );
    let mut tests = cargo_with(["test", "-p", "baylee-client-e2e", "--tests", "--"]);
    tests
        .arg("--ignored")
        .arg(format!("--test-threads={}", options.jobs.max(1)))
        .args(&options.filters)
        .env("BAYLEE_E2E_CLIENT", &binary)
        .env("BAYLEE_E2E_RUN", &name)
        .env("BAYLEE_E2E_JOBS", options.jobs.max(1).to_string());
    if let Some(monitor) = &options.monitor {
        tests.env("BAYLEE_E2E_MONITOR", monitor);
    }
    let result = code(status(&mut tests));
    let left = leftovers(&binary);
    if !left.is_empty() {
        // Close what a crashed test could not, then say so: a client left
        // open is a defect of the run even when every test passed.
        let _ = status(&mut command("kill", &left));
        bail!(
            "client process(es) {} outlived their tests and were killed",
            left.join(", ")
        );
    }
    Ok(result)
}

fn copy_client(dir: &Path) -> Result<PathBuf> {
    let built = PathBuf::from("target/debug")
        .join("baylee-client")
        .with_extension(std::env::consts::EXE_EXTENSION);
    let bin = dir.join("bin");
    fs::create_dir_all(&bin)?;
    let copy = bin
        .join("baylee-client")
        .with_extension(std::env::consts::EXE_EXTENSION);
    fs::copy(&built, &copy).with_context(|| {
        format!(
            "no client at {}: build it with `cargo build -p baylee-client --features dev-control`",
            built.display()
        )
    })?;
    copy.canonicalize().context("the copied client")
}

/// The ids of every process still running the copied binary.
fn leftovers(binary: &Path) -> Vec<String> {
    let Ok(out) = command("pgrep", ["-f"]).arg(binary).output() else {
        return Vec::new();
    };
    String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}
