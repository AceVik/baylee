//! The agent's command line: it runs only when it says nothing.
//!
//! Each launch here has everything it needs to run (a token, an engine
//! path and a gateway): the gateway is a loopback listener of the test's
//! own that nobody answers on. A launch that got past its command line
//! dials it and keeps redialling, as the control below shows; one that did
//! not exits at once and leaves the listener without a connection.

#![allow(clippy::missing_docs_in_private_items)]

use std::net::TcpListener;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

/// Long enough for a loaded machine to start a process and print a page.
const DEADLINE: Duration = Duration::from_secs(30);
const STEP: Duration = Duration::from_millis(20);

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

/// The agent with `args` and a cleared environment, pointed at `gateway`,
/// killed and reaped however the test leaves.
struct Agent(Option<Child>);

impl Agent {
    fn spawn(args: &[&str], gateway: &TcpListener) -> Self {
        let address = gateway.local_addr().expect("the listener's address");
        let child = Command::new(env!("CARGO_BIN_EXE_baylee-agent"))
            .args(args)
            .env_clear()
            .env("BAYLEE_GATEWAY", format!("http://{address}"))
            .env("BAYLEE_AGENT_TOKEN", "a-token-for-this-test-only")
            .env("BAYLEE_ENGINE_BIN", "/nonexistent/baylee-engine-server")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("the agent runs");
        Self(Some(child))
    }

    /// Whether it has exited.
    fn exited(&mut self) -> bool {
        self.0
            .as_mut()
            .and_then(|child| child.try_wait().expect("the agent can be waited on"))
            .is_some()
    }

    /// Its output once it exits, or a panic if it was still running at the
    /// deadline (and then it is killed).
    fn answer(mut self, args: &[&str]) -> Output {
        let started = Instant::now();
        while !self.exited() {
            assert!(
                started.elapsed() < DEADLINE,
                "baylee-agent {args:?} was still running: it started instead of answering"
            );
            std::thread::sleep(STEP);
        }
        let child = self.0.take().expect("the agent");
        child.wait_with_output().expect("the agent's output")
    }
}

impl Drop for Agent {
    fn drop(&mut self) {
        if let Some(child) = self.0.as_mut() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

#[test]
fn help_prints_the_usage_and_dials_nothing() {
    for help in ["--help", "-h"] {
        let gateway = gateway();
        let out = Agent::spawn(&[help], &gateway).answer(&[help]);
        assert_eq!(out.status.code(), Some(0), "{help}: {out:?}");
        let usage = String::from_utf8_lossy(&out.stdout);
        assert!(usage.starts_with("usage: baylee-agent"), "{usage}");
        assert!(!dialled(&gateway), "{help} dialled the gateway");
    }
}

#[test]
fn an_argument_it_does_not_take_is_refused_and_dials_nothing() {
    for args in [&["--version"][..], &["--gateway", "x"][..], &["run"][..]] {
        let gateway = gateway();
        let out = Agent::spawn(args, &gateway).answer(args);
        assert_eq!(out.status.code(), Some(2), "{args:?}: {out:?}");
        let refused = String::from_utf8_lossy(&out.stderr);
        assert!(refused.contains("usage: baylee-agent"), "{refused}");
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
    let mut agent = Agent::spawn(&[], &gateway);
    let started = Instant::now();
    while !dialled(&gateway) {
        assert!(!agent.exited(), "the agent exited instead of dialling");
        assert!(started.elapsed() < DEADLINE, "the agent never dialled");
        std::thread::sleep(STEP);
    }
}
