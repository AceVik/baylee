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
        .recv_timeout(common::WAIT_BUDGET)
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

/// Sign-in attempts are counted against the **address**, not the machine.
///
/// Keyed per IP, one development box is one bucket: the owner's typing and
/// every scripted call this session made shared ten attempts, and the owner
/// was the one locked out. An address is what guessing is aimed at, so it is
/// what the count belongs to — and a wrong password at one address must not
/// cost another address anything.
#[test]
fn eight_tries_are_eight_tries_at_one_address() {
    let gw = spawn_gateway("attempts");
    let port = gw.port;
    let (status, body) = register(port, "eight@example.com", "Eight", "en");
    assert_eq!(status, 200, "register: {body}");
    let (status, body) = register(port, "other@example.com", "Other", "en");
    assert_eq!(status, 200, "register: {body}");

    let wrong = |email: &str| {
        http(
            port,
            "POST",
            "/auth/login",
            None,
            &format!("{{\"email\":\"{email}\",\"password\":\"not-it\"}}"),
        )
        .0
    };

    // Eight are answered on their merits; the ninth is not answered at all.
    for i in 1..=8 {
        assert_eq!(wrong("eight@example.com"), 401, "try {i} is a real answer");
    }
    assert_eq!(wrong("eight@example.com"), 429, "the ninth is refused");
    // The same address in a different spelling is the same account, so it is
    // the same count.
    assert_eq!(wrong("Eight@Example.com"), 429, "one account, one count");

    // And the machine is not what was counted.
    assert_eq!(
        wrong("other@example.com"),
        401,
        "a neighbour's typing costs this address nothing"
    );
    let (status, body) = sign_in(port, "other@example.com");
    assert_eq!(status, 200, "and the right password still works: {body}");
}

/// Getting in is what the window was counting towards, so it hands the tries
/// back: a player who mistypes six times, signs in, then signs out has not
/// spent anything.
#[test]
fn signing_in_clears_what_the_typos_spent() {
    let gw = spawn_gateway("attempts-cleared");
    let port = gw.port;
    let (status, body) = register(port, "clear@example.com", "Clear", "en");
    assert_eq!(status, 200, "register: {body}");

    for _ in 0..7 {
        let (status, _) = http(
            port,
            "POST",
            "/auth/login",
            None,
            "{\"email\":\"clear@example.com\",\"password\":\"not-it\"}",
        );
        assert_eq!(status, 401);
    }
    let (status, body) = sign_in(port, "clear@example.com");
    assert_eq!(status, 200, "the eighth try is the right one: {body}");
    // Without the clearing this would be the ninth attempt and refused.
    let (status, body) = sign_in(port, "clear@example.com");
    assert_eq!(status, 200, "and signing in again is not rationed: {body}");
}

/// Registering an address twice answers exactly what registering it once
/// answers, and leaves the first account alone.
///
/// The refusal moved from a check in the gateway to a unique index in the
/// database, and the gateway recognises it by reading the driver's error
/// text. If that reading ever misses, `create_account` returns `Err` and the
/// route answers **503** — which is an oracle telling an attacker exactly
/// which addresses exist, in the one route written from top to bottom to
/// avoid saying so. Nothing else in the suite registers the same address
/// twice, so nothing else would notice.
#[test]
fn a_second_registration_says_no_more_than_the_first() {
    let gw = spawn_gateway("taken");

    let (status, body) = register(gw.port, "twice@example.com", "Twice", "en");
    assert_eq!(status, 200, "the first registration: {body}");
    let first = body;

    // The same address in another case is the same address: the index is on
    // `lower(email)`.
    let (status, body) = register(gw.port, "TWICE@Example.COM", "Somebody", "en");
    assert_eq!(
        status, 200,
        "a taken address must answer as a free one does, not 503: {body}"
    );
    assert_eq!(body, first, "the two answers differ, which is the leak");

    // A display name is **not** unique, and this is where that stops being
    // a claim in a doc comment. The second `twice` is a second account.
    let (status, body) = register(gw.port, "other@example.com", "twice", "en");
    assert_eq!(status, 200, "a name shared with somebody: {body}");
    assert_eq!(body, first, "the two answers differ, which is the leak");

    // The account that was there first is untouched, and the second one
    // exists — which is the half the old rule got wrong.
    let (status, body) = sign_in(gw.port, "twice@example.com");
    assert_eq!(status, 200, "the original account still signs in: {body}");
    let mine = token_of(&body);
    let (status, body) = sign_in(gw.port, "other@example.com");
    assert_eq!(status, 200, "the second `twice` must exist: {body}");
    let theirs = token_of(&body);

    // And what tells them apart is the tag, which each of them can read off
    // their own profile and neither of them chose.
    let (status, my_profile) = http(gw.port, "GET", "/me", Some(&mine), "");
    assert_eq!(status, 200, "/me: {my_profile}");
    let (status, their_profile) = http(gw.port, "GET", "/me", Some(&theirs), "");
    assert_eq!(status, 200, "/me: {their_profile}");
    assert!(
        my_profile.contains("\"display_name\":\"Twice\""),
        "{my_profile}"
    );
    assert!(
        their_profile.contains("\"display_name\":\"twice\""),
        "{their_profile}"
    );
    assert_ne!(
        handle_of(&my_profile),
        handle_of(&their_profile),
        "two players called twice were handed the same handle"
    );

    // The tag is how one of them finds the other, and a bare name is not.
    let (status, found) = http(
        gw.port,
        "GET",
        &format!("/players/%23{}", tag_of(&their_profile)),
        Some(&mine),
        "",
    );
    assert_eq!(status, 200, "looking somebody up by tag: {found}");
    assert_eq!(handle_of(&found), handle_of(&their_profile));

    let (status, refused) = http(gw.port, "GET", "/players/twice", Some(&mine), "");
    assert_eq!(
        status, 400,
        "a bare name is not a handle — it would answer for whichever twice \
         registered first: {refused}"
    );
}

/// The session token out of a sign-in answer.
fn token_of(body: &str) -> String {
    field(body, "token")
}

/// The `handle` field out of a `/me` or `/players` answer.
fn handle_of(body: &str) -> String {
    field(body, "handle")
}

/// The `tag` field out of a `/me` answer.
fn tag_of(body: &str) -> String {
    field(body, "tag")
}

/// One string field out of a flat JSON object, without a parser.
fn field(body: &str, name: &str) -> String {
    let key = format!("\"{name}\":\"");
    let from = body
        .find(&key)
        .unwrap_or_else(|| panic!("no {name} in {body}"))
        + key.len();
    let rest = &body[from..];
    rest[..rest.find('"').expect("unterminated string")].to_string()
}
