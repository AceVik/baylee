//! Minimal Scryfall card model + cached, rate-limited fetching.
//!
//! Responses are cached as committed JSON files so codegen stays
//! reproducible and CI needs no network.

use crate::catalog::{SubtypeCatalogs, module_name};
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
    /// Art for the card as a whole, when the printing is one piece of card.
    ///
    /// Scryfall puts the art at **exactly one** of two levels and never both:
    /// here for a printing with one physical face, and on each
    /// [`ScryfallFace`] for one with two. That is the only authoritative
    /// answer to "does this printing have a back", and it is per *printing* —
    /// the layout is not, because an Adventure can be printed double-faced
    /// (`is:dfc layout:adventure` finds three) and would then have a back
    /// while every other Adventure does not.
    #[serde(default)]
    pub image_uris: Option<ScryfallImages>,
    /// How far along Scryfall's scan of this printing is.
    ///
    /// Kept because it is the one thing that tells "Scryfall has published no
    /// picture yet" apart from "this cache file was written before the
    /// pictures were read". Both look like art at neither level, and they
    /// want opposite answers: the first is an honest *no back* and the second
    /// has to stop the run. Measured: 75 printings answering
    /// `image_status: missing` carry `image_uris` on no face and none at the
    /// top either, while every `placeholder` and `lowres` one does.
    #[serde(default)]
    pub image_status: Option<String>,
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
    /// Art for this face alone, when the printing has a face on each side.
    ///
    /// Present on both faces of a `transform` or `modal_dfc` printing and on
    /// neither face of an `adventure`, `split` or `prepare` one, where the two
    /// faces share a single piece of card. See [`ScryfallCard::image_uris`].
    #[serde(default)]
    pub image_uris: Option<ScryfallImages>,
}

/// The art URLs Scryfall serves for one face, or for a whole one-faced card.
///
/// Only `normal` is kept, of the six sizes Scryfall offers: nothing here
/// fetches art — the client and the gateway's mirror both *construct* a URL
/// from the printing id — so this is stored to be **asked a question**, not
/// to be followed. It earns the bytes twice over as the thing a refusal can
/// print, so a person reading a bail can open the picture the generator was
/// arguing about.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScryfallImages {
    /// The `normal`-size art, if Scryfall named one at this level.
    pub normal: Option<String>,
}

/// The three layouts CR 712.1 calls a double-faced card.
///
/// "A double-faced card has a Magic card face on one side and either a Magic
/// card face or half of an oversized card face on the other. (It does not
/// have a Magic card back.) There are three kinds of double-faced cards:
/// nonmodal double-faced cards (previously called 'transforming double-faced
/// cards'), modal double-faced cards, and meld cards." Scryfall's three
/// layout names for those three kinds, in that order.
const DOUBLE_FACED_LAYOUTS: [&str; 3] = ["transform", "modal_dfc", "meld"];

/// What a printing says about its own two sides.
///
/// Two questions rather than one, answered from different places because they
/// *are* different: one is a rule about the card, the other a fact about
/// Scryfall's asset store. In this pool they differ by the two meld cards,
/// which CR 712.1 calls double-faced and which Scryfall serves no back for —
/// a meld back is half of an oversized face and lives as a card of its own
/// rather than as a face of the component.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sides {
    /// Scryfall serves a second picture for this printing.
    ///
    /// The question a client is really asking when it offers to turn a card
    /// over, because the URL it would build is Scryfall's `back` shelf.
    pub back_image: bool,
    /// A double-faced card in the sense CR 712.1 gives the words.
    ///
    /// What the deck builder's "double-faced" filter means, which is a claim
    /// about the printed card and not about what this client can draw.
    pub double_faced: bool,
}

impl ScryfallCard {
    /// Reads both sides questions off the payload, or says why it cannot.
    ///
    /// The back is read from **`image_uris`** and not from the layout, because
    /// the layout is not the printing: `is:dfc layout:adventure` finds three,
    /// so a list of layouts that have a back is correct until one of those is
    /// the printing a card pins, and then it is silently wrong. Scryfall puts
    /// the art at exactly one of two levels — on the card for one piece of
    /// cardboard, on each face for two — which is the same question asked of
    /// the authority instead of of a proxy.
    ///
    /// Art at *neither* level has two causes that want opposite answers, and
    /// `image_status` is what tells them apart. Scryfall omits `image_uris`
    /// entirely while a scan is unpublished — measured: 75 printings at
    /// `missing` carry none at either level, every `placeholder` and `lowres`
    /// one carries them — and that is an honest "no back", because there is no
    /// picture on either side to draw. A payload with no `image_status` at all
    /// was written before this generator read these fields, and answering it
    /// would write "no back" for every double-faced card in the pool, so it
    /// stops the run instead.
    ///
    /// # Errors
    ///
    /// A cache file too old to answer, a card whose art is absent for a reason
    /// Scryfall does not give, and a printing that has a back the rules do not
    /// account for — see [`DOUBLE_FACED_LAYOUTS`].
    pub fn sides(&self) -> Result<Sides, String> {
        let layout = self.layout.as_deref().unwrap_or("normal");
        let double_faced = DOUBLE_FACED_LAYOUTS.contains(&layout);
        let faces = self.card_faces.as_deref().unwrap_or_default();
        let per_face = faces.iter().any(|face| face.image_uris.is_some());
        let back_image = if per_face {
            true
        } else if self.image_uris.is_some() {
            false
        } else {
            match self.image_status.as_deref() {
                // No scan published, so no back — and no front either. Correct
                // rather than merely safe: a client that built a back key here
                // would be asking for a picture nobody has.
                Some("missing") => false,
                Some(other) => {
                    return Err(format!(
                        "{}: art at neither level with image_status {other:?}, and Scryfall \
                         omits image_uris only at \"missing\"",
                        self.name
                    ));
                }
                None => {
                    return Err(format!(
                        "{}: this cache file predates image_uris and image_status. Answering it \
                         would write \"no back\" for every double-faced card in the pool, so run \
                         `cargo run -p xtask -- scryfall-cache --refetch` to rewrite the cache \
                         first. The flag is the whole cure: a plain run fills what is *missing*, \
                         and a payload of the wrong shape is not missing.",
                        self.name
                    ));
                }
            }
        };
        if back_image && !double_faced {
            let art = faces
                .iter()
                .find_map(|face| face.image_uris.as_ref()?.normal.as_deref())
                .unwrap_or("(no normal-size art named)");
            return Err(format!(
                "{}: layout {layout:?} has a face on each side ({art}), and CR 712.1 names three \
                 kinds of double-faced card, none of them that one. Read the printing, then \
                 either add the layout to DOUBLE_FACED_LAYOUTS or say why it is not one.",
                self.name
            ));
        }
        Ok(Sides {
            back_image,
            double_faced,
        })
    }
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

/// Fetches one *printing* by its Scryfall id, using the JSON cache.
///
/// [`fetch_named`] answers with whatever Scryfall currently **defaults** to for
/// a name, which is a moving target: a reprint set makes the new printing the
/// default, and a card header that named the old one is suddenly "wrong"
/// without anybody touching the repository. A printing id never moves, so this
/// cache entry is correct forever once written, and a header held against it
/// is held against the piece of cardboard it was written from.
///
/// `xtask`'s `Pinned` carries the measurement that made this necessary and the
/// reason it is deliberately **not** filled from a bulk feed.
///
/// # Errors
/// [`CodegenError::CardNotFound`] for an id Scryfall does not know, or an
/// HTTP/IO/JSON error.
pub fn fetch_printing(
    id: &str,
    agent: &ureq::Agent,
    cache_dir: &Path,
) -> Result<ScryfallCard, CodegenError> {
    let file = printing_cache_path(cache_dir, id);
    if file.exists() {
        let text = fs::read_to_string(&file).map_err(CodegenError::io(&file))?;
        return Ok(serde_json::from_str(&text)?);
    }
    let url = format!("{API}/cards/{id}");
    let card: ScryfallCard = get_json(agent, &url).map_err(|e| match e {
        FetchError::NotFound => CodegenError::CardNotFound(id.to_string()),
        FetchError::Other(message) => CodegenError::Http {
            url: url.clone(),
            message,
        },
    })?;
    fs::create_dir_all(cache_dir).map_err(CodegenError::io(cache_dir))?;
    let tmp: PathBuf = file.with_extension("part");
    fs::write(&tmp, serde_json::to_string_pretty(&card)?).map_err(CodegenError::io(&tmp))?;
    fs::rename(&tmp, &file).map_err(CodegenError::io(&file))?;
    Ok(card)
}

/// Where [`fetch_printing`] keeps a printing, and where a reader looks for one.
///
/// Prefixed rather than bare, because the name-keyed cache lives in the same
/// directory and a card called `abc123` is not impossible.
#[must_use]
pub fn printing_cache_path(cache_dir: &Path, id: &str) -> PathBuf {
    cache_dir.join(format!("printing-{id}.json"))
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

/// Fetches every subtype catalog there is, in id-assignment order, normalized.
///
/// Six callers used to spell the six catalogs out, one `fetch_catalog` line
/// each, and every one of them was a list of the kinds standing beside the
/// list of the kinds. Adding battle meant editing all six — and a caller that
/// had been missed would have compiled for as long as the struct had a
/// `..Default::default()` in it. The catalog's Scryfall name is the module
/// name plus `-types`, which is Scryfall's own spelling for all seven, so the
/// name is derived here rather than typed.
///
/// # Errors
/// HTTP/IO/JSON errors from any one catalog.
pub fn fetch_subtype_catalogs(
    agent: &ureq::Agent,
    cache_dir: &Path,
) -> Result<SubtypeCatalogs, CodegenError> {
    let mut cats = SubtypeCatalogs::default();
    for (kind, list) in cats.ordered_mut() {
        *list = fetch_catalog(&format!("{}-types", module_name(kind)), agent, cache_dir)?;
    }
    cats.normalize();
    Ok(cats)
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

/// What shape a held payload has to be in for the readers above to answer.
///
/// The cache is a **typed projection** of Scryfall's rows and not a copy of
/// them: what lands on disk is whatever [`ScryfallCard`] declares, so teaching
/// that struct a field the readers then depend on leaves every held file
/// unreadable while looking perfectly present. That is not hypothetical — the
/// whole cache went that way the day [`ScryfallCard::sides`] started reading
/// `image_uris`, and 3052 of 3052 payloads carried none of it.
///
/// **Bump this whenever a reader starts depending on a field the payloads may
/// not carry.** The number is the only thing that turns "I am missing a field"
/// into a repair somebody's machine performs by itself.
///
/// | version | what it added |
/// |---|---|
/// | 1 | `image_uris` / `image_status`, which [`ScryfallCard::sides`] reads |
pub const PAYLOAD_SCHEMA: u32 = 1;

/// Where the stamp sits. A leading dot, because [`slug`] never writes one, so
/// this name can never collide with a card called "Schema".
const STAMP_FILE: &str = ".schema";

/// Whether a stamp's contents promise payloads *this* reader can answer.
///
/// Older than the code is stale; **newer than the code is not**. Serde ignores
/// a field this version does not know, so a cache written by a later branch
/// reads perfectly here — and treating it as stale would make two branches
/// sharing one cache rewrite it past each other on every switch.
fn stamp_is_current(text: &str) -> bool {
    let first = text.lines().next().unwrap_or_default().trim();
    matches!(first.parse::<u32>(), Ok(held) if held >= PAYLOAD_SCHEMA)
}

/// Whether the held payloads predate what the readers now ask of them.
///
/// Missing, unparsable and too old all answer the same way, because all three
/// mean the same thing: nothing has promised these files carry today's fields.
/// A cache directory that does not exist yet is stale for free, which costs
/// nothing — everything in it is missing anyway.
///
/// The limit worth knowing: the stamp speaks for the last *full* rewrite. A
/// single payload fetched one at a time by an older branch lands in a stamped
/// cache without lowering the stamp, so a shared cache across branches is
/// approximate. Rewriting is one bulk download, so when in doubt, `--refetch`.
#[must_use]
pub fn schema_stale(cache_dir: &Path) -> bool {
    !fs::read_to_string(cache_dir.join(STAMP_FILE)).is_ok_and(|text| stamp_is_current(&text))
}

/// Records that every payload here was written by this version of the reader.
///
/// Written **after** a fill rather than before, so a run that dies halfway
/// leaves the cache stale and the next run repairs it. Exactly
/// [`PAYLOAD_SCHEMA`] and never the higher of the two: an older branch that
/// rewrites the cache has genuinely made it older, and saying so is what makes
/// the next run on the newer branch repair it.
pub fn write_schema_stamp(cache_dir: &Path) {
    let path = cache_dir.join(STAMP_FILE);
    let body = format!(
        "{PAYLOAD_SCHEMA}\n\n\
         The payload schema every file beside this one was written with.\n\
         Written by `xtask scryfall-cache`; see `scryfall::PAYLOAD_SCHEMA`.\n"
    );
    if let Err(message) = fs::write(&path, body) {
        eprintln!("scryfall: cannot stamp {}: {message}", path.display());
    }
}

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
pub fn fill_from_bulk(
    names: &[String],
    agent: &ureq::Agent,
    cache_dir: &Path,
    refetch: bool,
) -> usize {
    // `refetch` is for a change to *this* struct rather than to Scryfall's
    // rows: the cache holds whatever `ScryfallCard` declares, so a new field
    // leaves every held file without it and a run that fills only what is
    // missing fills nothing at all. Non-destructive on purpose — the files
    // are replaced one at a time as the stream arrives, so a download that
    // dies halfway leaves a cache that is older than it should be rather
    // than one that is empty.
    let missing: Vec<&String> = names
        .iter()
        .filter(|n| refetch || !cache_dir.join(format!("{}.json", slug(n))).exists())
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
    // faces, and no front face is also a whole name. The ledger is
    // append-only, so that is a test rather than a measurement written down
    // once — `a_pool_name_reaches_exactly_one_ledger_row`, because a collision
    // in either tier would hand a card another card's payload in silence.
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
        "scryfall: {} payloads {}, one {BULK_FEED} download instead of {} requests",
        missing.len(),
        if refetch { "to rewrite" } else { "missing" },
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

#[cfg(test)]
mod tests {
    use super::{PAYLOAD_SCHEMA, ScryfallCard, Sides, stamp_is_current};
    use std::collections::HashMap;

    /// A payload, built the way the cache holds one — from JSON, so the
    /// field names this reader depends on are pinned here too.
    fn card(json: &str) -> ScryfallCard {
        serde_json::from_str(json).expect("a payload the reader understands")
    }

    /// The two questions `sides` answers are asked of different authorities,
    /// and the art is the one that is per *printing*. Scryfall puts it at
    /// exactly one of two levels — on the card for one piece of cardboard, on
    /// each face for two — so that is what is read, rather than the layout,
    /// which an Adventure printed double-faced would make silently wrong.
    #[test]
    fn a_printing_with_a_face_on_each_side_says_so_through_its_art() {
        let dfc = card(
            r#"{
                "id": "1", "name": "Front // Back", "layout": "transform",
                "card_faces": [
                    {"name": "Front", "image_uris": {"normal": "front.jpg"}},
                    {"name": "Back", "image_uris": {"normal": "back.jpg"}}
                ]
            }"#,
        );
        assert_eq!(
            dfc.sides().expect("a transform card is readable"),
            Sides {
                back_image: true,
                double_faced: true,
            }
        );

        let one_piece = card(
            r#"{
                "id": "2", "name": "Lightning Bolt", "layout": "normal",
                "image_uris": {"normal": "bolt.jpg"}
            }"#,
        );
        assert_eq!(
            one_piece.sides().expect("an ordinary card is readable"),
            Sides {
                back_image: false,
                double_faced: false,
            }
        );
    }

    /// The two answers are not one answer, and the pool holds the case that
    /// proves it: a meld card is double-faced by CR 712.1 and Scryfall serves
    /// no back for it, because a meld back is half of an oversized face and
    /// lives as a card of its own.
    #[test]
    fn a_meld_card_is_double_faced_with_no_second_picture() {
        let meld = card(
            r#"{
                "id": "3", "name": "Bruna, the Fading Light", "layout": "meld",
                "image_uris": {"normal": "bruna.jpg"}
            }"#,
        );
        assert_eq!(
            meld.sides().expect("a meld card is readable"),
            Sides {
                back_image: false,
                double_faced: true,
            }
        );
    }

    /// Art at neither level has two causes that want opposite answers, and
    /// `image_status` is what tells them apart. An unpublished scan is an
    /// honest "no back" — there is no picture on either side to draw — and
    /// anything else stops the run rather than guessing.
    #[test]
    fn a_printing_with_no_art_is_answered_only_where_scryfall_says_why() {
        let unscanned = card(r#"{"id": "4", "name": "Brand New", "image_status": "missing"}"#);
        assert_eq!(
            unscanned.sides().expect("missing art is an answer"),
            Sides {
                back_image: false,
                double_faced: false,
            }
        );

        let odd = card(r#"{"id": "5", "name": "Odd One", "image_status": "lowres"}"#);
        let refusal = odd.sides().expect_err("a status that cannot mean no art");
        assert!(
            refusal.contains("Odd One") && refusal.contains("lowres"),
            "the refusal names the card and the status it could not read: {refusal}"
        );
    }

    /// A payload written before this reader existed carries no
    /// `image_status` at all, and answering it would write "no back" for
    /// every double-faced card in the pool. It stops the run instead, and the
    /// message names the flag that repairs it — a plain fill writes what is
    /// *missing*, and a payload of the wrong shape is not missing.
    #[test]
    fn a_payload_older_than_this_reader_stops_the_run_and_names_the_cure() {
        let old = card(r#"{"id": "6", "name": "Ancestral Recall"}"#);
        let refusal = old.sides().expect_err("an unanswerable payload");
        assert!(
            refusal.contains("--refetch"),
            "the refusal has to say what to run: {refusal}"
        );
        assert!(refusal.contains("Ancestral Recall"));
    }

    /// A printing with a face on each side and a layout the rules do not
    /// account for is a refusal rather than a guess: CR 712.1 names three
    /// kinds of double-faced card, and a fourth is a decision a person makes
    /// after reading the printing.
    #[test]
    fn a_two_sided_printing_of_an_unknown_layout_is_refused() {
        let surprise = card(
            r#"{
                "id": "7", "name": "Curious Thing", "layout": "adventure",
                "card_faces": [
                    {"name": "Curious Thing", "image_uris": {"normal": "front.jpg"}},
                    {"name": "Curious Deed", "image_uris": {"normal": "back.jpg"}}
                ]
            }"#,
        );
        let refusal = surprise
            .sides()
            .expect_err("a layout with an unexpected back");
        for wanted in ["Curious Thing", "adventure", "front.jpg", "712.1"] {
            assert!(
                refusal.contains(wanted),
                "the refusal has to carry {wanted:?} so a person can read the printing: \
                 {refusal}"
            );
        }
    }

    /// An ordinary Adventure — one piece of cardboard with two faces printed
    /// on the front — is not double-faced and has no back, which is the case
    /// the layout list would have got wrong in the other direction.
    #[test]
    fn an_adventure_printed_on_one_side_has_no_back() {
        let adventure = card(
            r#"{
                "id": "8", "name": "Bonecrusher Giant // Stomp",
                "layout": "adventure",
                "image_uris": {"normal": "giant.jpg"},
                "card_faces": [
                    {"name": "Bonecrusher Giant"},
                    {"name": "Stomp"}
                ]
            }"#,
        );
        assert_eq!(
            adventure.sides().expect("an adventure is readable"),
            Sides {
                back_image: false,
                double_faced: false,
            }
        );
    }

    /// The stamp is read from its **first line**, because the file it is read
    /// out of carries prose under the number — a person who opens the cache
    /// directory should find out what the file is for without going to the
    /// source. A reader that parsed the whole body would call every stamp it
    /// ever wrote unparsable, which fails in the safe direction and would
    /// therefore have gone unnoticed as one bulk download per run, forever.

    #[test]
    fn a_stamp_is_read_out_of_its_first_line() {
        assert!(stamp_is_current(&format!("{PAYLOAD_SCHEMA}")));
        assert!(stamp_is_current(&format!(
            "{PAYLOAD_SCHEMA}\n\nprose under it\n"
        )));
        assert!(stamp_is_current(&format!("  {PAYLOAD_SCHEMA}  \n")));
    }

    /// Older is stale, **newer is not**. Two worktrees share one cache here,
    /// and a `!=` comparison would have each of them rewrite it on every
    /// switch: serde drops a field this version does not know, so a payload
    /// from a later branch answers every question this one asks.
    #[test]
    fn a_newer_stamp_is_not_stale_and_an_older_one_is() {
        assert!(stamp_is_current(&format!("{}", PAYLOAD_SCHEMA + 1)));
        assert!(!stamp_is_current(""));
        assert!(!stamp_is_current("no number here"));
        if PAYLOAD_SCHEMA > 0 {
            assert!(!stamp_is_current(&format!("{}", PAYLOAD_SCHEMA - 1)));
        }
    }

    /// The error a pre-schema payload raises has to name the flag, because the
    /// command without it fills what is *missing* and nothing is missing. This
    /// is the whole of #146: the cure was reachable and the message sent every
    /// session that hit it to a no-op.
    #[test]
    fn the_stale_payload_error_names_the_flag_that_repairs_it() {
        let card: ScryfallCard =
            serde_json::from_str(r#"{"id":"x","name":"A.I.M. Labs","layout":"normal"}"#)
                .expect("a payload from before the fields existed still parses");
        let message = card
            .sides()
            .expect_err("a payload with no image_status stops the run");
        assert!(
            message.contains("scryfall-cache --refetch"),
            "the error has to name the flag, not just the command: {message}"
        );
    }

    /// [`super::fill_from_bulk`] looks a pool name up in the ledger twice — the
    /// whole name, then the front face of a multi-part one — and hands the
    /// matching row's payload to that card. Both tiers were *measured* exact
    /// when they were written, and the ledger is append-only: the next set can
    /// print a card whose whole name is some other card's front face, and the
    /// failure that follows is silent, one card written another card's oracle
    /// text. So the measurement is a test rather than a sentence in a comment.
    #[test]
    fn a_pool_name_reaches_exactly_one_ledger_row() {
        let mut by_name: HashMap<&str, &str> = HashMap::new();
        let mut by_front: HashMap<&str, &str> = HashMap::new();
        for row in &baylee_cards_index::ROWS {
            if let Some(other) = by_name.insert(row.name, row.oracle_id) {
                panic!(
                    "two ledger rows are named {}: {other}, {}",
                    row.name, row.oracle_id
                );
            }
            if let Some(front) = row.name.split_once(" // ").map(|(front, _)| front)
                && let Some(other) = by_front.insert(front, row.oracle_id)
            {
                panic!(
                    "two ledger rows share the front face {front}: {other}, {}",
                    row.oracle_id
                );
            }
        }
        for (front, oracle_id) in &by_front {
            assert!(
                !by_name.contains_key(front),
                "{front} is a whole card and another card's front face ({oracle_id}), \
                 so the front-face tier of fill_from_bulk can hand it the wrong payload"
            );
        }
    }
}
