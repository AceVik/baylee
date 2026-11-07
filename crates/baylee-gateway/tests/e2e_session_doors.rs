//! What a session of Central's and a stranger meet at the gateway's own doors:
//! a sign-out, the sign-in budget and the door Central pushes through. Against
//! a stub Central on loopback.

#![allow(clippy::missing_docs_in_private_items)]

mod common;
mod stub_central;

use common::{http, lobby_url};
use futures_util::StreamExt;
use std::io::{Read, Write};
use std::time::Duration;
use stub_central::{Stub, gateway, gateway_with, linked};
use tokio_tungstenite::tungstenite::Message;

const PLAYER: u8 = 1;
const OTHER: u8 = 2;

/// Addresses nobody else in a test uses.
const STRANGER: &str = "203.0.113.9";
const SECOND_STRANGER: &str = "203.0.113.10";
const ANOTHER: &str = "198.51.100.7";

/// `POST path` as `from`, behind a proxy the gateway trusts: its status.
fn post_from(port: u16, path: &str, from: &str, content_type: &str, body: &str) -> u16 {
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connected");
    write!(
        stream,
        "POST {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nX-Forwarded-For: {from}\r\n\
         Content-Type: {content_type}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .expect("written");
    let mut raw = String::new();
    stream.read_to_string(&mut raw).expect("read");
    raw.split_whitespace()
        .nth(1)
        .and_then(|status| status.parse().ok())
        .expect("a status")
}

fn trusting_the_proxy() -> [(&'static str, String); 1] {
    [("BAYLEE_TRUSTED_PROXIES", "127.0.0.1".into())]
}

/// A player who signs out of this gateway is out of it: the bearer opens
/// nothing, it renews nothing, and the sockets of the session close. Central
/// hears of none of this and need not.
#[tokio::test(flavor = "multi_thread")]
async fn signing_out_ends_a_central_session_here() {
    let stub = Stub::serve(Duration::ZERO);
    let (gw, _settings) = gateway(&stub, "doors_sign_out");
    let bearer = linked(&gw, &stub, PLAYER);
    let (mut socket, _) = tokio_tungstenite::connect_async(lobby_url(gw.port, &bearer, ""))
        .await
        .expect("the lobby");
    assert_eq!(http(gw.port, "GET", "/me", Some(&bearer), "").0, 200);

    assert_eq!(
        http(gw.port, "POST", "/auth/logout", Some(&bearer), "").0,
        204
    );

    let closed = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            match socket.next().await {
                Some(Ok(Message::Close(_)) | Err(_)) | None => return,
                Some(Ok(_)) => {}
            }
        }
    })
    .await;
    assert!(closed.is_ok(), "the socket of a signed-out session stayed");
    assert_eq!(http(gw.port, "GET", "/me", Some(&bearer), "").0, 401);
    assert_eq!(
        http(gw.port, "POST", "/auth/session/renew", Some(&bearer), "").0,
        401
    );
}

/// Signing out reaches only the session of the bearer that asked.
#[test]
fn signing_out_ends_no_other_session() {
    let stub = Stub::serve(Duration::ZERO);
    let (gw, _settings) = gateway(&stub, "doors_only_mine");
    let bearer = linked(&gw, &stub, PLAYER);
    let other = linked(&gw, &stub, OTHER);
    assert_eq!(
        http(gw.port, "POST", "/auth/logout", Some(&bearer), "").0,
        204
    );
    assert_eq!(http(gw.port, "GET", "/me", Some(&other), "").0, 200);
}

/// Sign-ins take no lane any more: junk from two addresses spends those
/// addresses' budgets and nobody else's.
#[test]
fn junk_sign_ins_from_a_few_addresses_keep_nobody_else_out() {
    let stub = Stub::serve(Duration::ZERO);
    let (gw, _settings) = gateway_with(&stub, "doors_budget", &trusting_the_proxy());
    stub.knows(PLAYER);
    let junk = r#"{"token":"not.a.token"}"#;
    let sign_in = |from: &str, body: &str| {
        post_from(
            gw.port,
            "/auth/central/login",
            from,
            "application/json",
            body,
        )
    };
    for from in [STRANGER, SECOND_STRANGER] {
        let mut refused = 0;
        for _ in 0..80 {
            refused += u32::from(sign_in(from, junk) == 429);
        }
        assert!(refused > 0, "{from} has a budget");
    }
    let token = stub.token(gw.port, PLAYER);
    let body = serde_json::json!({ "token": token.as_str() }).to_string();
    assert_eq!(sign_in(ANOTHER, &body), 200);
}

/// Anyone can reach the door Central pushes through. A stranger spending a
/// budget there must spend its own: otherwise it can keep every revocation
/// waiting for the next round of the catch-up.
#[test]
fn a_stranger_at_the_push_door_keeps_no_message_of_central_s_out() {
    let stub = Stub::serve(Duration::ZERO);
    let (gw, _settings) = gateway_with(&stub, "doors_push", &trusting_the_proxy());
    let bearer = linked(&gw, &stub, PLAYER);
    let path = "/auth/central/invalidate";
    let mut refused = 0;
    for _ in 0..700 {
        let status = post_from(gw.port, path, STRANGER, "application/jwt", "not.a.message");
        refused += u32::from(status == 429);
    }
    assert!(refused > 0, "the door has a budget");
    stub.revoke(PLAYER);
    // `push` asserts that the gateway took it.
    stub.push(gw.port);
    assert_eq!(http(gw.port, "GET", "/me", Some(&bearer), "").0, 401);
}
