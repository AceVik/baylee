//! A card's text and rulings from Scryfall's API, for a profile with
//! `lookup: "scryfall"` (`docs/llm-seat.md` §"Reports and lookups",
//! `docs/legal.md` §3).
//!
//! What Scryfall asks of an API client, as `docs/legal.md` §3 records it,
//! and how it is kept:
//!
//! - every request names its program (`User-Agent: baylee-seat/<version>`)
//!   and what it takes (`Accept: application/json;q=0.9,*/*;q=0.8`);
//! - at most ten requests a second: one at a time, each [`SPACING`] after
//!   the last one ended, for every mind of the process and, where there is
//!   a cache directory, for every process of this machine that keeps it
//!   ([`Turn`]: a lock file there, holding when the last request ended);
//! - what was fetched is kept, at least a day ([`KEEP`]): in memory, and as
//!   one file a card in the user's cache directory, written whole through a
//!   temporary file so seats of other processes never read half of one.
//!   Only this machine's seats read it; it is no mirror for anyone. A name
//!   Scryfall does not know is kept in memory only, so names a model made
//!   up never pile up on disk.
//!
//! A request carries a card's name and nothing else: no game, no player,
//! no key. Two requests a card at most: `/cards/named?fuzzy=`, then the
//! rulings at the card's `rulings_uri`, which is followed only when it is
//! Scryfall's API itself ([`Scryfall::ours`]). No redirect is followed.

use super::fold;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::io::{Read as _, Seek as _, Write as _};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

/// Scryfall's API.
pub const API: &str = "https://api.scryfall.com";

/// The least time between two requests: the top of the spacing Scryfall
/// asks for, so never more than ten a second.
pub const SPACING: Duration = Duration::from_millis(100);

/// How long a card fetched is kept before it is asked again: a week, past
/// the day Scryfall asks a client to keep its data.
pub const KEEP: Duration = Duration::from_hours(7 * 24);

/// The most rulings a card brings.
const MOST_RULINGS: usize = 10;

/// The most of an answer read.
const MAX_BODY: u64 = 2 << 20;

/// The `Accept` header Scryfall asks for.
const ACCEPT: &str = "application/json;q=0.9,*/*;q=0.8";

/// The lock file in the cache directory whose lock is this machine's turn
/// to ask Scryfall, and which holds when the last request ended
/// ([`Turn`]).
pub const TURN_FILE: &str = ".requests.lock";

/// How often a request waiting for another process's turn to end looks
/// again.
const TURN_POLL: Duration = Duration::from_millis(5);

/// A card as Scryfall has it: its text and its rulings.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Card {
    /// Every face's name, mana cost, type line, power and toughness or
    /// loyalty, and Oracle text.
    pub text: String,
    /// At most ten rulings, `date: text`.
    pub rulings: Vec<String>,
    /// Why the rulings are missing, when they are.
    pub note: Option<String>,
}

impl Card {
    /// The rulings as a lookup tells them.
    #[must_use]
    pub fn rulings_text(&self) -> String {
        if let Some(note) = &self.note {
            return note.clone();
        }
        if self.rulings.is_empty() {
            return "none".into();
        }
        self.rulings
            .iter()
            .map(|ruling| format!("  {ruling}"))
            .collect::<Vec<_>>()
            .join("\n")
    }
}

/// One card kept: under the name asked, when, and what was found (`None`
/// for no such card).
#[derive(Clone, Debug, Serialize, Deserialize)]
struct Kept {
    key: String,
    fetched: u64,
    card: Option<Card>,
}

/// Scryfall's API, as a process asks it.
pub struct Scryfall {
    base: String,
    cache: Option<PathBuf>,
    agent: ureq::Agent,
    memo: Mutex<BTreeMap<String, Kept>>,
    /// When the last request went: the next waits [`SPACING`] after it.
    last: Mutex<Option<Instant>>,
}

impl std::fmt::Debug for Scryfall {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Scryfall")
            .field("base", &self.base)
            .field("cache", &self.cache)
            .finish_non_exhaustive()
    }
}

impl Scryfall {
    /// Scryfall's API, over TLS only, kept in `scryfall/` of the user's
    /// cache directory where there is one.
    #[must_use]
    pub fn api() -> Self {
        use baylee_client_core::userdirs::{Kind, Os, real_env, user_dir};
        let cache = user_dir(Kind::Cache, Os::current(), &real_env).map(|dir| dir.join("scryfall"));
        Self::new(API, cache, true)
    }

    /// A stand-in at `base` on this machine, kept in `cache`: tests only,
    /// so a build asks no address but Scryfall's.
    #[cfg(test)]
    pub(crate) fn at(base: &str, cache: Option<PathBuf>) -> Self {
        Self::new(base, cache, false)
    }

    fn new(base: &str, cache: Option<PathBuf>, https_only: bool) -> Self {
        let agent = ureq::Agent::config_builder()
            .http_status_as_error(false)
            .max_redirects(0)
            .https_only(https_only)
            .timeout_global(Some(super::TIMEOUT))
            .build()
            .new_agent();
        Self {
            base: base.trim_end_matches('/').to_string(),
            cache,
            agent,
            memo: Mutex::new(BTreeMap::new()),
            last: Mutex::new(None),
        }
    }

    /// The card Scryfall finds by `name` (its fuzzy match), with its
    /// rulings; `None` when it finds none. Kept, so a card is asked once
    /// in [`KEEP`]. Each request is given `timeout`. Blocking.
    ///
    /// # Errors
    /// When Scryfall cannot be reached or answers otherwise than with a
    /// card or "not found"; nothing failed is kept.
    pub fn card(&self, name: &str, timeout: Duration) -> Result<Option<Card>, String> {
        let key = fold(name);
        let now = unix_now();
        if let Some(kept) = crate::llm::lock(&self.memo)
            .get(&key)
            .filter(|k| fresh(k, now))
        {
            return Ok(kept.card.clone());
        }
        if let Some(kept) = self.read_kept(&key).filter(|k| fresh(k, now)) {
            let card = kept.card.clone();
            crate::llm::lock(&self.memo).insert(key, kept);
            return Ok(card);
        }
        let url = format!(
            "{}/cards/named?fuzzy={}",
            self.base,
            crate::lobby::escape(name.trim())
        );
        let (status, body) = self.get(&url, timeout)?;
        let (card, whole) = match status {
            404 => (None, true),
            200..=299 => {
                let found = body.ok_or("Scryfall's card was no JSON")?;
                let (card, whole) = self.read_card(&found, timeout);
                (Some(card), whole)
            }
            status => return Err(format!("Scryfall answered HTTP {status}")),
        };
        let kept = Kept {
            key: key.clone(),
            fetched: now,
            card,
        };
        // A card whose rulings could not be fetched is asked again next
        // time, not kept without them. A name Scryfall does not know is
        // kept in memory only: a model may make up any number of them.
        if whole {
            if kept.card.is_some() {
                self.write_kept(&kept);
            }
            crate::llm::lock(&self.memo).insert(key, kept.clone());
        }
        Ok(kept.card)
    }

    /// A card from Scryfall's `card` object, and its rulings; whether it
    /// is whole (its rulings fetched, or none to fetch).
    fn read_card(&self, found: &Value, timeout: Duration) -> (Card, bool) {
        let mut card = Card {
            text: card_text(found),
            rulings: Vec::new(),
            note: None,
        };
        let Some(uri) = found.get("rulings_uri").and_then(Value::as_str) else {
            return (card, true);
        };
        if !self.ours(uri) {
            card.note = Some("not read: the rulings' address is not Scryfall's API".into());
            return (card, true);
        }
        match self.get(uri, timeout) {
            Ok((200..=299, Some(rulings))) => {
                card.rulings = rulings
                    .get("data")
                    .and_then(Value::as_array)
                    .map(Vec::as_slice)
                    .unwrap_or_default()
                    .iter()
                    .filter_map(|ruling| {
                        let date = ruling.get("published_at").and_then(Value::as_str)?;
                        let comment = ruling.get("comment").and_then(Value::as_str)?;
                        Some(format!("{date}: {}", comment.trim()))
                    })
                    .take(MOST_RULINGS)
                    .collect();
                (card, true)
            }
            Ok((status, _)) => {
                card.note = Some(format!("lookup failed: Scryfall answered HTTP {status}"));
                (card, false)
            }
            Err(why) => {
                card.note = Some(format!("lookup failed: {why}"));
                (card, false)
            }
        }
    }

    /// Whether `uri` is on Scryfall's API itself (this one's base, then a
    /// path): no other host, no user, no other port.
    #[must_use]
    pub fn ours(&self, uri: &str) -> bool {
        uri.strip_prefix(&self.base)
            .is_some_and(|rest| rest.starts_with('/'))
    }

    /// One `GET`, [`SPACING`] after the last one ended, and none beside
    /// it, in this process and, where there is a cache directory, in any
    /// other that keeps it ([`Turn`], waited for at most `timeout`): the
    /// status, and the body where it is JSON.
    ///
    /// # Errors
    /// When the request fails, or another process holds the turn past
    /// `timeout`: then nothing is asked.
    fn get(&self, url: &str, timeout: Duration) -> Result<(u16, Option<Value>), String> {
        let mut last = crate::llm::lock(&self.last);
        let mut turn = match &self.cache {
            Some(dir) => Turn::take(dir, timeout)?,
            None => None,
        };
        let here = last.map(|at| at.elapsed());
        let anywhere = turn.as_mut().and_then(Turn::since_last);
        if let Some(since) = here.into_iter().chain(anywhere).min() {
            std::thread::sleep(SPACING.saturating_sub(since));
        }
        let answer = self
            .agent
            .get(url)
            .header(
                "user-agent",
                concat!("baylee-seat/", env!("CARGO_PKG_VERSION")),
            )
            .header("accept", ACCEPT)
            .config()
            .timeout_global(Some(timeout))
            .build()
            .call()
            .map_err(|e| match e {
                ureq::Error::Timeout(_) => {
                    format!("Scryfall did not answer within {} s", timeout.as_secs())
                }
                other => format!("Scryfall could not be reached ({other})"),
            })
            .map(|mut answer| {
                let status = answer.status().as_u16();
                let body = answer
                    .body_mut()
                    .with_config()
                    .limit(MAX_BODY)
                    .read_json()
                    .ok();
                (status, body)
            });
        *last = Some(Instant::now());
        if let Some(turn) = &mut turn {
            turn.ended();
        }
        answer
    }

    /// The card kept on disk under `key`, if any.
    fn read_kept(&self, key: &str) -> Option<Kept> {
        let path = self.cache.as_ref()?.join(file_name(key));
        let kept: Kept = serde_json::from_slice(&std::fs::read(path).ok()?).ok()?;
        (kept.key == key).then_some(kept)
    }

    /// Keeps `kept` on disk, whole or not at all.
    fn write_kept(&self, kept: &Kept) {
        let Some(dir) = &self.cache else {
            return;
        };
        if let Err(e) = write_whole(dir, &file_name(&kept.key), kept) {
            tracing::debug!(error = %e, "a Scryfall card could not be kept");
        }
    }
}

/// Whether `kept` is younger than [`KEEP`] at `now`.
fn fresh(kept: &Kept, now: u64) -> bool {
    now.saturating_sub(kept.fetched) < KEEP.as_secs()
}

/// This machine's turn to ask Scryfall: the exclusive lock on
/// [`TURN_FILE`] in the cache directory, held from before a request's wait
/// until it ended, so the seats of every process that keeps the directory
/// ask one at a time. The file holds when the last request ended, in
/// milliseconds since 1970. The lock is the operating system's (`flock`
/// on Linux, as the spend book's), so a process that dies holding it lets
/// it go.
struct Turn {
    file: std::fs::File,
}

impl Turn {
    /// The turn in `dir`, waited for at most `within`; `None` where the
    /// file cannot be opened or locked at all, and the spacing holds within
    /// this process only.
    ///
    /// # Errors
    /// When another process holds the turn for longer than `within`.
    fn take(dir: &Path, within: Duration) -> Result<Option<Self>, String> {
        let opened = std::fs::create_dir_all(dir).and_then(|()| {
            let mut options = std::fs::OpenOptions::new();
            options.read(true).write(true).create(true).truncate(false);
            #[cfg(unix)]
            std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
            options.open(dir.join(TURN_FILE))
        });
        let file = match opened {
            Ok(file) => file,
            Err(e) => {
                tracing::debug!(error = %e, "Scryfall's turn file could not be opened");
                return Ok(None);
            }
        };
        let deadline = Instant::now() + within;
        loop {
            match file.try_lock() {
                Ok(()) => return Ok(Some(Self { file })),
                Err(std::fs::TryLockError::WouldBlock) if Instant::now() < deadline => {
                    std::thread::sleep(TURN_POLL);
                }
                Err(std::fs::TryLockError::WouldBlock) => {
                    return Err(format!(
                        "another seat of this machine was asking Scryfall for longer than \
                         {:.1} s",
                        within.as_secs_f32()
                    ));
                }
                Err(std::fs::TryLockError::Error(e)) => {
                    tracing::debug!(error = %e, "Scryfall's turn file could not be locked");
                    return Ok(None);
                }
            }
        }
    }

    /// How long ago the last request of any process ended, as the file
    /// says; `None` before the first. A time ahead of this machine's clock
    /// (a clock set back) counts as just now, so the wait is never longer
    /// than [`SPACING`].
    fn since_last(&mut self) -> Option<Duration> {
        let mut text = String::new();
        self.file.rewind().ok()?;
        (&self.file).take(32).read_to_string(&mut text).ok()?;
        let last: u64 = text.trim().parse().ok()?;
        Some(Duration::from_millis(unix_millis().saturating_sub(last)))
    }

    /// Writes that a request ended now; the lock goes with the turn.
    fn ended(&mut self) {
        let now = unix_millis().to_string();
        let written = self
            .file
            .rewind()
            .and_then(|()| self.file.set_len(0))
            .and_then(|()| self.file.write_all(now.as_bytes()));
        if let Err(e) = written {
            tracing::debug!(error = %e, "Scryfall's turn file could not be written");
        }
    }
}

/// Milliseconds since 1970.
fn unix_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| {
            u64::try_from(since.as_millis()).unwrap_or(u64::MAX)
        })
}

/// Seconds since 1970.
fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs())
}

/// The file a card is kept in: its name's letters and digits, and a hash
/// of the whole key, so two names never share one.
fn file_name(key: &str) -> String {
    let mut plain: String = key
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c } else { '-' })
        .take(64)
        .collect();
    // FNV-1a: stable across builds and machines, unlike the standard
    // library's hasher.
    let hash = key.bytes().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0000_0100_0000_01b3)
    });
    let _ = write!(plain, "-{hash:016x}.json");
    plain
}

/// Writes `value` to `name` in `dir` through a temporary file beside it,
/// so a reader finds the old file or the new one, never half of one.
fn write_whole(dir: &Path, name: &str, value: &impl Serialize) -> std::io::Result<()> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    std::fs::create_dir_all(dir)?;
    let temporary = dir.join(format!(
        ".{name}.{}.{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    std::fs::write(&temporary, serde_json::to_vec(value)?)?;
    std::fs::rename(&temporary, dir.join(name)).inspect_err(|_| {
        let _ = std::fs::remove_file(&temporary);
    })
}

/// A Scryfall card's text: each face's name, mana cost, type line, power
/// and toughness or loyalty, and Oracle text.
fn card_text(card: &Value) -> String {
    let faces: Vec<&Value> = card
        .get("card_faces")
        .and_then(Value::as_array)
        .filter(|faces| !faces.is_empty())
        .map_or_else(|| vec![card], |faces| faces.iter().collect());
    let mut text = String::new();
    for face in faces {
        let field = |key: &str| {
            face.get(key)
                .or_else(|| card.get(key))
                .and_then(Value::as_str)
                .map(str::trim)
                .unwrap_or_default()
        };
        let _ = write!(text, "{}", field("name"));
        if !field("mana_cost").is_empty() {
            let _ = write!(text, " {}", field("mana_cost"));
        }
        let _ = write!(text, " · {}", field("type_line"));
        match (field("power"), field("toughness"), field("loyalty")) {
            (p, t, _) if !p.is_empty() && !t.is_empty() => {
                let _ = write!(text, " · {p}/{t}");
            }
            (_, _, loyalty) if !loyalty.is_empty() => {
                let _ = write!(text, " · loyalty {loyalty}");
            }
            _ => {}
        }
        text.push('\n');
        for line in face
            .get("oracle_text")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .lines()
            .filter(|line| !line.trim().is_empty())
        {
            let _ = writeln!(text, "  {line}");
        }
    }
    text
}
