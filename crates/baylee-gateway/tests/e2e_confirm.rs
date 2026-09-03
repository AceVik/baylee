//! Registration confirmation, end to end through a real SMTP conversation.
//!
//! The link cannot be recovered from the store — only its hash is kept there,
//! for the same reason a session token's is — so the only honest way to test
//! the happy path is to receive the mail. The sink below speaks just enough
//! SMTP for lettre to hand over a message, which is also what proves the
//! gateway's transport is configured the way a real relay would need.
//!
//! The other half of the feature is the half that must not change: a gateway
//! with no `BAYLEE_SMTP_URL` confirms an account on creation, sends nothing,
//! and lets it log in immediately. Every other test in this directory is that
//! assertion, so it is only stated once here.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use common::{http, spawn_gateway, spawn_gateway_with};
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

fn register(port: u16, email: &str, name: &str, lang: &str) -> (u16, String) {
    let body = format!(
        "{{\"email\":\"{email}\",\"display_name\":\"{name}\",\
         \"password\":\"{PASSWORD}\",\"lang\":\"{lang}\"}}"
    );
    http(port, "POST", "/auth/register", None, &body)
}

fn sign_in(port: u16, email: &str) -> (u16, String) {
    let body = format!("{{\"email\":\"{email}\",\"password\":\"{PASSWORD}\"}}");
    http(port, "POST", "/auth/login", None, &body)
}

#[test]
fn a_gateway_with_no_mailer_confirms_on_the_spot() {
    let gw = spawn_gateway("confirm-off");
    let (status, body) = http(gw.port, "GET", "/auth/config", None, "");
    assert_eq!(status, 200);
    assert!(
        body.contains("\"confirmation_required\":false"),
        "config: {body}"
    );

    let (status, body) = register(gw.port, "nobody@example.com", "Nobody", "en");
    assert_eq!(status, 200, "register: {body}");
    let (status, body) = sign_in(gw.port, "nobody@example.com");
    assert_eq!(status, 200, "the account must be usable at once: {body}");
}

#[test]
fn a_mailed_link_is_what_lets_the_account_in() {
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
    assert!(
        body.contains("\"confirmation_required\":true"),
        "config ({status}): {body}"
    );

    let (status, body) = register(gw.port, "player@example.com", "Player", "de");
    assert_eq!(status, 200, "register: {body}");
    assert!(
        body.contains("\"confirmation_required\":true"),
        "the client is told to expect a mail: {body}"
    );

    // Unconfirmed is refused, and refused *differently* from a wrong
    // password — a player who cannot tell the two apart cannot act on it.
    let (status, body) = sign_in(gw.port, "player@example.com");
    assert_eq!(status, 403, "login before confirming: {body}");
    let (status, _) = http(
        gw.port,
        "POST",
        "/auth/login",
        None,
        r#"{"email":"player@example.com","password":"the-wrong-password"}"#,
    );
    assert_eq!(status, 401, "a wrong password stays a wrong password");

    let received = mail
        .recv_timeout(std::time::Duration::from_secs(10))
        .expect("the confirmation mail arrives");
    assert!(
        received.contains("Best") || received.contains("=?utf-8?"),
        "written in the language it registered in: {received}"
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
    let (status, body) = sign_in(gw.port, "player@example.com");
    assert_eq!(status, 200, "login after confirming: {body}");

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
    let (status, body) = register(gw.port, "someone@example.com", "Someone", "en");
    assert_eq!(status, 200, "register: {body}");
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
