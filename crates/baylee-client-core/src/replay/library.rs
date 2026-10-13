//! The Replays list (§6, §7.2): the games there are to watch, from this
//! computer's kept records and from the gateway's `GET /replays`.
//!
//! The wire shapes of `GET /replays` live here ([`ReplayPage`]) and are the
//! one definition both ends use: the gateway's test deserialises them, the
//! client reads them (`.claude/b8/REPLAY-DESIGN.md` §12, E0).

use serde::{Deserialize, Serialize};

use super::Perspectives;

// ------------------------------------------------------------ the wire

/// `GET /replays?offset=…&limit=…`: a page of the account's seated games,
/// newest first.
#[derive(Clone, PartialEq, Eq, Debug, Default, Serialize, Deserialize)]
pub struct ReplayPage {
    /// The page.
    pub games: Vec<HostedGame>,
    /// How many there are in all.
    pub total: u64,
}

/// One hosted game the account sat in.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct HostedGame {
    /// The gateway's game id.
    pub game: String,
    /// When it started, RFC 3339 in UTC.
    pub started_at: String,
    /// When it ended, when it did.
    #[serde(default)]
    pub ended_at: Option<String>,
    /// Whether the record holds the whole game.
    pub complete: bool,
    /// The requester's seat.
    pub seat: u8,
    /// The turn it ended in (or the record reached).
    #[serde(default)]
    pub turn: Option<u32>,
    /// How it went for the requester's seat.
    pub result: HostedResult,
    /// Whether the requester may ask for the public table.
    pub public: bool,
    /// How far today's build is known to play it the same.
    pub check: HostedCheck,
    /// The build that played it.
    #[serde(default)]
    pub build: Option<String>,
    /// Every seat, in seat order.
    pub seats: Vec<HostedSeat>,
    /// Whether the asking account archived it (§18.1).
    #[serde(default)]
    pub archived: bool,
}

/// How a hosted game went for the requester's seat.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostedResult {
    /// The seat won.
    Won,
    /// The seat lost.
    Lost,
    /// Nobody won.
    Draw,
    /// The record has no end.
    Unfinished,
    /// It ended before the gateway noted how (a record from before the
    /// column).
    Unknown,
}

/// How far today's build is known to play a hosted record the same.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostedCheck {
    /// This gateway's build recorded it.
    Exact,
    /// Another build recorded it, with the public table's digest.
    Checked,
    /// Another build recorded it, without the digest.
    Unverified,
}

/// One seat of a hosted game, as the list shows it.
#[derive(Clone, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct HostedSeat {
    /// The seat.
    pub seat: u8,
    /// Who sat there.
    pub who: Who,
    /// The handle as the gateway names it now ("House AI", "Language
    /// model", "Former player" for the others).
    pub name: String,
    /// The deck's colours, `WUBRG` letters: the requester's own seat only.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub colours: Option<String>,
    /// The commander's `CardIndex`, public at the table.
    #[serde(default)]
    pub commander: Option<u32>,
    /// Whether its player lets others see its hand and deck in the replay
    /// (REPLAY-DESIGN §18; the house and a seat bridge always do). Shared
    /// unless the player unticked it (owner, 13.10.2026); absent reads as
    /// that default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shared: Option<bool>,
    /// Whether the asking account played this seat (as `who == "you"`).
    #[serde(default)]
    pub mine: bool,
}

/// Who sat in a seat.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Who {
    /// The requester.
    You,
    /// Another person.
    Player,
    /// A person whose account is gone.
    Former,
    /// The house AI.
    House,
    /// A seat bridge (a language model).
    Model,
}

// ------------------------------------------------------------ the list

/// Where an entry's game is.
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum EntrySource {
    /// A kept record on this computer, by its file name.
    Local {
        /// The file's name among the kept records.
        file: String,
    },
    /// A game on a gateway.
    Hosted {
        /// The gateway's address.
        gateway: String,
        /// Its game id.
        game: String,
    },
}

/// Who a player in an entry is.
#[derive(Clone, PartialEq, Eq, Debug)]
#[allow(clippy::struct_excessive_bools)] // independent facts about one seat
pub struct Player {
    /// The seat.
    pub seat: u8,
    /// The name to show (a handle, "Former player", "Language model").
    pub name: String,
    /// The viewer.
    pub you: bool,
    /// The house, and its level where it is known.
    pub house: Option<String>,
    /// A seat bridge.
    pub model: bool,
    /// A person whose account is gone.
    pub former: bool,
    /// Whether others may see this seat's hand in the replay (the house's
    /// and a model's always; a person's when they said yes).
    pub shared: bool,
}

/// What an entry shows of one seat's deck: the viewer's colours, and every
/// seat's commander (public at the table). An opponent's colours never: a
/// deck's colours can name a splash it never showed.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct DeckMark {
    /// The seat.
    pub seat: u8,
    /// `WUBRG` letters, the viewer's own seat only.
    pub colours: Option<String>,
    /// The commander's `CardIndex`.
    pub commander: Option<u32>,
}

/// How the game went for the viewer.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum Outcome {
    /// Won.
    Won,
    /// Lost.
    Lost,
    /// Nobody won.
    Draw,
    /// No end in the record.
    Unfinished,
    /// Ended, but nobody noted how.
    Unknown,
}

impl From<HostedResult> for Outcome {
    fn from(result: HostedResult) -> Self {
        match result {
            HostedResult::Won => Self::Won,
            HostedResult::Lost => Self::Lost,
            HostedResult::Draw => Self::Draw,
            HostedResult::Unfinished => Self::Unfinished,
            HostedResult::Unknown => Self::Unknown,
        }
    }
}

/// How far this build is known to play an entry the same: the list's mark.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub enum EntryCheck {
    /// No mark: this build recorded it.
    Exact,
    /// ◐: another build, checked against the public table.
    Checked,
    /// ◌: recorded before replays were checked.
    Unverified,
}

impl EntryCheck {
    /// The `verify` a replay of this entry asks for (§7.1): an unverified
    /// entry plays on straight away, or the whole back catalogue would open
    /// on "the same up to turn 0".
    #[must_use]
    pub const fn verify(self) -> super::Verify {
        match self {
            Self::Exact | Self::Checked => super::Verify::Stop,
            Self::Unverified => super::Verify::Note,
        }
    }
}

impl From<HostedCheck> for EntryCheck {
    fn from(check: HostedCheck) -> Self {
        match check {
            HostedCheck::Exact => Self::Exact,
            HostedCheck::Checked => Self::Checked,
            HostedCheck::Unverified => Self::Unverified,
        }
    }
}

/// One row of the Replays list.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Entry {
    /// Where the game is.
    pub source: EntrySource,
    /// When it started, Unix milliseconds.
    pub started: u64,
    /// When it ended (or was last written), Unix milliseconds.
    pub ended: Option<u64>,
    /// The seats, "You" first.
    pub players: Vec<Player>,
    /// The decks' marks, in seat order.
    pub decks: Vec<DeckMark>,
    /// How it went for the viewer.
    pub result: Outcome,
    /// The turn it ended in.
    pub turns: Option<u32>,
    /// How far this build plays it the same.
    pub check: EntryCheck,
    /// The perspectives it offers.
    pub can: Perspectives,
    /// The viewer's seat.
    pub seat: u8,
    /// The house's level, for a game against it.
    pub house: Option<String>,
    /// Whether the viewer lets the other players see their hand and deck
    /// in this game's replays (REPLAY-DESIGN §18.2): a hosted game's own
    /// seat, shared unless unticked; `None` for a local record, which never
    /// leaves this computer.
    pub sharing: Option<bool>,
    /// Whether the viewer archived it: out of the list, under "Archived"
    /// (owner, 13.10.2026: nothing is deleted from the list, it is
    /// archived and can be brought back).
    pub archived: bool,
}

impl Entry {
    /// The entry a hosted game makes, on `gateway`.
    #[must_use]
    pub fn hosted(gateway: &str, game: &HostedGame) -> Self {
        let mut players: Vec<Player> = game
            .seats
            .iter()
            .map(|s| Player {
                seat: s.seat,
                name: s.name.clone(),
                you: s.who == Who::You,
                house: (s.who == Who::House).then(String::new),
                model: s.who == Who::Model,
                former: s.who == Who::Former,
                shared: matches!(s.who, Who::House | Who::Model) || s.shared.unwrap_or(true),
            })
            .collect();
        players.sort_by_key(|p| (!p.you, p.seat));
        Self {
            source: EntrySource::Hosted {
                gateway: gateway.to_owned(),
                game: game.game.clone(),
            },
            started: rfc3339_ms(&game.started_at).unwrap_or(0),
            ended: game.ended_at.as_deref().and_then(rfc3339_ms),
            players,
            decks: game
                .seats
                .iter()
                .map(|s| DeckMark {
                    seat: s.seat,
                    colours: s.colours.clone().filter(|_| s.who == Who::You),
                    commander: s.commander,
                })
                .collect(),
            result: game.result.into(),
            turns: game.turn,
            check: game.check.into(),
            // Any seat, the table, and all hands (§18.3): the gateway masks
            // what a seat's player did not share.
            can: Perspectives {
                seat: true,
                public: game.public,
                all_hands: true,
            },
            seat: game.seat,
            house: None,
            sharing: game
                .seats
                .iter()
                .find(|s| s.mine || s.who == Who::You)
                .map(|s| s.shared.unwrap_or(true)),
            archived: game.archived,
        }
    }

    /// Whether the entry can be deleted here: a local record that is not
    /// the game being played (`live`, its file name). A hosted one is only
    /// ever hidden from the list ([`Entry::hideable`]).
    #[must_use]
    pub fn deletable(&self, live: Option<&str>) -> bool {
        match &self.source {
            EntrySource::Local { file } => live != Some(file.as_str()),
            EntrySource::Hosted { .. } => false,
        }
    }

    /// Whether the entry is archived on the server
    /// (`PUT /replays/{game}/archived`, §18.1): a hosted game; archiving
    /// only moves it to the viewer's "Archived" list, its record stays.
    #[must_use]
    pub const fn hideable(&self) -> bool {
        matches!(self.source, EntrySource::Hosted { .. })
    }

    /// How long the game ran, in whole minutes, when it is known.
    #[must_use]
    pub fn minutes(&self) -> Option<u64> {
        self.ended
            .map(|end| end.saturating_sub(self.started).div_ceil(60_000))
    }

    /// Whether the entry is a game against the house on this computer.
    #[must_use]
    pub const fn is_local(&self) -> bool {
        matches!(self.source, EntrySource::Local { .. })
    }

    /// Whether the query (lower case) names one of its players or
    /// commanders (by the name `commander_name` gives a `CardIndex`).
    #[must_use]
    pub fn matches(&self, query: &str, commander_name: &dyn Fn(u32) -> Option<String>) -> bool {
        let query = query.trim().to_lowercase();
        if query.is_empty() {
            return true;
        }
        self.players
            .iter()
            .any(|p| p.name.to_lowercase().contains(&query))
            || self
                .decks
                .iter()
                .filter_map(|d| d.commander)
                .filter_map(commander_name)
                .any(|name| name.to_lowercase().contains(&query))
    }
}

/// What a local record's summary says, in this crate's words:
/// `baylee_gamehost::replay::Summary`, which this crate cannot link.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct LocalSummary {
    /// When the game started, Unix milliseconds.
    pub started: u64,
    /// When the last line was written.
    pub ended: Option<u64>,
    /// Whether the record has its end.
    pub complete: bool,
    /// The winners' seats (empty with an end: a draw).
    pub winners: Vec<u8>,
    /// The turn it ended in.
    pub turn: Option<u32>,
    /// The build that recorded it.
    pub build: String,
    /// Whether its lines carry the public table's digest.
    pub checked: bool,
    /// Every seat.
    pub seats: Vec<LocalSeat>,
}

/// One seat of a local record.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct LocalSeat {
    /// The player (`None`), or the house and its level.
    pub house: Option<String>,
    /// The deck's colours, `WUBRG` letters.
    pub colours: String,
    /// The commander's `CardIndex`.
    pub commander: Option<u32>,
}

impl Entry {
    /// The entry a kept record makes. `build` is this client's: a record
    /// it wrote itself is exact.
    #[must_use]
    pub fn of_record(file: &str, summary: &LocalSummary, build: &str, you: &str) -> Self {
        let seat = summary
            .seats
            .iter()
            .position(|s| s.house.is_none())
            .unwrap_or(0) as u8;
        let mut players: Vec<Player> = summary
            .seats
            .iter()
            .enumerate()
            .map(|(i, s)| Player {
                seat: i as u8,
                name: if s.house.is_none() {
                    you.to_owned()
                } else {
                    String::new()
                },
                you: i as u8 == seat,
                house: s.house.clone(),
                model: false,
                former: false,
                // A local record holds every hand whole, and the others are
                // the house's.
                shared: true,
            })
            .collect();
        players.sort_by_key(|p| (!p.you, p.seat));
        let result = if !summary.complete {
            Outcome::Unfinished
        } else if summary.winners.is_empty() {
            Outcome::Draw
        } else if summary.winners.contains(&seat) {
            Outcome::Won
        } else {
            Outcome::Lost
        };
        Self {
            source: EntrySource::Local {
                file: file.to_owned(),
            },
            started: summary.started,
            ended: summary.ended,
            house: summary.seats.iter().find_map(|s| s.house.clone()),
            players,
            decks: summary
                .seats
                .iter()
                .enumerate()
                .map(|(i, s)| DeckMark {
                    seat: i as u8,
                    // A local record holds every deck whole, and the
                    // others are the house's: every seat's colours show.
                    colours: Some(s.colours.clone()),
                    commander: s.commander,
                })
                .collect(),
            result,
            turns: summary.turn,
            check: if summary.build == build {
                EntryCheck::Exact
            } else if summary.checked {
                EntryCheck::Checked
            } else {
                EntryCheck::Unverified
            },
            can: Perspectives::LOCAL,
            seat,
            sharing: None,
            archived: false,
        }
    }
}

/// Which source the list shows.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum SourceFilter {
    /// Every row.
    #[default]
    All,
    /// Games against the house on this computer.
    House,
    /// Games on the gateway.
    Online,
    /// What the viewer archived, from both.
    Archived,
}

/// Which results the list shows.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, Default)]
pub enum ResultFilter {
    /// Every result.
    #[default]
    Any,
    /// Only this one.
    Only(Outcome),
}

/// The list's filters.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Filters {
    /// Which source.
    pub source: SourceFilter,
    /// Which result.
    pub result: ResultFilter,
    /// A query over players and commanders.
    pub query: String,
}

impl Filters {
    /// Whether `entry` passes.
    #[must_use]
    pub fn pass(&self, entry: &Entry, commander_name: &dyn Fn(u32) -> Option<String>) -> bool {
        let source = match self.source {
            SourceFilter::Archived => entry.archived,
            _ if entry.archived => false,
            SourceFilter::All => true,
            SourceFilter::House => entry.is_local(),
            SourceFilter::Online => !entry.is_local(),
        };
        let result = match self.result {
            ResultFilter::Any => true,
            ResultFilter::Only(outcome) => entry.result == outcome,
        };
        source && result && entry.matches(&self.query, commander_name)
    }
}

/// Every entry from both sources, newest first, deduplicated by source.
#[derive(Clone, PartialEq, Eq, Debug, Default)]
pub struct Library {
    entries: Vec<Entry>,
    /// The online games the gateway has in all, as its last page said.
    pub online_total: u64,
    /// How many online games are held.
    pub online_held: u64,
}

impl Library {
    /// Puts the local entries in, replacing every local one held.
    pub fn set_local(&mut self, local: Vec<Entry>) {
        self.entries.retain(|e| !e.is_local());
        self.entries.extend(local);
        self.sort();
    }

    /// Adds a page of online games (`offset` 0 replaces the online ones).
    pub fn add_page(&mut self, gateway: &str, offset: u64, page: &ReplayPage) {
        if offset == 0 {
            // The archived ones are their own list's (`add_archived_page`).
            self.entries.retain(|e| e.is_local() || e.archived);
            self.online_held = 0;
        }
        for game in &page.games {
            let entry = Entry::hosted(gateway, game);
            if !self.entries.iter().any(|e| e.source == entry.source) {
                self.entries.push(entry);
                self.online_held += 1;
            }
        }
        self.online_total = page.total;
        self.sort();
    }

    /// Puts a page of the viewer's archived online games in
    /// (`GET /replays?archived=true`; offset 0 replaces the archived online
    /// ones held).
    pub fn add_archived_page(&mut self, gateway: &str, offset: u64, page: &ReplayPage) {
        if offset == 0 {
            self.entries.retain(|e| e.is_local() || !e.archived);
        }
        for game in &page.games {
            let mut entry = Entry::hosted(gateway, game);
            entry.archived = true;
            if !self.entries.iter().any(|e| e.source == entry.source) {
                self.entries.push(entry);
            }
        }
        self.sort();
    }

    /// Archives an entry, or brings it back.
    pub fn set_archived(&mut self, source: &EntrySource, archived: bool) {
        for entry in &mut self.entries {
            if &entry.source == source {
                entry.archived = archived;
            }
        }
    }

    /// Forgets the online games (signed out, another gateway).
    pub fn forget_online(&mut self) {
        self.entries.retain(Entry::is_local);
        self.online_held = 0;
        self.online_total = 0;
    }

    /// Writes the viewer's sharing consent into a hosted entry.
    pub fn set_sharing(&mut self, source: &EntrySource, shared: bool) {
        for entry in &mut self.entries {
            if &entry.source == source && entry.sharing.is_some() {
                entry.sharing = Some(shared);
            }
        }
    }

    /// Drops one entry (a local record deleted, a hosted one hidden).
    pub fn remove(&mut self, source: &EntrySource) {
        self.entries.retain(|e| &e.source != source);
    }

    /// Whether the gateway has older games than those held.
    #[must_use]
    pub const fn more_online(&self) -> bool {
        self.online_held < self.online_total
    }

    /// Every entry, newest first.
    #[must_use]
    pub fn entries(&self) -> &[Entry] {
        &self.entries
    }

    /// The entries `filters` passes, newest first.
    pub fn shown<'a>(
        &'a self,
        filters: &'a Filters,
        commander_name: &'a dyn Fn(u32) -> Option<String>,
    ) -> impl Iterator<Item = &'a Entry> + 'a {
        self.entries
            .iter()
            .filter(move |e| filters.pass(e, commander_name))
    }

    fn sort(&mut self) {
        self.entries.sort_by(|a, b| {
            b.started
                .cmp(&a.started)
                .then_with(|| format!("{:?}", a.source).cmp(&format!("{:?}", b.source)))
        });
    }
}

/// The day a moment falls in, at `offset_secs` from UTC: days since
/// 1970-01-01, for grouping the list by day.
#[must_use]
pub fn day_of(ms: u64, offset_secs: i32) -> i64 {
    (ms as i64 / 1000 + i64::from(offset_secs)).div_euclid(86_400)
}

/// Unix milliseconds of an RFC 3339 time (`2026-10-13T18:31:02Z`,
/// fractions and an offset allowed); `None` for anything else.
#[must_use]
pub fn rfc3339_ms(text: &str) -> Option<u64> {
    let b = text.as_bytes();
    let num = |from: usize, len: usize| -> Option<i64> {
        let s = text.get(from..from + len)?;
        s.bytes()
            .all(|c| c.is_ascii_digit())
            .then(|| s.parse().ok())?
    };
    if b.len() < 20 || b[4] != b'-' || b[7] != b'-' || !matches!(b[10], b'T' | b't' | b' ') {
        return None;
    }
    let (year, month, day) = (num(0, 4)?, num(5, 2)?, num(8, 2)?);
    let (hour, minute, second) = (num(11, 2)?, num(14, 2)?, num(17, 2)?);
    let mut at = 19;
    let mut millis = 0;
    if b.get(at) == Some(&b'.') {
        at += 1;
        let start = at;
        while b.get(at).is_some_and(u8::is_ascii_digit) {
            at += 1;
        }
        let digits = &text[start..at];
        let padded = format!("{digits:0<3}");
        millis = padded.get(..3)?.parse().ok()?;
    }
    let offset = match b.get(at)? {
        b'Z' | b'z' => 0,
        sign @ (b'+' | b'-') => {
            let minutes = num(at + 1, 2)? * 60 + num(at + 4, 2)?;
            if *sign == b'+' { minutes } else { -minutes }
        }
        _ => return None,
    };
    let days = days_from_civil(year, month, day);
    let secs = days * 86_400 + hour * 3600 + minute * 60 + second - offset * 60;
    u64::try_from(secs * 1000 + millis).ok()
}

/// Days since 1970-01-01 of a proleptic Gregorian date (Howard Hinnant's
/// `days_from_civil`).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let y = if month <= 2 { year - 1 } else { year };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (month + 9) % 12;
    let doy = (153 * mp + 2) / 5 + day - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}
