//! Minimal Scryfall card model + cached, rate-limited fetching.
//!
//! Responses are cached as committed JSON files so codegen stays
//! reproducible and CI needs no network.

use crate::error::CodegenError;
use crate::stubgen::slug;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

const API: &str = "https://api.scryfall.com";
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
            .header(
                "User-Agent",
                "baylee-codegen/0.1 (non-commercial fan project)",
            )
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
