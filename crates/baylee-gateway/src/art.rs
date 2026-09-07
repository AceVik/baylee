//! The card-art cache.
//!
//! Card images are the one large asset in the game, and every seat at a table
//! needs the same ones. Left to itself each client fetches every printing from
//! Scryfall independently — four players at a table is four times the traffic
//! for one game's worth of pictures, and on a slow link it is four players
//! watching cards fill in.
//!
//! So the gateway mirrors them. It is a *cache keyed by printing id*, never a
//! proxy: the only thing a caller may name is an id, and the URL is rebuilt
//! here from a fixed shape. There is nothing to point at an arbitrary host
//! with, which is what keeps an open route from being an open relay.
//!
//! # Why this is the layer that may warm both decks
//!
//! The gateway builds the `GamePreset`, so it knows every printing at the
//! table — and it is the only party that legitimately does. A *seat* is
//! entitled to its own deck's printings and earns the rest by seeing the
//! cards, which is why `GameStatic.prints` is a list of holes rather than a
//! shorter list. Warming both decks in a client would therefore hand a player
//! their opponent's decklist; warming them here hands nobody anything, and the
//! client still learns a printing only when the rules say it may — it just
//! finds the picture already local when it does.
//!
//! `docs/legal.md` §3 is the other half of the design: caching card images is
//! encouraged, the rate limit is ten requests a second, and no image is ever
//! committed to the repo. The limiter below is the whole gateway's, shared by
//! the warming task and by live requests, so the cap holds however many games
//! are running.

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use axum::extract::{Path, State};
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use parking_lot::Mutex;

use crate::Shared;

/// Where the originals come from.
const ORIGIN: &str = "https://cards.scryfall.io";

/// The smallest gap between two requests leaving for Scryfall.
///
/// `docs/legal.md` §3: no more than ten requests a second. One gateway holds
/// one of these, so the cap is the gateway's and not each game's.
const MIN_GAP: Duration = Duration::from_millis(100);

/// How long a client may consider a fetched image fresh.
///
/// A printing's art never changes — the id *is* the version — so this is the
/// browser-side half of the cache, and the only one a wasm client has.
const MAX_AGE: &str = "public, max-age=31536000, immutable";

/// The sizes this cache mirrors, spelled exactly as Scryfall's paths do.
///
/// A fixed list rather than a passthrough: an unknown segment is a 404 here
/// instead of a request to the origin, so a caller cannot use this route to
/// probe or to fetch something the client would never ask for. The client's
/// own list is `baylee_client_core::images::ArtSize`, which the gateway cannot
/// see; `docs/protocol.md` §"Card art" is where the two are held to agree.
const SIZES: [&str; 3] = ["small", "normal", "art_crop"];

/// The faces, likewise.
const FACES: [&str; 2] = ["front", "back"];

/// A disk mirror of the printing images this gateway's games use.
pub struct ArtCache {
    /// Where images are kept, or `None` when the cache is switched off.
    ///
    /// Off is a real mode, not a degraded one: a gateway behind a CDN has no
    /// use for a second copy, and the test suite must never reach the network.
    dir: Option<PathBuf>,
    /// The earliest moment the next request may leave for the origin.
    ///
    /// A queue rather than a token bucket, because the guarantee wanted here
    /// is the simple one — two requests are never less than [`MIN_GAP`] apart.
    /// The lock is held for the arithmetic only and never across the sleep, so
    /// a live request slots in between two of the warming task's.
    next_slot: tokio::sync::Mutex<Instant>,
    /// Printings the origin has no art for.
    ///
    /// Without this a card whose image genuinely does not exist costs a round
    /// trip every time anyone looks at it. Kept in memory rather than on disk:
    /// one refetch per printing per gateway restart is cheap, and a negative
    /// answer that outlived a Scryfall backfill would be worse than the fetch.
    missing: Mutex<HashSet<String>>,
}

impl ArtCache {
    /// Builds the cache from the environment.
    ///
    /// `BAYLEE_ART_PATH` names the directory; `off` (or an empty value)
    /// switches the cache off entirely, and then `/art` answers 404 and the
    /// client falls back to fetching from Scryfall itself.
    #[must_use]
    pub fn from_env() -> Self {
        let dir = match std::env::var("BAYLEE_ART_PATH") {
            Ok(v) if v.is_empty() || v == "off" => None,
            Ok(v) => Some(PathBuf::from(v)),
            Err(_) => Some(PathBuf::from("art-cache")),
        };
        if let Some(dir) = &dir {
            tracing::info!(path = %dir.display(), "card art is cached on disk");
        }
        Self::new(dir)
    }

    /// Builds a cache over a given directory, or a disabled one for `None`.
    #[must_use]
    pub fn new(dir: Option<PathBuf>) -> Self {
        Self {
            dir,
            next_slot: tokio::sync::Mutex::new(Instant::now()),
            missing: Mutex::new(HashSet::new()),
        }
    }

    /// Whether this gateway mirrors art at all.
    #[must_use]
    pub const fn enabled(&self) -> bool {
        self.dir.is_some()
    }

    /// Where one printing's image lives on disk.
    ///
    /// Every component is validated before it reaches here, so the path is
    /// built only out of a fixed size, a fixed face, and an id that has passed
    /// [`well_formed`] — there is no caller-supplied separator anywhere in it.
    fn path(&self, size: &str, face: &str, id: &str) -> Option<PathBuf> {
        Some(self.dir.as_ref()?.join(size).join(face).join(id))
    }

    /// Waits for this request's turn to leave for the origin.
    async fn slot(&self) {
        let wait = {
            let mut next = self.next_slot.lock().await;
            let now = Instant::now();
            let at = (*next).max(now);
            *next = at + MIN_GAP;
            at.saturating_duration_since(now)
        };
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
    }

    /// Returns one printing's image, fetching and storing it if this is the
    /// first time anyone has asked.
    ///
    /// # Errors
    /// [`StatusCode::NOT_FOUND`] when the cache is off or the origin has no
    /// such image, [`StatusCode::BAD_GATEWAY`] when the origin could not be
    /// reached.
    async fn fetch(&self, size: &str, face: &str, id: &str) -> Result<Vec<u8>, StatusCode> {
        let path = self.path(size, face, id).ok_or(StatusCode::NOT_FOUND)?;
        if let Ok(bytes) = tokio::fs::read(&path).await {
            return Ok(bytes);
        }
        if self.missing.lock().contains(id) {
            return Err(StatusCode::NOT_FOUND);
        }
        self.slot().await;
        let url = origin_url(size, face, id);
        let fetched = tokio::task::spawn_blocking(move || download(&url))
            .await
            .map_err(|_| StatusCode::BAD_GATEWAY)?;
        match fetched {
            Ok(bytes) => {
                if let Some(parent) = path.parent() {
                    let _ = tokio::fs::create_dir_all(parent).await;
                }
                // A failed write is not a failed request: the caller still gets
                // its image, and the next ask simply fetches again.
                if let Err(e) = tokio::fs::write(&path, &bytes).await {
                    tracing::warn!(error = %e, path = %path.display(), "could not store card art");
                }
                Ok(bytes)
            }
            Err(Origin::Missing) => {
                self.missing.lock().insert(id.to_string());
                Err(StatusCode::NOT_FOUND)
            }
            Err(Origin::Unreachable) => Err(StatusCode::BAD_GATEWAY),
        }
    }

    /// Fills the cache with every printing at one table, in the background.
    ///
    /// This is the point of the whole module: the gateway knows both decks, so
    /// by the time a seat has earned the right to see one of its opponent's
    /// cards, the picture is already here. Nothing about this reaches a client
    /// — a warmed image is served only to a seat that asks for it, and a seat
    /// only knows to ask once the rules have shown it the card.
    ///
    /// Deliberately unhurried. It shares the gateway's one rate limiter with
    /// live requests, and at `small` a hundred-card deck is about three
    /// megabytes, so there is no reason to rush a table that has not started.
    pub fn warm(self: &Arc<Self>, prints: Vec<String>) {
        if !self.enabled() || prints.is_empty() {
            return;
        }
        let cache = Arc::clone(self);
        tokio::spawn(async move {
            let mut fetched = 0usize;
            for id in &prints {
                if cache.fetch("small", "front", id).await.is_ok() {
                    fetched += 1;
                }
            }
            tracing::debug!(asked = prints.len(), fetched, "warmed a table's card art");
        });
    }
}

/// The origin URL for one printing, in Scryfall's own path shape.
///
/// Its own function so a test can read it. It is the same string
/// `baylee_client_core::images::image_url` builds — the client asks the gateway
/// for it and the gateway asks the origin — and the two live in crates that
/// cannot see each other, so `docs/protocol.md` §"Card art" is the contract and
/// each side tests its half against what is written there. Written inline the
/// first time, it lost the `.jpg` and would have 404ed every image.
fn origin_url(size: &str, face: &str, id: &str) -> String {
    format!("{ORIGIN}/{size}/{face}/{}/{}/{id}.jpg", &id[..1], &id[1..2])
}

/// Why the origin did not hand over an image.
enum Origin {
    /// It answered, and it has no such image.
    Missing,
    /// It could not be reached, or answered with something else.
    Unreachable,
}

/// One blocking fetch. Runs on a blocking worker, never on the async runtime.
fn download(url: &str) -> Result<Vec<u8>, Origin> {
    const USER_AGENT: &str = concat!("baylee/", env!("CARGO_PKG_VERSION"));
    // 20 MB is far above any Scryfall image and far below anything that could
    // exhaust the gateway; it exists so a wrong URL cannot stream forever.
    const LIMIT: u64 = 20 * 1024 * 1024;
    match ureq::get(url).header("User-Agent", USER_AGENT).call() {
        Ok(response) => response
            .into_body()
            .with_config()
            .limit(LIMIT)
            .read_to_vec()
            .map_err(|_| Origin::Unreachable),
        Err(ureq::Error::StatusCode(404)) => Err(Origin::Missing),
        Err(e) => {
            tracing::debug!(error = %e, url, "card art fetch failed");
            Err(Origin::Unreachable)
        }
    }
}

/// Whether an id is a printing id this cache will look up.
///
/// The same shape `baylee_client_core::images::image_url` requires before it
/// will build a URL at all, and for the same two reasons: a malformed id is a
/// guaranteed 404 at the origin, and the nil UUID is what a preset carries when
/// it has no real print table — letting it through costs one wasted fetch per
/// card on the board. Checking it here also means the path built from it holds
/// nothing but hex and dashes.
fn well_formed(id: &str) -> bool {
    id.len() == 36
        && id.contains('-')
        && id
            .chars()
            .all(|c| c == '-' || c.is_ascii_hexdigit() && !c.is_ascii_uppercase())
        && !id.chars().all(|c| c == '0' || c == '-')
}

/// `GET /art/{size}/{face}/{a}/{b}/{id}.jpg`
///
/// A mirror of Scryfall's own path shape, so a client swaps one base URL and
/// changes nothing else. The two sharded directories are redundant — they are
/// derivable from the id — and are checked rather than ignored, so one printing
/// cannot be cached under two names.
///
/// Unauthenticated on purpose. This serves public artwork that the client
/// would otherwise fetch straight from a public CDN, and requiring a token
/// would buy nothing while breaking every plain image load. What bounds it is
/// that only an id can be named and that the outbound side is rate limited.
pub async fn art(
    State(state): State<Shared>,
    Path((size, face, a, b, file)): Path<(String, String, String, String, String)>,
) -> Response {
    let Some(id) = file.strip_suffix(".jpg") else {
        return StatusCode::NOT_FOUND.into_response();
    };
    if !SIZES.contains(&size.as_str())
        || !FACES.contains(&face.as_str())
        || !well_formed(id)
        || a != id[..1]
        || b != id[1..2]
    {
        return StatusCode::NOT_FOUND.into_response();
    }
    match state.art.fetch(&size, &face, id).await {
        Ok(bytes) => (
            [
                (header::CONTENT_TYPE, "image/jpeg"),
                (header::CACHE_CONTROL, MAX_AGE),
                // The browser client is served from one origin and talks to the
                // gateway on another, so without this a wasm build loads no art
                // at all — and fails silently, which is the worst shape a bug
                // can take. Public artwork, no credentials, so `*` is the whole
                // answer.
                (header::ACCESS_CONTROL_ALLOW_ORIGIN, "*"),
            ],
            bytes,
        )
            .into_response(),
        Err(code) => code.into_response(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The path shape is a contract with `baylee_client_core::images`, which
    /// this crate cannot see — the client builds these URLs and the gateway
    /// answers them, and nothing in the compiler holds the two together. So
    /// each side tests the shape it is responsible for against the one written
    /// down in `docs/protocol.md`.
    #[test]
    fn the_route_mirrors_the_path_the_client_builds() {
        let id = "e3285e6b-3e79-4d7c-bf96-d920f973b0d1";
        // Spelled out rather than computed, because computing it here from the
        // same expression the code uses would agree with it however wrong both
        // were. This is the shape in `docs/protocol.md` §"Card art", by hand.
        assert_eq!(
            origin_url("small", "front", id),
            "https://cards.scryfall.io/small/front/e/3/e3285e6b-3e79-4d7c-bf96-d920f973b0d1.jpg"
        );
        assert_eq!(
            origin_url("normal", "back", id),
            "https://cards.scryfall.io/normal/back/e/3/e3285e6b-3e79-4d7c-bf96-d920f973b0d1.jpg"
        );
        let cache = ArtCache::new(Some(PathBuf::from("/tmp/art")));
        assert_eq!(
            cache.path("small", "front", id).expect("enabled"),
            PathBuf::from("/tmp/art/small/front").join(id)
        );
    }

    #[test]
    fn a_disabled_cache_has_nowhere_to_put_anything() {
        let cache = ArtCache::new(None);
        assert!(!cache.enabled());
        assert!(cache.path("small", "front", "whatever").is_none());
    }

    /// Every one of these would be a guaranteed round trip to a 404, and the
    /// nil UUID would be one *per card on the board* — a preset with no real
    /// print table gives every card the same id.
    #[test]
    fn only_a_real_printing_id_is_worth_a_fetch() {
        assert!(well_formed("e3285e6b-3e79-4d7c-bf96-d920f973b0d1"));
        assert!(!well_formed("00000000-0000-0000-0000-000000000000"));
        assert!(!well_formed("e3285e6b3e794d7cbf96d920f973b0d1"));
        assert!(!well_formed("short"));
        assert!(!well_formed(""));
    }

    /// The id reaches the filesystem, so it must not be able to carry a path
    /// separator, a parent reference, or a percent escape into it.
    #[test]
    fn an_id_cannot_climb_out_of_the_cache_directory() {
        assert!(!well_formed("../../../../etc/passwd"));
        assert!(!well_formed("e3285e6b/3e79/4d7c/bf96/d920f973b0d1"));
        assert!(!well_formed("e3285e6b-3e79-4d7c-bf96-d920f973b0.."));
        // Upper-case hex is well-formed as a UUID but would be a *second* name
        // for a printing already cached under its lower-case one.
        assert!(!well_formed("E3285E6B-3E79-4D7C-BF96-D920F973B0D1"));
    }

    /// The limiter is the gateway's, not each game's, or four tables starting
    /// at once would leave at four times the rate `docs/legal.md` §3 allows.
    #[tokio::test]
    async fn requests_leave_no_faster_than_the_rate_limit() {
        let cache = ArtCache::new(None);
        let start = Instant::now();
        for _ in 0..4 {
            cache.slot().await;
        }
        // Four slots are three gaps: the first leaves at once.
        assert!(
            start.elapsed() >= MIN_GAP * 3,
            "four requests left in {:?}, faster than ten a second",
            start.elapsed()
        );
    }

    /// A gateway that mirrors nothing must not spawn a task that fetches
    /// anyway — the test suite runs with no network and has to stay that way.
    #[tokio::test]
    async fn a_disabled_cache_warms_nothing() {
        let cache = Arc::new(ArtCache::new(None));
        cache.warm(vec!["e3285e6b-3e79-4d7c-bf96-d920f973b0d1".into()]);
        assert!(
            cache
                .fetch("small", "front", "e3285e6b-3e79-4d7c-bf96-d920f973b0d1")
                .await
                .is_err()
        );
    }
}
