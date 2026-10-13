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

/// User agents that say what they are: tools, libraries, crawlers
/// (`Googlebot/2.1`, `bingbot/2.0`; a crawler's contact link `+http`).
/// "bot" alone would also be a phone maker's name.
const ROBOT_MARKERS: [&str; 17] = [
    "bot/",
    "+http",
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

#[cfg(test)]
mod tests {
    use super::*;

    const FIREFOX: &str = "Mozilla/5.0 (X11; Linux x86_64; rv:130.0) Gecko/20100101 Firefox/130.0";
    const HEADLESS: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like \
                            Gecko) HeadlessChrome/130.0.0.0 Safari/537.36";

    const KEY: [u8; 32] = [7; 32];

    fn at() -> OffsetDateTime {
        OffsetDateTime::from_unix_timestamp(1_800_000_000).unwrap()
    }

    fn b64(bytes: &[u8]) -> String {
        base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
    }

    // ------------------------------------------------------------ leading_zeros

    #[test]
    fn leading_zeros_counts_bits_on_both_sides_of_each_boundary() {
        // 0 bits: always, even for an empty hash.
        assert!(leading_zeros(&[], 0));
        assert!(leading_zeros(&[0xff], 0));
        // 1 bit.
        assert!(leading_zeros(&[0x7f], 1));
        assert!(!leading_zeros(&[0x80], 1));
        // 7 bits.
        assert!(leading_zeros(&[0x01], 7));
        assert!(!leading_zeros(&[0x02], 7));
        assert!(!leading_zeros(&[0x80], 7));
        // 8 bits: the whole first byte.
        assert!(leading_zeros(&[0x00, 0xff], 8));
        assert!(!leading_zeros(&[0x01, 0x00], 8));
        // 9 bits: the first byte and the top bit of the second.
        assert!(leading_zeros(&[0x00, 0x7f], 9));
        assert!(!leading_zeros(&[0x00, 0x80], 9));
        assert!(!leading_zeros(&[0x01, 0x00], 9));
        // 16 bits.
        assert!(leading_zeros(&[0x00, 0x00, 0xff], 16));
        assert!(!leading_zeros(&[0x00, 0x01, 0x00], 16));
        assert!(!leading_zeros(&[0x01, 0x00, 0x00], 16));
        // 17 bits, the base difficulty.
        assert!(leading_zeros(&[0x00, 0x00, 0x7f], 17));
        assert!(!leading_zeros(&[0x00, 0x00, 0x80], 17));
    }

    #[test]
    fn leading_zeros_of_a_hash_shorter_than_the_bits_asked_is_false() {
        assert!(leading_zeros(&[0x00], 8));
        assert!(!leading_zeros(&[0x00], 9));
        assert!(!leading_zeros(&[], 1));
        // all zero 32-byte hash holds every difficulty up to 256 bits
        assert!(leading_zeros(&[0; 32], 256));
        assert!(!leading_zeros(&[0; 32], 257));
    }

    // ------------------------------------------------------------------ solves

    /// Counters computed with Python's hashlib:
    /// `sha256(f"{nonce}:{part}:{c}")` starting with 8 zero bits.
    #[test]
    fn solves_known_vector_from_hashlib() {
        let nonce = "AAAAAAAAAAAAAAAAAAAAAA";
        let smallest_at_8 = [218_u64, 8, 247];
        for (part, counter) in (0..).zip(smallest_at_8) {
            assert!(solves(nonce, part, counter, 8), "part {part}");
            // The smallest: no counter below it solves.
            for below in 0..counter {
                assert!(!solves(nonce, part, below, 8), "part {part}, {below}");
            }
        }
        // A counter solves its own part only: 218 is not part 1's answer.
        assert!(!solves(nonce, 1, 218, 8));
        assert!(!solves(nonce, 0, 8, 8));
        // The first digits in hex, from hashlib: the leading byte is 00.
        assert_eq!(
            &Sha256::digest(b"AAAAAAAAAAAAAAAAAAAAAA:0:218")[..4],
            &[0x00, 0xf1, 0xe0, 0xb9]
        );
    }

    #[test]
    fn solves_reads_bits_not_bytes() {
        let nonce = "AAAAAAAAAAAAAAAAAAAAAA";
        // At 9 bits part 0's answer moves from 218 to 961 (hashlib), at 4 it
        // is 4, and part 2's 247 holds at 9 as well.
        assert!(!solves(nonce, 0, 218, 9));
        assert!(solves(nonce, 0, 961, 9));
        assert!(solves(nonce, 0, 4, 4));
        assert!(solves(nonce, 2, 247, 9));
        assert!(solves(nonce, 1, 1478, 9));
        // The part number is part of the preimage.
        assert!(!solves(nonce, 1, 961, 9));
    }

    #[test]
    fn solves_with_difficulty_zero_takes_any_counter() {
        assert!(solves("x", 0, 0, 0));
        assert!(solves("x", 7, u64::MAX, 0));
    }

    // ------------------------------------------------------------------ assess

    fn browser() -> Signals {
        Signals {
            recent: 1,
            accept_language: true,
            user_agent: Some(FIREFOX.to_owned()),
            sec_fetch: true,
            client: 0,
            flagged: false,
        }
    }

    #[test]
    fn an_ordinary_browser_gets_the_base_puzzle_and_the_real_answer() {
        assert_eq!(
            assess(&browser()),
            Verdict {
                extra: 0,
                decoy: false
            }
        );
    }

    #[test]
    #[allow(clippy::too_many_lines)]
    fn each_signal_alone_raises_the_difficulty_and_never_gives_the_decoy() {
        let alone: Vec<(&str, Signals, u32)> = vec![
            (
                "11 requests",
                Signals {
                    recent: 11,
                    ..browser()
                },
                1,
            ),
            (
                "31 requests",
                Signals {
                    recent: 31,
                    ..browser()
                },
                2,
            ),
            (
                "101 requests",
                Signals {
                    recent: 101,
                    ..browser()
                },
                3,
            ),
            (
                "5000 requests",
                Signals {
                    recent: 5000,
                    ..browser()
                },
                3,
            ),
            (
                "no accept-language",
                Signals {
                    accept_language: false,
                    ..browser()
                },
                1,
            ),
            (
                "no sec-fetch",
                Signals {
                    sec_fetch: false,
                    ..browser()
                },
                1,
            ),
            (
                "no user agent",
                Signals {
                    user_agent: None,
                    ..browser()
                },
                2,
            ),
            (
                "blank user agent",
                Signals {
                    user_agent: Some("  ".to_owned()),
                    ..browser()
                },
                2,
            ),
            (
                "headless user agent",
                Signals {
                    user_agent: Some(HEADLESS.to_owned()),
                    ..browser()
                },
                2,
            ),
            (
                "curl",
                Signals {
                    user_agent: Some("curl/8.4.0".to_owned()),
                    ..browser()
                },
                2,
            ),
            (
                "a crawler",
                Signals {
                    user_agent: Some("Mozilla/5.0 (compatible; ExampleBot/2.1)".to_owned()),
                    ..browser()
                },
                2,
            ),
            (
                "navigator.webdriver",
                Signals {
                    client: CLIENT_WEBDRIVER,
                    ..browser()
                },
                2,
            ),
            (
                "no navigator.languages",
                Signals {
                    client: CLIENT_NO_LANGUAGES,
                    ..browser()
                },
                1,
            ),
            (
                "a software renderer",
                Signals {
                    client: CLIENT_SOFTWARE_GL,
                    ..browser()
                },
                1,
            ),
        ];
        for (what, signals, extra) in alone {
            let verdict = assess(&signals);
            assert_eq!(verdict.extra, extra, "{what}");
            assert!(!verdict.decoy, "{what} alone never gives the decoy");
        }
    }

    #[test]
    fn the_request_count_steps_at_ten_thirty_and_a_hundred() {
        for (recent, extra) in [
            (0, 0),
            (1, 0),
            (10, 0),
            (11, 1),
            (30, 1),
            (31, 2),
            (100, 2),
            (101, 3),
            (usize::MAX, 3),
        ] {
            let verdict = assess(&Signals {
                recent,
                ..browser()
            });
            assert_eq!(verdict.extra, extra, "{recent} requests");
        }
    }

    #[test]
    fn points_cap_at_the_most_extra_bits() {
        // 3 (requests) + 1 + 1 + 2 + 1 + 1 = 9 points, one strong signal.
        let verdict = assess(&Signals {
            recent: 5000,
            accept_language: false,
            user_agent: None,
            sec_fetch: false,
            client: CLIENT_NO_LANGUAGES | CLIENT_SOFTWARE_GL,
            flagged: false,
        });
        assert_eq!(verdict.extra, MAX_EXTRA);
        assert!(!verdict.decoy, "no strong signal at all");
        // Plenty of points and one strong signal (the user agent) still is no
        // robot beyond doubt.
        let verdict = assess(&Signals {
            recent: 5000,
            accept_language: false,
            user_agent: Some(HEADLESS.to_owned()),
            sec_fetch: false,
            client: CLIENT_NO_LANGUAGES | CLIENT_SOFTWARE_GL,
            flagged: false,
        });
        assert_eq!(verdict.extra, MAX_EXTRA);
        assert!(!verdict.decoy, "one strong signal is not two");
        // Nor is the other one alone.
        let verdict = assess(&Signals {
            recent: 5000,
            accept_language: false,
            user_agent: Some(FIREFOX.to_owned()),
            sec_fetch: false,
            client: CLIENT_WEBDRIVER | CLIENT_NO_LANGUAGES | CLIENT_SOFTWARE_GL,
            flagged: false,
        });
        assert_eq!(verdict.extra, MAX_EXTRA);
        assert!(!verdict.decoy, "webdriver alone is not two strong signals");
    }

    #[test]
    fn a_headless_browser_driven_by_webdriver_gets_the_decoy_at_six_points() {
        // Headless (2) + webdriver (2) + no accept-language (1) + no
        // sec-fetch (1) = 6 = DECOY_POINTS.
        let robot = Signals {
            recent: 1,
            accept_language: false,
            user_agent: Some(HEADLESS.to_owned()),
            sec_fetch: false,
            client: CLIENT_WEBDRIVER,
            flagged: false,
        };
        assert_eq!(DECOY_POINTS, 6);
        assert_eq!(
            assess(&robot),
            Verdict {
                extra: MAX_EXTRA,
                decoy: true
            }
        );
        // One point fewer (it sends sec-fetch-site): two strong signals but
        // only five points, so the real answer after a hard puzzle.
        let verdict = assess(&Signals {
            sec_fetch: true,
            ..robot.clone()
        });
        assert_eq!(
            verdict,
            Verdict {
                extra: MAX_EXTRA,
                decoy: false
            }
        );
        // And the other way: six points with the same two strong signals
        // from other weak ones.
        let verdict = assess(&Signals {
            sec_fetch: true,
            accept_language: true,
            client: CLIENT_WEBDRIVER | CLIENT_NO_LANGUAGES | CLIENT_SOFTWARE_GL,
            ..robot
        });
        assert!(verdict.decoy, "2 + 2 + 1 + 1 points");
    }

    #[test]
    fn a_flagged_address_gets_the_hardest_puzzle_and_the_decoy_whatever_it_sends() {
        let verdict = assess(&Signals {
            flagged: true,
            ..browser()
        });
        assert_eq!(
            verdict,
            Verdict {
                extra: MAX_EXTRA,
                decoy: true
            }
        );
    }

    /// The owner's rule (13.10.2026): nobody is locked out. Whatever the
    /// request looks like, a challenge comes back, at most `MAX_EXTRA` bits
    /// over the base; the decoy needs the honeypot or two strong signals.
    #[test]
    fn never_lock_out_every_combination_gets_a_bounded_challenge() {
        let agents = [
            None,
            Some(FIREFOX),
            Some(HEADLESS),
            Some("curl/8.4.0"),
            Some(""),
        ];
        let mut seen = 0;
        for accept_language in [false, true] {
            for sec_fetch in [false, true] {
                for flagged in [false, true] {
                    for agent in agents {
                        for client in 0..8 {
                            for recent in [1, 50, 5000] {
                                let signals = Signals {
                                    recent,
                                    accept_language,
                                    user_agent: agent.map(str::to_owned),
                                    sec_fetch,
                                    client,
                                    flagged,
                                };
                                let verdict = assess(&signals);
                                let what = format!("{signals:?}");
                                assert!(verdict.extra <= MAX_EXTRA, "{what}");
                                if verdict.decoy {
                                    let strong_agent = agent.is_some_and(|a| {
                                        a.contains("Headless") || a.contains("curl")
                                    });
                                    assert!(
                                        flagged || (strong_agent && client & CLIENT_WEBDRIVER != 0),
                                        "a decoy without cause: {what}"
                                    );
                                }
                                let c = challenge(&KEY, verdict, at());
                                assert!(c.difficulty >= BASE_DIFFICULTY, "{what}");
                                assert!(c.difficulty <= BASE_DIFFICULTY + MAX_EXTRA, "{what}");
                                assert_eq!(c.parts, PARTS, "{what}");
                                seen += 1;
                            }
                        }
                    }
                }
            }
        }
        assert_eq!(seen, 2 * 2 * 2 * 5 * 8 * 3);
    }

    // ------------------------------------------------------- challenge, verify

    /// The smallest counters for every part, by brute force: cheap at a few
    /// bits.
    fn solve(nonce: &str, parts: u32, difficulty: u32) -> Vec<u64> {
        (0..parts)
            .map(|part| {
                (0..u64::MAX)
                    .find(|c| solves(nonce, part, *c, difficulty))
                    .unwrap()
            })
            .collect()
    }

    /// A solved puzzle of 4 bits, 2 parts, signed under `KEY`, expiring five
    /// minutes after [`at`]; `decoy` picks the mode it was signed in.
    fn solution(nonce_byte: u8, decoy: bool) -> Solution {
        let nonce = b64(&[nonce_byte; NONCE_LEN]);
        let (parts, difficulty) = (2, 4);
        let expires = (at() + LIFETIME).unix_timestamp();
        let sig = signed(&KEY, &nonce, parts, difficulty, expires, decoy);
        Solution {
            counters: solve(&nonce, parts, difficulty),
            nonce,
            parts,
            difficulty,
            expires,
            sig: b64(&sig),
        }
    }

    /// A counter that does not solve `part`.
    fn failing(solution: &Solution, part: u32) -> u64 {
        (0..u64::MAX)
            .find(|c| !solves(&solution.nonce, part, *c, solution.difficulty))
            .unwrap()
    }

    #[test]
    fn a_solved_puzzle_is_taken_once() {
        let mut watch = Watch::default();
        let s = solution(1, false);
        assert_eq!(verify(&KEY, &s, &mut watch, at()), Ok(false));
        assert_eq!(verify(&KEY, &s, &mut watch, at()), Err(Rejected::Used));
        // Still used a moment later, and with another watch it is new.
        assert_eq!(
            verify(&KEY, &s, &mut watch, at() + Duration::minutes(1)),
            Err(Rejected::Used)
        );
        assert_eq!(verify(&KEY, &s, &mut Watch::default(), at()), Ok(false));
    }

    #[test]
    fn a_decoy_signed_puzzle_verifies_as_the_decoy() {
        let mut watch = Watch::default();
        let s = solution(2, true);
        assert_eq!(verify(&KEY, &s, &mut watch, at()), Ok(true));
        assert_eq!(verify(&KEY, &s, &mut watch, at()), Err(Rejected::Used));
    }

    #[test]
    fn two_different_nonces_are_two_puzzles() {
        let mut watch = Watch::default();
        assert_eq!(
            verify(&KEY, &solution(3, false), &mut watch, at()),
            Ok(false)
        );
        assert_eq!(
            verify(&KEY, &solution(4, false), &mut watch, at()),
            Ok(false)
        );
    }

    #[test]
    fn a_changed_challenge_is_forged() {
        let good = solution(5, false);
        let mut watch = Watch::default();
        let mut fiddled: Vec<(&str, Solution)> = Vec::new();
        let mut s = good.clone();
        s.difficulty = 3;
        fiddled.push(("easier", s));
        let mut s = good.clone();
        s.parts = 1;
        s.counters.truncate(1);
        fiddled.push(("fewer parts", s));
        let mut s = good.clone();
        s.expires += 3600;
        fiddled.push(("longer life", s));
        let mut s = good.clone();
        s.nonce = b64(&[6; NONCE_LEN]);
        s.counters = solve(&s.nonce, s.parts, s.difficulty);
        fiddled.push(("another nonce under the old signature", s));
        let mut s = good.clone();
        let flipped = if s.sig.starts_with('A') { 'B' } else { 'A' };
        s.sig.replace_range(..1, &flipped.to_string());
        fiddled.push(("a flipped signature", s));
        let mut s = good.clone();
        s.sig = "not base64 !!".to_owned();
        fiddled.push(("a signature that is no base64", s));
        let mut s = good.clone();
        s.sig = String::new();
        fiddled.push(("no signature", s));
        let mut s = good.clone();
        s.nonce = "short".to_owned();
        fiddled.push(("a nonce of the wrong length", s));
        let mut s = good.clone();
        s.nonce = "!!!!".to_owned();
        fiddled.push(("a nonce that is no base64", s));
        for (what, s) in fiddled {
            assert_eq!(
                verify(&KEY, &s, &mut watch, at()),
                Err(Rejected::Forged),
                "{what}"
            );
        }
        // None of them used the real one up.
        assert_eq!(verify(&KEY, &good, &mut watch, at()), Ok(false));
    }

    #[test]
    fn another_key_signs_another_challenge() {
        let s = solution(7, false);
        assert_eq!(
            verify(&[8; 32], &s, &mut Watch::default(), at()),
            Err(Rejected::Forged)
        );
    }

    #[test]
    fn an_expired_puzzle_is_refused_and_not_used_up() {
        let s = solution(8, false);
        let mut watch = Watch::default();
        let expires = OffsetDateTime::from_unix_timestamp(s.expires).unwrap();
        assert_eq!(
            verify(&KEY, &s, &mut watch, expires + Duration::SECOND),
            Err(Rejected::Expired)
        );
        // Not consumed: before it expired, the very same answer still holds,
        // and at the second it expires (`now > expires` is false) as well.
        assert_eq!(verify(&KEY, &s, &mut watch, expires), Ok(false));
        // A fresh puzzle works after an expiry as before.
        assert_eq!(
            verify(
                &KEY,
                &solution(9, false),
                &mut watch,
                expires + Duration::DAY
            ),
            Err(Rejected::Expired)
        );
        assert_eq!(
            verify(&KEY, &solution(9, false), &mut watch, at()),
            Ok(false)
        );
    }

    #[test]
    fn a_wrong_answer_is_refused_and_does_not_use_the_nonce_up() {
        let good = solution(10, false);
        let mut watch = Watch::default();

        let mut wrong_counter = good.clone();
        wrong_counter.counters[1] = failing(&good, 1);
        assert_eq!(
            verify(&KEY, &wrong_counter, &mut watch, at()),
            Err(Rejected::Wrong)
        );
        let mut swapped = good.clone();
        // A counter of the right puzzle in the wrong place only holds by luck:
        // find the shape where it does not.
        swapped.counters[0] = failing(&good, 0);
        assert_eq!(
            verify(&KEY, &swapped, &mut watch, at()),
            Err(Rejected::Wrong)
        );
        let mut short = good.clone();
        short.counters.pop();
        assert_eq!(verify(&KEY, &short, &mut watch, at()), Err(Rejected::Wrong));
        let mut long = good.clone();
        long.counters.push(good.counters[0]);
        assert_eq!(verify(&KEY, &long, &mut watch, at()), Err(Rejected::Wrong));
        let mut none = good.clone();
        none.counters.clear();
        assert_eq!(verify(&KEY, &none, &mut watch, at()), Err(Rejected::Wrong));

        // None of those took the nonce: the right answer still goes through.
        assert_eq!(verify(&KEY, &good, &mut watch, at()), Ok(false));
        assert_eq!(verify(&KEY, &good, &mut watch, at()), Err(Rejected::Used));
    }

    #[test]
    fn a_signature_is_checked_before_the_counters_and_the_expiry_before_both() {
        let mut s = solution(11, false);
        s.counters = vec![];
        s.sig = b64(&[0; 32]);
        assert_eq!(
            verify(&KEY, &s, &mut Watch::default(), at()),
            Err(Rejected::Forged)
        );
        let mut s = solution(12, false);
        s.counters = vec![];
        let late = OffsetDateTime::from_unix_timestamp(s.expires).unwrap() + Duration::HOUR;
        assert_eq!(
            verify(&KEY, &s, &mut Watch::default(), late),
            Err(Rejected::Expired)
        );
    }

    #[test]
    fn a_challenge_carries_a_signature_verify_accepts() {
        for (verdict, decoy) in [
            (
                Verdict {
                    extra: 0,
                    decoy: false,
                },
                false,
            ),
            (
                Verdict {
                    extra: MAX_EXTRA,
                    decoy: true,
                },
                true,
            ),
        ] {
            let c = challenge(&KEY, verdict, at());
            assert_eq!(c.parts, PARTS);
            assert_eq!(c.difficulty, BASE_DIFFICULTY + verdict.extra);
            assert_eq!(c.expires, (at() + LIFETIME).unix_timestamp());
            let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
                .decode(&c.nonce)
                .unwrap();
            assert_eq!(raw.len(), NONCE_LEN);
            assert_eq!(c.nonce, b64(&raw), "canonical base64url without padding");
            let want = signed(&KEY, &c.nonce, c.parts, c.difficulty, c.expires, decoy);
            assert_eq!(c.sig, b64(&want));
            // The signature holds (the counters do not: Wrong, not Forged),
            // and that failure leaves the nonce free.
            let solution = Solution {
                nonce: c.nonce.clone(),
                parts: c.parts,
                difficulty: c.difficulty,
                expires: c.expires,
                sig: c.sig.clone(),
                counters: vec![0; PARTS as usize],
            };
            let mut watch = Watch::default();
            assert_eq!(
                verify(&KEY, &solution, &mut watch, at()),
                Err(Rejected::Wrong)
            );
            assert!(watch.used.is_empty());
        }
    }

    #[test]
    fn challenges_are_never_the_same_twice_and_cap_the_extra_bits() {
        let verdict = Verdict {
            extra: 0,
            decoy: false,
        };
        let a = challenge(&KEY, verdict, at());
        let b = challenge(&KEY, verdict, at());
        assert_ne!(a.nonce, b.nonce);
        assert_ne!(a.sig, b.sig);
        // A verdict beyond the cap is cut, not obeyed.
        let c = challenge(
            &KEY,
            Verdict {
                extra: 1000,
                decoy: false,
            },
            at(),
        );
        assert_eq!(c.difficulty, BASE_DIFFICULTY + MAX_EXTRA);
    }

    // ------------------------------------------------------------------- watch

    #[test]
    fn requests_are_counted_per_address_within_the_window() {
        let mut watch = Watch::default();
        let t0 = at();
        assert_eq!(watch.count("203.0.113.1", t0), 1);
        assert_eq!(watch.count("203.0.113.1", t0 + Duration::minutes(30)), 2);
        assert_eq!(watch.count("203.0.113.2", t0 + Duration::minutes(30)), 1);
        // One request left the window the moment WINDOW has gone by.
        assert_eq!(
            watch.count("203.0.113.1", t0 + WINDOW - Duration::SECOND),
            3
        );
        assert_eq!(
            watch.count("203.0.113.1", t0 + WINDOW),
            3,
            "t0 is out, this is in"
        );
        // Everything is forgotten well after.
        assert_eq!(watch.count("203.0.113.1", t0 + Duration::hours(5)), 1);
        assert_eq!(watch.count("203.0.113.2", t0 + Duration::hours(5)), 1);
    }

    #[test]
    fn the_count_stops_growing_at_a_thousand() {
        let mut watch = Watch::default();
        let mut last = 0;
        for _ in 0..1500 {
            last = watch.count("203.0.113.9", at());
        }
        assert_eq!(last, 1000);
    }

    #[test]
    fn many_addresses_are_swept_when_stale_without_losing_the_fresh() {
        let mut watch = Watch::default();
        let t0 = at();
        for i in 0..SWEEP_AT {
            watch.count(
                &format!("10.{}.{}.{}", i / 65_536, (i / 256) % 256, i % 256),
                t0,
            );
        }
        assert_eq!(watch.by_address.len(), SWEEP_AT);
        // Two hours on, a new address sweeps the stale ones out.
        assert_eq!(watch.count("198.51.100.1", t0 + Duration::hours(2)), 1);
        assert_eq!(watch.by_address.len(), 1);
    }

    #[test]
    fn an_ipv6_subscriber_is_one_address() {
        assert_eq!(address_key("2001:db8:1:2::1"), "2001:db8:1:2::/64");
        assert_eq!(
            address_key("2001:db8:1:2::1"),
            address_key("2001:db8:1:2:aaaa:bbbb:cccc:dddd")
        );
        assert_ne!(
            address_key("2001:db8:1:2::1"),
            address_key("2001:db8:1:3::1")
        );
        // IPv4 and anything that is no address stand for themselves.
        assert_eq!(address_key("203.0.113.7"), "203.0.113.7");
        assert_eq!(address_key("unknown"), "unknown");

        let mut watch = Watch::default();
        assert_eq!(watch.count("2001:db8:1:2::1", at()), 1);
        assert_eq!(watch.count("2001:db8:1:2:aaaa:bbbb:cccc:dddd", at()), 2);
        assert_eq!(watch.count("2001:db8:1:3::1", at()), 1);
    }

    #[test]
    fn the_honeypot_marks_an_address_for_a_day_and_not_a_moment_longer() {
        let mut watch = Watch::default();
        let t0 = at();
        assert!(!watch.is_flagged("203.0.113.1", t0));
        assert_eq!(watch.flag("203.0.113.1", t0), 1);
        assert!(watch.is_flagged("203.0.113.1", t0));
        assert!(watch.is_flagged("203.0.113.1", t0 + FLAGGED_FOR - Duration::SECOND));
        assert!(!watch.is_flagged("203.0.113.1", t0 + FLAGGED_FOR));
        assert!(!watch.is_flagged("203.0.113.2", t0), "another address");
        // Asking again renews the mark and counts the hit.
        assert_eq!(watch.flag("203.0.113.1", t0 + Duration::hours(20)), 2);
        assert!(watch.is_flagged("203.0.113.1", t0 + FLAGGED_FOR));
        assert!(!watch.is_flagged("203.0.113.1", t0 + Duration::hours(20) + FLAGGED_FOR));
        // One /64 shares a mark.
        assert_eq!(watch.flag("2001:db8:1:2::1", t0), 3);
        assert!(watch.is_flagged("2001:db8:1:2::ffff", t0));
        assert!(!watch.is_flagged("2001:db8:1:9::1", t0));
    }

    #[test]
    fn marks_are_swept_when_many_and_stale() {
        let mut watch = Watch::default();
        let t0 = at();
        for i in 0..SWEEP_AT {
            watch.flag(
                &format!("10.{}.{}.{}", i / 65_536, (i / 256) % 256, i % 256),
                t0,
            );
        }
        watch.flag("198.51.100.1", t0 + FLAGGED_FOR + Duration::SECOND);
        assert_eq!(watch.flagged.len(), 1);
        assert!(watch.is_flagged("198.51.100.1", t0 + FLAGGED_FOR + Duration::SECOND));
    }

    #[test]
    fn a_nonce_is_taken_once() {
        let mut watch = Watch::default();
        let until = at() + LIFETIME;
        assert!(watch.take([1; NONCE_LEN], until, at()));
        assert!(!watch.take([1; NONCE_LEN], until, at()));
        assert!(watch.take([2; NONCE_LEN], until, at()));
        assert!(!watch.take([2; NONCE_LEN], until, at()));
    }

    #[test]
    fn answered_puzzles_are_forgotten_when_swept_after_they_expire() {
        let mut watch = Watch::default();
        let until = at() + LIFETIME;
        for i in 0..SWEEP_AT {
            let mut nonce = [0_u8; NONCE_LEN];
            nonce[..8].copy_from_slice(&(i as u64).to_be_bytes());
            assert!(watch.take(nonce, until, at()));
        }
        // After the lifetime a new one sweeps them out; the expired nonces
        // are not worth remembering because verify() refuses them anyway.
        assert!(watch.take([0xee; NONCE_LEN], until, until + Duration::SECOND));
        assert_eq!(watch.used.len(), 1);
    }

    // ------------------------------------------------------------------- decoy

    #[test]
    fn a_decoy_is_the_same_for_one_seed_and_differs_between_seeds() {
        assert_eq!(decoy(&[7; 32]), decoy(&[7; 32]));
        assert_ne!(decoy(&[1; 32]), decoy(&[2; 32]));
        assert_ne!(decoy(&[3; 32]), decoy(&[4; 32]));
    }

    fn all_digits(text: &str) -> bool {
        !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit())
    }

    #[test]
    fn every_decoy_is_invented_in_the_shape_of_a_real_one() {
        for first in 0..=255_u8 {
            for second in [0_u8, 77, 255] {
                let mut seed = [first; 32];
                seed[3] = second;
                seed[9] = second.wrapping_add(first);
                seed[10] = second ^ first;
                seed[7] = second;
                seed[8] = first;
                let d = decoy(&seed);
                let what = format!("{d:?}");
                // RFC 2606: no mailbox anywhere.
                assert!(
                    ["example.com", "example.net", "example.org"]
                        .iter()
                        .any(|domain| d.email.ends_with(&format!("@{domain}"))),
                    "{what}"
                );
                let local = d.email.split('@').next().unwrap();
                assert!(
                    local.bytes().all(|b| b.is_ascii_lowercase() || b == b'.'),
                    "{what}"
                );
                // +49 <area> 0ddd dddd: a subscriber number that starts with 0.
                let phone: Vec<&str> = d.phone.split(' ').collect();
                assert_eq!(phone.len(), 4, "{what}");
                assert_eq!(phone[0], "+49", "{what}");
                assert!(all_digits(phone[1]) && !phone[1].starts_with('0'), "{what}");
                assert!(all_digits(phone[2]) && phone[2].len() == 4, "{what}");
                assert!(phone[2].starts_with('0'), "{what}");
                assert!(all_digits(phone[3]) && phone[3].len() == 4, "{what}");
                // A five-digit postcode, a street with a number, two names.
                assert!(all_digits(&d.postcode) && d.postcode.len() == 5, "{what}");
                let (street, number) = d.street.rsplit_once(' ').unwrap();
                assert!(!street.is_empty(), "{what}");
                let number: u32 = number.parse().unwrap();
                assert!((1..=120).contains(&number), "{what}");
                assert_eq!(d.name.split(' ').count(), 2, "{what}");
                assert!(!d.city.is_empty(), "{what}");
            }
        }
    }

    /// The decoy's pairs read back: what `impressum.js` does with the blob.
    fn unblob(blob: &str) -> serde_json::Value {
        let raw = base64::engine::general_purpose::URL_SAFE_NO_PAD
            .decode(blob)
            .unwrap();
        assert_eq!(&raw[..MAGIC.len()], MAGIC);
        assert_eq!(raw[MAGIC.len()], VERSION);
        let nonce = &raw[MAGIC.len() + 1..MAGIC.len() + 1 + NONCE_LEN];
        let cipher = &raw[MAGIC.len() + 1 + NONCE_LEN..];
        let plain: Vec<u8> = cipher
            .iter()
            .zip(keystream(nonce, cipher.len()))
            .map(|(c, k)| c ^ k)
            .collect();
        serde_json::from_slice(&plain).unwrap()
    }

    fn field(plain: &serde_json::Value, name: &str) -> String {
        let mut pairs: Vec<(u64, u64)> = plain["fields"][name]
            .as_array()
            .unwrap_or_else(|| panic!("no field {name}"))
            .iter()
            .map(|p| (p[0].as_u64().unwrap(), p[1].as_u64().unwrap()))
            .collect();
        pairs.sort_unstable();
        for (i, (position, _)) in pairs.iter().enumerate() {
            assert_eq!(*position, i as u64, "{name}: positions are 0..n");
        }
        pairs
            .iter()
            .map(|(_, code)| char::from_u32(u32::try_from(*code).unwrap()).unwrap())
            .collect()
    }

    #[test]
    fn a_decoy_blob_has_the_shape_of_the_real_one_and_holds_the_decoy() {
        let d = decoy(&[7; 32]);
        let blob = decoy_blob(&d);
        assert!(looks_like_blob(blob.as_bytes()));
        assert!(
            blob.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
        );
        let plain = unblob(&blob);
        assert_eq!(plain["v"], VERSION);
        let fields = plain["fields"].as_object().unwrap();
        let mut names: Vec<&str> = fields.keys().map(String::as_str).collect();
        names.sort_unstable();
        assert_eq!(
            names,
            [
                "city",
                "country_de",
                "country_en",
                "email",
                "name",
                "phone",
                "postcode",
                "street"
            ]
        );
        assert_eq!(field(&plain, "name"), d.name);
        assert_eq!(field(&plain, "street"), d.street);
        assert_eq!(field(&plain, "postcode"), d.postcode);
        assert_eq!(field(&plain, "city"), d.city);
        assert_eq!(field(&plain, "email"), d.email);
        assert_eq!(field(&plain, "phone"), d.phone);
        assert_eq!(field(&plain, "country_de"), "Deutschland");
        assert_eq!(field(&plain, "country_en"), "Germany");
        // Umlauts survive as code points.
        let umlaut = Decoy {
            city: "Würzburg".to_owned(),
            ..d
        };
        assert_eq!(field(&unblob(&decoy_blob(&umlaut)), "city"), "Würzburg");
    }

    #[test]
    fn a_decoy_blob_is_a_new_one_each_time_the_details_are_not_in_the_clear() {
        let d = decoy(&[7; 32]);
        let (a, b) = (decoy_blob(&d), decoy_blob(&d));
        assert_ne!(a, b, "a fresh nonce and a fresh shuffle");
        for blob in [a, b] {
            for word in [d.name.as_str(), d.city.as_str(), d.email.as_str()] {
                assert!(!blob.contains(word));
            }
        }
    }

    #[test]
    fn looks_like_blob_wants_the_magic_the_version_and_a_body() {
        let make = |magic: &[u8], version: u8, body: usize| {
            let mut raw = magic.to_vec();
            raw.push(version);
            raw.extend([9_u8; NONCE_LEN]);
            raw.extend(vec![1_u8; body]);
            b64(&raw)
        };
        assert!(looks_like_blob(make(MAGIC, VERSION, 1).as_bytes()));
        assert!(looks_like_blob(
            format!("  {}\n", make(MAGIC, VERSION, 40)).as_bytes()
        ));
        assert!(
            !looks_like_blob(make(MAGIC, VERSION, 0).as_bytes()),
            "no body"
        );
        assert!(
            !looks_like_blob(make(MAGIC, VERSION + 1, 5).as_bytes()),
            "version"
        );
        assert!(!looks_like_blob(make(MAGIC, 1, 5).as_bytes()), "version");
        assert!(
            !looks_like_blob(make(b"BYIX", VERSION, 5).as_bytes()),
            "magic"
        );
        assert!(!looks_like_blob(b""));
        assert!(!looks_like_blob(b"  \n"));
        assert!(!looks_like_blob(b"Erika Mustermann, Musterweg 12"));
        assert!(!looks_like_blob(b"!!!!"));
    }

    // ------------------------------------------------------------- client_bits

    #[test]
    fn the_pages_report_is_read_and_cut_to_the_bits_we_know() {
        assert_eq!(
            client_bits(Some("s=5")),
            CLIENT_WEBDRIVER | CLIENT_SOFTWARE_GL
        );
        assert_eq!(client_bits(Some("s=0")), 0);
        assert_eq!(client_bits(Some("s=7")), 7);
        assert_eq!(client_bits(Some("s=255")), 7, "unknown bits are dropped");
        assert_eq!(client_bits(Some("s=8")), 0);
        assert_eq!(client_bits(Some("x=1&s=3&y=2")), 3);
        assert_eq!(client_bits(Some("s=4294967295")), 7);
    }

    #[test]
    fn a_report_that_cannot_be_read_says_nothing() {
        for query in [
            None,
            Some(""),
            Some("s="),
            Some("s=abc"),
            Some("s=-1"),
            Some("s=1.5"),
            Some("s=99999999999"),
            Some("ss=3"),
            Some("S=3"),
            Some("x=3"),
            Some("s"),
        ] {
            assert_eq!(client_bits(query), 0, "{query:?}");
        }
    }

    // ---------------------------------------------------------------- settings

    fn scratch(contents: &[u8]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("baylee-impressum-{}", uuid::Uuid::now_v7()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("impressum.blob");
        std::fs::write(&path, contents).unwrap();
        path
    }

    fn forget(path: &Path) {
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    const GOOD_KEY: &str = "0123456789abcdef0123456789abcdef";

    #[test]
    fn settings_take_a_long_key_and_a_blob_file() {
        let blob = decoy_blob(&decoy(&[7; 32]));
        let path = scratch(blob.as_bytes());
        let settings = Settings::new(GOOD_KEY, &path).unwrap();
        assert_eq!(settings.blob_path(), path);
        assert_eq!(
            settings.key_hash(),
            <[u8; 32]>::from(Sha256::digest(GOOD_KEY.as_bytes()))
        );
        // A newline after the blob, as an editor leaves it.
        let with_newline = scratch(format!("{blob}\n").as_bytes());
        assert!(Settings::new(GOOD_KEY, &with_newline).is_ok());
        // The length counts characters, not bytes.
        assert!(Settings::new(&"ä".repeat(MIN_KEY_CHARS), &path).is_ok());
        forget(&path);
        forget(&with_newline);
    }

    #[test]
    fn settings_refuse_a_short_key() {
        let path = scratch(decoy_blob(&decoy(&[7; 32])).as_bytes());
        let err = Settings::new(&GOOD_KEY[..MIN_KEY_CHARS - 1], &path)
            .err()
            .unwrap()
            .to_string();
        assert!(err.contains("FEEDBACK_IMPRESSUM_KEY"), "{err}");
        assert!(err.contains("32"), "{err}");
        assert!(Settings::new("", &path).is_err());
        assert!(Settings::new(&"ä".repeat(MIN_KEY_CHARS - 1), &path).is_err());
        forget(&path);
    }

    #[test]
    fn settings_refuse_a_file_that_is_not_an_impressum_blob() {
        for (what, contents) in [
            ("prose", b"Erika Mustermann, Musterweg 12".as_slice()),
            ("empty", b""),
            ("blank", b"\n\n"),
            ("not base64", b"!!!! not base64 !!!!"),
        ] {
            let path = scratch(contents);
            let err = Settings::new(GOOD_KEY, &path).err().unwrap().to_string();
            assert!(err.contains("FEEDBACK_IMPRESSUM_BLOB"), "{what}: {err}");
            forget(&path);
        }
        // Base64 of something else.
        let path = scratch(b64(&[0_u8; 64]).as_bytes());
        assert!(Settings::new(GOOD_KEY, &path).is_err());
        forget(&path);
        // A file that is not there.
        let missing = std::env::temp_dir().join(format!("baylee-none-{}", uuid::Uuid::now_v7()));
        let err = Settings::new(GOOD_KEY, &missing).err().unwrap().to_string();
        assert!(err.contains("FEEDBACK_IMPRESSUM_BLOB"), "{err}");
    }

    // --------------------------------------------------------------- constants

    #[test]
    fn the_script_key_is_the_pages() {
        let js = include_str!("../../../web/site/impressum.js");
        let marker = "var SCRIPT_KEY = \"";
        let start = js.find(marker).expect("impressum.js names SCRIPT_KEY") + marker.len();
        let end = start + js[start..].find("\";").expect("the constant ends");
        assert_eq!(&js[start..end], SCRIPT_KEY);
        assert_eq!(SCRIPT_KEY.len(), 32);
        assert!(SCRIPT_KEY.bytes().all(|b| b.is_ascii_hexdigit()));
    }

    #[test]
    fn the_constants_keep_the_promises_the_docs_make() {
        assert_eq!(BASE_DIFFICULTY, 17);
        assert_eq!(PARTS, 8);
        assert_eq!(LIFETIME, Duration::minutes(5));
        assert_eq!(FLAGGED_FOR, Duration::hours(24));
        assert_eq!(WINDOW, Duration::HOUR);
        // Three client bits, each its own.
        assert_eq!(
            CLIENT_WEBDRIVER | CLIENT_NO_LANGUAGES | CLIENT_SOFTWARE_GL,
            7
        );
        assert_eq!(NONCE_LEN, 16);
    }
}
