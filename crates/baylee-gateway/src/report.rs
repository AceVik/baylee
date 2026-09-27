//! `POST /reports` (#307): a signed-in player's bug report, crash report or
//! feedback, passed on to the feedback service (`docs/feedback.md`).
//!
//! The gateway adds what only it knows: which gateway and build this is, a
//! pseudonym for the reporter that is the same for every report of one
//! account on this gateway and says nothing else about it, and, when the
//! report names a game the reporter sat at, that game's record (#315). It
//! never passes on a name, an address or an IP.
//!
//! Configured by `BAYLEE_FEEDBACK_URL` (the service; unset = `503`),
//! `BAYLEE_FEEDBACK_TOKEN` (this gateway's intake token there) and
//! `BAYLEE_FEEDBACK_KEY` (the key the pseudonym is made with; unset = one
//! derived from the token, so rotating the token renames every reporter).

use std::time::Duration;

use axum::Json;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::{HeaderMap, StatusCode};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::{ErrorBody, Shared, err};

/// The longest report text, in characters.
pub const MAX_TEXT_CHARS: usize = 20_000;

/// The longest `game_id`, in characters.
pub const MAX_GAME_ID_CHARS: usize = 128;

/// The largest `client` object, serialized.
pub const MAX_CLIENT_BYTES: usize = 2 * 1024 * 1024;

/// The largest request body the route reads: a full `client` object and a
/// full text, with room for the JSON around them.
pub const MAX_BODY_BYTES: usize = MAX_CLIENT_BYTES + 256 * 1024;

/// How many reports one account may send per [`REPORT_WINDOW`].
pub const REPORTS_PER_WINDOW: usize = 64;

/// The window [`REPORTS_PER_WINDOW`] counts in.
pub const REPORT_WINDOW: Duration = Duration::from_secs(3600);

/// How long the service has to take a report.
const FORWARD_TIMEOUT: Duration = Duration::from_secs(20);

/// How long a report about a game that goes on waits for the game's engine
/// to send its record as it stands and for that to be stored (#323), before
/// it attaches what is there. An engine answers in milliseconds; this bounds
/// one that is busy, stuck or from before the ask (which never answers), and
/// it is what a player filing such a report can wait at most for it.
pub const FLUSH_WAIT: Duration = Duration::from_secs(3);

/// Where reports go, from the environment.
pub struct Feedback {
    /// `BAYLEE_FEEDBACK_URL` without a trailing slash; `None` = not
    /// configured.
    url: Option<String>,
    /// `BAYLEE_FEEDBACK_TOKEN`.
    token: String,
    /// What the pseudonyms are made with.
    key: [u8; 32],
    /// `BAYLEE_PUBLIC_URL`, what the gateway calls its own address.
    public_url: Option<String>,
    /// Reports per account ([`REPORTS_PER_WINDOW`]). A limiter of its own,
    /// so a client sending crash reports never costs its player a sign-in.
    limiter: crate::auth::RateLimiter,
}

impl Feedback {
    /// Reads the three variables.
    pub fn from_env() -> Self {
        let var = |name: &str| std::env::var(name).ok().filter(|v| !v.is_empty());
        Self::new(
            var("BAYLEE_FEEDBACK_URL"),
            var("BAYLEE_FEEDBACK_TOKEN").unwrap_or_default(),
            var("BAYLEE_FEEDBACK_KEY").as_deref(),
            var("BAYLEE_PUBLIC_URL"),
        )
    }

    /// The service at `url`, taking reports with `token`; pseudonyms made
    /// under `key`, or under one derived from `token` when there is none.
    fn new(
        url: Option<String>,
        token: String,
        key: Option<&str>,
        public_url: Option<String>,
    ) -> Self {
        let key = key.map_or_else(
            || {
                let mut h = Sha256::new();
                h.update(b"baylee reporter pseudonym\0");
                h.update(token.as_bytes());
                h.finalize().into()
            },
            |key| Sha256::digest(key.as_bytes()).into(),
        );
        Self {
            url: url.map(|u| u.trim_end_matches('/').to_owned()),
            token,
            key,
            public_url,
            limiter: crate::auth::RateLimiter::new(REPORT_WINDOW, REPORTS_PER_WINDOW),
        }
    }

    /// The reporter's pseudonym: HMAC-SHA256 of the account id under this
    /// gateway's key, in hex.
    fn pseudonym(&self, account_id: &str) -> String {
        hex(&hmac_sha256(&self.key, account_id.as_bytes()))
    }
}

/// HMAC-SHA256 (RFC 2104), from the hash the gateway already links.
fn hmac_sha256(key: &[u8; 32], message: &[u8]) -> [u8; 32] {
    const BLOCK: usize = 64;
    let mut inner = [0x36_u8; BLOCK];
    let mut outer = [0x5c_u8; BLOCK];
    for (i, k) in key.iter().enumerate() {
        inner[i] ^= k;
        outer[i] ^= k;
    }
    let inside = Sha256::new()
        .chain_update(inner)
        .chain_update(message)
        .finalize();
    Sha256::new()
        .chain_update(outer)
        .chain_update(inside)
        .finalize()
        .into()
}

fn hex(bytes: &[u8]) -> String {
    use std::fmt::Write as _;
    bytes
        .iter()
        .fold(String::with_capacity(bytes.len() * 2), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        })
}

/// What a report is about.
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// Something is broken.
    Bug,
    /// Something could be better.
    Improvement,
    /// Anything the player wants to say.
    Feedback,
    /// The client crashed; sent by the client.
    Crash,
    /// None of these.
    Other,
}

/// The request body (frozen: `docs/feedback.md`).
#[derive(Deserialize)]
struct Report {
    kind: Kind,
    text: String,
    #[serde(default)]
    game_id: Option<String>,
    client: serde_json::Value,
}

/// A request body as a report, or why not (`docs/feedback.md`, the table
/// of answers). An empty `game_id` is no game id.
fn validate(body: &[u8]) -> Result<Report, (StatusCode, &'static str)> {
    if body.len() > MAX_BODY_BYTES {
        return Err((StatusCode::PAYLOAD_TOO_LARGE, "the report is too large"));
    }
    let mut report: Report =
        serde_json::from_slice(body).map_err(|_| (StatusCode::BAD_REQUEST, "not a report"))?;
    if !report.client.is_object() {
        return Err((StatusCode::BAD_REQUEST, "client must be an object"));
    }
    if report.text.chars().count() > MAX_TEXT_CHARS {
        return Err((StatusCode::PAYLOAD_TOO_LARGE, "the text is too long"));
    }
    let client_bytes = serde_json::to_vec(&report.client).map_or(usize::MAX, |v| v.len());
    if client_bytes > MAX_CLIENT_BYTES {
        return Err((
            StatusCode::PAYLOAD_TOO_LARGE,
            "the client details are too large",
        ));
    }
    report.game_id = report.game_id.filter(|g| !g.is_empty());
    if report
        .game_id
        .as_deref()
        .is_some_and(|g| g.chars().count() > MAX_GAME_ID_CHARS)
    {
        return Err((StatusCode::BAD_REQUEST, "not a game id"));
    }
    Ok(report)
}

/// What the service is sent.
#[derive(Serialize)]
struct Forward<'a> {
    gateway: GatewayInfo<'a>,
    reporter: String,
    kind: Kind,
    text: &'a str,
    game_id: Option<&'a str>,
    client: &'a serde_json::Value,
    record: Option<RecordOut>,
}

#[derive(Serialize)]
struct GatewayInfo<'a> {
    name: Option<&'a str>,
    url: Option<&'a str>,
    version: &'static str,
}

/// A game's record as the service is sent it.
#[derive(Serialize)]
struct RecordOut {
    /// Whether the last piece arrived before the engine went.
    complete: bool,
    /// The record, gzip, in standard base64.
    gzip_base64: String,
}

#[derive(Serialize)]
pub struct Created {
    report_id: String,
}

#[derive(Deserialize)]
struct ServiceCreated {
    report_id: String,
}

/// The route, with a body limit of its own: a whole `client` object is
/// larger than axum's default.
pub fn routes() -> axum::Router<Shared> {
    axum::Router::new().route(
        "/reports",
        axum::routing::post(post_report)
            .layer(axum::extract::DefaultBodyLimit::max(MAX_BODY_BYTES)),
    )
}

/// `POST /reports`.
pub async fn post_report(
    State(state): State<Shared>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<(StatusCode, Json<Created>), (StatusCode, Json<ErrorBody>)> {
    let session = crate::authed_session(&state, &headers).await?;
    let feedback = &state.feedback;
    let Some(url) = feedback.url.clone() else {
        return Err(err(
            StatusCode::SERVICE_UNAVAILABLE,
            "reports are not configured",
        ));
    };
    let report = validate(&body).map_err(|(status, why)| err(status, why))?;
    let game_id = report.game_id.as_deref();
    // Checked after everything else, so a report refused for what it is
    // costs nothing of the budget, and before anything is sent; and never
    // given back unless this took it (`give_back` returns the latest).
    if !feedback.limiter.allow(&session.account_id) {
        return Err(err(StatusCode::TOO_MANY_REQUESTS, "too many reports"));
    }
    // A report the service did not take was not sent, and costs the
    // player nothing of the budget.
    let give_back = || feedback.limiter.give_back(&session.account_id);
    if let Some(game) = game_id {
        flush_record(&state, game, &session.account_id).await;
    }
    let record = match (game_id, uuid::Uuid::parse_str(&session.account_id)) {
        (Some(game), Ok(account)) => baylee_db::records::for_seated(&state.db, game, account)
            .await
            .map_err(|e| {
                give_back();
                crate::db_down(&anyhow::Error::from(e))
            })?
            .map(|r| RecordOut {
                complete: r.complete,
                gzip_base64: base64::engine::general_purpose::STANDARD.encode(r.data),
            }),
        _ => None,
    };
    let forward = Forward {
        gateway: GatewayInfo {
            name: state.display_name.as_deref(),
            url: feedback.public_url.as_deref(),
            version: baylee_build::short(),
        },
        reporter: feedback.pseudonym(&session.account_id),
        kind: report.kind,
        text: &report.text,
        game_id,
        client: &report.client,
        record,
    };
    let payload = serde_json::to_vec(&forward)
        .map_err(|_| err(StatusCode::INTERNAL_SERVER_ERROR, "encoding the report"))?;
    let token = feedback.token.clone();
    let sent = tokio::task::spawn_blocking(move || send(&url, &token, &payload))
        .await
        .unwrap_or_else(|_| Err("the sender stopped".to_owned()));
    match sent {
        Ok(report_id) => Ok((StatusCode::CREATED, Json(Created { report_id }))),
        Err(why) => {
            tracing::warn!("the feedback service did not take a report: {why}");
            give_back();
            Err(err(
                StatusCode::BAD_GATEWAY,
                "the feedback service did not take the report",
            ))
        }
    }
}

/// Asks the engine of `game_id` for its record as it stands and waits, up to
/// [`FLUSH_WAIT`], until what it sent is stored (#323), when `account_id`
/// sits at that game and it goes on; otherwise asks nothing. Whether it was
/// stored in time.
///
/// Never an error: a report is not refused because its game's engine is
/// slow or gone. The record attached is then what was stored before, and
/// `complete` says what it said before: whether the game's end is in it.
async fn flush_record(state: &Shared, game_id: &str, account_id: &str) -> bool {
    // The link is cloned out, so no lock is held across the wait. A game
    // has one exactly while it goes on: it is set when the engine attaches
    // and cleared when the game ends (`LobbyGame::finish`), after the last
    // piece.
    let engine = {
        let lobby = state.lobby.lock();
        lobby
            .games
            .get(game_id)
            .filter(|game| {
                game.seats
                    .iter()
                    .any(|seat| seat.account_id.as_deref() == Some(account_id))
            })
            .and_then(|game| game.engine.clone())
    };
    let Some(engine) = engine else {
        return false;
    };
    let flushes = &state.record_flushes;
    let (nonce, stored) = flushes.ask(game_id);
    let ask = baylee_protocol::v1::Envelope {
        msg: Some(baylee_protocol::v1::envelope::Msg::FlushRecord(
            baylee_protocol::v1::FlushRecord {
                game_id: game_id.to_owned(),
                nonce,
            },
        )),
    };
    if engine.send(ask).is_err() {
        flushes.forget(nonce);
        return false;
    }
    if let Ok(Ok(())) = tokio::time::timeout(FLUSH_WAIT, stored).await {
        return true;
    }
    flushes.forget(nonce);
    tracing::warn!(
        game_id,
        "the engine did not send its record in time; the report carries what was stored"
    );
    false
}

/// Posts one report to the service; its id, or why not.
fn send(url: &str, token: &str, payload: &[u8]) -> Result<String, String> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(FORWARD_TIMEOUT))
        .build()
        .into();
    let mut response = agent
        .post(format!("{url}/intake/reports"))
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .send(payload)
        .map_err(|e| e.to_string())?;
    let created: ServiceCreated = response
        .body_mut()
        .with_config()
        .limit(64 * 1024)
        .read_json()
        .map_err(|e| e.to_string())?;
    Ok(created.report_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// RFC 4231, test case 2, with its key padded the way a 32-byte key is.
    /// The key there is 4 bytes; HMAC pads short keys with zeros, which is
    /// what a 32-byte array holding them does too.
    #[test]
    fn hmac_matches_rfc_4231() {
        let mut key = [0_u8; 32];
        key[..4].copy_from_slice(b"Jefe");
        assert_eq!(
            hex(&hmac_sha256(&key, b"what do ya want for nothing?")),
            "5bdcc146bf60754e6a042426089575c75a003f089d2739839dec58b964ec3843"
        );
    }

    #[allow(clippy::needless_pass_by_value)] // the callers build the values in place
    fn body(
        kind: &str,
        text: &str,
        game_id: serde_json::Value,
        client: serde_json::Value,
    ) -> Vec<u8> {
        serde_json::json!({ "kind": kind, "text": text, "game_id": game_id, "client": client })
            .to_string()
            .into_bytes()
    }

    fn status(body: &[u8]) -> Option<StatusCode> {
        validate(body).err().map(|(status, _)| status)
    }

    /// Every field's bound from both sides: the longest the gateway takes
    /// and one more (`docs/feedback.md`).
    #[test]
    fn every_bound_holds_at_its_edge() {
        let none = serde_json::Value::Null;
        let empty = serde_json::json!({});
        // Characters, not bytes: `é` is two.
        let text = |n: usize| "é".repeat(n);
        assert_eq!(
            status(&body(
                "bug",
                &text(MAX_TEXT_CHARS),
                none.clone(),
                empty.clone()
            )),
            None
        );
        assert_eq!(
            status(&body(
                "bug",
                &text(MAX_TEXT_CHARS + 1),
                none.clone(),
                empty.clone()
            )),
            Some(StatusCode::PAYLOAD_TOO_LARGE)
        );
        let game = |n: usize| serde_json::json!("g".repeat(n));
        assert_eq!(
            status(&body("bug", "", game(MAX_GAME_ID_CHARS), empty.clone())),
            None
        );
        assert_eq!(
            status(&body("bug", "", game(MAX_GAME_ID_CHARS + 1), empty.clone())),
            Some(StatusCode::BAD_REQUEST)
        );
        // `{"l":"…"}` is 8 bytes around the value.
        let client = |n: usize| serde_json::json!({ "l": "x".repeat(n - 8) });
        assert_eq!(
            status(&body("crash", "", none.clone(), client(MAX_CLIENT_BYTES))),
            None
        );
        assert_eq!(
            status(&body(
                "crash",
                "",
                none.clone(),
                client(MAX_CLIENT_BYTES + 1)
            )),
            Some(StatusCode::PAYLOAD_TOO_LARGE)
        );
        let mut huge = body("bug", "", none, empty);
        huge.resize(MAX_BODY_BYTES + 1, b' ');
        assert_eq!(status(&huge), Some(StatusCode::PAYLOAD_TOO_LARGE));
    }

    #[test]
    fn a_report_is_read_as_the_contract_says() {
        let empty = serde_json::json!({});
        for kind in ["bug", "improvement", "feedback", "crash", "other"] {
            let report = validate(&body(kind, "t", serde_json::Value::Null, empty.clone()))
                .unwrap_or_else(|e| panic!("{kind}: {e:?}"));
            assert_eq!(serde_json::to_value(report.kind).unwrap(), kind);
        }
        for bad in [
            body("rant", "t", serde_json::Value::Null, empty.clone()),
            body("Bug", "t", serde_json::Value::Null, empty.clone()),
            body("bug", "t", serde_json::Value::Null, serde_json::json!([])),
            body("bug", "t", serde_json::Value::Null, serde_json::json!("x")),
            body("bug", "t", serde_json::json!(7), empty.clone()),
            br#"{"kind":"bug","game_id":null,"client":{}}"#.to_vec(),
            br#"{"kind":"bug","text":"t","game_id":null}"#.to_vec(),
            b"not json".to_vec(),
        ] {
            assert_eq!(
                status(&bad),
                Some(StatusCode::BAD_REQUEST),
                "{}",
                String::from_utf8_lossy(&bad)
            );
        }
        // No game, said three ways.
        for game in [
            br#"{"kind":"bug","text":"","client":{}}"#.to_vec(),
            body("bug", "", serde_json::Value::Null, empty.clone()),
            body("bug", "", serde_json::json!(""), empty.clone()),
        ] {
            assert_eq!(validate(&game).map(|r| r.game_id).ok(), Some(None));
        }
        assert_eq!(
            validate(&body("bug", "", serde_json::json!("g1"), empty))
                .map(|r| r.game_id)
                .ok(),
            Some(Some("g1".to_owned()))
        );
    }

    /// Without a key the pseudonym is made under one derived from the
    /// token, so rotating the token renames every reporter; with one, the
    /// token can change and nobody is renamed.
    #[test]
    fn a_pseudonym_follows_the_key_and_not_the_token() {
        let account = "0190a1b2-0000-7000-8000-000000000001";
        let at = |token: &str, key: Option<&str>| {
            Feedback::new(None, token.to_owned(), key, None).pseudonym(account)
        };
        assert_eq!(at("token-one", None), at("token-one", None));
        assert_ne!(at("token-one", None), at("token-two", None));
        assert_eq!(at("token-one", Some("key")), at("token-two", Some("key")));
        assert_ne!(
            at("token-one", Some("key")),
            at("token-one", Some("other key"))
        );
        assert_ne!(at("token-one", Some("key")), at("token-one", None));
        // HMAC under SHA-256 of the key, which a reader can check by hand.
        let key: [u8; 32] = Sha256::digest(b"key").into();
        assert_eq!(
            at("anything", Some("key")),
            hex(&hmac_sha256(&key, account.as_bytes()))
        );
    }

    /// The budget: [`REPORTS_PER_WINDOW`] in a window, per account, a slot
    /// given back is spendable again, and the window slides.
    #[test]
    fn the_budget_is_per_account_and_slides() {
        let feedback = Feedback::new(Some("http://x".into()), "t".into(), None, None);
        let limiter = &feedback.limiter;
        let start = std::time::Instant::now();
        for i in 0..REPORTS_PER_WINDOW {
            assert!(limiter.allow_at("a", start), "report {i}");
        }
        assert!(!limiter.allow_at("a", start), "one past the budget");
        assert!(limiter.allow_at("b", start), "another account");
        limiter.give_back("a");
        assert!(limiter.allow_at("a", start), "a slot given back");
        assert!(!limiter.allow_at("a", start));
        let almost = (start + REPORT_WINDOW)
            .checked_sub(Duration::from_millis(1))
            .unwrap();
        assert!(!limiter.allow_at("a", almost), "the window has not passed");
        assert!(
            limiter.allow_at("a", start + REPORT_WINDOW),
            "and now it has"
        );

        // The same edge where the periodic sweep cannot have cleared it: the
        // budget spent half a window in, a sweep at a full window, and the
        // edge half a window after that, which the key's own count decides.
        let spent = start + REPORT_WINDOW / 2;
        for _ in 0..REPORTS_PER_WINDOW {
            assert!(limiter.allow_at("c", spent));
        }
        assert!(limiter.allow_at("d", start + REPORT_WINDOW), "a sweep");
        let edge = spent + REPORT_WINDOW;
        let before = edge.checked_sub(Duration::from_millis(1)).unwrap();
        assert!(!limiter.allow_at("c", before));
        assert!(limiter.allow_at("c", edge), "a window after it was spent");
    }

    #[test]
    fn a_pseudonym_is_stable_and_names_nobody() {
        let feedback = Feedback {
            url: None,
            token: "t".into(),
            key: [7; 32],
            public_url: None,
            limiter: crate::auth::RateLimiter::new(REPORT_WINDOW, 1),
        };
        let a = feedback.pseudonym("0190a1b2-0000-7000-8000-000000000001");
        assert_eq!(
            a,
            feedback.pseudonym("0190a1b2-0000-7000-8000-000000000001")
        );
        assert_ne!(
            a,
            feedback.pseudonym("0190a1b2-0000-7000-8000-000000000002")
        );
        assert!(!a.contains("0190a1b2"));
        assert_eq!(a.len(), 64);
    }
}
