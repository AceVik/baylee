//! End-to-end test for whom the card-art mirror serves (#273).
//!
//! Scryfall welcomes a cache for one's own players and forbids a public mirror
//! of its data, so `/art` answers a signed-in session and nobody else. The
//! picture is put into the mirror's directory before the gateway starts, so
//! nothing here reaches the network: the one request that could have gone on
//! to Scryfall is refused before it may.

#![allow(clippy::missing_docs_in_private_items)]

mod common;

use common::{http, http_headers, login, spawn_gateway_with};

/// A printing whose picture the mirror already holds.
const CACHED: &str = "e3285e6b-3e79-4d7c-bf96-d920f973b0d1";

/// Its route, in Scryfall's own path shape.
const CACHED_PATH: &str = "/art/small/front/e/3/e3285e6b-3e79-4d7c-bf96-d920f973b0d1.jpg";

/// A printing the mirror does not hold: served, it would be fetched.
const UNCACHED_PATH: &str = "/art/small/front/0/a/0aeebaf5-8c7d-4636-9e82-8c27447861f7.jpg";

#[tokio::test(flavor = "multi_thread")]
async fn the_art_mirror_serves_a_signed_in_player_and_nobody_else() {
    let dir = std::env::temp_dir().join(format!("baylee-e2e-art-{}", std::process::id()));
    std::fs::create_dir_all(dir.join("small").join("front")).expect("a mirror directory");
    std::fs::write(dir.join("small").join("front").join(CACHED), "a picture")
        .expect("a mirrored picture");
    let gw = spawn_gateway_with("art", &[("BAYLEE_ART_PATH", dir.display().to_string())]);

    // Without a session: refused, and nothing of the picture comes back.
    let (status, body) = http(gw.port, "GET", CACHED_PATH, None, "");
    assert_eq!(status, 401, "{body}");
    assert!(!body.contains("a picture"));
    // Nor does a picture the mirror would have to fetch: refused the same
    // way, before anything leaves for the origin.
    let (status, _) = http(gw.port, "GET", UNCACHED_PATH, None, "");
    assert_eq!(status, 401);

    // With one: served, and for this player only.
    let token = login(gw.port, "art", "Artist");
    let (status, body) = http(gw.port, "GET", CACHED_PATH, Some(&token), "");
    assert_eq!((status, body.as_str()), (200, "a picture"));
    let bearer = format!("Bearer {token}");
    let (_, head) = http_headers(gw.port, "GET", CACHED_PATH, &[("Authorization", &bearer)]);
    assert!(
        head.contains("cache-control: private"),
        "a proxy on the way could hand this to anyone: {head}"
    );

    // A browser may send the session to it: the preflight allows the header,
    // and still no credentials beside the wildcard.
    let (status, head) = http_headers(
        gw.port,
        "OPTIONS",
        CACHED_PATH,
        &[
            ("Origin", "http://localhost:8080"),
            ("Access-Control-Request-Method", "GET"),
            ("Access-Control-Request-Headers", "authorization"),
        ],
    );
    assert_eq!(status, 204, "{head}");
    assert!(head.contains("access-control-allow-headers: authorization"));
    assert!(!head.contains("access-control-allow-credentials"));

    // A session past its end is no session.
    {
        use sea_orm::ConnectionTrait as _;
        let db = sea_orm::Database::connect(&gw.database_url())
            .await
            .expect("connecting to the gateway's schema");
        db.execute_unprepared("UPDATE session_token SET expires_at = now() - interval '1 hour'")
            .await
            .expect("every session ended an hour ago");
    }
    let (status, body) = http(gw.port, "GET", CACHED_PATH, Some(&token), "");
    assert_eq!(status, 401, "an expired session was served: {body}");

    let _ = std::fs::remove_dir_all(dir);
}
