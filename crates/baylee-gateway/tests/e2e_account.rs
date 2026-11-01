//! End-to-end tests for `DELETE /account` (#292): a player deletes their own
//! account, and with it everything the gateway keeps about them.
//!
//! Asked of a real gateway over HTTP, and checked twice: by what the routes
//! answer afterwards (no session works, no picture is served) and by what is
//! left in the database's own tables, because a route that answers 401 says
//! nothing about a row nobody reads.
//!
//! A deleted account's seat at a running table and its open sockets are
//! `e2e_hvh`'s, where there is a game to sit at.
//!
//! The password is not asked again: no account here has one to ask since
//! accounts became Baylee Central's (`docs/central-login.md` §"Cutover"),
//! and the routes that made, checked and changed passwords are gone.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use common::{Gateway, http, http_bytes, json_field, login, spawn_gateway, spawn_gateway_with};

/// A picture of its own for each `seed`, as JPEG bytes.
fn a_picture(seed: u8) -> Vec<u8> {
    let mut img = image::RgbImage::new(400, 560);
    for (x, y, pixel) in img.enumerate_pixels_mut() {
        *pixel = image::Rgb([(x % 251) as u8, (y % 241) as u8, seed]);
    }
    let mut out = std::io::Cursor::new(Vec::new());
    image::DynamicImage::ImageRgb8(img)
        .write_to(&mut out, image::ImageFormat::Jpeg)
        .expect("encode a jpeg");
    out.into_inner()
}

fn upload(gw: &Gateway, token: &str, picture: &[u8]) -> String {
    let (status, body) = http_bytes(
        gw.port,
        "POST",
        "/images?kind=sleeve",
        Some(token),
        "image/jpeg",
        picture,
    );
    let body = String::from_utf8_lossy(&body).to_string();
    assert_eq!(status, 200, "upload: {body}");
    json_field(&body, "id").to_string()
}

/// Whether `GET /images/{id}` still serves the picture.
fn served(gw: &Gateway, id: &str) -> bool {
    http_bytes(gw.port, "GET", &format!("/images/{id}"), None, "", &[]).0 == 200
}

/// A second session for an account `common::login` already made, as a
/// second device holds one: its bearer token. Written into the table as
/// `common::login` writes the first, because no route makes one any more;
/// an account that is not there fails the insert rather than the test's
/// next assertion.
fn another_session(gw: &Gateway, username: &str) -> String {
    use sha2::Digest as _;
    use std::fmt::Write as _;
    let hex = |bytes: &[u8]| {
        bytes.iter().fold(String::new(), |mut out, b| {
            write!(out, "{b:02x}").expect("writing to a String");
            out
        })
    };
    let mut bytes = [0u8; 32];
    getrandom::fill(&mut bytes).expect("OS randomness");
    let token = hex(&bytes);
    let digest = hex(&sha2::Sha256::digest(token.as_bytes()));
    gw.sql(&format!(
        "INSERT INTO session_token (token_hash, account_id, expires_at)
         VALUES (decode('{digest}', 'hex'),
                 (SELECT id FROM account WHERE username_key = '{}'),
                 clock_timestamp() + interval '12 hours')",
        username.to_ascii_lowercase()
    ));
    token
}

/// `DELETE /account` as a client asks it now: `{}`.
fn farewell(gw: &Gateway, token: &str) -> (u16, String) {
    http(gw.port, "DELETE", "/account", Some(token), "{}")
}

/// How many rows of `table` belong to `username`'s account.
fn rows(gw: &Gateway, table: &str, username: &str) -> i64 {
    gw.scalar(&format!(
        "SELECT count(*) FROM {table} t JOIN account a ON a.id = t.account_id \
         WHERE a.username = '{username}'"
    ))
}

/// #292: everything the leaver owned goes, everything the stayer owns stays.
///
/// The two share one picture, which is one file, so the file outlives the
/// leaver and goes with the stayer. The leaver's other picture goes with
/// the leaver. Both of the leaver's sessions end, not only the one that
/// asked.
#[test]
#[allow(clippy::too_many_lines)] // e2e scenario script
fn deleting_an_account_takes_everything_it_owned_and_nothing_else() {
    let dir = std::env::temp_dir().join(format!("baylee-farewell-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let gw = spawn_gateway_with(
        "farewell",
        &[("BAYLEE_DECK_IMAGE_PATH", dir.to_string_lossy().into_owned())],
    );

    let leaver = login(gw.port, "leaver", "Leaver");
    let other_session = another_session(&gw, "leaver");
    let stayer = login(gw.port, "stayer", "Stayer");

    let shared = upload(&gw, &leaver, &a_picture(1));
    assert_eq!(upload(&gw, &stayer, &a_picture(1)), shared, "one file");
    let own = upload(&gw, &leaver, &a_picture(2));
    for (token, sleeve) in [(&leaver, &own), (&stayer, &shared)] {
        let deck =
            format!("{{\"name\":\"Sleeved\",\"cards\":[\"60 Forest\"],\"sleeve\":\"{sleeve}\"}}");
        let (status, body) = http(gw.port, "POST", "/decks", Some(token), &deck);
        assert_eq!(status, 200, "{body}");
        let (status, body) = http(gw.port, "PUT", "/settings", Some(token), "{\"keymap\":{}}");
        assert!(status == 200 || status == 204, "{status} {body}");
    }
    let tables = ["deck", "client_settings", "upload", "session_token"];
    let total = |table: &str| gw.scalar(&format!("SELECT count(*) FROM {table}"));
    let before: Vec<(i64, i64)> = tables
        .iter()
        .map(|table| (total(table), rows(&gw, table, "leaver")))
        .collect();
    for (table, (_, theirs)) in tables.iter().zip(&before) {
        assert!(*theirs > 0, "the leaver has {table} rows");
    }

    let (status, body) = farewell(&gw, &leaver);
    assert_eq!(status, 204, "{body}");

    for token in [&leaver, &other_session] {
        let (status, _) = http(gw.port, "GET", "/me", Some(token), "");
        assert_eq!(status, 401, "every session of the account ended");
    }
    assert_eq!(
        gw.scalar("SELECT count(*) FROM account WHERE username = 'leaver'"),
        0
    );
    for (table, (all, theirs)) in tables.iter().zip(&before) {
        assert_eq!(
            total(table),
            all - theirs,
            "{table}: the leaver's rows went, and only theirs"
        );
    }
    assert!(!served(&gw, &own), "the leaver's own picture is gone");
    assert!(!dir.join(format!("{own}.jpg")).exists(), "and its file");
    assert!(served(&gw, &shared), "the shared one is the stayer's too");

    // The stayer lost nothing.
    let (status, _) = http(gw.port, "GET", "/me", Some(&stayer), "");
    assert_eq!(status, 200);
    for table in ["deck", "client_settings", "upload", "session_token"] {
        assert!(rows(&gw, table, "stayer") > 0, "the stayer's {table} rows");
    }

    // And when the stayer goes, nobody claims the shared picture. Asked as
    // a client from before the cutover asks, with a password, which is
    // ignored rather than refused.
    let (status, body) = http(
        gw.port,
        "DELETE",
        "/account",
        Some(&stayer),
        r#"{"password":"what it used to be"}"#,
    );
    assert_eq!(status, 204, "{body}");
    assert!(!served(&gw, &shared), "the last owner took the file");
    assert!(!dir.join(format!("{shared}.jpg")).exists());
    assert_eq!(gw.scalar("SELECT count(*) FROM upload"), 0);

    let _ = std::fs::remove_dir_all(&dir);
}

/// A guest has no password, so its session is enough, and it goes as a
/// signed-out guest does.
#[test]
fn a_guest_deletes_itself_with_its_session() {
    let gw = spawn_gateway("farewell-guest");
    let (status, body) = http(
        gw.port,
        "POST",
        "/auth/guest",
        None,
        "{\"display_name\":\"Passer\"}",
    );
    assert_eq!(status, 200, "{body}");
    let token = json_field(&body, "token").to_string();
    let (status, body) = http(gw.port, "DELETE", "/account", Some(&token), "{}");
    assert_eq!(status, 204, "{body}");
    let (status, _) = http(gw.port, "GET", "/me", Some(&token), "");
    assert_eq!(status, 401);
    assert_eq!(gw.scalar("SELECT count(*) FROM account WHERE guest"), 0);
}

/// The routes a gateway's own accounts were made, signed in, confirmed and
/// given new passwords by went with them (`docs/central-login.md`
/// §"Cutover"): each is no route at all, not a refusal, and asking one
/// makes nothing. The console's reset is `e2e_moderation`'s, where there is
/// a console to ask.
#[test]
fn the_password_routes_are_no_routes() {
    let gw = spawn_gateway("farewell-routes");
    for (method, path, body) in [
        (
            "POST",
            "/auth/register",
            r#"{"username":"newcomer","display_name":"Newcomer","password":"a-very-fine-password"}"#,
        ),
        (
            "POST",
            "/auth/login",
            r#"{"username":"newcomer","password":"a-very-fine-password"}"#,
        ),
        (
            "POST",
            "/auth/password",
            r#"{"username":"newcomer","password":"a-very-fine-password","new_password":"another-fine-password"}"#,
        ),
        ("GET", "/auth/confirm?token=0123456789abcdef", ""),
        ("POST", "/auth/confirm/resend", r#"{"username":"newcomer"}"#),
    ] {
        let (status, answer) = http(gw.port, method, path, None, body);
        assert_eq!(status, 404, "{method} {path}: {answer}");
    }
    assert_eq!(
        gw.scalar("SELECT count(*) FROM account"),
        0,
        "nobody was made"
    );
}
