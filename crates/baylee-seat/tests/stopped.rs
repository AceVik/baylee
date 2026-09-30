//! A bridge that is asked to stop settles its game in the spend book.
//!
//! `docker stop`, `kill` and a service manager stop a process with
//! SIGTERM, whose default is to end it where it stands: no destructor runs,
//! and a game's reservation would stay counted in full. The bridge answers
//! SIGTERM as it answers ctrl-c, by dropping the game, which settles it.
//!
//! The real binary, on its own multi-threaded runtime, against a stand-in
//! gateway that takes the connection and never answers, so the bridge
//! waits at sign-in with its game reserved. It also says at sit-down which
//! clock the game counts in: on unix the player's local offset is read
//! whatever the number of threads, so it is local time, never UTC by
//! default. No model is ever asked: the game never sits down.
#![cfg(unix)]

use baylee_client_core::llmseat::ledger::Book;
use std::io::{BufRead, BufReader, Read};
use std::net::TcpListener;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// A directory of this test's own, empty.
fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("baylee-seat-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn a_bridge_stopped_by_sigterm_settles_its_game() {
    let dir = scratch("sigterm");
    let config = dir.join("llm-seat.json");
    // The model's address is a closed loopback port: nothing could reach a
    // provider even if the game sat down.
    std::fs::write(
        &config,
        r#"{"default": "test", "profiles": {"test": {
            "provider": "anthropic", "model": "claude-sonnet-5-5",
            "game_usd": 1, "base_url": "http://127.0.0.1:9"}}}"#,
    )
    .unwrap();
    let gateway = TcpListener::bind("127.0.0.1:0").unwrap();
    let port = gateway.local_addr().unwrap().port();
    std::thread::spawn(move || {
        let held: Vec<_> = gateway.incoming().collect();
        drop(held);
    });

    let mut bridge = Command::new(env!("CARGO_BIN_EXE_baylee-seat"))
        .args(["join", "TEST-room", "--config"])
        .arg(&config)
        .args(["--gateway", &format!("http://127.0.0.1:{port}")])
        // Nothing of the machine's own: no config directory, no key but a
        // placeholder.
        .env_clear()
        .env("HOME", &dir)
        .env("ANTHROPIC_API_KEY", "TEST-placeholder-key")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the bridge starts");
    let (lines, said) = mpsc::channel();
    let stdout = bridge.stdout.take().unwrap();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let _ = lines.send(line);
        }
    });
    let sat = loop {
        let line = said
            .recv_timeout(Duration::from_secs(60))
            .expect("the bridge reserves and says so");
        if line.starts_with("reserved ") {
            break line;
        }
    };
    assert!(sat.contains("reserved $1.00 for this game"), "{sat}");
    assert!(sat.contains("(local time, UTC"), "{sat}");
    let book = Book::new(dir.join("llm-spend.json"));
    let open = book.read().unwrap().games;
    assert_eq!(open.len(), 1);
    assert!(
        open[0].settled.is_none(),
        "reserved while it waits at sign-in"
    );

    let sent = Command::new("kill")
        .args(["-TERM", &bridge.id().to_string()])
        .status()
        .unwrap();
    assert!(sent.success());
    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = bridge.try_wait().unwrap() {
            break status;
        }
        assert!(Instant::now() < deadline, "the bridge did not stop");
        std::thread::sleep(Duration::from_millis(50));
    };
    let mut stderr = String::new();
    bridge
        .stderr
        .take()
        .unwrap()
        .read_to_string(&mut stderr)
        .unwrap();
    let entry = book.read().unwrap().games.remove(0);
    assert!(entry.settled.is_some(), "settled on SIGTERM: {entry:?}");
    assert_eq!(entry.spent_usd, Some(0.0), "{entry:?}");
    assert!(!status.success(), "{status}");
    assert!(stderr.contains("stopped by SIGTERM"), "{stderr}");
    let _ = std::fs::remove_dir_all(&dir);
}
