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

/// The session of whoever signs in.
const SIGNING_IN: u8 = 7;

/// The session of whoever has a slow request.
const SLOW: u8 = 9;

/// How long a request is held up: past the barrier's five seconds.
const SLOW_BY_SECS: u32 = 7;

/// A fresh sign-in as [`SIGNING_IN`]: its status and how long it took.
fn sign_in(stub: &Stub, port: u16) -> (u16, Duration) {
    let started = Instant::now();
    let (status, _) = stub.sign_in(port, SIGNING_IN);
    (status, started.elapsed())
}

/// A stranger who starts a request and never finishes its body holds up
/// nobody's sign-in. It takes no account and no secret, so if this held the
/// barrier one connection would keep a gateway's players out. The account
/// signing in is new here, so this is the sign-in that needs the barrier.
#[test]
fn a_body_that_never_arrives_holds_up_no_sign_in() {
    let stub = Stub::serve(Duration::ZERO);
    let (gw, _settings) = gateway(&stub, "barrier_body");
    stub.knows(SIGNING_IN);
    let mut stalled = std::net::TcpStream::connect(("127.0.0.1", gw.port)).expect("connected");
    stalled
        .write_all(
            b"POST /auth/guest HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\n\
              Content-Length: 100\r\n\r\n{",
        )
        .expect("written");
    std::thread::sleep(Duration::from_millis(300));
    let (status, took) = sign_in(&stub, gw.port);
    eprintln!("a stalled body: sign-in {status} in {took:?}");
    assert_eq!(status, 200);
    assert!(took < Duration::from_secs(3), "{took:?}");
}

/// A sign-in that fixes no link has nothing to drain requests for, so a slow
/// request of somebody else's does not stand in its way. The request here is
/// slow on a table no sign-in reads.
#[test]
fn a_slow_request_holds_up_no_sign_in_of_a_linked_account() {
    let stub = Stub::serve(Duration::ZERO);
    let (gw, _settings) = gateway(&stub, "barrier_slow");
    linked(&gw, &stub, SIGNING_IN);
    let bearer = linked(&gw, &stub, SLOW);
    let port = gw.port;
    let (status, took) = std::thread::scope(|scope| {
        let locked = format!(
            "BEGIN; LOCK TABLE deck IN ACCESS EXCLUSIVE MODE; \
             SELECT pg_sleep({SLOW_BY_SECS}); COMMIT;"
        );
        let gw = &gw;
        scope.spawn(move || gw.sql(&locked));
        std::thread::sleep(Duration::from_millis(300));
        let slow = scope.spawn(move || {
            let started = Instant::now();
            let (status, _) = http(port, "GET", "/decks", Some(&bearer), "");
            (status, started.elapsed())
        });
        std::thread::sleep(Duration::from_millis(300));
        let signed_in = sign_in(&stub, port);
        let (status, slow_took) = slow.join().expect("the slow request");
        eprintln!("the slow request: {status} in {slow_took:?}");
        assert!(
            slow_took > Duration::from_secs(5),
            "nothing held the barrier: the request took {slow_took:?}"
        );
        signed_in
    });
    eprintln!("a slow request: sign-in {status} in {took:?}");
    assert_eq!(status, 200);
    assert!(took < Duration::from_secs(3), "{took:?}");
}
