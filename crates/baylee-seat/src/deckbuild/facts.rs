//! The host's word on legality beyond the rules, kept beside an AI's decks:
//! the catalog's table (`GET /pool/facts`, `docs/protocol.md` §"Pool facts")
//! and the cards its operator blocked (`GET /cards/blocked`, §"Blocked
//! cards").
//!
//! A ranked host admits a deck only when its catalog has a verdict for
//! every card (`baylee_cards::ranked::admit`), so a deck is checked here
//! against the same table the host holds, fetched from it. [`fetch`] signs
//! in as a guest under [`GUEST_NAME`], reads both routes and signs out
//! again (a guest is deleted with its session); it never uses an AI
//! account's own session, because a second sign-in ends the seats that
//! account holds. [`save`] writes the table beside the decks
//! (`<dir>/facts.json`, [`FILE`]) through a temporary file, so a reader
//! never sees half of one.

use baylee_client_core::cardquery::{PoolFacts, PoolFactsBody};
use baylee_client_core::lobby::BlockedCard;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::Duration;

/// The file's name in an AI's deck directory.
pub const FILE: &str = "facts.json";

/// The display name [`fetch`] signs in under: what it is, never a person.
pub const GUEST_NAME: &str = "KI-Decks";

/// The longest one request of [`fetch`] may take.
const TIMEOUT: Duration = Duration::from_secs(30);

/// The most of an answer [`fetch`] reads: the table is under half a
/// megabyte (`docs/banlists-and-formats.md`), and this leaves it room.
const MOST_BYTES: u64 = 16 * 1024 * 1024;

/// What a host checks a deck against beyond the rules of its format.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Facts {
    /// The gateway's origin they came from (`https://host`).
    pub gateway: String,
    /// When they were read there, RFC 3339 in UTC.
    pub fetched: String,
    /// The catalog's table, as `GET /pool/facts` sent it.
    pub pool: PoolFactsBody,
    /// The cards blocked there when they were read.
    pub blocked: Vec<BlockedCard>,
}

impl Facts {
    /// The table's version (16 hex digits), which every deck version
    /// records it was checked against.
    #[must_use]
    pub fn version(&self) -> &str {
        &self.pool.version
    }

    /// The table, decoded for lookups.
    #[must_use]
    pub fn table(&self) -> PoolFacts {
        self.pool.clone().into()
    }

    /// The block on `card` in force at `now` (RFC 3339 UTC, as
    /// [`utc`] writes it), if any: one with an end counts until then.
    #[must_use]
    pub fn blocked_at(&self, card: u32, now: &str) -> Option<&BlockedCard> {
        self.blocked
            .iter()
            .find(|block| block.index == card && block.until.as_deref().is_none_or(|end| end > now))
    }
}

/// Where the facts of the AI whose decks are in `dir` are kept.
#[must_use]
pub fn path_in(dir: &Path) -> PathBuf {
    dir.join(FILE)
}

/// Writes `facts` to `path` through a temporary file beside it, then moves
/// it into place, so a reader sees the old file or the new one whole.
///
/// # Errors
/// When the directory cannot be made or the file written.
pub fn save(path: &Path, facts: &Facts) -> std::io::Result<()> {
    let text = serde_json::to_string(facts).map_err(std::io::Error::other)?;
    write_whole(path, text.as_bytes())
}

/// The facts at `path`.
///
/// # Errors
/// A sentence: no file there (and how to make one), one that is not facts,
/// or an empty table (a gateway without a catalog), against which no deck
/// could pass.
pub fn load(path: &Path) -> Result<Facts, String> {
    let text = std::fs::read_to_string(path).map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            format!(
                "there are no legality facts at {}: fetch them from the host with `baylee-seat \
                 deck facts --gateway <url> --out {}`",
                path.display(),
                path.display()
            )
        } else {
            format!("reading {}: {e}", path.display())
        }
    })?;
    let facts: Facts = serde_json::from_str(&text)
        .map_err(|e| format!("{} holds no legality facts: {e}", path.display()))?;
    if facts.pool.cards.is_empty() {
        return Err(format!(
            "the legality facts at {} are empty ({} had no catalog when they were read), and a \
             ranked host admits no deck without its verdicts",
            path.display(),
            facts.gateway
        ));
    }
    Ok(facts)
}

/// `bytes` to `path`, whole or not at all.
pub(crate) fn write_whole(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    std::fs::create_dir_all(dir)?;
    let name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("file");
    let temporary = dir.join(format!(".{name}.{}.tmp", std::process::id()));
    std::fs::write(&temporary, bytes)?;
    std::fs::rename(&temporary, path).inspect_err(|_| {
        let _ = std::fs::remove_file(&temporary);
    })
}

/// The moment `unix` (seconds) in UTC, as the gateway writes one
/// (`2026-11-27T03:15:00Z`), so two of them compare as text.
#[must_use]
pub fn utc(unix: i64) -> String {
    let at =
        time::OffsetDateTime::from_unix_timestamp(unix).unwrap_or(time::OffsetDateTime::UNIX_EPOCH);
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        at.year(),
        u8::from(at.month()),
        at.day(),
        at.hour(),
        at.minute(),
        at.second()
    )
}

/// Now, as [`utc`] writes it.
#[must_use]
pub fn now_utc() -> String {
    utc(crate::spend::now().unix)
}

/// The facts of the gateway at `gateway` (`https://host`, or `http://` on
/// this machine) as they are now: a guest signs in as [`GUEST_NAME`],
/// reads `GET /pool/facts` and `GET /cards/blocked`, and signs out. With
/// `have`, facts already kept from there, the table is asked with `?have=`
/// and kept when the gateway says it has not changed.
///
/// # Errors
/// A sentence: an address that is neither, a gateway that takes no guests
/// (said as such), one that asks its terms to be accepted first, or one
/// that cannot be reached or answers something else.
pub async fn fetch(gateway: &str, have: Option<&Facts>) -> anyhow::Result<Facts> {
    let origin = gateway.trim().trim_end_matches('/').to_string();
    if let Some(why) = baylee_client_core::llmseat::address_fault(&origin, "the gateway") {
        anyhow::bail!(why);
    }
    let kept = have.filter(|facts| facts.gateway == origin).cloned();
    tokio::task::spawn_blocking(move || fetch_blocking(&origin, kept.as_ref()))
        .await
        .map_err(|e| anyhow::anyhow!("fetching the facts: {e}"))?
}

fn fetch_blocking(origin: &str, kept: Option<&Facts>) -> anyhow::Result<Facts> {
    let builder = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .max_redirects(0)
        .timeout_global(Some(TIMEOUT));
    let builder = if baylee_client_core::llmseat::is_loopback(origin) {
        builder.proxy(None)
    } else {
        builder
    };
    let http = Http {
        agent: builder.build().new_agent(),
        origin,
    };
    let body = serde_json::json!({"display_name": GUEST_NAME, "lang": "en"});
    let (status, signed) = http.send("POST", "/auth/guest", None, Some(&body))?;
    if status == 403 {
        anyhow::bail!(
            "{origin} takes no guests ({}), so its legality facts cannot be read here",
            said(&signed)
        );
    }
    if !(200..300).contains(&status) {
        anyhow::bail!(
            "{origin} refused the guest sign-in: {status} {}",
            said(&signed)
        );
    }
    let token = serde_json::from_slice::<serde_json::Value>(&signed)
        .ok()
        .and_then(|v| v.get("token").and_then(|t| t.as_str()).map(str::to_string))
        .ok_or_else(|| anyhow::anyhow!("{origin} signed the guest in without a token"))?;
    let token = zeroize::Zeroizing::new(token);
    let read = read_both(&http, &token, kept);
    // Signed out whatever was read: the guest goes with its session.
    let _ = http.send("POST", "/auth/logout", Some(&token), None);
    let (pool, blocked) = read?;
    Ok(Facts {
        gateway: origin.to_string(),
        fetched: now_utc(),
        pool,
        blocked,
    })
}

fn read_both(
    http: &Http<'_>,
    token: &str,
    kept: Option<&Facts>,
) -> anyhow::Result<(PoolFactsBody, Vec<BlockedCard>)> {
    #[derive(Deserialize)]
    struct Blocked {
        blocked: Vec<BlockedCard>,
    }
    let origin = http.origin;
    let path = match kept {
        Some(facts) if !facts.pool.version.is_empty() => {
            format!("/pool/facts?have={}", facts.pool.version)
        }
        _ => "/pool/facts".to_string(),
    };
    let (status, body) = http.send("GET", &path, Some(token), None)?;
    let pool = match (status, kept) {
        (304, Some(facts)) => facts.pool.clone(),
        (200, _) => serde_json::from_slice::<PoolFactsBody>(&body)
            .map_err(|e| anyhow::anyhow!("{origin}'s /pool/facts is not the table: {e}"))?,
        (403, _) if said(&body).contains("terms") => anyhow::bail!(
            "{origin} asks a guest to accept its terms before it reads the legality facts: {}",
            said(&body)
        ),
        (404, _) => anyhow::bail!("{origin} has no /pool/facts (a gateway from before it)"),
        _ => anyhow::bail!(
            "{origin} answered /pool/facts with {status} {}",
            said(&body)
        ),
    };
    let (status, body) = http.send("GET", "/cards/blocked", Some(token), None)?;
    if status != 200 {
        anyhow::bail!(
            "{origin} answered /cards/blocked with {status} {}",
            said(&body)
        );
    }
    let blocked = serde_json::from_slice::<Blocked>(&body)
        .map_err(|e| anyhow::anyhow!("{origin}'s /cards/blocked is not the list: {e}"))?
        .blocked;
    Ok((pool, blocked))
}

/// What the gateway said, in a few words: its `error`, else nothing.
fn said(body: &[u8]) -> String {
    serde_json::from_slice::<serde_json::Value>(body)
        .ok()
        .and_then(|v| v.get("error").and_then(|e| e.as_str()).map(str::to_string))
        .map_or_default(|error| error.chars().take(200).collect())
}

struct Http<'a> {
    agent: ureq::Agent,
    origin: &'a str,
}

impl Http<'_> {
    /// One request: its status and its body. A redirect is not followed.
    fn send(
        &self,
        method: &str,
        path: &str,
        bearer: Option<&str>,
        body: Option<&serde_json::Value>,
    ) -> anyhow::Result<(u16, Vec<u8>)> {
        let url = format!("{}{path}", self.origin);
        let shown = format!("{method} {}", path.split('?').next().unwrap_or(path));
        let mut request = ureq::http::Request::builder().method(method).uri(&url);
        if let Some(bearer) = bearer {
            request = request.header("authorization", format!("Bearer {bearer}"));
        }
        let answer = match body {
            Some(body) => self.agent.run(
                request
                    .header("content-type", "application/json")
                    .body(body.to_string().into_bytes())?,
            ),
            None => self.agent.run(request.body(())?),
        };
        let mut answer = answer.map_err(|e| anyhow::anyhow!("{shown} at {}: {e}", self.origin))?;
        let status = answer.status().as_u16();
        if answer.status().is_redirection() && status != 304 {
            anyhow::bail!(
                "{shown}: {} answered {status}, a redirect, which is not followed",
                self.origin
            );
        }
        let bytes = answer
            .body_mut()
            .with_config()
            .limit(MOST_BYTES)
            .read_to_vec()
            .map_err(|e| anyhow::anyhow!("{shown} at {}: {e}", self.origin))?;
        Ok((status, bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts() -> Facts {
        facts_at("https://gateway.test")
    }

    pub(super) fn facts_at(gateway: &str) -> Facts {
        Facts {
            gateway: gateway.into(),
            fetched: "2026-11-27T03:15:00Z".into(),
            pool: PoolFactsBody {
                version: "0123456789abcdef".into(),
                formats: vec!["standard".into()],
                sets: Vec::new(),
                verdicts: vec!["l".into()],
                cards: vec![vec![7, 0]],
            },
            blocked: vec![
                BlockedCard {
                    index: 7,
                    name: "Seven".into(),
                    reason: "a fault".into(),
                    until: Some("2026-11-28T00:00:00Z".into()),
                },
                BlockedCard {
                    index: 8,
                    name: "Eight".into(),
                    reason: "for good".into(),
                    until: None,
                },
            ],
        }
    }

    #[test]
    fn facts_are_saved_whole_and_read_back() {
        let dir = std::env::temp_dir().join(format!("baylee-deck-facts-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = path_in(&dir);
        assert!(load(&path).unwrap_err().contains("deck facts --gateway"));
        save(&path, &facts()).unwrap();
        assert_eq!(load(&path).unwrap(), facts());
        assert_eq!(load(&path).unwrap().version(), "0123456789abcdef");
        let leftovers: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name())
            .collect();
        assert_eq!(leftovers, [FILE], "no temporary file is left behind");
        let mut empty = facts();
        empty.pool = PoolFactsBody::default();
        save(&path, &empty).unwrap();
        assert!(load(&path).unwrap_err().contains("had no catalog"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_block_counts_until_its_end() {
        let facts = facts();
        assert!(facts.blocked_at(7, "2026-11-27T23:59:59Z").is_some());
        assert!(facts.blocked_at(7, "2026-11-28T00:00:00Z").is_none());
        assert!(facts.blocked_at(8, "2099-01-01T00:00:00Z").is_some());
        assert!(facts.blocked_at(9, "2026-11-27T00:00:00Z").is_none());
        assert_eq!(utc(1_764_213_300), "2025-11-27T03:15:00Z");
    }

    #[tokio::test]
    async fn an_address_in_the_clear_is_refused_before_anything_is_sent() {
        let why = fetch("http://gateway.example", None).await.unwrap_err();
        assert!(format!("{why}").contains("https://"), "{why}");
    }
}

#[cfg(test)]
mod gateway_tests {
    //! [`fetch`] against a stand-in gateway on this machine.

    use super::*;
    use axum::Json;
    use axum::Router;
    use axum::extract::RawQuery;
    use axum::http::{HeaderMap, StatusCode};
    use axum::response::IntoResponse;
    use axum::routing::{get, post};
    use std::sync::{Arc, Mutex, PoisonError};

    /// What the stand-in heard, in order: `POST /auth/guest`, `GET
    /// /pool/facts?have=…`, and so on, each with whether it carried the
    /// guest's token.
    type Heard = Arc<Mutex<Vec<String>>>;

    async fn gateway(guests: bool) -> (String, Heard) {
        let heard: Heard = Arc::default();
        let note = |heard: &Heard, what: String| {
            heard
                .lock()
                .unwrap_or_else(PoisonError::into_inner)
                .push(what);
        };
        let token = |headers: &HeaderMap| {
            headers.get("authorization").and_then(|v| v.to_str().ok()) == Some("Bearer guest-token")
        };
        let (h1, h2, h3, h4) = (heard.clone(), heard.clone(), heard.clone(), heard.clone());
        let app = Router::new()
            .route(
                "/auth/guest",
                post(move |Json(body): Json<serde_json::Value>| async move {
                    note(&h1, format!("guest {}", body["display_name"]));
                    if guests {
                        Json(serde_json::json!({"token": "guest-token"})).into_response()
                    } else {
                        (
                            StatusCode::FORBIDDEN,
                            Json(serde_json::json!({"error": "this gateway takes no guests"})),
                        )
                            .into_response()
                    }
                }),
            )
            .route(
                "/pool/facts",
                get(
                    move |headers: HeaderMap, RawQuery(query): RawQuery| async move {
                        note(&h2, format!("facts {query:?} {}", token(&headers)));
                        if query.as_deref() == Some("have=0123456789abcdef") {
                            return StatusCode::NOT_MODIFIED.into_response();
                        }
                        Json(serde_json::json!({
                            "version": "fedcba9876543210", "formats": ["standard"], "sets": [],
                            "verdicts": ["l"], "cards": [[7, 0]]
                        }))
                        .into_response()
                    },
                ),
            )
            .route(
                "/cards/blocked",
                get(move |headers: HeaderMap| async move {
                    note(&h3, format!("blocked {}", token(&headers)));
                    Json(serde_json::json!({"blocked": [
                        {"index": 7, "name": "Seven", "reason": "a fault", "until": null}
                    ]}))
                }),
            )
            .route(
                "/auth/logout",
                post(move |headers: HeaderMap| async move {
                    note(&h4, format!("logout {}", token(&headers)));
                    StatusCode::NO_CONTENT
                }),
            );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let port = listener.local_addr().unwrap().port();
        tokio::spawn(async move { axum::serve(listener, app).await });
        (format!("http://127.0.0.1:{port}"), heard)
    }

    fn heard(heard: &Heard) -> Vec<String> {
        heard.lock().unwrap_or_else(PoisonError::into_inner).clone()
    }

    #[tokio::test]
    async fn a_guest_reads_both_routes_and_signs_out() {
        let (base, log) = gateway(true).await;
        let facts = fetch(&format!("{base}/"), None).await.unwrap();
        assert_eq!(facts.gateway, base, "the origin, without a trailing slash");
        assert_eq!(facts.version(), "fedcba9876543210");
        assert_eq!(facts.blocked.len(), 1);
        assert_eq!(
            heard(&log),
            [
                "guest \"KI-Decks\"",
                "facts None true",
                "blocked true",
                "logout true"
            ]
        );
    }

    #[tokio::test]
    async fn a_table_that_did_not_change_is_kept() {
        let (base, log) = gateway(true).await;
        let mut kept = super::tests::facts_at(&base);
        kept.blocked.clear();
        let facts = fetch(&base, Some(&kept)).await.unwrap();
        assert_eq!(facts.pool, kept.pool, "304: the table kept");
        assert_eq!(facts.blocked.len(), 1, "the blocks read afresh");
        assert!(heard(&log).contains(&"facts Some(\"have=0123456789abcdef\") true".to_string()));
    }

    #[tokio::test]
    async fn a_gateway_without_guests_says_so() {
        let (base, log) = gateway(false).await;
        let why = format!("{:#}", fetch(&base, None).await.unwrap_err());
        assert!(why.contains("takes no guests"), "{why}");
        assert_eq!(heard(&log), ["guest \"KI-Decks\""], "nothing more is asked");
    }
}
