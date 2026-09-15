//! Minimal Scryfall card model + cached, rate-limited fetching.
//!
//! Responses are cached as committed JSON files so codegen stays
//! reproducible and CI needs no network.

use crate::error::CodegenError;
use crate::stubgen::slug;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

const API: &str = "https://api.scryfall.com";
/// Scryfall asks every client to identify itself.
const USER_AGENT: &str = "baylee-codegen/0.1 (non-commercial fan project)";
const RATE_LIMIT_PAUSE: Duration = Duration::from_millis(200);
const RATE_LIMIT_BACKOFF: Duration = Duration::from_mins(1);
const MAX_ATTEMPTS: u32 = 4;

/// Why a single Scryfall request failed.
#[derive(Debug)]
enum FetchError {
    NotFound,
    Other(String),
}

/// The subset of a Scryfall card object we care about.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScryfallCard {
    /// Scryfall printing UUID.
    pub id: String,
    /// Oracle UUID (rules identity).
    pub oracle_id: Option<String>,
    /// Card name (both faces joined with " // " for multi-face cards).
    pub name: String,
    /// Mana cost (single-face cards).
    pub mana_cost: Option<String>,
    /// Type line (single-face cards).
    pub type_line: Option<String>,
    /// Oracle text (single-face cards).
    pub oracle_text: Option<String>,
    /// Colors.
    pub colors: Option<Vec<String>>,
    /// Color identity letters.
    pub color_identity: Option<Vec<String>>,
    /// Set code.
    pub set: Option<String>,
    /// Set name.
    pub set_name: Option<String>,
    /// Collector number.
    pub collector_number: Option<String>,
    /// Rarity.
    pub rarity: Option<String>,
    /// Layout (`normal`, `modal_dfc`, `adventure`, …).
    pub layout: Option<String>,
    /// Power (string, may contain `*`).
    pub power: Option<String>,
    /// Toughness.
    pub toughness: Option<String>,
    /// Loyalty.
    pub loyalty: Option<String>,
    /// Faces for multi-face layouts.
    pub card_faces: Option<Vec<ScryfallFace>>,
}

/// One face of a multi-face Scryfall card.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScryfallFace {
    /// Face name.
    pub name: String,
    /// Mana cost.
    pub mana_cost: Option<String>,
    /// Type line.
    pub type_line: Option<String>,
    /// Oracle text.
    pub oracle_text: Option<String>,
    /// Power.
    pub power: Option<String>,
    /// Toughness.
    pub toughness: Option<String>,
    /// Loyalty.
    pub loyalty: Option<String>,
}

fn get_json<T: for<'de> Deserialize<'de>>(agent: &ureq::Agent, url: &str) -> Result<T, FetchError> {
    for attempt in 1..=MAX_ATTEMPTS {
        std::thread::sleep(RATE_LIMIT_PAUSE);
        let result = agent
            .get(url)
            .header("User-Agent", USER_AGENT)
            .header("Accept", "application/json")
            .call();
        let mut resp = match result {
            Ok(resp) => resp,
            Err(ureq::Error::StatusCode(404)) => return Err(FetchError::NotFound),
            Err(ureq::Error::StatusCode(429)) => {
                eprintln!(
                    "scryfall: rate limited, backing off 60s (attempt {attempt}/{MAX_ATTEMPTS})"
                );
                std::thread::sleep(RATE_LIMIT_BACKOFF);
                continue;
            }
            Err(e) => return Err(FetchError::Other(e.to_string())),
        };
        return resp
            .body_mut()
            .read_json::<T>()
            .map_err(|e| FetchError::Other(e.to_string()));
    }
    Err(FetchError::Other("rate limited repeatedly".to_string()))
}

/// Percent-encodes everything except unreserved characters.
#[must_use]
pub fn url_encode(s: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'-' | b'_' | b'.' | b'~') {
            out.push(b as char);
        } else {
            let _ = write!(out, "%{b:02X}");
        }
    }
    out
}

/// Fetches a card by exact name (fuzzy fallback), using the JSON cache.
///
/// # Errors
/// [`CodegenError::CardNotFound`] when neither exact nor fuzzy match exists,
/// or an HTTP/IO/JSON error.
pub fn fetch_named(
    name: &str,
    agent: &ureq::Agent,
    cache_dir: &Path,
) -> Result<ScryfallCard, CodegenError> {
    let file = cache_dir.join(format!("{}.json", slug(name)));
    if file.exists() {
        let text = fs::read_to_string(&file).map_err(CodegenError::io(&file))?;
        return Ok(serde_json::from_str(&text)?);
    }
    let exact = format!("{API}/cards/named?exact={}", url_encode(name));
    let card = match get_json::<ScryfallCard>(agent, &exact) {
        Ok(card) => card,
        Err(FetchError::NotFound) => {
            let fuzzy = format!("{API}/cards/named?fuzzy={}", url_encode(name));
            get_json::<ScryfallCard>(agent, &fuzzy).map_err(|e| match e {
                FetchError::NotFound => CodegenError::CardNotFound(name.to_string()),
                FetchError::Other(message) => CodegenError::Http {
                    url: fuzzy.clone(),
                    message,
                },
            })?
        }
        Err(FetchError::Other(message)) => {
            return Err(CodegenError::Http {
                url: exact.clone(),
                message,
            });
        }
    };
    fs::create_dir_all(cache_dir).map_err(CodegenError::io(cache_dir))?;
    let tmp: PathBuf = file.with_extension("part");
    fs::write(&tmp, serde_json::to_string_pretty(&card)?).map_err(CodegenError::io(&tmp))?;
    fs::rename(&tmp, &file).map_err(CodegenError::io(&file))?;
    Ok(card)
}

#[derive(Deserialize)]
struct CatalogResponse {
    data: Vec<String>,
}

/// Fetches a Scryfall catalog (`/catalog/<name>`) with caching.
///
/// # Errors
/// HTTP/IO/JSON errors.
pub fn fetch_catalog(
    name: &str,
    agent: &ureq::Agent,
    cache_dir: &Path,
) -> Result<Vec<String>, CodegenError> {
    let file = cache_dir.join(format!("catalog-{name}.json"));
    if file.exists() {
        let text = fs::read_to_string(&file).map_err(CodegenError::io(&file))?;
        return Ok(serde_json::from_str(&text)?);
    }
    let url = format!("{API}/catalog/{name}");
    let resp: CatalogResponse = get_json(agent, &url).map_err(|e| match e {
        FetchError::NotFound => CodegenError::Http {
            url: url.clone(),
            message: "catalog not found".to_string(),
        },
        FetchError::Other(message) => CodegenError::Http {
            url: url.clone(),
            message,
        },
    })?;
    fs::create_dir_all(cache_dir).map_err(CodegenError::io(cache_dir))?;
    fs::write(&file, serde_json::to_string_pretty(&resp.data)?).map_err(CodegenError::io(&file))?;
    Ok(resp.data)
}

/// One entry of Scryfall's `/bulk-data` listing.
#[derive(Deserialize)]
struct BulkEntry {
    #[serde(rename = "type")]
    kind: String,
    jsonl_download_uri: String,
}

#[derive(Deserialize)]
struct BulkList {
    data: Vec<BulkEntry>,
}

/// The bulk feed whose choice of printing is the one `/cards/named?exact=`
/// makes — one row per *oracle* card, Scryfall's own default printing.
///
/// `default_cards` is the wrong feed for this and the difference is not a
/// preference: it carries every English printing, so 1097 of the pool's 1365
/// names match several rows there and nothing in the file says which one the
/// API would have picked. Measured against the 1365 payloads this cache was
/// filled with one request at a time, `oracle_cards` agrees on 1359 ids with
/// no ambiguity at all, and the six it disagrees on are the six the *live
/// API* has since moved too (Deserted Beach INR 276 → FRA 176, and five
/// like it). So the feed is API-equivalent: it introduces no printing this
/// repo would not have fetched anyway.
const BULK_FEED: &str = "oracle_cards";

/// Below this many missing payloads, one 25 MB download costs more than the
/// requests it saves.
const BULK_THRESHOLD: usize = 50;

/// Fills the cache from one bulk download instead of one request per card.
///
/// The cold cache is the expensive case and it was expensive for a reason
/// worth naming: 1365 requests at a 200 ms pause is four and a half minutes
/// at best, and hosted runners share an IP, so four concurrent CI runs earned
/// thirteen 60-second backoffs — 13 of 28 minutes spent asleep. Scryfall asks
/// in its own guidelines that bulk data be used for exactly this.
///
/// Cards are matched on **`oracle_id`**, taken from the ledger, and never on
/// the name. A name is ambiguous in the feed — three cards in this pool share
/// theirs with a token (Mutavault, Llanowar Elves, Savage Lands) — while
/// `oracle_id` matched all 1365 with nothing left over. The payload is
/// written through the same [`ScryfallCard`] as [`fetch_named`] writes, so
/// the two paths cannot emit different files for one card.
///
/// It returns how many payloads it wrote and **never fails**: anything that
/// goes wrong is reported and leaves the per-card path behind it to fill what
/// is still missing, which is what makes the pair self-healing. A stream that
/// dies halfway leaves the cards it did write, and they are correct.
pub fn fill_from_bulk(names: &[String], agent: &ureq::Agent, cache_dir: &Path) -> usize {
    let missing: Vec<&String> = names
        .iter()
        .filter(|n| !cache_dir.join(format!("{}.json", slug(n))).exists())
        .collect();
    if missing.len() < BULK_THRESHOLD {
        return 0;
    }

    // Name → oracle id, out of the compiled ledger, in two tiers. The pool
    // names a two-faced card by its **front face** ("Sheoldred") where the
    // ledger follows Scryfall and writes the whole name ("Sheoldred // The
    // True Scriptures"), so the whole name alone misses those cards and the
    // first draft of this silently left three of them to the API. Both tiers
    // are exact rather than merely likely: the ledger's 33 694 rows carry
    // 33 694 distinct names, its 874 multi-part names have 874 distinct front
    // faces, and no front face is also a whole name — measured, because a
    // collision in either tier would hand a card another card's payload.
    let mut by_name: HashMap<&str, &str> = HashMap::with_capacity(baylee_cards_index::ROWS.len());
    let mut by_front: HashMap<&str, &str> = HashMap::new();
    for row in &baylee_cards_index::ROWS {
        by_name.insert(row.name, row.oracle_id);
        if let Some(front) = row.name.split_once(" // ").map(|(front, _)| front) {
            by_front.insert(front, row.oracle_id);
        }
    }
    let mut wanted: HashMap<&str, &str> = HashMap::with_capacity(missing.len());
    for name in &missing {
        let key = name.as_str();
        if let Some(oracle_id) = by_name.get(key).or_else(|| by_front.get(key)) {
            wanted.insert(oracle_id, key);
        }
    }
    if wanted.is_empty() {
        return 0;
    }
    eprintln!(
        "scryfall: {} payloads missing, one {BULK_FEED} download instead of {} requests",
        missing.len(),
        missing.len()
    );

    if let Err(message) = fs::create_dir_all(cache_dir) {
        eprintln!("scryfall: cannot create {}: {message}", cache_dir.display());
        return 0;
    }
    let url = match bulk_uri(agent) {
        Ok(url) => url,
        Err(message) => {
            eprintln!("scryfall: bulk listing unavailable ({message}), fetching one by one");
            return 0;
        }
    };
    match stream_bulk(&url, agent, &wanted, cache_dir) {
        Ok(written) => {
            eprintln!("scryfall: {written} payloads written from the bulk feed");
            written
        }
        Err(message) => {
            eprintln!("scryfall: bulk download failed ({message}), fetching the rest one by one");
            0
        }
    }
}

/// Resolves [`BULK_FEED`] to its current download URL.
fn bulk_uri(agent: &ureq::Agent) -> Result<String, String> {
    let url = format!("{API}/bulk-data");
    let list: BulkList = get_json(agent, &url).map_err(|e| match e {
        FetchError::NotFound => "no bulk-data listing".to_string(),
        FetchError::Other(message) => message,
    })?;
    list.data
        .into_iter()
        .find(|e| e.kind == BULK_FEED)
        .map(|e| e.jsonl_download_uri)
        .ok_or_else(|| format!("Scryfall has no {BULK_FEED} feed"))
}

/// Streams the feed line by line, writing the payloads that were asked for.
///
/// The feed is JSONL and is never held whole: 30 000 cards decompress to
/// several hundred megabytes, and only the 1365 lines this pool asked for are
/// ever turned into a [`ScryfallCard`].
fn stream_bulk(
    url: &str,
    agent: &ureq::Agent,
    wanted: &std::collections::HashMap<&str, &str>,
    cache_dir: &Path,
) -> Result<usize, String> {
    use std::io::{BufRead as _, BufReader};

    let response = agent
        .get(url)
        .header("User-Agent", USER_AGENT)
        .call()
        .map_err(|e| e.to_string())?;
    let decoder = flate2::read::GzDecoder::new(response.into_body().into_reader());
    let mut written = 0usize;
    for line in BufReader::new(decoder).lines() {
        let line = line.map_err(|e| e.to_string())?;
        let line = line.trim().trim_end_matches(',');
        // JSONL, but the older bracketed form costs nothing to tolerate.
        if line.is_empty() || line == "[" || line == "]" {
            continue;
        }
        // Read the oracle id before paying for the whole card: this loop runs
        // thirty thousand times and keeps at most 1365 of them.
        let Some(oracle_id) = peek_oracle_id(line) else {
            continue;
        };
        let Some(name) = wanted.get(oracle_id) else {
            continue;
        };
        let card: ScryfallCard = match serde_json::from_str(line) {
            Ok(card) => card,
            Err(message) => {
                eprintln!("scryfall: {name} unreadable in the bulk feed ({message})");
                continue;
            }
        };
        if let Err(message) = write_payload(&card, name, cache_dir) {
            eprintln!("scryfall: {name} not written ({message})");
            continue;
        }
        written += 1;
    }
    Ok(written)
}

/// The `"oracle_id"` of one JSONL line, without parsing the rest of it.
fn peek_oracle_id(line: &str) -> Option<&str> {
    const KEY: &str = "\"oracle_id\":\"";
    let at = line.find(KEY)? + KEY.len();
    let rest = &line[at..];
    let end = rest.find('"')?;
    Some(&rest[..end])
}

/// Writes one payload the way [`fetch_named`] writes it — `.part` first, so a
/// reader never sees a half-written file under the name it looks a card up by.
fn write_payload(card: &ScryfallCard, name: &str, cache_dir: &Path) -> Result<(), String> {
    let file = cache_dir.join(format!("{}.json", slug(name)));
    let tmp = file.with_extension("part");
    let text = serde_json::to_string_pretty(card).map_err(|e| e.to_string())?;
    fs::write(&tmp, text).map_err(|e| e.to_string())?;
    fs::rename(&tmp, &file).map_err(|e| e.to_string())
}
