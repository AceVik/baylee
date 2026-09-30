//! The spend book: what the language-model seat's games spent, and what the
//! games still running may spend, so that the [`Caps`] hold across games
//! and across bridges running at once (`docs/llm-seat.md` §"The spend
//! book").
//!
//! # How a game is counted
//!
//! Before a game, the bridge reserves what it may spend
//! ([`Ledger::reserve`]): the game's budget, or less when less is left of
//! the day's cap or the month's. It plays under that reservation as a hard
//! limit, and when the game is over it settles with what it really spent
//! ([`Ledger::settle`]). A reservation counts in full until it is settled,
//! so a bridge that crashed, or one still playing, is counted at the most
//! it may spend: the book errs high and never lower than the bill.
//!
//! A game counts in the day it was reserved in, the player's own calendar
//! day where the platform says what that is and UTC where it does not
//! ([`Moment`]), and in that day's month. Dollars count the games of models
//! with a price; tokens count the games of models without one.
//!
//! # The file
//!
//! [`FILE`] beside the settings file, one JSON document rewritten whole
//! under an exclusive lock on a file beside it ([`Book`]), so two bridges
//! reserving at once take turns and the second sees the first. It holds
//! the player's own record and nothing of the game: when, which profile and
//! model, what was reserved and what was spent.

use super::Caps;
use serde::{Deserialize, Serialize};

#[cfg(not(target_arch = "wasm32"))]
use std::path::{Path, PathBuf};

/// The spend book's name, beside the settings file.
pub const FILE: &str = "llm-spend.json";

/// The book's format; a book of a newer one is not read.
pub const VERSION: u32 = 1;

/// A moment on the player's machine: the Unix time, and the offset of
/// its clock from UTC then, when the platform said what it was. The caller
/// reads the clock; nothing here does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Moment {
    /// Seconds since 1970-01-01 UTC.
    pub unix: i64,
    /// Seconds east of UTC, or `None` where the platform would not say:
    /// then days are UTC's.
    pub offset: Option<i32>,
}

impl Moment {
    /// The calendar day it falls in, `YYYY-MM-DD`, on the player's clock.
    #[must_use]
    pub fn day(self) -> String {
        let offset = self
            .offset
            .and_then(|secs| time::UtcOffset::from_whole_seconds(secs).ok())
            .unwrap_or(time::UtcOffset::UTC);
        let date = time::OffsetDateTime::from_unix_timestamp(self.unix)
            .unwrap_or(time::OffsetDateTime::UNIX_EPOCH)
            .to_offset(offset)
            .date();
        format!(
            "{:04}-{:02}-{:02}",
            date.year(),
            u8::from(date.month()),
            date.day()
        )
    }

    /// The month it falls in, `YYYY-MM`.
    #[must_use]
    pub fn month(self) -> String {
        self.day()[..7].to_string()
    }

    /// Whose midnight a day ends at, for a sentence: "local time, UTC+02:00"
    /// or "UTC".
    #[must_use]
    pub fn zone(self) -> String {
        match self.offset {
            Some(secs) => {
                let sign = if secs < 0 { '-' } else { '+' };
                let secs = secs.unsigned_abs();
                format!(
                    "local time, UTC{sign}{:02}:{:02}",
                    secs / 3600,
                    secs / 60 % 60
                )
            }
            None => "UTC".into(),
        }
    }
}

/// An amount a game may spend: dollars for a model with a price, tokens
/// for one without.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Budget {
    /// US dollars.
    Usd(f64),
    /// Tokens, in and out together.
    Tokens(u64),
}

/// One game in the book.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    /// Its number in this book.
    pub id: u64,
    /// When it was reserved, in Unix seconds.
    pub at: i64,
    /// The calendar day it counts in, `YYYY-MM-DD`.
    pub day: String,
    /// The clock's offset from UTC that day was read with, in seconds;
    /// absent where the platform would not say, and the day is UTC's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset: Option<i32>,
    /// The profile it played, if one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    /// The model it played.
    pub model: String,
    /// Dollars reserved, for a model with a price.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reserved_usd: Option<f64>,
    /// Tokens reserved, for a model without one.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reserved_tokens: Option<u64>,
    /// Dollars spent, once settled, for a model with a price.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spent_usd: Option<f64>,
    /// Tokens spent, once settled.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub spent_tokens: Option<u64>,
    /// When it was settled, in Unix seconds; absent while it is open.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub settled: Option<i64>,
}

impl Entry {
    /// Whether it counts in dollars.
    #[must_use]
    pub const fn priced(&self) -> bool {
        self.reserved_usd.is_some()
    }

    /// The dollars it counts: what it spent once settled, else all it
    /// reserved.
    #[must_use]
    pub fn counted_usd(&self) -> f64 {
        let reserved = self.reserved_usd.unwrap_or(0.0);
        match (self.settled, self.spent_usd) {
            (Some(_), Some(spent)) => spent,
            _ => reserved,
        }
    }

    /// The tokens it counts, for a model with no price: what it spent once
    /// settled, else all it reserved.
    #[must_use]
    pub fn counted_tokens(&self) -> u64 {
        let reserved = self.reserved_tokens.unwrap_or(0);
        match (self.settled, self.spent_tokens) {
            (Some(_), Some(spent)) => spent,
            _ => reserved,
        }
    }
}

/// What a day's or a month's games count against the caps.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Spent {
    /// Dollars, over the games of models with a price.
    pub usd: f64,
    /// Tokens, over the games of models without one.
    pub tokens: u64,
    /// The games counted in dollars.
    pub priced: Games,
    /// The games counted in tokens.
    pub unpriced: Games,
}

/// How many games a sum is over.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Games {
    /// Games reserved.
    pub played: u32,
    /// Of them, the ones not settled: still playing, or never to settle.
    pub open: u32,
}

/// A reservation made.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Grant {
    /// Its number, to settle it by.
    pub id: u64,
    /// What the game may spend: its hard limit.
    pub budget: Budget,
}

/// What a game asks to reserve.
#[derive(Clone, Copy, Debug)]
pub struct Ask<'a> {
    /// The profile it plays, if one.
    pub profile: Option<&'a str>,
    /// The model.
    pub model: &'a str,
    /// Its own budget: dollars for a model with a price, else tokens.
    pub game: Budget,
    /// The least worth sitting down with, in the same unit: what one call
    /// may cost at most.
    pub floor: Budget,
}

/// The book.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Ledger {
    /// Its format.
    pub version: u32,
    /// Every game, oldest first.
    #[serde(default)]
    pub games: Vec<Entry>,
}

impl Default for Ledger {
    fn default() -> Self {
        Self {
            version: VERSION,
            games: Vec::new(),
        }
    }
}

impl Ledger {
    /// Reads the book's text.
    ///
    /// # Errors
    /// For text that is not the book, one written by a newer build, or an
    /// entry that reserves nothing or both dollars and tokens.
    pub fn parse(text: &str) -> Result<Self, String> {
        let ledger: Self = serde_json::from_str(text).map_err(|e| e.to_string())?;
        if ledger.version > VERSION {
            return Err(format!(
                "it is written in format {}, and this build reads up to {VERSION}",
                ledger.version
            ));
        }
        if let Some(entry) = ledger
            .games
            .iter()
            .find(|e| e.reserved_usd.is_some() == e.reserved_tokens.is_some())
        {
            return Err(format!(
                "game {} reserves {}",
                entry.id,
                if entry.priced() {
                    "both dollars and tokens"
                } else {
                    "nothing"
                }
            ));
        }
        Ok(ledger)
    }

    /// The book's text.
    #[must_use]
    pub fn to_json(&self) -> String {
        let mut text = serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".into());
        text.push('\n');
        text
    }

    /// What the games of `day` (`YYYY-MM-DD`) count.
    #[must_use]
    pub fn on_day(&self, day: &str) -> Spent {
        self.count(|entry| entry.day == day)
    }

    /// What the games of `month` (`YYYY-MM`) count.
    #[must_use]
    pub fn in_month(&self, month: &str) -> Spent {
        self.count(|entry| entry.day.get(..7) == Some(month))
    }

    fn count(&self, within: impl Fn(&Entry) -> bool) -> Spent {
        let mut spent = Spent::default();
        for entry in self.games.iter().filter(|entry| within(entry)) {
            let games = if entry.priced() {
                spent.usd += entry.counted_usd();
                &mut spent.priced
            } else {
                spent.tokens = spent.tokens.saturating_add(entry.counted_tokens());
                &mut spent.unpriced
            };
            games.played += 1;
            games.open += u32::from(entry.settled.is_none());
        }
        spent
    }

    /// Reserves what a game may spend at `now`: its own budget, or what is
    /// left of the day's cap or the month's where that is less.
    ///
    /// # Errors
    /// A sentence, when what could be reserved is below the ask's floor
    /// (one call's worth), naming the limit that leaves too little and when
    /// it is renewed; and for a model with no price under a dollar cap with
    /// no token cap beside it ([`Caps::unpriced_fault`]).
    pub fn reserve(&mut self, caps: &Caps, ask: &Ask<'_>, now: Moment) -> Result<Grant, String> {
        let today = self.on_day(&now.day());
        let this_month = self.in_month(&now.month());
        let budget = match (ask.game, ask.floor) {
            (Budget::Usd(game), Budget::Usd(floor)) => {
                let limits = [
                    Some(Limit::game(game)),
                    caps.day_usd
                        .map(|cap| Limit::day(cap, today.usd, today.priced)),
                    caps.month_usd
                        .map(|cap| Limit::month(cap, this_month.usd, this_month.priced)),
                ];
                let binding = tightest(limits.into_iter().flatten());
                if binding.left < floor {
                    return Err(binding.refusal(ask.model, now, &dollars(floor), &dollars));
                }
                Budget::Usd(binding.left)
            }
            (Budget::Tokens(game), Budget::Tokens(floor)) => {
                if let Some(why) = caps.unpriced_fault(ask.model) {
                    return Err(why);
                }
                #[allow(clippy::cast_precision_loss)] // token counts stay far below 2^52
                let as_f = |n: u64| n as f64;
                let limits = [
                    Some(Limit::game(as_f(game))),
                    caps.day_tokens
                        .map(|cap| Limit::day(as_f(cap), as_f(today.tokens), today.unpriced)),
                    caps.month_tokens.map(|cap| {
                        Limit::month(as_f(cap), as_f(this_month.tokens), this_month.unpriced)
                    }),
                ];
                let binding = tightest(limits.into_iter().flatten());
                if binding.left < as_f(floor) {
                    return Err(binding.refusal(ask.model, now, &tokens(as_f(floor)), &tokens));
                }
                // The least of whole numbers, so exact.
                Budget::Tokens(binding.left as u64)
            }
            _ => return Err("a game's budget and its floor are in one unit".into()),
        };
        let id = self.games.iter().map(|e| e.id).max().unwrap_or(0) + 1;
        let (reserved_usd, reserved_tokens) = match budget {
            Budget::Usd(usd) => (Some(usd), None),
            Budget::Tokens(tokens) => (None, Some(tokens)),
        };
        self.games.push(Entry {
            id,
            at: now.unix,
            day: now.day(),
            offset: now.offset,
            profile: ask.profile.map(str::to_string),
            model: ask.model.to_string(),
            reserved_usd,
            reserved_tokens,
            spent_usd: None,
            spent_tokens: None,
            settled: None,
        });
        Ok(Grant { id, budget })
    }

    /// Settles reservation `id` with what its game spent: `usd` for a model
    /// with a price (rounded up to a millionth of a dollar), and its tokens.
    ///
    /// # Errors
    /// For a reservation this book does not have, or one already settled.
    pub fn settle(
        &mut self,
        id: u64,
        usd: Option<f64>,
        tokens: u64,
        now: Moment,
    ) -> Result<(), String> {
        let Some(entry) = self.games.iter_mut().find(|e| e.id == id) else {
            return Err(format!("the spend book has no game {id}"));
        };
        if entry.settled.is_some() {
            return Err(format!("game {id} is already settled"));
        }
        if entry.priced() {
            let usd = usd.unwrap_or(f64::INFINITY);
            // Never below the bill: up to the next millionth, and an amount
            // that cannot be counted is all that was reserved.
            let usd = if usd.is_finite() && usd >= 0.0 {
                (usd * 1_000_000.0).ceil() / 1_000_000.0
            } else {
                entry.counted_usd()
            };
            entry.spent_usd = Some(usd);
        }
        entry.spent_tokens = Some(tokens);
        entry.settled = Some(now.unix);
        Ok(())
    }
}

/// One limit a reservation is held under.
struct Limit {
    /// Which: the game's own budget, the day's cap or the month's.
    kind: LimitKind,
    /// The cap, or the game's budget.
    cap: f64,
    /// What is left of it.
    left: f64,
    /// The games the period counts already, in this unit.
    games: Games,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum LimitKind {
    Game,
    Day,
    Month,
}

impl Limit {
    fn game(budget: f64) -> Self {
        Self {
            kind: LimitKind::Game,
            cap: budget,
            left: budget.max(0.0),
            games: Games::default(),
        }
    }

    fn day(cap: f64, counted: f64, games: Games) -> Self {
        Self {
            kind: LimitKind::Day,
            cap,
            left: (cap - counted).max(0.0),
            games,
        }
    }

    fn month(cap: f64, counted: f64, games: Games) -> Self {
        Self {
            kind: LimitKind::Month,
            cap,
            left: (cap - counted).max(0.0),
            games,
        }
    }

    /// Why a game cannot sit down under this limit: the sentence.
    fn refusal(
        &self,
        model: &str,
        now: Moment,
        floor: &str,
        amount: &dyn Fn(f64) -> String,
    ) -> String {
        let call = format!("one call of «{model}» may cost up to {floor}");
        let open = match self.games.open {
            0 => String::new(),
            1 => ", 1 of them still open and counted in full".into(),
            n => format!(", {n} of them still open and counted in full"),
        };
        let games = match self.games.played {
            1 => "1 game".to_string(),
            n => format!("{n} games"),
        };
        match self.kind {
            LimitKind::Game => format!(
                "a game's budget of {} cannot pay for one call of «{model}», which may cost up \
                 to {floor}: raise the game's budget",
                amount(self.cap)
            ),
            LimitKind::Day => format!(
                "the day's cap of {} has {} left after {games} today{open}, and {call}; the \
                 next day begins at midnight ({})",
                amount(self.cap),
                amount(self.left),
                now.zone()
            ),
            LimitKind::Month => format!(
                "the month's cap of {} has {} left after {games} this month{open}, and {call}; \
                 the next month begins on the 1st at midnight ({})",
                amount(self.cap),
                amount(self.left),
                now.zone()
            ),
        }
    }
}

/// The limit that leaves the least, the game's own first among equals.
fn tightest(limits: impl Iterator<Item = Limit>) -> Limit {
    limits
        .reduce(|best, next| if next.left < best.left { next } else { best })
        .unwrap_or_else(|| Limit::game(0.0))
}

fn dollars(usd: f64) -> String {
    format!("${usd:.2}")
}

fn tokens(count: f64) -> String {
    format!("{count:.0} tokens")
}

/// The spend book on disk: [`FILE`], with a lock file beside it that every
/// change holds.
#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Debug)]
pub struct Book {
    path: PathBuf,
}

#[cfg(not(target_arch = "wasm32"))]
impl Book {
    /// The book at `path`.
    #[must_use]
    pub const fn new(path: PathBuf) -> Self {
        Self { path }
    }

    /// The book beside the settings file at `settings`.
    #[must_use]
    pub fn beside(settings: &Path) -> Self {
        Self::new(settings.with_file_name(FILE))
    }

    /// Where it is.
    #[must_use]
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The lock every change holds: a file of its own, since the book is
    /// replaced by a rename and a lock on it would stay on the old one.
    fn lock_path(&self) -> PathBuf {
        self.path.with_extension("lock")
    }

    /// The book as it stands, without the lock: a rename replaces it whole,
    /// so a reader sees it before a change or after, never in between.
    /// What a panel shows; a reservation goes through [`Self::update`].
    ///
    /// # Errors
    /// For a book that is there and cannot be read.
    pub fn read(&self) -> Result<Ledger, String> {
        match std::fs::read_to_string(&self.path) {
            Ok(text) => Ledger::parse(&text).map_err(|why| self.unreadable(&why)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Ledger::default()),
            Err(e) => Err(self.unreadable(&e.to_string())),
        }
    }

    /// Changes the book under its lock: waits for any other bridge's
    /// change to finish, reads the book, hands it to `change`, and writes
    /// it back whole when `change` answers `Ok`. The lock is the operating
    /// system's, so it goes with a process that dies holding it.
    ///
    /// # Errors
    /// What `change` answers, or why the book could not be locked, read or
    /// written. A book that is there and cannot be read is never started
    /// again: that would forget what it counted.
    pub fn update<R>(
        &self,
        change: impl FnOnce(&mut Ledger) -> Result<R, String>,
    ) -> Result<R, String> {
        let io = |e: std::io::Error| format!("the spend book {}: {e}", self.path.display());
        if let Some(dir) = self.path.parent().filter(|d| !d.as_os_str().is_empty()) {
            std::fs::create_dir_all(dir).map_err(io)?;
        }
        let mut options = std::fs::OpenOptions::new();
        options.read(true).write(true).create(true).truncate(false);
        #[cfg(unix)]
        std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
        let lock = options.open(self.lock_path()).map_err(io)?;
        lock.lock().map_err(io)?;
        let mut ledger = self.read()?;
        let answer = change(&mut ledger)?;
        super::store::write_whole(&self.path, ledger.to_json().as_bytes()).map_err(io)?;
        drop(lock);
        Ok(answer)
    }

    fn unreadable(&self, why: &str) -> String {
        format!(
            "the spend book {} cannot be read ({why}); it is not started again on its own, \
             which would forget what it counted: move it aside to begin a new one",
            self.path.display()
        )
    }
}

#[cfg(test)]
#[path = "ledger_tests.rs"]
mod tests;
