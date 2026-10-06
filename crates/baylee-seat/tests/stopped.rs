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
        // Never the player's credential store.
        .env("BAYLEE_KEY_STORE", "off")
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

/// One bridge, signalled the instant it prints "reserved", used by the
/// stress test below to keep this invariant honest under load: a SIGTERM
/// delivered any time after the game's reservation must always be caught by
/// `Stoppers` and settle the reservation, never meet the operating system's
/// own default disposition (the process ends where it stands, uncounted).
/// A prior version of the bridge created its signal listeners lazily inside
/// `stopped()`'s first poll rather than before `spend::reserve`; this did
/// not reproduce a failure in testing against it (dozens of runs, under
/// artificial CPU load), because a `biased` `select!` already polled
/// `stopped()`, and so registered its listeners, before the branch that
/// reserves ever ran. `Stoppers::arm` makes that guarantee explicit instead
/// of resting on `select!`'s polling order, and this test guards it going
/// forward. Errs with a description rather than panicking, so the caller
/// can gather every lane's result under one assertion.
fn one_stress_round(name: &str) -> Result<(), String> {
    let dir = scratch(name);
    let config = dir.join("llm-seat.json");
    std::fs::write(
        &config,
        r#"{"default": "test", "profiles": {"test": {
            "provider": "anthropic", "model": "claude-sonnet-5-5",
            "game_usd": 1, "base_url": "http://127.0.0.1:9"}}}"#,
    )
    .map_err(|e| format!("{name}: writing the settings file: {e}"))?;
    let gateway = TcpListener::bind("127.0.0.1:0")
        .map_err(|e| format!("{name}: binding the stand-in gateway: {e}"))?;
    let port = gateway.local_addr().unwrap().port();
    std::thread::spawn(move || {
        let held: Vec<_> = gateway.incoming().collect();
        drop(held);
    });

    let mut bridge = Command::new(env!("CARGO_BIN_EXE_baylee-seat"))
        .args(["join", "TEST-room", "--config"])
        .arg(&config)
        .args(["--gateway", &format!("http://127.0.0.1:{port}")])
        .env_clear()
        .env("HOME", &dir)
        // Never the player's credential store.
        .env("BAYLEE_KEY_STORE", "off")
        .env("ANTHROPIC_API_KEY", "TEST-placeholder-key")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("{name}: the bridge did not start: {e}"))?;
    let pid = bridge.id();
    let (lines, said) = mpsc::channel();
    let stdout = bridge.stdout.take().unwrap();
    std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let _ = lines.send(line);
        }
    });
    loop {
        let line = said
            .recv_timeout(Duration::from_secs(60))
            .map_err(|_| format!("{name}: the bridge never said it reserved"))?;
        if line.starts_with("reserved ") {
            break;
        }
    }
    // As fast as this process can: no channel round trip, no assertion, no
    // formatting between seeing "reserved" and sending the signal.
    let sent = Command::new("kill")
        .args(["-TERM", &pid.to_string()])
        .status()
        .map_err(|e| format!("{name}: kill did not run: {e}"))?;
    if !sent.success() {
        return Err(format!("{name}: kill -TERM refused: {sent}"));
    }
    let deadline = Instant::now() + Duration::from_secs(20);
    let status = loop {
        if let Some(status) = bridge
            .try_wait()
            .map_err(|e| format!("{name}: waiting on the bridge: {e}"))?
        {
            break status;
        }
        if Instant::now() >= deadline {
            return Err(format!("{name}: the bridge did not stop"));
        }
        std::thread::sleep(Duration::from_millis(5));
    };
    let book = Book::new(dir.join("llm-spend.json"));
    let games = book
        .read()
        .map_err(|e| format!("{name}: reading the spend book: {e}"))?
        .games;
    let result = match games.first() {
        Some(entry) if entry.settled.is_some() && status.success() => {
            Err(format!("{name}: settled but exited 0: {status}"))
        }
        Some(entry) if entry.settled.is_some() => Ok(()),
        Some(entry) => Err(format!(
            "{name}: SIGTERM raced the reservation: never settled ({entry:?}, exit {status})"
        )),
        None => Err(format!("{name}: no game in the spend book")),
    };
    let _ = std::fs::remove_dir_all(&dir);
    result
}

/// Stress form of the test above: several bridges at once, across several
/// rounds, each signalled the instant it reserves, to give a stop-signal
/// race a real chance to show up under the scheduling jitter of a loaded
/// machine (several `cargo test` processes at once, as the gate runs it)
/// rather than only in a single, unloaded run.
#[test]
fn a_bridge_stopped_by_sigterm_settles_its_game_under_load() {
    let lanes = 6;
    let rounds = 4;
    let mut failures = Vec::new();
    for round in 0..rounds {
        let handles: Vec<_> = (0..lanes)
            .map(|lane| {
                let name = format!("sigterm-load-{round}-{lane}");
                std::thread::spawn(move || one_stress_round(&name))
            })
            .collect();
        for handle in handles {
            if let Err(e) = handle.join().expect("a lane thread does not panic") {
                failures.push(e);
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} of {} runs did not settle cleanly:\n{}",
        failures.len(),
        lanes * rounds,
        failures.join("\n")
    );
}
