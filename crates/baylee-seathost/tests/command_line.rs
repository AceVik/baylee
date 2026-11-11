//! The seat agent's command line: it runs only when it says nothing.
//!
//! Each launch here has everything it needs to run: a token, a state
//! directory, a bridge that answers `--version` as a release build would,
//! and a gateway, a loopback listener of the test's own that nobody answers
//! on. A launch that got past its command line dials it and keeps
//! redialling, as the control below shows; one that did not exits at once
//! and leaves the listener without a connection.

#![allow(clippy::missing_docs_in_private_items)]

use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

/// Long enough for a loaded machine to start a process and print a page.
const DEADLINE: Duration = Duration::from_secs(30);
const STEP: Duration = Duration::from_millis(20);

/// A bridge that says nothing to `--version`, which the seat agent takes for
/// a release build.
fn quiet_bridge() -> &'static Path {
    ["/bin/true", "/usr/bin/true"]
        .into_iter()
        .map(Path::new)
        .find(|path| path.is_file())
        .expect("a `true` program")
}

/// A gateway on loopback that counts who dialled it.
fn gateway() -> TcpListener {
    let listener = TcpListener::bind("127.0.0.1:0").expect("a loopback port");
    listener
        .set_nonblocking(true)
        .expect("a non-blocking listener");
    listener
}

fn dialled(gateway: &TcpListener) -> bool {
    match gateway.accept() {
        Ok(_) => true,
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => false,
        Err(e) => panic!("the test's listener: {e}"),
    }
}

/// The seat agent with `args` and a cleared environment, pointed at
/// `gateway`, in a state directory of its own; killed and reaped, and its
/// directory removed, however the test leaves.
struct SeatHost {
    child: Option<Child>,
    state: PathBuf,
}

impl SeatHost {
    fn spawn(label: &str, args: &[&str], gateway: &TcpListener) -> Self {
        let state = std::env::temp_dir().join(format!(
            "baylee-seathost-cli-{label}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&state);
        std::fs::create_dir_all(&state).expect("a state directory");
        let address = gateway.local_addr().expect("the listener's address");
        let child = Command::new(env!("CARGO_BIN_EXE_baylee-seathost"))
            .args(args)
            .env_clear()
            .env("BAYLEE_GATEWAY", format!("http://{address}"))
            .env("BAYLEE_SEATHOST_TOKEN", "a-token-for-this-test-only")
            .env("BAYLEE_SEATHOST_STATE", &state)
            .env("BAYLEE_SEAT_BIN", quiet_bridge())
            .env("BAYLEE_KEY_STORE", "off")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the seat agent runs");
        Self {
            child: Some(child),
            state,
        }
    }

    /// Whether it has exited.
    fn exited(&mut self) -> bool {
        self.child
            .as_mut()
            .and_then(|child| child.try_wait().expect("the seat agent can be waited on"))
            .is_some()
    }

    /// Its output once it exits, or a panic if it was still running at the
    /// deadline (and then it is killed).
    fn answer(mut self, args: &[&str]) -> Output {
        let started = Instant::now();
        while !self.exited() {
            assert!(
                started.elapsed() < DEADLINE,
                "baylee-seathost {args:?} was still running: it started instead of answering"
            );
            std::thread::sleep(STEP);
        }
        let child = self.child.take().expect("the seat agent");
        child.wait_with_output().expect("the seat agent's output")
    }
}

impl Drop for SeatHost {
    fn drop(&mut self) {
        if let Some(child) = self.child.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
        let _ = std::fs::remove_dir_all(&self.state);
    }
}

#[test]
fn help_prints_the_usage_and_dials_nothing() {
    for help in ["--help", "-h"] {
        let gateway = gateway();
        let out = SeatHost::spawn("help", &[help], &gateway).answer(&[help]);
        assert_eq!(out.status.code(), Some(0), "{help}: {out:?}");
        let usage = String::from_utf8_lossy(&out.stdout);
        assert!(usage.starts_with("usage: baylee-seathost"), "{usage}");
        assert!(!dialled(&gateway), "{help} dialled the gateway");
    }
}

#[test]
fn an_argument_it_does_not_take_is_refused_and_dials_nothing() {
    for args in [&["--version"][..], &["--name", "main"][..], &["run"][..]] {
        let gateway = gateway();
        let out = SeatHost::spawn("refused", args, &gateway).answer(args);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {out:?}");
        let refused = String::from_utf8_lossy(&out.stderr);
        assert!(refused.contains("usage: baylee-seathost"), "{refused}");
        assert!(!refused.contains("a-token-for-this-test-only"), "{refused}");
        assert!(out.stdout.is_empty(), "{args:?}");
        assert!(!dialled(&gateway), "{args:?} dialled the gateway");
    }
}

/// The control: the same launch with no argument runs and dials, so the
/// silence above is the command line's doing and not the setup's.
#[test]
fn with_no_argument_the_same_launch_dials() {
    let gateway = gateway();
    let mut host = SeatHost::spawn("control", &[], &gateway);
    let started = Instant::now();
    while !dialled(&gateway) {
        assert!(!host.exited(), "the seat agent exited instead of dialling");
        assert!(started.elapsed() < DEADLINE, "the seat agent never dialled");
        std::thread::sleep(STEP);
    }
}
