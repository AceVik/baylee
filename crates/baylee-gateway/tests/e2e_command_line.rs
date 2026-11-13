//! The gateway's command line: it serves only when it says nothing
//! (`src/launch.rs`).
//!
//! Every launch here names a database nobody listens on, and the gateway
//! opens its database before it binds a port: a launch that got past its
//! command line would end in a panic at the database, after reading its
//! settings. A clean answer and no port file is a launch that opened
//! neither. Should one ever start anyway, it is told to bind loopback on a
//! port the kernel picks, and it is killed at the deadline.

#![allow(clippy::missing_docs_in_private_items)]

use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

/// Long enough for a loaded machine to start a process and print a page.
const DEADLINE: Duration = Duration::from_secs(30);

/// A directory of the test's own, removed on the way out.
struct Scratch(PathBuf);

impl Scratch {
    fn new(label: &str) -> Self {
        let dir =
            std::env::temp_dir().join(format!("baylee-gateway-cli-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("a scratch directory");
        Self(dir)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// Runs the gateway with `args` and a cleared environment; its output, or a
/// panic if it was still running at the deadline.
fn launch(args: &[&str], dir: &Path) -> Output {
    let mut child = Command::new(env!("CARGO_BIN_EXE_baylee-gateway"))
        .args(args)
        .current_dir(dir)
        .env_clear()
        .env("PATH", std::env::var("PATH").unwrap_or_default())
        .env("DATABASE_URL", "postgres://nobody@127.0.0.1:1/nothing")
        .env("PORT", "0")
        .env("BAYLEE_BIND", "127.0.0.1")
        .env("BAYLEE_PORT_FILE", dir.join("port"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the gateway runs");
    let started = Instant::now();
    loop {
        if child
            .try_wait()
            .expect("the gateway can be waited on")
            .is_some()
        {
            return child.wait_with_output().expect("the gateway's output");
        }
        if started.elapsed() > DEADLINE {
            let _ = child.kill();
            let out = child.wait_with_output().expect("the gateway's output");
            panic!(
                "baylee-gateway {args:?} was still running after {DEADLINE:?}: it started \
                 instead of answering\n{}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// No port: the file the gateway writes just before it serves is absent,
/// and it did not die at the database it never should have opened.
fn started_nothing(args: &[&str], out: &Output, dir: &Path) {
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        !dir.join("port").exists(),
        "{args:?} bound a port: {stderr}"
    );
    assert!(!stderr.contains("panicked"), "{args:?}: {stderr}");
}

#[test]
fn help_prints_the_usage_and_starts_nothing() {
    for help in ["--help", "-h", "help"] {
        let dir = Scratch::new("help");
        let out = launch(&[help], &dir.0);
        assert_eq!(out.status.code(), Some(0), "{help}: {out:?}");
        let usage = String::from_utf8_lossy(&out.stdout);
        assert!(
            usage.starts_with("usage: baylee-gateway"),
            "{help}: {usage}"
        );
        assert!(usage.contains("BAYLEE_BIND"), "{help}: {usage}");
        started_nothing(&[help], &out, &dir.0);
    }
}

#[test]
fn an_argument_it_does_not_take_is_refused_and_starts_nothing() {
    for (args, named) in [
        (&["--version"][..], "\"--version\""),
        (&["--bind", "0.0.0.0"][..], "\"--bind\""),
        (&["--port=28766"][..], "\"--port\""),
        (&["serve"][..], "\"serve\""),
        (&["--bogus", "--help"][..], "\"--bogus\""),
    ] {
        let dir = Scratch::new("refused");
        let out = launch(args, &dir.0);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {out:?}");
        let refused = String::from_utf8_lossy(&out.stderr);
        assert!(refused.contains(named), "{args:?}: {refused}");
        assert!(
            refused.contains("usage: baylee-gateway"),
            "{args:?}: {refused}"
        );
        assert!(out.stdout.is_empty(), "{args:?}");
        started_nothing(args, &out, &dir.0);
    }
}
