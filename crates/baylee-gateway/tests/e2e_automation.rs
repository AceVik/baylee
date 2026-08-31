//! End-to-end test for remembered standing answers: the gateway stores a
//! seat's "always answer yes to this ability" per **account**, so it can be
//! replayed into every new game.
//!
//! The engine addresses those answers by `AbilityRef`, a handle that says
//! nothing about a particular game — that is what makes them storable at
//! all — and the gateway must not take a client's word for what a valid
//! handle is.

#![allow(clippy::missing_docs_in_private_items)]

use std::io::{Read, Write};

fn http(port: u16, method: &str, path: &str, token: Option<&str>, body: &str) -> (u16, String) {
    let mut stream = std::net::TcpStream::connect(("127.0.0.1", port)).expect("connect http");
    let auth = token.map_or(String::new(), |t| format!("Authorization: Bearer {t}\r\n"));
    let request = format!(
        "{method} {path} HTTP/1.1\r\nHost: 127.0.0.1\r\nContent-Type: application/json\r\n{auth}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    );
    stream.write_all(request.as_bytes()).expect("write http");
    let mut raw = String::new();
    stream.read_to_string(&mut raw).expect("read http");
    let status: u16 = raw
        .split_whitespace()
        .nth(1)
        .and_then(|s| s.parse().ok())
        .expect("http status");
    let body = raw.split("\r\n\r\n").nth(1).unwrap_or("").to_string();
    (status, body)
}

fn json_field<'a>(body: &'a str, field: &str) -> &'a str {
    let marker = format!("\"{field}\":\"");
    let start = body.find(&marker).expect("field present") + marker.len();
    let rest = &body[start..];
    let end = rest.find('"').expect("field ends");
    &rest[..end]
}

struct Gateway {
    port: u16,
    child: std::process::Child,
    store_path: std::path::PathBuf,
}

impl Drop for Gateway {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
        let _ = std::fs::remove_file(&self.store_path);
    }
}

fn spawn_gateway() -> Gateway {
    let probe = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let port = probe.local_addr().unwrap().port();
    drop(probe);
    let store_path = std::env::temp_dir().join(format!("baylee-gateway-auto-{port}.json"));
    let _ = std::fs::remove_file(&store_path);
    let child = std::process::Command::new(env!("CARGO_BIN_EXE_baylee-gateway"))
        .env("PORT", port.to_string())
        .env("STORE_PATH", &store_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("spawn gateway");
    for _ in 0..50 {
        if std::net::TcpStream::connect(("127.0.0.1", port)).is_ok() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    Gateway {
        port,
        child,
        store_path,
    }
}

fn login(port: u16, email: &str, name: &str) -> String {
    let register = format!(
        "{{\"email\":\"{email}\",\"display_name\":\"{name}\",\"password\":\"a-very-fine-password\"}}"
    );
    let (status, body) = http(port, "POST", "/auth/register", None, &register);
    assert_eq!(status, 200, "register: {body}");
    let creds = format!("{{\"email\":\"{email}\",\"password\":\"a-very-fine-password\"}}");
    let (status, body) = http(port, "POST", "/auth/login", None, &creds);
    assert_eq!(status, 200, "login: {body}");
    json_field(&body, "token").to_string()
}

fn ondu_cleric() -> u32 {
    baylee_cards::by_oracle_id("f4232466-dd6a-49bf-be6c-95905c3ded17")
        .expect("the card pool has Ondu Cleric")
        .index
        .get()
}

#[test]
fn standing_answers_are_remembered_per_account() {
    let gw = spawn_gateway();
    let token = login(gw.port, "cleric@example.com", "cleric_fan");

    let (status, body) = http(gw.port, "GET", "/automation", Some(&token), "");
    assert_eq!(status, 200, "empty listing: {body}");
    assert!(
        body.contains("\"answers\":[]"),
        "expected nothing yet: {body}"
    );

    // Ondu Cleric's rally trigger — the card the feature was asked for by
    // name. Sent twice and out of order to prove the gateway normalises.
    let card = ondu_cleric();
    let put = format!(
        "{{\"answers\":[{{\"card\":{card},\"ability\":1,\"yes\":false}},\
          {{\"card\":{card},\"ability\":0,\"yes\":true}},\
          {{\"card\":{card},\"ability\":0,\"yes\":true}}]}}"
    );
    let (status, body) = http(gw.port, "PUT", "/automation", Some(&token), &put);
    assert_eq!(status, 200, "store answers: {body}");
    assert!(
        body.contains("\"stored\":2"),
        "the duplicate was not collapsed: {body}"
    );

    let (status, body) = http(gw.port, "GET", "/automation", Some(&token), "");
    assert_eq!(status, 200);
    assert!(
        body.contains(&format!("\"card\":{card}")) && body.contains("\"yes\":true"),
        "the answers did not come back: {body}"
    );

    // A handle no card can ever produce is refused rather than stored: it
    // could never fire, and junk in the store outlives the request.
    let bad = "{\"answers\":[{\"card\":4000000,\"ability\":0,\"yes\":true}]}";
    let (status, _) = http(gw.port, "PUT", "/automation", Some(&token), bad);
    assert_eq!(status, 400, "an unknown card was accepted");

    // And it is per account: a second account sees none of it.
    let other = login(gw.port, "other@example.com", "other_player");
    let (status, body) = http(gw.port, "GET", "/automation", Some(&other), "");
    assert_eq!(status, 200);
    assert!(
        body.contains("\"answers\":[]"),
        "one account's setting leaked into another: {body}"
    );

    // Unauthenticated callers get nothing.
    let (status, _) = http(gw.port, "GET", "/automation", None, "");
    assert_eq!(
        status, 401,
        "an anonymous caller read an account's settings"
    );
}
