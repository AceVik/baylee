//! The landing page's Impressum (`web/site/impressum.html`,
//! `docs/feedback.md` §"The Impressum"): the blob with the operator's
//! details, handed out for a solved proof of work, and a honeypot.
//!
//! - `GET /site/impressum/challenge?s=<bits>`: a signed puzzle. Its
//!   difficulty rises with what the request looks like ([`assess`]); `s`
//!   carries what the page saw of its own browser ([`client_bits`]).
//! - `POST /site/impressum` `{nonce, parts, difficulty, expires, sig,
//!   counters}`: the solved puzzle, once, before it expires; the answer is
//!   the blob `scripts/site/impressum_blob.py` wrote
//!   (`FEEDBACK_IMPRESSUM_BLOB`), read on each request, or for a request
//!   that looks like a robot beyond doubt a blob of the same shape holding
//!   invented details ([`decoy`]).
//! - `GET /site/impressum/kontakt`: a link no person sees (hidden by the
//!   page's CSS, `aria-hidden`, disallowed in robots.txt); it answers an
//!   invented Impressum as plain text and marks the asking address for
//!   [`FLAGGED_FOR`], under which it gets the decoy and the hardest puzzle.
//!
//! **No person is ever locked out** (owner, 13.10.2026; the Impressum must
//! stay easy to reach): nothing here answers a challenge with a refusal.
//! A suspicious request only gets a harder puzzle, at most [`MAX_EXTRA`]
//! bits more, and only a robot beyond doubt gets the decoy. A request from
//! another site (an `Origin` that is not the page's) is refused, which no
//! visitor of the page ever sends.
//!
//! The puzzle, the same in `web/site/impressum-worker.js` and the tests:
//! for `part` in `0..parts`, a counter `c` with
//! `SHA-256(nonce + ":" + part + ":" + c)` (UTF-8, decimal numbers)
//! starting with `difficulty` zero bits.
//!
//! Memory only, like the allowances: requests per address for the last
//! hour, the honeypot's marks for [`FLAGGED_FOR`], the puzzles answered
//! until they expire. A restart forgets all of it; no address is written
//! down or logged. Off (`503`) until `FEEDBACK_IMPRESSUM_KEY` and
//! `FEEDBACK_IMPRESSUM_BLOB` are set.

use std::collections::{HashMap, VecDeque};
use std::net::IpAddr;
use std::path::{Path, PathBuf};

use axum::extract::{ConnectInfo, RawQuery, State};
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Json, Response};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use subtle::ConstantTimeEq as _;
use time::{Duration, OffsetDateTime};

use crate::{Shared, refuse};

/// The shortest `FEEDBACK_IMPRESSUM_KEY`.
pub const MIN_KEY_CHARS: usize = 32;

/// Zero bits each part of a puzzle needs for an ordinary request: about a
/// second on a laptop for [`PARTS`] parts (`docs/feedback.md`).
pub const BASE_DIFFICULTY: u32 = 17;

/// The most bits a suspicious request adds: each doubles the work.
pub const MAX_EXTRA: u32 = 4;

/// How many small puzzles one challenge is: the same work as one larger
/// one on average, with far less spread between visitors.
pub const PARTS: u32 = 8;

/// How long a challenge may be answered: well past the hardest puzzle's
/// solving time on a slow phone.
pub const LIFETIME: Duration = Duration::minutes(5);

/// How long an address that asked the honeypot gets the decoy.
pub const FLAGGED_FOR: Duration = Duration::hours(24);

/// The points from which a request is a robot beyond doubt ([`assess`]),
/// with two strong signals among them.
pub const DECOY_POINTS: u32 = 6;

/// The window requests per address are counted in.
pub const WINDOW: Duration = Duration::HOUR;

/// The script's half of the blob's key (`SCRIPT_KEY` in
/// `scripts/site/impressum_blob.py` and `web/site/impressum.js`; a test
/// holds the three equal).
pub const SCRIPT_KEY: &str = "6b1f0c9e52d84a37b0e1f6a2c9d35e48";

/// The blob's first bytes.
pub const MAGIC: &[u8; 4] = b"BYIM";

/// The blob's version: fields as shuffled `[position, code point]` pairs.
pub const VERSION: u8 = 2;

/// The blob's nonce.
pub const NONCE_LEN: usize = 16;

/// What the page reports of its browser in `?s=`: `navigator.webdriver`.
pub const CLIENT_WEBDRIVER: u32 = 1;
/// … no `navigator.languages`.
pub const CLIENT_NO_LANGUAGES: u32 = 2;
/// … a software WebGL renderer (`SwiftShader`, `llvmpipe`).
pub const CLIENT_SOFTWARE_GL: u32 = 4;

/// The most addresses, marks or answered puzzles held before the stale
/// ones are swept out.
const SWEEP_AT: usize = 4096;

/// The most answered puzzles held at all; past it the oldest go first.
const MAX_USED: usize = 200_000;

/// The service's settings for the Impressum.
#[derive(Clone)]
pub struct Settings {
    key: [u8; 32],
    blob: PathBuf,
}

impl Settings {
    /// `key` (`FEEDBACK_IMPRESSUM_KEY`) signs the challenges; `blob`
    /// (`FEEDBACK_IMPRESSUM_BLOB`) is the file the page's details are in.
    ///
    /// # Errors
    ///
    /// When `key` is shorter than [`MIN_KEY_CHARS`] or the file is not an
    /// Impressum blob of this version: better to refuse to start than to
    /// hand out a page that cannot show its details.
    pub fn new(key: &str, blob: impl Into<PathBuf>) -> anyhow::Result<Self> {
        if key.chars().count() < MIN_KEY_CHARS {
            anyhow::bail!(
                "FEEDBACK_IMPRESSUM_KEY: a key needs at least {MIN_KEY_CHARS} characters"
            );
        }
        let blob = blob.into();
        let text = std::fs::read(&blob)
            .map_err(|e| anyhow::anyhow!("FEEDBACK_IMPRESSUM_BLOB {}: {e}", blob.display()))?;
        if !looks_like_blob(&text) {
            anyhow::bail!(
                "FEEDBACK_IMPRESSUM_BLOB {}: not an Impressum blob of version {VERSION} \
                 (scripts/site/impressum_blob.py writes one)",
                blob.display()
            );
        }
        Ok(Self {
            key: Sha256::digest(key.as_bytes()).into(),
            blob,
        })
    }

    /// The key's hash, so the configuration can refuse it as another token.
    pub(crate) fn key_hash(&self) -> [u8; 32] {
        self.key
    }

    /// The file the details are read from.
    #[must_use]
    pub fn blob_path(&self) -> &Path {
        &self.blob
    }
}

/// Whether `text` is a base64url blob that starts with [`MAGIC`] and
/// [`VERSION`].
#[must_use]
pub fn looks_like_blob(text: &[u8]) -> bool {
    let text = text.trim_ascii();
    if text.is_empty() {
        return false;
    }
    base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(text)
        .is_ok_and(|raw| {
            raw.len() > MAGIC.len() + 1 + NONCE_LEN
                && raw[..MAGIC.len()] == MAGIC[..]
                && raw[MAGIC.len()] == VERSION
        })
}

// ------------------------------------------------------------------ assessing

/// What a challenge request looks like.
#[derive(Debug, Default, Clone)]
pub struct Signals {
    /// Challenges this address asked for in the last [`WINDOW`], this one
    /// included.
    pub recent: usize,
    /// Whether it sent `Accept-Language`, as every browser does.
    pub accept_language: bool,
    /// Its `User-Agent`, if any.
    pub user_agent: Option<String>,
    /// Whether it sent `Sec-Fetch-Site`, as every current browser does.
    pub sec_fetch: bool,
    /// What the page saw of its browser ([`CLIENT_WEBDRIVER`], …).
    pub client: u32,
    /// Whether the address asked the honeypot within [`FLAGGED_FOR`].
    pub flagged: bool,
}

/// What a request gets.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Verdict {
    /// Bits over [`BASE_DIFFICULTY`], at most [`MAX_EXTRA`].
    pub extra: u32,
    /// Whether the answer is the decoy.
    pub decoy: bool,
}

/// User agents that say what they are: tools, libraries, crawlers.
const ROBOT_MARKERS: [&str; 16] = [
    "bot",
    "crawl",
    "spider",
    "curl",
    "wget",
    "python",
    "go-http",
    "java/",
    "okhttp",
    "libwww",
    "scrapy",
    "httpclient",
    "phantomjs",
    "selenium",
    "puppeteer",
    "playwright",
];

/// Points per signal, and the verdict. A request from a browser a person
/// uses scores 0; every signal only ever adds difficulty, and only
/// [`DECOY_POINTS`] with two strong signals (a headless or tool user agent,
/// `navigator.webdriver`), or the honeypot, give the decoy.
#[must_use]
pub fn assess(s: &Signals) -> Verdict {
    if s.flagged {
        return Verdict {
            extra: MAX_EXTRA,
            decoy: true,
        };
    }
    let mut points = 0;
    let mut strong = 0;
    points += match s.recent {
        0..=10 => 0,
        11..=30 => 1,
        31..=100 => 2,
        _ => 3,
    };
    if !s.accept_language {
        points += 1;
    }
    if !s.sec_fetch {
        points += 1;
    }
    let agent = s.user_agent.as_deref().unwrap_or("").to_ascii_lowercase();
    if agent.trim().is_empty() {
        points += 2;
    } else if agent.contains("headless") || ROBOT_MARKERS.iter().any(|m| agent.contains(m)) {
        points += 2;
        strong += 1;
    }
    if s.client & CLIENT_WEBDRIVER != 0 {
        points += 2;
        strong += 1;
    }
    if s.client & CLIENT_NO_LANGUAGES != 0 {
        points += 1;
    }
    if s.client & CLIENT_SOFTWARE_GL != 0 {
        points += 1;
    }
    Verdict {
        extra: points.min(MAX_EXTRA),
        decoy: points >= DECOY_POINTS && strong >= 2,
    }
}

// ------------------------------------------------------------------ memory

/// Requests, marks and answered puzzles, in memory only.
pub struct Watch {
    by_address: HashMap<String, VecDeque<OffsetDateTime>>,
    flagged: HashMap<String, OffsetDateTime>,
    used: HashMap<[u8; NONCE_LEN], OffsetDateTime>,
    honeypot_hits: u64,
    /// Seeds the decoys when no key is configured (the honeypot answers
    /// regardless): random per process.
    salt: [u8; 32],
}

impl Default for Watch {
    fn default() -> Self {
        let mut salt = [0_u8; 32];
        getrandom::fill(&mut salt).expect("OS RNG available");
        Self {
            by_address: HashMap::new(),
            flagged: HashMap::new(),
            used: HashMap::new(),
            honeypot_hits: 0,
            salt,
        }
    }
}

/// The key an address is counted and marked under: an IPv6 address by its
/// /64, which one subscriber usually holds whole.
#[must_use]
pub fn address_key(address: &str) -> String {
    match address.parse::<IpAddr>() {
        Ok(IpAddr::V6(v6)) => {
            let s = v6.segments();
            format!("{:x}:{:x}:{:x}:{:x}::/64", s[0], s[1], s[2], s[3])
        }
        _ => address.to_owned(),
    }
}

impl Watch {
    /// Counts a challenge from `address` at `now`; how many it asked for in
    /// the last [`WINDOW`], this one included. Never refuses.
    pub fn count(&mut self, address: &str, now: OffsetDateTime) -> usize {
        if self.by_address.len() >= SWEEP_AT {
            self.by_address
                .retain(|_, times| times.back().is_some_and(|t| now - *t < WINDOW));
        }
        let times = self.by_address.entry(address_key(address)).or_default();
        while times.front().is_some_and(|t| now - *t >= WINDOW) {
            times.pop_front();
        }
        // A thousand is as suspicious as a hundred; the queue stays small.
        if times.len() < 1000 {
            times.push_back(now);
        }
        times.len()
    }

    /// Marks `address` as one that asked the honeypot, until
    /// [`FLAGGED_FOR`] from `now`; the hits so far.
    pub fn flag(&mut self, address: &str, now: OffsetDateTime) -> u64 {
        if self.flagged.len() >= SWEEP_AT {
            self.flagged.retain(|_, until| *until > now);
        }
        self.flagged.insert(address_key(address), now + FLAGGED_FOR);
        self.honeypot_hits += 1;
        self.honeypot_hits
    }

    /// Whether `address` asked the honeypot within [`FLAGGED_FOR`].
    #[must_use]
    pub fn is_flagged(&self, address: &str, now: OffsetDateTime) -> bool {
        self.flagged
            .get(&address_key(address))
            .is_some_and(|until| *until > now)
    }

    /// Takes the puzzle `nonce`, valid until `expires`: `false` if it was
    /// answered before.
    pub fn take(
        &mut self,
        nonce: [u8; NONCE_LEN],
        expires: OffsetDateTime,
        now: OffsetDateTime,
    ) -> bool {
        if self.used.len() >= SWEEP_AT {
            self.used.retain(|_, until| *until > now);
        }
        if self.used.len() >= MAX_USED {
            // Only under a flood of solved puzzles: forget the one closest
            // to expiring rather than refuse anybody.
            if let Some(oldest) = self.used.iter().min_by_key(|(_, t)| **t).map(|(n, _)| *n) {
                self.used.remove(&oldest);
            }
        }
        if self.used.contains_key(&nonce) {
            return false;
        }
        self.used.insert(nonce, expires);
        true
    }
}

// ------------------------------------------------------------------ puzzles

/// A challenge as the page gets it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Challenge {
    /// 16 random bytes, base64url.
    pub nonce: String,
    /// How many parts ([`PARTS`]).
    pub parts: u32,
    /// Zero bits each part needs.
    pub difficulty: u32,
    /// Unix seconds after which it is not taken.
    pub expires: i64,
    /// HMAC-SHA256 over the rest and what the answer will be, base64url.
    pub sig: String,
}

/// A solved challenge as the page sends it.
#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Solution {
    /// The challenge's nonce.
    pub nonce: String,
    /// The challenge's parts.
    pub parts: u32,
    /// The challenge's difficulty.
    pub difficulty: u32,
    /// The challenge's expiry.
    pub expires: i64,
    /// The challenge's signature.
    pub sig: String,
    /// One counter per part.
    pub counters: Vec<u64>,
}

fn signed(
    key: &[u8; 32],
    nonce: &str,
    parts: u32,
    difficulty: u32,
    expires: i64,
    decoy: bool,
) -> [u8; 32] {
    let mode = if decoy { "decoy" } else { "real" };
    crate::direct::hmac_sha256(
        key,
        format!("impressum\0{nonce}\0{parts}\0{difficulty}\0{expires}\0{mode}").as_bytes(),
    )
}

/// A new challenge under `key` for `verdict`, at `now`.
///
/// # Panics
///
/// When the operating system has no random numbers to give.
#[must_use]
pub fn challenge(key: &[u8; 32], verdict: Verdict, now: OffsetDateTime) -> Challenge {
    let mut raw = [0_u8; NONCE_LEN];
    getrandom::fill(&mut raw).expect("OS RNG available");
    let nonce = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw);
    let difficulty = BASE_DIFFICULTY + verdict.extra.min(MAX_EXTRA);
    let expires = (now + LIFETIME).unix_timestamp();
    let sig = signed(key, &nonce, PARTS, difficulty, expires, verdict.decoy);
    Challenge {
        nonce,
        parts: PARTS,
        difficulty,
        expires,
        sig: base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(sig),
    }
}

/// Why a solution is not taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Rejected {
    /// Not a challenge this service signed, or changed since.
    Forged,
    /// Past its expiry.
    Expired,
    /// Answered before.
    Used,
    /// A counter that does not solve its part, or the wrong number of them.
    Wrong,
}

/// Whether `hash` starts with `bits` zero bits.
#[must_use]
pub fn leading_zeros(hash: &[u8], bits: u32) -> bool {
    let mut left = bits;
    for byte in hash {
        if left == 0 {
            return true;
        }
        if left >= 8 {
            if *byte != 0 {
                return false;
            }
            left -= 8;
        } else {
            return byte >> (8 - left) == 0;
        }
    }
    left == 0
}

/// Whether `counter` solves part `part` of the puzzle `nonce`.
#[must_use]
pub fn solves(nonce: &str, part: u32, counter: u64, difficulty: u32) -> bool {
    leading_zeros(
        &Sha256::digest(format!("{nonce}:{part}:{counter}").as_bytes()),
        difficulty,
    )
}

/// Checks `solution` under `key` at `now` and, if it holds, takes its
/// nonce in `watch`; whether the answer is the decoy.
///
/// # Errors
///
/// [`Rejected`] says why not.
pub fn verify(
    key: &[u8; 32],
    solution: &Solution,
    watch: &mut Watch,
    now: OffsetDateTime,
) -> Result<bool, Rejected> {
    let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD;
    let nonce: [u8; NONCE_LEN] = b64
        .decode(&solution.nonce)
        .ok()
        .and_then(|n| n.try_into().ok())
        .ok_or(Rejected::Forged)?;
    let sig = b64.decode(&solution.sig).map_err(|_| Rejected::Forged)?;
    let mode = [false, true].into_iter().find(|decoy| {
        let want = signed(
            key,
            &solution.nonce,
            solution.parts,
            solution.difficulty,
            solution.expires,
            *decoy,
        );
        bool::from(want.as_slice().ct_eq(sig.as_slice()))
    });
    let decoy = mode.ok_or(Rejected::Forged)?;
    let expires =
        OffsetDateTime::from_unix_timestamp(solution.expires).map_err(|_| Rejected::Forged)?;
    if now > expires {
        return Err(Rejected::Expired);
    }
    let parts = usize::try_from(solution.parts).map_err(|_| Rejected::Wrong)?;
    if solution.counters.len() != parts {
        return Err(Rejected::Wrong);
    }
    let all = solution
        .counters
        .iter()
        .zip(0..)
        .all(|(c, part)| solves(&solution.nonce, part, *c, solution.difficulty));
    if !all {
        return Err(Rejected::Wrong);
    }
    if !watch.take(nonce, expires, now) {
        return Err(Rejected::Used);
    }
    Ok(decoy)
}

// ------------------------------------------------------------------ the decoy

/// Invented details, the same for one seed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decoy {
    /// A common first and last name.
    pub name: String,
    /// A common street name and a number.
    pub street: String,
    /// A city's postcode area and two random digits.
    pub postcode: String,
    /// The city.
    pub city: String,
    /// At `example.com`, `.net` or `.org` (RFC 2606): no mailbox anywhere.
    pub email: String,
    /// The city's area code and a number starting with 0, which no German
    /// subscriber number does: it reaches nobody.
    pub phone: String,
}

const FIRST: [&str; 16] = [
    "Thomas",
    "Andreas",
    "Michael",
    "Stefan",
    "Markus",
    "Christian",
    "Daniel",
    "Martin",
    "Sabine",
    "Claudia",
    "Andrea",
    "Petra",
    "Julia",
    "Katrin",
    "Nicole",
    "Susanne",
];
const LAST: [&str; 16] = [
    "Schneider",
    "Fischer",
    "Weber",
    "Wagner",
    "Becker",
    "Schulz",
    "Hoffmann",
    "Koch",
    "Richter",
    "Klein",
    "Wolf",
    "Neumann",
    "Schwarz",
    "Zimmermann",
    "Braun",
    "Hartmann",
];
const STREETS: [&str; 12] = [
    "Lindenstraße",
    "Gartenweg",
    "Bahnhofstraße",
    "Schulstraße",
    "Ahornweg",
    "Birkenallee",
    "Mühlenweg",
    "Kirchplatz",
    "Am Anger",
    "Feldstraße",
    "Rosenweg",
    "Waldstraße",
];
/// City, its postcode area (three digits), its area code without the 0.
const CITIES: [(&str, &str, &str); 10] = [
    ("Bielefeld", "336", "521"),
    ("Kassel", "341", "561"),
    ("Würzburg", "970", "931"),
    ("Osnabrück", "490", "541"),
    ("Regensburg", "930", "941"),
    ("Göttingen", "370", "551"),
    ("Paderborn", "331", "5251"),
    ("Ulm", "890", "731"),
    ("Trier", "542", "651"),
    ("Erfurt", "990", "361"),
];
const DOMAINS: [&str; 3] = ["example.com", "example.net", "example.org"];

/// The invented details for `seed`.
#[must_use]
pub fn decoy(seed: &[u8; 32]) -> Decoy {
    let pick = |i: usize, n: usize| usize::from(seed[i]) % n;
    let first = FIRST[pick(0, FIRST.len())];
    let last = LAST[pick(1, LAST.len())];
    let (city, area, dial) = CITIES[pick(2, CITIES.len())];
    let number = 1 + usize::from(seed[3]) % 120;
    Decoy {
        name: format!("{first} {last}"),
        street: format!("{} {number}", STREETS[pick(4, STREETS.len())]),
        postcode: format!("{area}{:02}", seed[5] % 100),
        city: city.to_owned(),
        email: format!(
            "{}.{}@{}",
            first.to_lowercase(),
            last.to_lowercase(),
            DOMAINS[pick(6, DOMAINS.len())]
        ),
        phone: format!(
            "+49 {dial} 0{:03} {:04}",
            u16::from_be_bytes([seed[7], seed[8]]) % 1000,
            u16::from_be_bytes([seed[9], seed[10]]) % 10_000
        ),
    }
}

fn keystream(nonce: &[u8], length: usize) -> Vec<u8> {
    let key: Vec<u8> = (0..SCRIPT_KEY.len() / 2)
        .map(|i| u8::from_str_radix(&SCRIPT_KEY[i * 2..i * 2 + 2], 16).expect("hex constant"))
        .collect();
    let mut out = Vec::with_capacity(length + 32);
    let mut counter: u32 = 0;
    while out.len() < length {
        out.extend_from_slice(
            &Sha256::new()
                .chain_update(&key)
                .chain_update(nonce)
                .chain_update(counter.to_be_bytes())
                .finalize(),
        );
        counter += 1;
    }
    out.truncate(length);
    out
}

/// `0..n`, uniformly, from the OS's generator.
fn below(n: usize) -> usize {
    let mut b = [0_u8; 8];
    getrandom::fill(&mut b).expect("OS RNG available");
    usize::try_from(u64::from_le_bytes(b) % n as u64).unwrap_or(0)
}

fn scattered(value: &str) -> Vec<[u32; 2]> {
    let mut pairs: Vec<[u32; 2]> = value
        .chars()
        .zip(0_u32..)
        .map(|(c, i)| [i, u32::from(c)])
        .collect();
    for i in (1..pairs.len()).rev() {
        pairs.swap(i, below(i + 1));
    }
    pairs
}

/// The decoy as a blob of the real one's shape (`impressum_blob.py`).
///
/// # Panics
///
/// When the operating system has no random numbers to give.
#[must_use]
pub fn decoy_blob(d: &Decoy) -> String {
    let fields = serde_json::json!({
        "name": scattered(&d.name),
        "street": scattered(&d.street),
        "postcode": scattered(&d.postcode),
        "city": scattered(&d.city),
        "country_de": scattered("Deutschland"),
        "country_en": scattered("Germany"),
        "email": scattered(&d.email),
        "phone": scattered(&d.phone),
    });
    let plain = serde_json::to_vec(&serde_json::json!({ "v": VERSION, "fields": fields }))
        .expect("JSON of numbers");
    let mut nonce = [0_u8; NONCE_LEN];
    getrandom::fill(&mut nonce).expect("OS RNG available");
    let mut raw = Vec::with_capacity(MAGIC.len() + 1 + NONCE_LEN + plain.len());
    raw.extend_from_slice(MAGIC);
    raw.push(VERSION);
    raw.extend_from_slice(&nonce);
    raw.extend(
        plain
            .iter()
            .zip(keystream(&nonce, plain.len()))
            .map(|(p, k)| p ^ k),
    );
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(raw)
}

// ------------------------------------------------------------------ routes

/// What the page reported in a challenge's query (`s=<bits>`); 0 when
/// it says nothing or nothing readable.
#[must_use]
pub fn client_bits(query: Option<&str>) -> u32 {
    query
        .unwrap_or("")
        .split('&')
        .find_map(|pair| pair.strip_prefix("s="))
        .and_then(|v| v.parse::<u32>().ok())
        .map_or(0, |bits| {
            bits & (CLIENT_WEBDRIVER | CLIENT_NO_LANGUAGES | CLIENT_SOFTWARE_GL)
        })
}

/// Whether a request may be answered at all, and the origin to name in
/// the answer: a fetch from the page's own origin (through the site's
/// proxy), or from one of the contact form's origins (CORS). A request
/// from another site is refused; one that says nothing about where it
/// comes from is answered (an older browser), the assessment weighs that.
fn origin_check(shared: &Shared, headers: &HeaderMap) -> Result<Option<String>, ()> {
    let site = headers
        .get("sec-fetch-site")
        .and_then(|v| v.to_str().ok())
        .map(str::to_ascii_lowercase);
    let origin = headers.get(header::ORIGIN).and_then(|v| v.to_str().ok());
    let allowed = origin.filter(|o| shared.state.config.contact_origin(o));
    match (site.as_deref(), origin, allowed) {
        (_, _, Some(o)) => Ok(Some(o.to_owned())),
        (Some("same-origin"), _, None) => Ok(None),
        (Some("same-site" | "cross-site"), _, None) | (_, Some(_), None) => Err(()),
        _ => Ok(None),
    }
}

fn with_cors(mut response: Response, origin: Option<&str>) -> Response {
    let headers = response.headers_mut();
    headers.insert(header::VARY, HeaderValue::from_static("Origin"));
    headers.insert(
        "x-robots-tag",
        HeaderValue::from_static("noindex, noarchive, nosnippet"),
    );
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_static("no-store"));
    if let Some(value) = origin.and_then(|o| HeaderValue::from_str(o).ok()) {
        headers.insert(header::ACCESS_CONTROL_ALLOW_ORIGIN, value);
    }
    response
}

fn peer_address(
    shared: &Shared,
    peer: Option<axum::Extension<ConnectInfo<std::net::SocketAddr>>>,
    headers: &HeaderMap,
) -> String {
    shared
        .ui
        .address(peer.map(|axum::Extension(ConnectInfo(a))| a.ip()), headers)
}

/// `OPTIONS /site/impressum`: the preflight of the page's `POST` from a
/// contact origin.
pub(crate) async fn preflight(State(shared): State<Shared>, headers: HeaderMap) -> Response {
    crate::contact::preflight(State(shared), headers).await
}

/// `GET /site/impressum/challenge`.
pub(crate) async fn get_challenge(
    State(shared): State<Shared>,
    peer: Option<axum::Extension<ConnectInfo<std::net::SocketAddr>>>,
    headers: HeaderMap,
    RawQuery(query): RawQuery,
) -> Response {
    let Ok(origin) = origin_check(&shared, &headers) else {
        return with_cors(
            refuse(StatusCode::FORBIDDEN, "not from the Impressum page").into_response(),
            None,
        );
    };
    let Some(settings) = shared.state.config.impressum() else {
        return with_cors(
            refuse(StatusCode::SERVICE_UNAVAILABLE, "no Impressum here").into_response(),
            origin.as_deref(),
        );
    };
    let address = peer_address(&shared, peer, &headers);
    let now = shared.ui.clock_now();
    let (recent, flagged) = shared
        .ui
        .impressum(|w| (w.count(&address, now), w.is_flagged(&address, now)));
    let has = |name: &str| headers.get(name).is_some_and(|v| !v.is_empty());
    let signals = Signals {
        recent,
        accept_language: has("accept-language"),
        user_agent: headers
            .get(header::USER_AGENT)
            .and_then(|v| v.to_str().ok())
            .map(str::to_owned),
        sec_fetch: has("sec-fetch-site"),
        client: client_bits(query.as_deref()),
        flagged,
    };
    let verdict = assess(&signals);
    let c = challenge(&settings.key, verdict, now);
    with_cors(Json(c).into_response(), origin.as_deref())
}

/// `POST /site/impressum`.
pub(crate) async fn post(
    State(shared): State<Shared>,
    peer: Option<axum::Extension<ConnectInfo<std::net::SocketAddr>>>,
    headers: HeaderMap,
    body: Result<axum::body::Bytes, axum::extract::rejection::BytesRejection>,
) -> Response {
    let Ok(origin) = origin_check(&shared, &headers) else {
        return with_cors(
            refuse(StatusCode::FORBIDDEN, "not from the Impressum page").into_response(),
            None,
        );
    };
    let answer = |r: Response| with_cors(r, origin.as_deref());
    let Some(settings) = shared.state.config.impressum() else {
        return answer(
            refuse(StatusCode::SERVICE_UNAVAILABLE, "no Impressum here").into_response(),
        );
    };
    let Some(solution) = body
        .ok()
        .filter(|b| b.len() <= 4096)
        .and_then(|b| serde_json::from_slice::<Solution>(&b).ok())
    else {
        return answer(refuse(StatusCode::BAD_REQUEST, "not a solution").into_response());
    };
    let now = shared.ui.clock_now();
    let decoy = match shared
        .ui
        .impressum(|w| verify(&settings.key, &solution, w, now))
    {
        Ok(decoy) => decoy,
        Err(Rejected::Forged) => {
            return answer(
                refuse(StatusCode::FORBIDDEN, "not a challenge from here").into_response(),
            );
        }
        Err(Rejected::Expired) => {
            return answer(refuse(StatusCode::GONE, "the challenge expired").into_response());
        }
        Err(Rejected::Used) => {
            return answer(
                refuse(StatusCode::CONFLICT, "the challenge was answered").into_response(),
            );
        }
        Err(Rejected::Wrong) => {
            return answer(refuse(StatusCode::BAD_REQUEST, "not solved").into_response());
        }
    };
    let blob = if decoy {
        let address = peer_address(&shared, peer, &headers);
        decoy_blob(&self::decoy(&seed(&settings.key, &address)))
    } else {
        match std::fs::read(&settings.blob) {
            Ok(text) if looks_like_blob(&text) => {
                String::from_utf8_lossy(text.trim_ascii()).into_owned()
            }
            _ => {
                tracing::warn!("the Impressum blob is missing or not a blob");
                return answer(
                    refuse(StatusCode::SERVICE_UNAVAILABLE, "the Impressum is not here")
                        .into_response(),
                );
            }
        }
    };
    let mut response = blob.into_response();
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/plain; charset=us-ascii"),
    );
    answer(response)
}

fn seed(key: &[u8; 32], address: &str) -> [u8; 32] {
    crate::direct::hmac_sha256(key, format!("decoy\0{}", address_key(address)).as_bytes())
}

/// `GET /site/impressum/kontakt`: the honeypot.
pub(crate) async fn honeypot(
    State(shared): State<Shared>,
    peer: Option<axum::Extension<ConnectInfo<std::net::SocketAddr>>>,
    headers: HeaderMap,
) -> Response {
    let address = peer_address(&shared, peer, &headers);
    let now = shared.ui.clock_now();
    let (hits, salt) = shared.ui.impressum(|w| (w.flag(&address, now), w.salt));
    tracing::info!(hits, "the Impressum honeypot was asked");
    let key = shared.state.config.impressum().map_or(salt, |s| s.key);
    let d = decoy(&seed(&key, &address));
    let page = format!(
        "<!doctype html>\n<html lang=\"de\"><head><meta charset=\"utf-8\">\
         <meta name=\"robots\" content=\"noindex, noarchive, nosnippet\">\
         <title>Impressum</title></head><body>\n<h1>Impressum</h1>\n\
         <h2>Angaben gemäß § 5 DDG</h2>\n<p>{}<br>\n{}<br>\n{} {}<br>\nDeutschland</p>\n\
         <h2>Kontakt</h2>\n<p>E-Mail: {}<br>\nTelefon: {}</p>\n</body></html>\n",
        d.name, d.street, d.postcode, d.city, d.email, d.phone
    );
    let mut response = page.into_response();
    let h = response.headers_mut();
    h.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    h.insert(
        "x-robots-tag",
        HeaderValue::from_static("noindex, noarchive, nosnippet"),
    );
    response
}
