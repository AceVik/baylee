//! Confirming an address, end to end through a real SMTP conversation.
//!
//! Since usernames (#269) a new account has no address and is sent nothing,
//! and an unconfirmed address keeps nobody out. What is left is the accounts
//! that registered with an address before: a link already mailed still
//! confirms it, and its owner may ask for another. The address stays the way
//! to reach a player, for the day it is used for recovery.
//!
//! The link cannot be recovered from the store — only its hash is kept there,
//! for the same reason a session token's is — so the only honest way to test
//! it is to receive the mail. The sink below speaks just enough SMTP for
//! lettre to hand over a message, which is also what proves the gateway's
//! transport is configured the way a real relay would need.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use common::{http, spawn_gateway_with};
use std::io::{BufRead, BufReader, Write};
use std::sync::mpsc;

const PASSWORD: &str = "a-very-fine-password";

/// A one-shot SMTP sink: accepts exactly one message and sends its body back
/// over the channel.
fn smtp_sink() -> (u16, mpsc::Receiver<String>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("a free port");
    let port = listener.local_addr().expect("bound").port();
    let (tx, rx) = mpsc::channel();
    std::thread::spawn(move || {
        let Ok((stream, _)) = listener.accept() else {
            return;
        };
        let mut out = stream.try_clone().expect("clone");
        let mut lines = BufReader::new(stream);
        let mut say = |text: &str| {
            let _ = out.write_all(text.as_bytes());
            let _ = out.flush();
        };
        say("220 sink ready\r\n");
        let mut line = String::new();
        let mut body = String::new();
        let mut in_data = false;
        while lines.read_line(&mut line).is_ok_and(|n| n > 0) {
            let text = line.trim_end().to_string();
            line.clear();
            if in_data {
                if text == "." {
                    in_data = false;
                    say("250 taken\r\n");
                    let _ = tx.send(std::mem::take(&mut body));
                    continue;
                }
                body.push_str(&text);
                body.push('\n');
                continue;
            }
            let verb = text.split_whitespace().next().unwrap_or("").to_uppercase();
            match verb.as_str() {
                // Deliberately no STARTTLS in the greeting: a sink with no
                // certificate is exactly the local catcher a developer runs,
                // and lettre has to be willing to talk to one.
                "EHLO" | "HELO" => say("250-sink\r\n250 SIZE 10240000\r\n"),
                "MAIL" | "RCPT" | "RSET" | "NOOP" => say("250 ok\r\n"),
                "DATA" => {
                    in_data = true;
                    say("354 go ahead\r\n");
                }
                "QUIT" => {
                    say("221 bye\r\n");
                    return;
                }
                _ => say("502 no\r\n"),
            }
        }
    });
    (port, rx)
}

/// The confirmation link out of a received message.
///
/// The body is quoted-printable, which does two things to a link: it breaks
/// long lines with a trailing `=`, and it escapes the `=` in `?token=` as
/// `=3D`. Both have to be undone or the token that comes out is not the token
/// that was mailed — which is exactly the bug this test would otherwise pass
/// straight through.
fn link_in(body: &str) -> String {
    let mut unfolded = String::with_capacity(body.len());
    for line in body.lines() {
        if let Some(head) = line.strip_suffix('=') {
            unfolded.push_str(head);
        } else {
            unfolded.push_str(line);
            unfolded.push('\n');
        }
    }
    let start = unfolded.find("http://").expect("a link in the mail");
    let raw = unfolded[start..]
        .split_whitespace()
        .next()
        .expect("the link ends");
    raw.replace("=3D", "=")
}

fn register(port: u16, username: &str, name: &str) -> (u16, String) {
    let body = format!(
        "{{\"username\":\"{username}\",\"display_name\":\"{name}\",\
         \"password\":\"{PASSWORD}\",\"lang\":\"de\"}}"
    );
    http(port, "POST", "/auth/register", None, &body)
}

fn sign_in(port: u16, who: &str) -> (u16, String) {
    let body = format!("{{\"username\":\"{who}\",\"password\":\"{PASSWORD}\"}}");
    http(port, "POST", "/auth/login", None, &body)
}

/// An account as registration made them before usernames: with an address,
/// not yet confirmed.
fn give_an_address(gw: &common::Gateway, username: &str, address: &str) {
    gw.sql(&format!(
        "UPDATE account SET email = '{address}', confirmed_at = NULL \
         WHERE username_key = '{username}'"
    ));
}

#[test]
fn a_new_account_is_sent_nothing_and_an_old_address_keeps_nobody_out() {
    let (smtp_port, mail) = smtp_sink();
    let gw = spawn_gateway_with(
        "confirm-on",
        &[
            ("BAYLEE_SMTP_URL", format!("smtp://127.0.0.1:{smtp_port}")),
            ("BAYLEE_MAIL_FROM", "baylee <no-reply@example.com>".into()),
        ],
    );
    // `BAYLEE_PUBLIC_URL` is left at its default, so the *host* in the link
    // is not this gateway's. That is the point being made: the link's path
    // and token are what carry the meaning, and the host is a deployment
    // setting the gateway cannot work out for itself — reading it off a
    // request header is how a confirmation link ends up pointing wherever
    // the `Host:` header said.
    let (status, body) = http(gw.port, "GET", "/auth/config", None, "");
    assert_eq!(status, 200);
    assert!(
        !body.contains("confirmation_required"),
        "nothing waits on a mail any more: {body}"
    );

    // A new account signs in at once, mailer or not.
    let (status, body) = register(gw.port, "fresh", "Fresh");
    assert_eq!(status, 200, "register: {body}");
    assert_eq!(body, r#"{"ok":true}"#);
    let (status, body) = sign_in(gw.port, "fresh");
    assert_eq!(status, 200, "a new account is usable at once: {body}");

    // One that registered with an address and never confirmed it signs in
    // too, by its name and, until the end of 2026, by the address.
    let (status, body) = register(gw.port, "oldtimer", "Oldtimer");
    assert_eq!(status, 200, "register: {body}");
    give_an_address(&gw, "oldtimer", "oldtimer@example.com");
    let (status, body) = sign_in(gw.port, "oldtimer");
    assert_eq!(
        status, 200,
        "an unconfirmed address kept its owner out: {body}"
    );
    let (status, body) = sign_in(gw.port, "OldTimer@Example.com");
    assert_eq!(status, 200, "by the address: {body}");

    // Its owner asks for the link again. The sink takes exactly one message,
    // so this being the one it got is also what says that registering sent
    // none.
    let (status, body) = http(
        gw.port,
        "POST",
        "/auth/confirm/resend",
        None,
        r#"{"email":"oldtimer@example.com"}"#,
    );
    assert_eq!(status, 200, "resend: {body}");
    let received = mail
        .recv_timeout(common::WAIT_BUDGET)
        .expect("the confirmation mail arrives");
    assert!(
        received.contains("Oldtimer") && !received.contains("Fresh"),
        "the one mail sent is the one asked for: {received}"
    );
    let link = link_in(&received);
    let path = link
        .split_once("/auth/confirm")
        .map(|(_, rest)| format!("/auth/confirm{rest}"))
        .expect("the link is a confirm link");

    // A token that was never issued is refused before anything else.
    let (status, _) = http(gw.port, "GET", "/auth/confirm?token=nonsense", None, "");
    assert_eq!(status, 400);

    let (status, body) = http(gw.port, "GET", &path, None, "");
    assert_eq!(status, 200, "following the link: {body}");

    // The link is spent: a mailbox is not a place to leave a working one.
    let (status, _) = http(gw.port, "GET", &path, None, "");
    assert_eq!(status, 400);
}

#[test]
fn a_resend_says_nothing_about_who_exists() {
    let (smtp_port, _mail) = smtp_sink();
    let gw = spawn_gateway_with(
        "confirm-resend",
        &[("BAYLEE_SMTP_URL", format!("smtp://127.0.0.1:{smtp_port}"))],
    );
    let (status, body) = register(gw.port, "someone", "Someone");
    assert_eq!(status, 200, "register: {body}");
    give_an_address(&gw, "someone", "someone@example.com");
    for email in ["someone@example.com", "nobody-at-all@example.com"] {
        let (status, body) = http(
            gw.port,
            "POST",
            "/auth/confirm/resend",
            None,
            &format!("{{\"email\":\"{email}\"}}"),
        );
        assert_eq!(status, 200, "resend for {email}: {body}");
        assert!(body.contains("\"ok\":true"), "resend for {email}: {body}");
    }
}
