//! What the Central barrier may be held up by.
//!
//! A sign-in that fixes a link takes the barrier exclusively, so every request
//! holds it shared. A request that holds it for long therefore keeps people
//! from signing in, and one that only waits for it stalls every route behind
//! that sign-in. Against a stub Central on loopback.

#![allow(clippy::missing_docs_in_private_items)]

mod common;
mod stub_central;

use common::http;
use std::io::Write;
use std::time::{Duration, Instant};
use stub_central::{Stub, gateway, linked};

/// The lease a sign-in is handed.
const SIGNING_IN: u8 = 7;

/// The lease whose checks the stub answers slowly.
const SLOW: u8 = 9;

/// How long the stub takes over [`SLOW`]: past the barrier's five seconds.
const SLOW_BY: Duration = Duration::from_secs(7);

/// `POST /auth/central/login` with a ticket the stub takes: its status and
/// how long it took.
fn sign_in(port: u16) -> (u16, Duration) {
    let bytes = [1_u8; 32];
    let ticket = serde_json::json!({ "ticket": bytes }).to_string();
    let started = Instant::now();
    let (status, _) = http(port, "POST", "/auth/central/login", None, &ticket);
    (status, started.elapsed())
}

/// A stranger who starts a request and never finishes its body holds up
/// nobody's sign-in. It takes no account and no secret, so if this held the
/// barrier one connection would keep a gateway's players out.
#[test]
fn a_body_that_never_arrives_holds_up_no_sign_in() {
    let stub = Stub::serve(Duration::ZERO);
    let (gw, _settings) = gateway(&stub, "barrier_body");
    linked(&gw, &stub, SIGNING_IN);
    stub.shared_sign_in(SIGNING_IN);
    let mut stalled = std::net::TcpStream::connect(("127.0.0.1", gw.port)).expect("connected");
    stalled
        .write_all(
            b"POST /auth/guest HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\n\
              Content-Length: 100\r\n\r\n{",
        )
        .expect("written");
    std::thread::sleep(Duration::from_millis(300));
    let (status, took) = sign_in(gw.port);
    eprintln!("a stalled body: sign-in {status} in {took:?}");
    assert_eq!(status, 200);
    assert!(took < Duration::from_secs(3), "{took:?}");
}

/// A sign-in that fixes no link has nothing to drain requests for, so a slow
/// request of somebody else's does not stand in its way.
#[test]
fn a_slow_request_holds_up_no_sign_in_of_a_linked_account() {
    let stub = Stub::serve(Duration::ZERO);
    let (gw, _settings) = gateway(&stub, "barrier_slow");
    linked(&gw, &stub, SIGNING_IN);
    stub.shared_sign_in(SIGNING_IN);
    let bearer = linked(&gw, &stub, SLOW);
    stub.slow_down(SLOW, SLOW_BY);
    let port = gw.port;
    let slow = std::thread::spawn(move || http(port, "GET", "/me", Some(&bearer), ""));
    std::thread::sleep(Duration::from_millis(300));
    let (status, took) = sign_in(gw.port);
    eprintln!("a slow request: sign-in {status} in {took:?}");
    assert_eq!(status, 200);
    assert!(took < Duration::from_secs(3), "{took:?}");
    let _ = slow.join();
}
