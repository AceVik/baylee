//! Watching a game again (`docs/client.md` §"Replays";
//! `.claude/b8/REPLAY-DESIGN.md` §7).
//!
//! A replay is a spectator's table driven by a tape instead of a socket:
//! [`host::ReplayHost`] is a [`crate::host::DuelHost`] whose frames come off
//! a [`baylee_client_core::replay::Tape`], walked by the transport. Every
//! system above the host draws it as it draws a live table; what differs is
//! asked of the host ([`crate::host::DuelHost::replay`]) and drawn by the
//! transport bar in the actions bar's place.
//!
//! What is decided lives in `baylee_client_core::replay`; this module holds
//! the host, the worker that projects a record or reads a tape, the Replays
//! tab's state, and the systems that tick the transport.

use std::sync::Arc;

use baylee_client_core::replay::{
    Check, Header, Perspective, Perspectives, Speed, StopLine, Timeline, Verify,
};
use baylee_core::ids::PlayerId;

#[cfg(not(target_arch = "wasm32"))]
pub mod host;
#[cfg(not(target_arch = "wasm32"))]
pub mod local;
/// The browser keeps no records on disk, so its Replays list has no local
/// rows and nothing to remove (`lobby::replays::SHOWN` hides the tab there).
#[cfg(target_arch = "wasm32")]
pub mod local {
    use baylee_client_core::replay::library::Entry;

    /// No record to remove in a browser.
    pub fn delete(_file: &str) {}

    /// No record kept in a browser.
    #[must_use]
    pub fn entries(_build: &str, _you: &str) -> Vec<Entry> {
        Vec::new()
    }
}
mod rewind;
#[cfg(test)]
mod tests;

#[cfg(not(target_arch = "wasm32"))]
pub use host::{HostedAccess, ReplayHost, ReplaySource};

/// A replay's timeline, shared between the host and whoever draws it.
/// Compared by identity: a timeline is built once per tape and never
/// changed, so "the same `Arc`" is "the same timeline", and comparing it
/// costs nothing on a frame where nothing moved.
#[derive(Clone, Debug)]
pub struct SharedTimeline(pub Arc<Timeline>);

impl PartialEq for SharedTimeline {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

/// What a replay is still waiting for.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Loading {
    /// A record is being played again here; how far, in percent, when the
    /// record's length is known.
    Here {
        /// Percent done.
        percent: Option<u8>,
    },
    /// The gateway is playing it again on its side.
    Gateway {
        /// What the gateway is called (or its address).
        name: String,
    },
}

/// Where and when the game was played, for the top line.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Origin {
    /// "This computer", the gateway's name, or the file's.
    pub place: String,
    /// When it started, Unix milliseconds, where known.
    pub started: Option<u64>,
    /// The viewer's own seat ("Your seat").
    pub viewer: PlayerId,
    /// The seats whose hands their players did not share (hosted games,
    /// REPLAY-DESIGN §18): seen from such a seat its hidden cards stay
    /// hidden, and the bar says so. Local records hold every hand whole.
    pub unshared: Vec<PlayerId>,
}

impl Default for Origin {
    fn default() -> Self {
        Self {
            place: String::new(),
            started: None,
            viewer: PlayerId::new(0),
            unshared: Vec::new(),
        }
    }
}

/// A seat of the game being replayed, as the seat picker names it.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct SeatName {
    /// The seat.
    pub seat: PlayerId,
    /// What the table called it.
    pub name: String,
}

/// Where a replay stands: what the transport bar, the top line and the stop
/// sheets draw. Written into `Duel::replay` only when it differs.
#[derive(Clone, PartialEq, Debug)]
pub struct ReplayStatus {
    /// The tape's header, once there is a tape.
    pub header: Option<Header>,
    /// The step shown.
    pub step: usize,
    /// How many steps there are.
    pub steps: usize,
    /// Whether it plays on by itself.
    pub playing: bool,
    /// How fast.
    pub speed: Speed,
    /// The timeline, once there is a tape.
    pub timeline: Option<SharedTimeline>,
    /// How far the tape is the recorded game.
    pub stop: Option<StopLine>,
    /// How far the step shown is known to be the recorded game's.
    pub check: Option<Check>,
    /// Whose table it is.
    pub perspective: Perspective,
    /// The perspectives this source offers.
    pub offers: Perspectives,
    /// What a failing check does in the tape shown.
    pub verify: Verify,
    /// Still waiting for a tape (a first one, or another perspective's).
    pub loading: Option<Loading>,
    /// Why no tape came, in words a player may read.
    pub problem: Option<String>,
    /// Where and when.
    pub origin: Origin,
    /// The game's seats, once a tape named them.
    pub seats: Vec<SeatName>,
}

impl ReplayStatus {
    /// Every perspective this replay offers, in the order the picker lists
    /// them and `V` steps through them: the viewer's seat, every other
    /// seat, the table, all hands.
    #[must_use]
    pub fn choices(&self) -> Vec<Perspective> {
        let viewer = self.origin.viewer;
        let mut out = Vec::new();
        if self.offers.seat {
            out.push(Perspective::Seat(viewer));
            out.extend(
                self.seats
                    .iter()
                    .filter(|s| s.seat != viewer)
                    .map(|s| Perspective::Seat(s.seat)),
            );
        }
        if self.offers.public {
            out.push(Perspective::Public);
        }
        if self.offers.all_hands {
            out.push(Perspective::AllHands(viewer));
        }
        out
    }

    /// The perspective after the one shown, round the [`Self::choices`].
    #[must_use]
    pub fn next_choice(&self) -> Perspective {
        let choices = self.choices();
        let at = choices.iter().position(|p| *p == self.perspective);
        at.map_or(self.perspective, |i| choices[(i + 1) % choices.len()])
    }

    /// Whether the hand of the seat a perspective is drawn from is hidden
    /// here: the spectators' table, and a seat whose player did not share.
    #[must_use]
    pub fn hand_hidden(&self, perspective: Perspective) -> bool {
        // The tape shown says it itself (§18.3: `own`, whether the drawn
        // seat's hidden cards are in it); another perspective goes by the
        // list's consent.
        if perspective == self.perspective
            && perspective != Perspective::Public
            && let Some(header) = &self.header
        {
            return !header.own;
        }
        match perspective {
            Perspective::Public => true,
            Perspective::Seat(seat) | Perspective::AllHands(seat) => {
                self.origin.unshared.contains(&seat)
            }
        }
    }

    /// What the picker and the top line call a perspective.
    #[must_use]
    pub fn words(&self, perspective: Perspective, lang: baylee_client_core::Lang) -> String {
        use baylee_client_core::replay::words::ReplayPhrase;
        match perspective {
            Perspective::Seat(seat) if seat != self.origin.viewer => {
                let name = self
                    .seats
                    .iter()
                    .find(|s| s.seat == seat)
                    .map_or_else(|| format!("{}", seat.get() + 1), |s| s.name.clone());
                ReplayPhrase::SeatOf.fill(lang, &[&name])
            }
            other => perspective_words(other, lang).to_string(),
        }
    }

    /// Whether the step shown is the last.
    #[must_use]
    pub const fn at_end(&self) -> bool {
        self.steps > 0 && self.step + 1 >= self.steps
    }
}

/// What a replay is asked to do: the transport's commands, and what only
/// the host can carry out (another tape).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ReplayAsk {
    /// A transport command.
    Transport(baylee_client_core::replay::Command),
    /// Show the game from another perspective, keeping the place.
    Perspective(Perspective),
    /// The next perspective the source offers (`V`).
    NextPerspective,
    /// Play on past where this build would differ, keeping the place.
    PlayOn,
    /// Stop waiting for a tape.
    Cancel,
}

/// What a perspective is called on the pill and the top line.
#[must_use]
pub fn perspective_words(perspective: Perspective, lang: baylee_client_core::Lang) -> &'static str {
    use baylee_client_core::replay::words::ReplayPhrase;
    match perspective {
        Perspective::Seat(_) => ReplayPhrase::YourSeat.text(lang),
        Perspective::Public => ReplayPhrase::TheTableShort.text(lang),
        Perspective::AllHands(_) => ReplayPhrase::AllHands.text(lang),
    }
}

/// When a replayed game started, as the top line says it: today's and
/// yesterday's by their names, older ones by their date; local time.
#[must_use]
pub fn when_words(ms: Option<u64>, lang: baylee_client_core::Lang) -> String {
    use baylee_client_core::replay::words::ReplayPhrase;
    let Some(ms) = ms else {
        return String::new();
    };
    let Ok(utc) = time::OffsetDateTime::from_unix_timestamp(i64::try_from(ms / 1000).unwrap_or(0))
    else {
        return String::new();
    };
    let offset = time::UtcOffset::local_offset_at(utc).unwrap_or(time::UtcOffset::UTC);
    let at = utc.to_offset(offset);
    let now = web_time::SystemTime::now()
        .duration_since(web_time::UNIX_EPOCH)
        .ok()
        .and_then(|d| time::OffsetDateTime::from_unix_timestamp(d.as_secs() as i64).ok())
        .map(|now| now.to_offset(offset));
    let clock = format!("{:02}:{:02}", at.hour(), at.minute());
    match now.map(|now| (now.date() - at.date()).whole_days()) {
        Some(0) => format!("{} {clock}", ReplayPhrase::Today.text(lang)),
        Some(1) => format!("{} {clock}", ReplayPhrase::Yesterday.text(lang)),
        _ => format!(
            "{:04}-{:02}-{:02} {clock}",
            at.year(),
            u8::from(at.month()),
            at.day()
        ),
    }
}

/// The table's top line in a replay: "Replay · <where> · <when> ·
/// <perspective>" (§7.2).
#[must_use]
pub fn top_line(status: &ReplayStatus, lang: baylee_client_core::Lang) -> String {
    use baylee_client_core::replay::words::ReplayPhrase;
    let when = when_words(status.origin.started, lang);
    let line = ReplayPhrase::TopLine.fill(
        lang,
        &[
            &status.origin.place,
            &when,
            &status.words(status.perspective, lang),
        ],
    );
    // An empty part leaves no empty slot between its dots.
    line.replace(" \u{b7}  \u{b7} ", " \u{b7} ")
}
