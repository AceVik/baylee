//! End-to-end tests for guests (#269): a player reaches a table with a name
//! and nothing else.
//!
//! `POST /auth/guest` hands out an account with no username, no address and
//! no password, and its session. The session is the guest: it lives thirty
//! days from the last time it played, nothing signs in as a guest, and once
//! no session leads to one it is deleted with its decks. Whether a gateway
//! takes guests is its auth mode's (`guests`, `central`, the default, or
//! `mixed`; `docs/central-login.md` §"Who a gateway lets in"), whether or
//! not it is connected to Baylee Central, and it takes a bounded number of
//! them (`BAYLEE_GUEST_CAP`).

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use common::{
    Gateway, attach_agent, http, http_bytes, json_field, json_number, login, spawn_gateway,
    spawn_gateway_with,
};

/// A guest's token and handle, asked for with `body`.
fn a_guest(port: u16, body: &str) -> (String, String) {
    let (status, answer) = http(port, "POST", "/auth/guest", None, body);
    assert_eq!(status, 200, "guest: {answer}");
    assert!(answer.contains("\"guest\":true"), "{answer}");
    (
        json_field(&answer, "token").to_string(),
        json_field(&answer, "handle").to_string(),
    )
}

/// A deck this account owns, and its id.
fn make_deck(port: u16, token: &str, name: &str) -> String {
    let body = format!("{{\"name\":\"{name}\",\"cards\":[\"40 Forest\",\"20 Swamp\"]}}");
    let (status, body) = http(port, "POST", "/decks", Some(token), &body);
    assert_eq!(status, 200, "create deck: {body}");
    json_field(&body, "deck_id").to_string()
}

/// How many guests the gateway holds, and how many decks they own.
fn guests_and_their_decks(gw: &Gateway) -> (i64, i64) {
    (
        gw.scalar("SELECT count(*) FROM account WHERE guest"),
        gw.scalar(
            "SELECT count(*) FROM deck d JOIN account a ON a.id = d.account_id WHERE a.guest",
        ),
    )
}

/// The acceptance of #269: a fresh player on a gateway reaches a table
/// without an address — here as its host, which is the most a guest does.
#[tokio::test]
async fn a_guest_hosts_a_table_that_a_registered_player_joins() {
    let gw = spawn_gateway("guests_table");
    let port = gw.port;
    let _agent = attach_agent(&gw).await;

    let (guest, handle) = a_guest(port, "{}");
    assert!(handle.starts_with("Guest#"), "{handle}");
    let (status, me) = http(port, "GET", "/me", Some(&guest), "");
    assert_eq!(status, 200, "{me}");
    assert!(me.contains("\"guest\":true"), "{me}");
    assert!(
        me.contains("\"username\":null"),
        "a guest has no name: {me}"
    );
    assert!(me.contains("\"email\":null"), "{me}");

    let player = login(port, "player", "Player");
    let guest_deck = make_deck(port, &guest, "guest-deck");
    let player_deck = make_deck(port, &player, "player-deck");

    let create = format!("{{\"deck_id\":\"{guest_deck}\",\"seats\":2,\"name\":\"Guest table\"}}");
    let (status, body) = http(port, "POST", "/lobby/games", Some(&guest), &create);
    assert_eq!(status, 200, "a guest opens a table: {body}");
    let game_id = json_field(&body, "game_id").to_string();

    let join = format!("{{\"deck_id\":\"{player_deck}\"}}");
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game_id}/join"),
        Some(&player),
        &join,
    );
    assert_eq!(status, 200, "join: {body}");
    for token in [&guest, &player] {
        let (status, body) = http(
            port,
            "POST",
            &format!("/lobby/games/{game_id}/ready"),
            Some(token),
            "{}",
        );
        assert_eq!(status, 200, "ready: {body}");
    }
    let (status, body) = http(
        port,
        "POST",
        &format!("/lobby/games/{game_id}/start"),
        Some(&guest),
        "",
    );
    assert_eq!(status, 200, "the guest starts it: {body}");
    let (_, listing) = http(port, "GET", "/lobby/games", Some(&player), "");
    assert!(listing.contains("\"state\":\"playing\""), "{listing}");
}

/// A guest is called what it asked to be, under the display-name rule, and
/// `Guest` when it asked for nothing.
#[test]
fn a_guest_is_called_what_it_asked_to_be_or_guest() {
    let gw = spawn_gateway("guests_named");
    let port = gw.port;
    let (_, handle) = a_guest(port, "{\"display_name\":\"Casper\",\"lang\":\"de\"}");
    assert!(handle.starts_with("Casper#"), "{handle}");
    let (_, handle) = a_guest(port, "{\"display_name\":\"\"}");
    assert!(handle.starts_with("Guest#"), "empty is nothing: {handle}");
    let (status, body) = http(
        port,
        "POST",
        "/auth/guest",
        None,
        "{\"display_name\":\"no spaces\"}",
    );
    assert_eq!(status, 400, "{body}");
    assert!(body.contains("invalid display name"), "{body}");
}

/// A guest's session is the only way in, and it lives thirty days.
#[test]
fn a_guests_session_lives_thirty_days() {
    let gw = spawn_gateway("guests_session");
    let port = gw.port;
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("after 1970")
        .as_secs();
    let (status, answer) = http(port, "POST", "/auth/guest", None, "{}");
    assert_eq!(status, 200, "{answer}");
    let expires = u64::try_from(json_number(&answer, "expires_at")).expect("a time");
    let thirty_days = 30 * 24 * 3600;
    assert!(
        expires.abs_diff(now + thirty_days) < 60,
        "{expires} against {now}"
    );
}

/// A guest's session is renewed once a day of it has gone, to thirty days;
/// an account's at its half-life, to twelve hours — never to a guest's
/// thirty days. Read from the table, because the answer carries no expiry.
#[test]
fn a_guest_session_is_renewed_to_thirty_days_and_an_accounts_to_twelve_hours() {
    let gw = spawn_gateway("guests_renew");
    let port = gw.port;
    let (guest, _) = a_guest(port, "{}");
    let account = login(port, "keeper", "Keeper");
    let left = |guest: bool| {
        gw.scalar(&format!(
            "SELECT extract(epoch FROM s.expires_at - now())::bigint FROM session_token s \
             JOIN account a ON a.id = s.account_id WHERE a.guest = {guest}"
        ))
    };
    let set_left = |guest: bool, interval: &str| {
        gw.sql(&format!(
            "UPDATE session_token s SET expires_at = now() + interval '{interval}' \
             FROM account a WHERE a.id = s.account_id AND a.guest = {guest}"
        ));
    };
    let day = 24 * 3600;
    let hour = 3600;

    // Less than a day in: nothing is written.
    set_left(true, "29 days 12 hours");
    assert_eq!(http(port, "GET", "/me", Some(&guest), "").0, 200);
    assert!((29 * day..29 * day + 13 * hour).contains(&left(true)));
    // Two days in: renewed, to thirty from now.
    set_left(true, "28 days");
    assert_eq!(http(port, "GET", "/me", Some(&guest), "").0, 200);
    assert!(left(true) > 30 * day - 60, "{}", left(true));

    // An account's, past its half-life: twelve hours, not thirty days.
    set_left(false, "5 hours");
    assert_eq!(http(port, "GET", "/me", Some(&account), "").0, 200);
    let renewed = left(false);
    assert!((12 * hour - 60..=12 * hour).contains(&renewed), "{renewed}");
}

/// Signing out is the end of a guest: its session was the only way in, so
/// it goes at once with its decks, and its token opens nothing.
#[test]
fn a_guest_that_signs_out_is_gone_with_its_decks() {
    let gw = spawn_gateway("guests_leave");
    let port = gw.port;
    let (guest, _) = a_guest(port, "{}");
    let (staying, _) = a_guest(port, "{}");
    make_deck(port, &guest, "leaving");
    make_deck(port, &staying, "staying");
    let account = login(port, "keeper", "Keeper");
    make_deck(port, &account, "kept");
    assert_eq!(guests_and_their_decks(&gw), (2, 2));

    let (status, body) = http(port, "POST", "/auth/logout", Some(&guest), "");
    assert_eq!(status, 204, "{body}");
    assert_eq!(
        guests_and_their_decks(&gw),
        (1, 1),
        "the one that left, and only it"
    );
    assert_eq!(http(port, "GET", "/me", Some(&guest), "").0, 401);

    // An account signing out keeps everything.
    let (status, _) = http(port, "POST", "/auth/logout", Some(&account), "");
    assert_eq!(status, 204);
    assert_eq!(
        gw.scalar("SELECT count(*) FROM account WHERE NOT guest"),
        1,
        "an account outlives its session"
    );
    assert_eq!(
        gw.scalar("SELECT count(*) FROM deck WHERE name = 'kept'"),
        1
    );
}

/// The default mode, `central` (a blank `BAYLEE_AUTH_MODE` is unset): no
/// guests, and the client is told before it offers. A session already made
/// is the mode's no longer: it goes on.
#[test]
fn the_default_takes_no_guests_and_says_so() {
    let open = spawn_gateway_with("guests_open", &[("BAYLEE_AUTH_MODE", "guests".into())]);
    let (_, config) = http(open.port, "GET", "/auth/config", None, "");
    assert!(config.contains("\"guests_enabled\":true"), "{config}");
    assert!(config.contains("\"auth_mode\":\"guests\""), "{config}");

    let closed = spawn_gateway_with("guests_closed", &[("BAYLEE_AUTH_MODE", String::new())]);
    let (_, config) = http(closed.port, "GET", "/auth/config", None, "");
    assert!(config.contains("\"guests_enabled\":false"), "{config}");
    assert!(config.contains("\"auth_mode\":\"central\""), "{config}");
    assert!(
        config.contains("\"central\":false"),
        "nothing reaches it: {config}"
    );
    let (_, info) = http(closed.port, "GET", "/info", None, "");
    assert!(info.contains("\"auth_mode\":\"central\""), "{info}");
    assert!(info.contains("\"guests\":false"), "{info}");
    let (status, body) = http(closed.port, "POST", "/auth/guest", None, "{}");
    assert_eq!(status, 403, "{body}");
    assert!(body.contains("this gateway takes no guests"), "{body}");
    let member = login(closed.port, "member", "Member");
    assert_eq!(http(closed.port, "GET", "/me", Some(&member), "").0, 200);
}

/// The Baylee Central configuration a gateway is started with, written for
/// one test and removed with it, as `e2e_central`'s are, and readable by
/// its owner alone.
struct CentralConfig(std::path::PathBuf);

impl CentralConfig {
    /// A bridge to `endpoint`, with a bridge credential nobody holds.
    fn to(endpoint: &str) -> Self {
        use std::io::Write as _;
        let path = std::env::temp_dir().join(format!(
            "baylee-central-bridge-{}.json",
            uuid::Uuid::now_v7()
        ));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt as _;
            options.mode(0o600);
        }
        let mut file = options.open(&path).expect("the bridge file");
        let config = Self(path);
        let bridge = [7_u8; 32];
        let settings = serde_json::json!({ "endpoint": endpoint, "bridge": bridge });
        file.write_all(settings.to_string().as_bytes())
            .expect("the bridge file written");
        config
    }
}

impl Drop for CentralConfig {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

/// A gateway connected to Baylee Central takes whom its mode names, as an
/// unconnected one does: `central` no guest, `mixed` both, `guests` no
/// Central account, refused before Central is asked anything. Nothing here
/// reaches Central, so a Central nobody answers at (port 9) is enough.
#[test]
fn a_connected_gateway_takes_whom_its_mode_names() {
    let config = CentralConfig::to("http://127.0.0.1:9");
    let file = config.0.to_string_lossy().into_owned();
    let connected = |name: &str, mode: &str| {
        spawn_gateway_with(
            name,
            &[
                ("BAYLEE_CENTRAL_CONFIG_FILE", file.clone()),
                ("BAYLEE_AUTH_MODE", mode.into()),
            ],
        )
    };
    // Shaped as a ticket, so the refusal is the mode's and not the body's.
    let ticket = serde_json::json!({ "ticket": vec![0_u8; 32] }).to_string();

    let central = connected("guests_central", "central");
    let (_, config_answer) = http(central.port, "GET", "/auth/config", None, "");
    assert!(
        config_answer.contains("\"guests_enabled\":false"),
        "{config_answer}"
    );
    assert!(
        config_answer.contains("\"central\":true"),
        "{config_answer}"
    );
    let (status, body) = http(central.port, "POST", "/auth/guest", None, "{}");
    assert_eq!(status, 403, "{body}");
    assert!(body.contains("this gateway takes no guests"), "{body}");
    assert_eq!(
        central.scalar("SELECT count(*) FROM account"),
        0,
        "nobody was made"
    );

    let mixed = connected("guests_mixed", "mixed");
    let (_, config_answer) = http(mixed.port, "GET", "/auth/config", None, "");
    assert!(
        config_answer.contains("\"guests_enabled\":true"),
        "{config_answer}"
    );
    assert!(
        config_answer.contains("\"central\":true"),
        "{config_answer}"
    );
    let (token, _) = a_guest(mixed.port, "{}");
    assert_eq!(http(mixed.port, "GET", "/me", Some(&token), "").0, 200);

    let guests = connected("guests_only", "guests");
    let (_, config_answer) = http(guests.port, "GET", "/auth/config", None, "");
    assert!(
        config_answer.contains("\"central\":false"),
        "{config_answer}"
    );
    let (status, body) = http(guests.port, "POST", "/auth/central/login", None, &ticket);
    assert_eq!(status, 403, "no Central account comes in: {body}");
    a_guest(guests.port, "{}");
}

/// `BAYLEE_GUEST_CAP`: once that many guests are playing, the next is sent
/// away until one leaves.
#[test]
fn a_gateway_takes_as_many_guests_as_its_cap() {
    let gw = spawn_gateway_with("guests_cap", &[("BAYLEE_GUEST_CAP", "2".into())]);
    let port = gw.port;
    let (first, _) = a_guest(port, "{}");
    a_guest(port, "{}");
    let (status, body) = http(port, "POST", "/auth/guest", None, "{}");
    assert_eq!(status, 503, "{body}");
    assert!(
        body.contains("no guest seats free, sign up or try later"),
        "{body}"
    );
    // Accounts are not guests, and are not counted.
    login(port, "member", "Member");

    let (status, _) = http(port, "POST", "/auth/logout", Some(&first), "");
    assert_eq!(status, 204);
    a_guest(port, "{}");
}

/// A guest cannot put a picture on the gateway (#269): an account anybody
/// gets by asking is nobody the pictures could be answered for by. The same
/// picture from an account is taken.
#[test]
fn a_guest_cannot_upload_a_picture() {
    let dir = std::env::temp_dir().join(format!("baylee-guest-images-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let gw = spawn_gateway_with(
        "guests_images",
        &[("BAYLEE_DECK_IMAGE_PATH", dir.to_string_lossy().into_owned())],
    );
    let picture = {
        let mut out = std::io::Cursor::new(Vec::new());
        image::DynamicImage::ImageRgb8(image::RgbImage::from_pixel(
            64,
            64,
            image::Rgb([40, 90, 160]),
        ))
        .write_to(&mut out, image::ImageFormat::Jpeg)
        .expect("encode a jpeg");
        out.into_inner()
    };
    let upload = |token: &str| {
        let (status, body) = http_bytes(
            gw.port,
            "POST",
            "/images?kind=sleeve",
            Some(token),
            "image/jpeg",
            &picture,
        );
        (status, String::from_utf8_lossy(&body).into_owned())
    };

    let (guest, _) = a_guest(gw.port, "{}");
    let (status, body) = upload(&guest);
    assert_eq!(status, 403, "{body}");
    assert!(body.contains("guests cannot upload images"), "{body}");

    let account = login(gw.port, "member", "Member");
    let (status, body) = upload(&account);
    assert_eq!(status, 200, "{body}");
    let _ = std::fs::remove_dir_all(&dir);
}
