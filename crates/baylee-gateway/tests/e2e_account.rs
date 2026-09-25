//! End-to-end tests for `DELETE /account` (#292): a player deletes their own
//! account, and with it everything the gateway keeps about them.
//!
//! Asked of a real gateway over HTTP, and checked twice: by what the routes
//! answer afterwards (no session works, no sign-in works, no picture is
//! served) and by what is left in the database's own tables, because a
//! route that answers 401 says nothing about a row nobody reads.
//!
//! A deleted account's seat at a running table and its open sockets are
//! `e2e_hvh`'s, where there is a game to sit at.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use common::{Gateway, http, http_bytes, json_field, login, spawn_gateway, spawn_gateway_with};

/// The password `common::login` registers every account with.
const PASSWORD: &str = "a-very-fine-password";

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

/// A second session for an account `common::login` already made.
fn sign_in(gw: &Gateway, username: &str) -> (u16, String) {
    let creds = format!("{{\"username\":\"{username}\",\"password\":\"{PASSWORD}\"}}");
    http(gw.port, "POST", "/auth/login", None, &creds)
}

fn farewell(gw: &Gateway, token: &str, password: &str) -> (u16, String) {
    let body = format!("{{\"password\":\"{password}\"}}");
    http(gw.port, "DELETE", "/account", Some(token), &body)
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
/// asked, and the password signs in nobody.
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
    let (status, body) = sign_in(&gw, "leaver");
    assert_eq!(status, 200, "{body}");
    let other_session = json_field(&body, "token").to_string();
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

    // A wrong password deletes nothing, and says so as a refusal rather
    // than as a spent session.
    let (status, body) = farewell(&gw, &leaver, "not-the-password");
    assert_eq!(status, 403, "{body}");
    let (status, _) = http(gw.port, "GET", "/me", Some(&leaver), "");
    assert_eq!(status, 200, "still there");

    let (status, body) = farewell(&gw, &leaver, PASSWORD);
    assert_eq!(status, 204, "{body}");

    for token in [&leaver, &other_session] {
        let (status, _) = http(gw.port, "GET", "/me", Some(token), "");
        assert_eq!(status, 401, "every session of the account ended");
    }
    assert_eq!(sign_in(&gw, "leaver").0, 401, "nobody to sign in as");
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

    // And when the stayer goes, nobody claims the shared picture.
    let (status, body) = farewell(&gw, &stayer, PASSWORD);
    assert_eq!(status, 204, "{body}");
    assert!(!served(&gw, &shared), "the last owner took the file");
    assert!(!dir.join(format!("{shared}.jpg")).exists());
    assert_eq!(gw.scalar("SELECT count(*) FROM upload"), 0);

    let _ = std::fs::remove_dir_all(&dir);
}

/// The password is asked again under the same eight tries a sign-in has, so
/// a session left open on somebody else's machine cannot guess its way to
/// deleting the account.
#[test]
fn a_deletion_is_counted_like_a_sign_in() {
    let gw = spawn_gateway("farewell-tries");
    let token = login(gw.port, "careful", "Careful");
    for attempt in 1..=8 {
        let (status, body) = farewell(&gw, &token, "guess");
        assert_eq!(status, 403, "try {attempt}: {body}");
    }
    let (status, body) = farewell(&gw, &token, PASSWORD);
    assert_eq!(status, 429, "the ninth try waits, right or wrong: {body}");
    let (status, _) = http(gw.port, "GET", "/me", Some(&token), "");
    assert_eq!(status, 200, "and nothing was deleted");
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
