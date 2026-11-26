//! baylee-engine-server — the process a game actually lives in.
//!
//! The gateway routes; it runs no rules. A game runs here, in a process of its
//! own that an agent started and that dialled the gateway back, and the two
//! talk over the engine link described in `docs/protocol.md`.
//!
//! ```text
//!   gateway ──> GameSetup / SeatAttached / SeatFrame ──> EngineRunner
//!   gateway <── SeatFrame / GameRecordChunk / GameEnded ─ EngineRunner
//!   gateway ──> FlushRecord ──> EngineRunner ──> RecordFlushed ──> gateway
//! ```
//!
//! [`EngineRunner`] is that whole conversation with no socket in it: frames
//! in, frames out, plus a clock the caller is expected to run. Keeping the
//! transport out means the game side can be tested without one, and lets a
//! test drive a real engine over a real link without spawning a process.
//!
//! One process per game is the panic boundary. A rules path that panics takes
//! down exactly one game, and the agent reports the exit — where a gateway
//! hosting sessions in-process had to catch unwinds to get the same effect.
#![deny(unsafe_code)]
#![warn(missing_docs)]

// The tests look cards up, so their binary holds the pool.
#[cfg(test)]
use baylee_cards_pool as _;

use baylee_core::ids::{PlayerId, SeatSet};
use baylee_core::preset::GamePreset;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::{DetectLoops, LoopDetection};
use baylee_gamehost::session::SessionRegistry;
use baylee_gamehost::{RegistryLookup, SeatKind, Session};
use baylee_protocol::v1::{self, Envelope};
use baylee_view::SeatSetting;

mod challenge;
mod clocks;
mod driver;
pub mod foresight;
mod frames;
pub mod projection;
#[cfg(all(feature = "loop-detection", not(feature = "tower")))]
pub mod ranked;
pub mod recording;
mod startup;
#[cfg(any(test, feature = "tower"))]
pub mod tower;
pub mod variant;

pub use clocks::{Armed, Clock, Deadline, HOLD_SECS};
pub use frames::{curtain, seat_frame};
use frames::{ended, error, seat_frames};

/// Default maximum house steps answered in one synchronous call to
/// [`EngineRunner`]; [`EngineRunner::with_pump_budget`] lowers this on an
/// event loop so incoming control frames and incremental state deltas are
/// interleaved without blocking the link.
pub const DEFAULT_PUMP_BUDGET: usize = 4096;

/// Slice budget for an attached engine's async event loop: small enough that
/// `ClockProbe`, `FlushRecord`, `SeatAttached`, `SeatSetting` and priority
/// holds are answered within milliseconds even under maximum AI/combo load,
/// while streaming live `StateDelta` progress after each slice.
pub const ATTACHED_PUMP_SLICE: usize = 128;

/// How long a table waits for preparation before failing explicitly.
/// A slow or missing seat never causes an unready table to open.
pub const CURTAIN_SECS: u32 = 120;

/// How much of the game's record (#315), uncompressed, gathers before it is
/// sent to the gateway as a [`v1::GameRecordChunk`]. A two-seat game of the
/// acceptance decks writes 100–190 KiB, so a handful of pieces a game; an
/// engine that dies loses at most this much.
pub const RECORD_CHUNK_BYTES: usize = 32 * 1024;

/// How long, in milliseconds of wall time, record that has gathered may wait
/// for [`RECORD_CHUNK_BYTES`] before it is sent anyway (#323): so a game that
/// is lost with its engine, or reported on after that, has its record in the
/// gateway's store up to two seconds before, which is also as far back as a
/// game resumed after a crash goes (`docs/proposals/games-survive-restarts.md`;
/// it was 30 s until then). Counted from when the bytes
/// waiting began to wait ([`EngineRunner::record_deadline`]), not from the
/// last piece, so a quiet game sends no piece per action. A bug report
/// filed at a game that goes on does not wait for it: it asks for the record
/// there and then ([`v1::FlushRecord`]).
pub const RECORD_FLUSH_MS: u64 = 2_000;

/// The most answers the house gives alone, once every seat that answers over
/// a socket has left, before the game is closed with no winners
/// ([`EngineRunner::remainder_bound`]). A house-only remainder plays by the
/// heuristics alone (no search), so this bounds what a game everybody left
/// can still cost. Measured 12.11.2026 (release build, a person conceding at
/// turn 4 against `expert` houses): two houses finished in 760 to 990
/// answers (0.1 s), three in 4,045 (0.9 s), seven in 15,000 to 25,000
/// (4.6 to 10 s of a core). So the remainders of the tables people mostly
/// play are played out, and the result stands; an eight-chair one is cut at
/// about two seconds.
pub const REMAINDER_ANSWERS: u64 = 5_000;

/// The most wall time, in milliseconds as the engine is told it, the house
/// plays alone before the game is closed ([`REMAINDER_ANSWERS`]'s other
/// half): a remainder of large boards, whose answers each cost more, is cut
/// by this.
pub const REMAINDER_MS: u64 = 20_000;

/// Whether `pending` bytes of the record, uncompressed, are a piece to send
/// while the game goes on: [`RECORD_CHUNK_BYTES`] or more.
const fn record_due(pending: usize) -> bool {
    pending >= RECORD_CHUNK_BYTES
}

/// The curtain while it is down (#256): the seats it waits for, and which of
/// them have said they are ready.
///
/// Two sets rather than one that shrinks, so a `SeatReady` from a seat it
/// never waited for (an AI chair a socket drives) is not counted, and one
/// from a seat that already said it is counted once. Readiness bookkeeping
/// uses two bitmasks; only the outgoing status frames allocate.
#[derive(Clone, Copy, Debug)]
struct Barrier {
    /// Every seat that answers over a socket, as the game was built.
    waits_for: SeatSet,
    /// Those of them that have drawn their table.
    ready: SeatSet,
}

impl Barrier {
    /// Whether every seat it waits for is ready.
    fn met(self) -> bool {
        self.waits_for.iter().all(|seat| self.ready.contains(seat))
    }
}

/// A game, and everything the gateway can ask of it.
pub struct EngineRunner<
    D: LoopDetection = DetectLoops,
    R: SessionRegistry = RegistryLookup,
    T: recording::Transport = recording::Legacy,
> {
    /// The game this process was started for.
    game_id: String,
    /// Private execution policy from the assigned gateway preset.
    ranked_duel: bool,
    /// Set by `GameSetup`; every other frame is ignored until then.
    session: Option<Session<D, R>>,
    /// Which seats have a live socket. The clock only runs for a seat that
    /// can answer.
    attached: Vec<u8>,
    /// How many spectator sockets the gateway says are open. Spectators
    /// are no seat: not in `attached`, never waited for by the curtain,
    /// never on a clock (`docs/protocol.md` §"Spectators").
    spectators: u32,
    /// Seats whose connection was lost (not left on purpose) and that have
    /// not come back, so their return can be told.
    lost: Vec<u8>,
    /// Whether `GameEnded` has already been reported.
    ended: bool,
    /// The gateway is stopping and asked for the record with `hold`
    /// ([`v1::FlushRecord::hold`]): nothing moves the game any more, so the
    /// record it was sent is the whole game a restart resumes.
    held: bool,
    /// Terminal seats already reported to the gateway, independently of game end.
    participation_reported: SeatSet,
    /// The next [`v1::GameRecordChunk::seq`] (#315).
    record_seq: u32,
    records: T,
    /// Since when (wall ms, as last told) the record bytes not yet sent have
    /// been waiting; `None` when nothing was seen waiting since the last
    /// piece (#323). What [`RECORD_FLUSH_MS`] counts from.
    record_waiting_since: Option<u64>,
    /// When (wall ms) the last piece went, or the game was built: where
    /// [`RECORD_FLUSH_MS`] counts from for bytes nobody saw begin to wait.
    record_sent_at: u64,
    /// The curtain, while it is down; `None` before the game is built and
    /// from the moment it goes up, which is for good: nothing brings it
    /// down again (#256).
    curtain: Option<Barrier>,
    /// What the caller last read off its own timers: each armed clock, and
    /// how much of it was left.
    ///
    /// Kept rather than passed straight through because it is read once per
    /// frame but resolved more than once: registering a socket can move an
    /// awaited seat from the reconnect window onto the decision clock, and
    /// that happens after the frame has arrived and before any view is built.
    readings: Vec<(Clock, u32)>,
    /// The wall time as the caller last told it ([`Self::tell_time`]).
    now: u64,
    /// Shared portal departure, cancelled if a prepared socket drops.
    enter_at: Option<u64>,
    /// Maximum house steps answered per synchronous call before yielding
    /// intermediate views back to the caller's event loop.
    pump_budget: usize,
    /// What the house's search may spend at this game's table, or `None`
    /// where it does not search ([`Self::with_foresight`]).
    foresight: Option<baylee_gamehost::HostForesight>,
    /// The house's answer count ([`Session::decision_seq`]) and the time (as
    /// told) when the last seat that answers over a socket left, while
    /// nobody has come back.
    remainder_from: Option<(u64, u64)>,
    /// The most answers the house gives alone before the game is closed
    /// ([`REMAINDER_ANSWERS`], [`Self::remainder_bound`]).
    remainder_answers: u64,
    /// Private challenge evidence; never a field in a player's game view.
    challenge: challenge::Evidence,
}

impl<D: LoopDetection, R: SessionRegistry, T: crate::recording::Transport> Default
    for EngineRunner<D, R, T>
{
    fn default() -> Self {
        Self {
            game_id: String::new(),
            ranked_duel: false,
            session: None,
            attached: Vec::new(),
            spectators: 0,
            lost: Vec::new(),
            ended: false,
            held: false,
            participation_reported: SeatSet::new(),
            record_seq: 0,
            records: T::default(),
            record_waiting_since: None,
            record_sent_at: 0,
            curtain: None,
            readings: Vec::new(),
            now: 0,
            enter_at: None,
            pump_budget: DEFAULT_PUMP_BUDGET,
            foresight: None,
            remainder_from: None,
            remainder_answers: REMAINDER_ANSWERS,
            challenge: challenge::Evidence::default(),
        }
    }
}

impl EngineRunner {
    /// A runner with no game in it yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }
}

#[cfg(test)]
mod tests;
