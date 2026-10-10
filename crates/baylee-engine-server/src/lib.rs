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

#![warn(missing_docs)]

use std::io::Write as _;

use baylee_core::ids::{PlayerId, SeatSet};
use baylee_core::preset::GamePreset;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_gamehost::{SeatKind, Session};
use baylee_protocol::v1::{self, Envelope};
use baylee_view::SeatSetting;
use flate2::Compression;
use flate2::write::GzEncoder;

mod clocks;
mod frames;

pub use clocks::{Armed, Clock, Deadline, HOLD_SECS};
pub use frames::{curtain, seat_frame};
use frames::{ended, error};

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
/// gateway's store up to half a minute before. Counted from when the bytes
/// waiting began to wait ([`EngineRunner::record_deadline`]), not from the
/// last piece, so a quiet game sends no piece per action. A bug report
/// filed at a game that goes on does not wait for it: it asks for the record
/// there and then ([`v1::FlushRecord`]).
pub const RECORD_FLUSH_MS: u64 = 30_000;

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
#[derive(Default)]
pub struct EngineRunner {
    /// The game this process was started for.
    game_id: String,
    /// Set by `GameSetup`; every other frame is ignored until then.
    session: Option<Session>,
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
    /// The next [`v1::GameRecordChunk::seq`] (#315).
    record_seq: u32,
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
}

impl EngineRunner {
    /// A runner with no game in it yet.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Tells the game the wall time, in milliseconds since the Unix epoch,
    /// which its log stamps each line with (#300). The caller tells it before
    /// every call that can move the game: the runner reads no clock, for the
    /// reason [`Self::timeout`] gives.
    pub fn tell_time(&mut self, unix_ms: u64) {
        self.now = unix_ms;
        if let Some(session) = self.session.as_mut() {
            session.tell_time(unix_ms);
        }
    }

    /// Whether the game has been built.
    #[must_use]
    pub fn ready(&self) -> bool {
        self.session.is_some()
    }

    /// Whether the table has been built and not yet opened (#256): what
    /// the caller arms [`CURTAIN_SECS`] against, and what it disarms on.
    ///
    /// Nothing is decided while this holds. No seat is asked a question, an
    /// action that arrives is dropped, the house plays no AI chair, and no
    /// clock runs.
    #[must_use]
    pub const fn curtain_pending(&self) -> bool {
        self.curtain.is_some()
    }

    /// Whether the game is over and the process may exit.
    #[must_use]
    pub fn finished(&self) -> bool {
        self.ended
    }

    /// The game underneath, for tests and for a harness that wants to look.
    #[must_use]
    pub fn session(&self) -> Option<&Session> {
        self.session.as_ref()
    }

    /// What is on the clock: one clock per seat being asked, if any.
    ///
    /// A seat being asked is on exactly one of the two: the decision clock if
    /// it can see the question, the reconnect window if its socket is gone. A
    /// player who walked away is not on a clock they cannot see — but the
    /// table is not left waiting on them forever either, which is the half
    /// that was missing.
    ///
    /// Nothing is on either clock when nobody is being asked, when the table
    /// set that limit to zero, or when the awaited seat is an AI chair — it
    /// never had a socket to lose, so its absence means nothing. Nor before
    /// the curtain is up (#256): every clock starts when the table opens, so
    /// no seat spends its decision time or its reconnect window on another
    /// seat's loading. During the
    /// opening mulligans several seats are asked at once, each on its own
    /// clock, and one seat's answer leaves every other seat's clock exactly
    /// as it was.
    #[must_use]
    pub fn clocks(&self) -> Vec<Clock> {
        let Some(session) = self.session.as_ref() else {
            return Vec::new();
        };
        let mut clocks: Vec<Clock> = session
            .awaited()
            .iter()
            .filter_map(|seat| self.clock_for(session, seat))
            .collect();
        clocks.extend(self.hold_clock(session));
        clocks
    }

    /// Whether a player other than `seat` is at the table: a chair answered
    /// over a socket, with its socket open. What decides between waiting
    /// for a lost seat (someone is here to wait) and pausing the game
    /// (nobody is).
    fn another_player_here(&self, session: &Session, seat: Option<PlayerId>) -> bool {
        self.attached.iter().any(|&other| {
            Some(other) != seat.map(PlayerId::get)
                && session
                    .seat_kind(PlayerId::new(other))
                    .is_some_and(SeatKind::answers_over_socket)
        })
    }

    /// A player's chair still in the game whose socket is gone.
    fn absent_players(&self, session: &Session) -> Vec<PlayerId> {
        (0..session.seat_count())
            .filter_map(|seat| u8::try_from(seat).ok())
            .map(PlayerId::new)
            .filter(|&seat| !self.attached.contains(&seat.get()))
            .filter(|&seat| {
                matches!(
                    session.seat_kind(seat),
                    Some(SeatKind::Human | SeatKind::StandIn(_))
                )
            })
            .filter(|&seat| !session.has_left(seat))
            .collect()
    }

    /// The table's hold, while nobody is at it ([`Deadline::Hold`]).
    fn hold_clock(&self, session: &Session) -> Option<Clock> {
        if self.curtain.is_some() || self.ended || self.another_player_here(session, None) {
            return None;
        }
        if matches!(session.pending(), Pending::GameOver(_)) {
            return None;
        }
        let seat = *self.absent_players(session).first()?;
        Some(Clock {
            seat,
            what: Deadline::Hold,
            seq: 0,
            secs: HOLD_SECS,
        })
    }

    /// `seat`'s clock, if it is on one.
    fn clock_for(&self, session: &Session, seat: PlayerId) -> Option<Clock> {
        if self.curtain.is_some() {
            return None;
        }
        let (what, secs) = if self.attached.contains(&seat.get()) {
            (Deadline::Decide, session.decision_timeout_secs())
        } else if session
            .seat_kind(seat)
            .is_some_and(SeatKind::answers_over_socket)
            // Nobody else at the table: nobody is waiting, so the game is
            // paused rather than handed to the house (owner, 10.10.2026).
            && self.another_player_here(session, Some(seat))
        {
            (Deadline::StandIn, self.stand_in_secs(session, seat))
        } else {
            return None;
        };
        if secs == 0 {
            return None;
        }
        Some(Clock {
            seat,
            what,
            seq: session.asked_at(seat)?,
            secs,
        })
    }

    /// What is left of `seat`'s reconnect window, in whole seconds rounded
    /// up: counted from the **loss** ([`Session::reconnect_deadline_ms`]),
    /// not from the question the seat is asked next, so a player lost
    /// between questions is waited for the window once and every view's
    /// countdown ([`baylee_view::PlayerView::lost`]) names the same moment.
    /// At least one second, because zero means no clock at all: a window
    /// already past fires on the next turn of the loop. A seat with no
    /// recorded loss (lost before the curtain, or while the game was paused)
    /// gets the whole window from its question.
    fn stand_in_secs(&self, session: &Session, seat: PlayerId) -> u32 {
        let window = session.reconnect_window_secs();
        if window == 0 {
            return 0;
        }
        session.reconnect_deadline_ms(seat).map_or(window, |until| {
            let left = until.saturating_sub(self.now).div_ceil(1_000);
            u32::try_from(left).unwrap_or(window).clamp(1, window)
        })
    }

    /// The clock ran out. Acts for the seat, and only for that seat and the
    /// question it was armed for — the seat may have answered between the
    /// timer firing and this being called, and one seat's expired clock must
    /// never take another seat's decision, nor this seat's next one.
    pub fn timeout(&mut self, clock: Clock) -> Vec<Envelope> {
        if self.curtain.is_some() {
            return Vec::new();
        }
        if clock.what == Deadline::Hold {
            return self.hold_expired();
        }
        let Some(session) = self.session.as_mut() else {
            return Vec::new();
        };
        if session.asked_at(clock.seat) != Some(clock.seq) {
            return Vec::new();
        }
        match clock.what {
            // One answer, because the seat is there and simply took too long
            // over this question.
            // Through the session's clock door rather than `apply`, so the
            // views sent on the way out say the clock answered this one.
            Deadline::Decide => match session.answer_by_clock(clock.seat, clock.seq) {
                None => Vec::new(),
                Some(Ok(routed)) => {
                    let mut out = self.route(&routed);
                    out.extend(self.ending());
                    out
                }
                Some(Err(reason)) => self.refused(clock.seat, &reason),
            },
            // A change of who answers, because the seat is not there at all.
            // The same race as above, in the other direction: the socket may
            // have come back between the timer firing and this being called,
            // and a player at the table must not have their chair taken.
            Deadline::Hold => Vec::new(),
            Deadline::StandIn => {
                if self.attached.contains(&clock.seat.get()) {
                    return Vec::new();
                }
                if !session.stand_in(clock.seat) {
                    return Vec::new();
                }
                let routed = session.pump();
                let mut out = self.route(&routed);
                out.extend(self.ending());
                out
            }
        }
    }

    /// Hands the session the decision clock's remainder, so the views it is
    /// about to build can carry it.
    ///
    /// This is where the four cases that have no clock are decided, and it is
    /// here rather than in the session because three of the four are things
    /// only this side knows. [`EngineRunner::clock`] already answers all of
    /// them — nobody being asked, a zero allowance, an AI chair — and the
    /// fourth is the one this function adds: a [`Deadline::StandIn`] is not a
    /// decision clock. The seat it belongs to has no socket and is being
    /// waited *for* rather than deciding, and a countdown drawn against it on
    /// everyone else's screen would name the wrong thing happening.
    ///
    /// With nothing read for exactly this clock, the seat gets the allowance
    /// whole, which is what a question nobody has spent time on is worth. A
    /// reading of the reconnect window is not one: a seat that has just come
    /// back is armed a fresh decision clock, and shown that.
    fn absorb_clock(&mut self) {
        let Some(session) = self.session.as_ref() else {
            return;
        };
        let resolved: Vec<(PlayerId, u64, Option<u32>)> = session
            .awaited()
            .iter()
            .filter_map(|seat| {
                let asked_at = session.asked_at(seat)?;
                let remaining = self
                    .clock_for(session, seat)
                    .filter(|clock| clock.what == Deadline::Decide)
                    .map(|clock| {
                        self.readings
                            .iter()
                            .find(|(armed, _)| *armed == clock)
                            .map_or_else(|| clock.secs.saturating_mul(1_000), |(_, ms)| *ms)
                    });
                Some((seat, asked_at, remaining))
            })
            .collect();
        if let Some(session) = self.session.as_mut() {
            for (seat, asked_at, remaining) in resolved {
                session.set_decision_remaining(seat, asked_at, remaining);
            }
        }
    }

    /// Applies one frame from the gateway and returns what to send back.
    ///
    /// `remaining` is how much of each **currently armed** deadline is left,
    /// read from the caller's clocks a moment ago, and empty if the caller
    /// has nothing armed. It is a parameter rather than something the
    /// runner reads for itself because the `Instant` lives with whoever runs
    /// the timer, and it is a parameter rather than an optional setter
    /// because a caller that forgot it would produce views whose countdown
    /// silently restarts on every frame.
    ///
    /// It matters for exactly one shape, and that shape is the point of the
    /// ticket: a seat that **reconnects** in the middle of a question is sent
    /// a fresh snapshot, and must be shown the twenty seconds it has left
    /// rather than the ten minutes it started with. Every other frame either
    /// moves the game — in which case the question is new and the allowance
    /// is whole — or is answered the same way by both.
    pub fn handle(&mut self, envelope: Envelope, remaining: &[(Clock, u32)]) -> Vec<Envelope> {
        self.readings = remaining.to_vec();
        self.absorb_clock();
        match envelope.msg {
            Some(v1::envelope::Msg::GameSetup(setup)) => self.setup(&setup),
            Some(v1::envelope::Msg::SeatAttached(attached)) => self.attach(attached),
            Some(v1::envelope::Msg::SeatDetached(detached)) => self.detach(detached),
            Some(v1::envelope::Msg::SeatFrame(frame)) => self.seat_frame(&frame),
            Some(v1::envelope::Msg::SpectatorsChanged(changed)) => self.spectators_changed(changed),
            Some(v1::envelope::Msg::FlushRecord(flush)) => self.flush_record(flush.nonce),
            _ => Vec::new(),
        }
    }

    /// The gateway asked for the record as it stands (#323), for a bug
    /// report filed at this game: whatever of it has gathered, as a piece
    /// that is not `last`, and then [`v1::RecordFlushed`] with the gateway's
    /// `nonce`, on the same socket and so after the piece.
    ///
    /// Answered whatever the state: with nothing waiting, before the game is
    /// built, and after it ended, the answer is the acknowledgement alone,
    /// never an empty piece (it would cost the store a row and a `seq` for
    /// nothing). An unanswered ask costs the reporter the gateway's whole
    /// wait.
    ///
    /// Moves nothing in the game: no pump, no clock, no frame to a seat.
    fn flush_record(&mut self, nonce: u64) -> Vec<Envelope> {
        let mut out = Vec::new();
        if !self.ended {
            let data = self
                .session
                .as_mut()
                .map(Session::take_record)
                .unwrap_or_default();
            if !data.is_empty() {
                out.push(self.record_chunk(&data, false));
            }
        }
        out.push(Envelope {
            msg: Some(v1::envelope::Msg::RecordFlushed(v1::RecordFlushed {
                game_id: self.game_id.clone(),
                nonce,
                pieces: self.record_seq,
            })),
        });
        out
    }

    /// When (wall ms) the record waiting to be sent is due by
    /// [`RECORD_FLUSH_MS`] (#323), for the caller to arm a timer at and then
    /// call [`Self::flush_record_due`]; `None` when nothing waits, before
    /// the game is built, after it ended, and while the curtain is down.
    ///
    /// Not while the curtain is down, because a seat that is loading should
    /// have the database to itself (see the gateway's `record::Sink`); the
    /// first move after the curtain rises finds the header overdue.
    #[must_use]
    pub fn record_deadline(&self) -> Option<u64> {
        if self.ended || self.curtain.is_some() {
            return None;
        }
        let pending = self.session.as_ref()?.record_pending();
        (pending > 0).then(|| {
            self.record_waiting_since
                .unwrap_or(self.record_sent_at)
                .saturating_add(RECORD_FLUSH_MS)
        })
    }

    /// The record that has waited [`RECORD_FLUSH_MS`], as a piece that is
    /// not `last` (#323); nothing when none has, or when the time told
    /// ([`Self::tell_time`]) is not yet [`Self::record_deadline`]. Early
    /// and stale timers do nothing.
    pub fn flush_record_due(&mut self) -> Vec<Envelope> {
        if self.record_deadline().is_none_or(|at| self.now < at) {
            return Vec::new();
        }
        let Some(session) = self.session.as_mut() else {
            return Vec::new();
        };
        let data = session.take_record();
        vec![self.record_chunk(&data, false)]
    }

    /// Builds the game.
    fn setup(&mut self, setup: &v1::GameSetup) -> Vec<Envelope> {
        if self.session.is_some() {
            return Vec::new();
        }
        let Ok(preset) = serde_json::from_slice::<GamePreset>(&setup.preset_json) else {
            return vec![ended(&setup.game_id, "the preset did not decode")];
        };
        let Some(mut session) = Session::new_recorded(&preset, baylee_build::short()) else {
            return vec![ended(&setup.game_id, "the preset does not make a game")];
        };
        session.describe(setup.game_id.clone(), setup.seat_names.clone());
        session.tell_time(self.now);
        self.game_id.clone_from(&setup.game_id);
        // The header is waiting from here.
        self.record_sent_at = self.now;
        self.record_waiting_since = Some(self.now);
        // Down until every seat that answers over a socket has drawn its
        // table. An expired preparation fails explicitly. An AI chair is
        // ready from the start and never holds the barrier.
        let waits_for: SeatSet = session.human_seats().into_iter().collect();
        self.curtain = (!waits_for.is_empty()).then_some(Barrier {
            waits_for,
            ready: SeatSet::new(),
        });
        self.session = Some(session);
        Vec::new()
    }

    /// Opens the table once, after the scheduled entrance. Also available
    /// to in-process harnesses that deliberately bypass presentation.
    ///
    /// The house plays its chairs in the pump that follows, which is the
    /// first moment anything is decided, and every attached seat is sent
    /// what that pump produced and then [`curtain`], last, so a client opens
    /// on the table as it stands afterwards. A seat that attaches later is
    /// sent the same at the end of its own batch. The clocks start here too,
    /// because this is the first moment [`EngineRunner::clocks`] names any.
    pub fn raise_curtain(&mut self) -> Vec<Envelope> {
        if self.curtain.take().is_none() {
            return Vec::new();
        }
        self.enter_at = None;
        // The clocks exist from here on, and the views below are the first
        // to carry them.
        self.absorb_clock();
        let Some(session) = self.session.as_mut() else {
            return Vec::new();
        };
        let routed = session.pump();
        let mut out = self.route(&routed);
        out.extend(
            self.attached
                .iter()
                .map(|&seat| seat_frame(seat, &curtain())),
        );
        out.extend(self.ending());
        out
    }

    /// Scheduled arrival, at which play and its clocks may begin.
    #[must_use]
    pub fn entrance_deadline(&self) -> Option<u64> {
        self.enter_at
            .map(|at| at.saturating_add(baylee_protocol::TABLE_ENTRANCE_MS))
    }

    /// Called by the transport timer. Early and cancelled timers do nothing.
    pub fn finish_entrance(&mut self) -> Vec<Envelope> {
        if self.entrance_deadline().is_some_and(|at| self.now >= at) {
            self.raise_curtain()
        } else {
            Vec::new()
        }
    }

    /// Preparation failed. Never start decision clocks for an unready seat.
    pub fn preparation_expired(&mut self) -> Vec<Envelope> {
        if self.curtain.is_none() || self.enter_at.is_some() {
            return Vec::new();
        }
        self.ended = true;
        let reason = "The table could not start: not every player finished loading. Return to the lobby and try again.";
        let mut out: Vec<_> = self
            .attached
            .iter()
            .map(|&seat| seat_frame(seat, &error(reason)))
            .collect();
        out.push(ended(&self.game_id, reason));
        out
    }

    fn loading_status(&self) -> Vec<Envelope> {
        let Some(barrier) = self.curtain else {
            return Vec::new();
        };
        let status = Envelope {
            msg: Some(v1::envelope::Msg::TableLoading(v1::TableLoading {
                ready: barrier.ready.iter().count() as u32,
                total: barrier.waits_for.iter().count() as u32,
                enter_at_ms: self.enter_at.unwrap_or(0),
            })),
        };
        self.attached
            .iter()
            .map(|&seat| seat_frame(seat, &status))
            .collect()
    }

    /// Count only attached human seats, once. All ready schedules a flight;
    /// it does not pump the game or start a decision clock.
    fn seat_ready(&mut self, player: PlayerId) -> Vec<Envelope> {
        let Some(barrier) = self.curtain.as_mut() else {
            return Vec::new();
        };
        if !self.attached.contains(&player.get())
            || !barrier.waits_for.contains(player)
            || barrier.ready.contains(player)
        {
            return Vec::new();
        }
        barrier.ready.insert(player);
        if barrier.met() {
            self.enter_at = Some(
                self.now
                    .saturating_add(baylee_protocol::TABLE_ENTRANCE_LEAD_MS),
            );
        }
        self.loading_status()
    }

    /// A seat's socket opened. Sends it the payload every later frame refers
    /// to, then either advances the game for it or hands it what it missed.
    fn attach(&mut self, attached: v1::SeatAttached) -> Vec<Envelope> {
        let Ok(seat) = u8::try_from(attached.seat) else {
            return Vec::new();
        };
        let player = PlayerId::new(seat);
        if self.session.is_none() {
            return Vec::new();
        }
        // A fresh snapshot must earn a fresh render acknowledgement, also
        // when the gateway requests resynchronization on the same socket.
        // Only then has the count the other seats hold changed.
        let mut recounted = false;
        if let Some(barrier) = self.curtain.as_mut() {
            recounted = barrier.ready.contains(player) || self.enter_at.is_some();
            barrier.ready = barrier.ready.iter().filter(|p| *p != player).collect();
            self.enter_at = None;
        }
        if !self.attached.contains(&seat) {
            self.attached.push(seat);
        }
        // Once more, now that the socket is registered. A seat that was on
        // the reconnect window a line ago is on the decision clock as of
        // this moment, and every view below is built after it — including
        // the snapshot a resyncing player is about to be sent, which is the
        // one view in the whole system that exists to tell a returning seat
        // where it stands.
        self.absorb_clock();
        let Some(session) = self.session.as_mut() else {
            return Vec::new();
        };
        // The player is back, so the chair is theirs again — before the
        // roster below is built, or the payload that says who is at the table
        // would be the one payload still saying they are not. It comes first
        // in the *other* order too: `hand_back` marks every seat's roster as
        // out of date, and this seat's is cleared by sending it.
        if !session.hand_back(player) && self.lost.contains(&seat) && self.curtain.is_none() {
            session.connection_back(player);
        }
        self.lost.retain(|s| *s != seat);
        // A new socket says anew what answers the seat (`v1::SeatMind`);
        // until it does, the record does not credit it with what the last
        // one said. A resync is the same socket.
        if !attached.resync {
            session.socket_opened(player);
        }
        // The roster and the print table go first: everything after this
        // points into them, and a seat earns printings as it sees cards, so
        // this is not a payload that "cannot have changed".
        let mut out = vec![seat_frame(seat, &session.game_static_envelope(player))];
        if self.spectators > 0 {
            out.push(seat_frame(seat, &frames::spectators(self.spectators)));
        }
        // Before the curtain the seat is given its table to draw and nothing
        // to answer: pumping here would let the house play its chairs while
        // the other seats are still loading (#256).
        if self.curtain.is_some() {
            // A fresh socket holds no log either, even one that opened
            // before (a loader that dropped and dialled again).
            session.retell_log(player);
            out.extend(session.show(player).iter().map(|env| seat_frame(seat, env)));
            let status = self.loading_status();
            out.extend(
                status
                    .into_iter()
                    .filter(|env| {
                        recounted
                            || matches!(&env.msg, Some(v1::envelope::Msg::SeatFrame(f)) if f.seat == u32::from(seat))
                    }),
            );
            return out;
        }
        if attached.resync {
            out.extend(
                session
                    .snapshot(player)
                    .iter()
                    .map(|env| seat_frame(seat, env)),
            );
        } else {
            // A socket that just opened holds none of the game log, and whatever
            // went to this seat while nobody was on it was dropped in `route`.
            session.retell_log(player);
            let routed = session.pump();
            out.extend(self.route(&routed));
        }
        // The table is open, and has been since before this seat arrived:
        // it is told so last, after what it opens on.
        out.push(seat_frame(seat, &curtain()));
        out.extend(self.ending());
        out
    }

    /// The gateway's count of spectator sockets moved. Every seat is told
    /// the number; a spectator who has just arrived is sent the whole
    /// public table, with the curtain up at once: a spectator is never
    /// waited for. Moves nothing in the game, so no pump and no clock.
    fn spectators_changed(&mut self, changed: v1::SpectatorsChanged) -> Vec<Envelope> {
        self.spectators = changed.count;
        let Some(session) = self.session.as_mut() else {
            return Vec::new();
        };
        session.set_spectators(changed.count);
        let mut out: Vec<Envelope> = self
            .attached
            .iter()
            .map(|&seat| seat_frame(seat, &frames::spectators(changed.count)))
            .collect();
        if changed.joined {
            out.extend(
                session
                    .spectator_snapshot()
                    .iter()
                    .map(frames::spectator_frame),
            );
            out.push(frames::spectator_frame(&curtain()));
        }
        out
    }

    /// A seat's socket is gone (`docs/protocol.md` §"Leaving, and losing the
    /// connection").
    ///
    /// A lost connection moves nothing: the table is told, and waits
    /// ([`Deadline::StandIn`] while another player is here) or pauses
    /// ([`Deadline::Hold`] when nobody is). A deliberate leave
    /// (`left`) hands the chair to the house at once, and ends the game
    /// when no player's chair is left answered by a player.
    fn detach(&mut self, detached: v1::SeatDetached) -> Vec<Envelope> {
        let Ok(seat) = u8::try_from(detached.seat) else {
            return Vec::new();
        };
        let was_here = self.attached.contains(&seat);
        self.attached.retain(|s| *s != seat);
        if let Some(barrier) = self.curtain.as_mut() {
            barrier.ready = barrier.ready.iter().filter(|p| p.get() != seat).collect();
            self.enter_at = None;
            if !detached.left {
                return self.loading_status();
            }
        }
        if self.ended {
            return Vec::new();
        }
        let player = PlayerId::new(seat);
        let here = self.another_player_here_now(player);
        let Some(session) = self.session.as_mut() else {
            return Vec::new();
        };
        if detached.left {
            self.lost.retain(|s| *s != seat);
            session.stand_in(player);
            let anyone = (0..session.seat_count())
                .filter_map(|s| u8::try_from(s).ok())
                .map(PlayerId::new)
                .any(|s| {
                    !session.has_left(s)
                        && session
                            .seat_kind(s)
                            .is_some_and(SeatKind::answers_over_socket)
                });
            let routed = if anyone {
                // Before the curtain the house plays nothing (#256): the
                // chair is simply the house's when the table opens.
                if self.curtain.is_some() {
                    return self.loading_status();
                }
                session.pump()
            } else {
                // Nobody's own chair is left: the house concedes for every
                // player it is holding, and the game ends.
                let away: Vec<PlayerId> = (0..session.seat_count())
                    .filter_map(|s| u8::try_from(s).ok())
                    .map(PlayerId::new)
                    .filter(|&s| matches!(session.seat_kind(s), Some(SeatKind::StandIn(_))))
                    .collect();
                let mut routed = Vec::new();
                for s in away {
                    routed.extend(session.concede_for_absent(s).unwrap_or_default());
                }
                routed
            };
            let mut out = self.route(&routed);
            out.extend(self.ending());
            return out;
        }
        if !was_here {
            return Vec::new();
        }
        let wait = here.then(|| session.reconnect_window_secs());
        if !session.connection_lost(player, wait) {
            return Vec::new();
        }
        if !self.lost.contains(&seat) {
            self.lost.push(seat);
        }
        // Nothing moves: every seat still here is shown the line.
        let routed = session.pump();
        self.route(&routed)
    }

    /// [`Self::another_player_here`] against the session as it stands.
    fn another_player_here_now(&self, seat: PlayerId) -> bool {
        self.session
            .as_ref()
            .is_some_and(|session| self.another_player_here(session, Some(seat)))
    }

    /// The table was held a whole [`HOLD_SECS`] with nobody at it: the house
    /// concedes for every absent player, and the game ends.
    fn hold_expired(&mut self) -> Vec<Envelope> {
        let Some(session) = self.session.as_ref() else {
            return Vec::new();
        };
        if self.ended || self.another_player_here(session, None) {
            return Vec::new();
        }
        let away = self.absent_players(session);
        let Some(session) = self.session.as_mut() else {
            return Vec::new();
        };
        let mut routed = Vec::new();
        for seat in away {
            routed.extend(session.concede_for_absent(seat).unwrap_or_default());
        }
        let mut out = self.route(&routed);
        out.extend(self.ending());
        out
    }

    /// Tags routed envelopes for the seats that can actually receive them.
    ///
    /// A seat with no socket is dropped here rather than one hop later at the
    /// gateway, and that is what keeps a seat's own opening payload first on
    /// its wire: the frames another seat's attach produced for a player who
    /// had not arrived yet are gone before they can overtake it. Nothing is
    /// lost by it — every attach pumps, and a pump re-sends the current view
    /// to every seat that is present.
    fn route(&self, routed: &[(PlayerId, Envelope)]) -> Vec<Envelope> {
        routed
            .iter()
            .filter_map(|(p, env)| {
                if *p == baylee_gamehost::view::SPECTATOR {
                    (self.spectators > 0).then(|| frames::spectator_frame(env))
                } else {
                    self.attached
                        .contains(&p.get())
                        .then(|| seat_frame(p.get(), env))
                }
            })
            .collect()
    }

    /// A player-facing frame from one seat.
    fn seat_frame(&mut self, frame: &v1::SeatFrame) -> Vec<Envelope> {
        let Ok(seat) = u8::try_from(frame.seat) else {
            return Vec::new();
        };
        let player = PlayerId::new(seat);
        // A frame in no chair's name (a spectator is none) says nothing:
        // a spectator answers nothing, and the rules are never asked.
        if self
            .session
            .as_ref()
            .is_some_and(|session| usize::from(seat) >= session.state().players.len())
        {
            return Vec::new();
        }
        let Ok(inner) = <Envelope as prost::Message>::decode(&frame.envelope[..]) else {
            return Vec::new();
        };
        match inner.msg {
            Some(v1::envelope::Msg::ClockProbe(probe)) => vec![seat_frame(
                seat,
                &Envelope {
                    msg: Some(v1::envelope::Msg::ClockProbe(v1::ClockProbe {
                        client_time_ms: probe.client_time_ms,
                        server_time_ms: self.now,
                    })),
                },
            )],
            Some(v1::envelope::Msg::SeatReady(_)) => self.seat_ready(player),
            Some(v1::envelope::Msg::AiLog(ai_log)) => self.reasoning(player, ai_log),
            // What answers the seat, as its client says, for the record
            // only (`docs/protocol.md` §"Who answers a seat, as it says").
            // Not a move: no view, pump or clock, and taken before the
            // curtain as after. A refused one is dropped without a word:
            // an `Error` on a seat's socket reads as its answer refused,
            // and the client sends only what passes the same check. Its
            // text is never logged.
            Some(v1::envelope::Msg::SeatMind(said)) => {
                if let Some(session) = self.session.as_mut()
                    && let Err(why) = session.declare_mind(player, &said)
                {
                    tracing::warn!(seat, why, "a declared mind was refused");
                }
                Vec::new()
            }
            // Dropped rather than refused before the curtain is up (#256): no
            // seat has been asked anything, so a correct client has nothing
            // to answer, and a refusal would reach it as a failure.
            Some(v1::envelope::Msg::PlayerAction(_)) if self.curtain.is_some() => {
                tracing::debug!(seat, "an action before the curtain went up; dropped");
                Vec::new()
            }
            Some(v1::envelope::Msg::PlayerAction(action_msg)) => {
                let Ok(action) = serde_json::from_slice::<PlayerAction>(&action_msg.action_json)
                else {
                    return Vec::new();
                };
                self.apply(player, action)
            }
            // What it missed, before the curtain, is the table it has not
            // been shown, and still no question.
            Some(v1::envelope::Msg::Resume(_)) if self.curtain.is_some() => {
                let Some(session) = self.session.as_mut() else {
                    return Vec::new();
                };
                session
                    .show(player)
                    .iter()
                    .map(|env| seat_frame(seat, env))
                    .collect()
            }
            // Read-only: a seat asking for what it missed must not advance the
            // game, or reconnecting would play an AI seat's turn for it.
            Some(v1::envelope::Msg::Resume(resume)) => {
                let Some(session) = self.session.as_ref() else {
                    return Vec::new();
                };
                session
                    .resume(player, resume.last_seq)
                    .iter()
                    .map(|env| seat_frame(seat, env))
                    .collect()
            }
            // Not a move in the game (#265): the engine never sees it, so it
            // is not `apply`. A refusal is only said: the seat submitted no
            // answer, so it still holds its question and needs none re-sent.
            // Taken before the curtain is up as well: it is no decision and
            // runs no clock, and the views it changes go out as `show` builds
            // them, with no pump and no question.
            Some(v1::envelope::Msg::SeatSetting(setting_msg)) => {
                let Ok(setting) = serde_json::from_slice::<SeatSetting>(&setting_msg.setting_json)
                else {
                    return Vec::new();
                };
                let Some(session) = self.session.as_mut() else {
                    return Vec::new();
                };
                match session.seat_setting(player, setting) {
                    Ok(routed) => self.route(&routed),
                    Err(reason) => self.route(&[(player, error(&reason))]),
                }
            }
            _ => Vec::new(),
        }
    }

    /// Forwards what a seat's mind said (`v1::AiLog`) to every other
    /// attached seat in a debug build, and drops it in a release build
    /// (`docs/protocol.md` §"An AI seat's reasoning").
    ///
    /// The AI log is a debugging and sparring tool, open to the whole table
    /// by the owner's decision: a model's reasoning reads its whole view out
    /// loud, its hand included, so a debug engine hands every seat hidden
    /// information on purpose, and a release engine, the one deployed,
    /// forwards nothing. Never back to the sender, and to no seat without a
    /// socket. The sender is stamped here, so a seat cannot speak in
    /// another's name; nothing else in it is read. Not a move in the game:
    /// no journal, no clock, no pump.
    fn reasoning(&self, from: PlayerId, mut said: v1::AiLog) -> Vec<Envelope> {
        if !cfg!(debug_assertions) || self.session.is_none() {
            return Vec::new();
        }
        said.seat = u32::from(from.get());
        let envelope = Envelope {
            msg: Some(v1::envelope::Msg::AiLog(said)),
        };
        self.attached
            .iter()
            .copied()
            .filter(|&seat| seat != from.get())
            .map(|seat| seat_frame(seat, &envelope))
            .collect()
    }

    /// Applies one action and routes everything it produced.
    fn apply(&mut self, player: PlayerId, action: PlayerAction) -> Vec<Envelope> {
        let Some(session) = self.session.as_mut() else {
            return Vec::new();
        };
        match session.act(player, action) {
            Ok(routed) => {
                let mut out = self.route(&routed);
                out.extend(self.ending());
                out
            }
            Err(reason) => self.refused(player, &reason),
        }
    }

    /// Answers a refused action: why it was refused, and the question again.
    ///
    /// Silence here is what turns a wrong action into a fatal one. The client
    /// clears its `Interaction` the moment it submits (`flush_outbox` in
    /// `baylee-client`), so a seat told nothing is left with no question in
    /// front of it: every later key and click does nothing, and when the
    /// decision clock runs out the house agent plays that seat's turn — which
    /// looks, from the table, like the phase advancing on its own.
    ///
    /// Re-asking is safe because `Session::reask` is read-only: it never
    /// pumps, so it cannot take a turn on an AI seat's behalf. It also sends
    /// the choice only to the seat actually being awaited, so a refusal from a
    /// seat that does not hold the decision gets the error and nothing more.
    /// It sends no log: a refusal lost no frame, and the whole log for each
    /// refused action would be a large answer to a small request.
    fn refused(&self, player: PlayerId, reason: &str) -> Vec<Envelope> {
        let Some(session) = self.session.as_ref() else {
            return Vec::new();
        };
        let mut frames = vec![(player, error(reason))];
        frames.extend(session.reask(player).into_iter().map(|env| (player, env)));
        self.route(&frames)
    }

    /// What the gateway is owed after the game moved: the game's record
    /// (#315) once [`RECORD_CHUNK_BYTES`] of it have gathered, and when the
    /// game is over the rest of it and then `GameEnded`, once.
    ///
    /// The gateway cannot read a `Pending` — it does not link the engine — so
    /// the end of a game has to be said outright. The record goes before it
    /// on the same socket, so a gateway that has read `GameEnded` has read
    /// the whole record.
    fn ending(&mut self) -> Vec<Envelope> {
        if self.ended {
            return Vec::new();
        }
        let Some(session) = self.session.as_mut() else {
            return Vec::new();
        };
        let Pending::GameOver(result) = session.pending() else {
            let pending = session.record_pending();
            if pending > 0 && self.record_waiting_since.is_none() {
                self.record_waiting_since = Some(self.now);
            }
            if !record_due(pending) {
                return Vec::new();
            }
            let data = session.take_record();
            return vec![self.record_chunk(&data, false)];
        };
        let result = *result;
        let data = session.take_record();
        self.ended = true;
        // A draw has no winner, which is a shorter list rather than a
        // different message — and a team win is a longer one, which is why
        // the field was a list from the start.
        let winners = session
            .winning_seats(result)
            .into_iter()
            .map(|p| u32::from(p.get()))
            .collect();
        let reason = format!("{:?}", result.reason);
        vec![
            self.record_chunk(&data, true),
            Envelope {
                msg: Some(v1::envelope::Msg::GameEnded(v1::GameEnded {
                    game_id: self.game_id.clone(),
                    winners,
                    reason,
                })),
            },
        ]
    }

    /// The next piece of the record, as one gzip member: pieces stored one
    /// after another in `seq` order are one gzip stream of the record.
    fn record_chunk(&mut self, data: &[u8], last: bool) -> Envelope {
        let seq = self.record_seq;
        self.record_seq += 1;
        self.record_sent_at = self.now;
        self.record_waiting_since = None;
        let mut gz = GzEncoder::new(Vec::new(), Compression::default());
        // Writing into a `Vec` cannot fail.
        gz.write_all(data).expect("compressing into memory");
        let data = gz.finish().expect("compressing into memory");
        Envelope {
            msg: Some(v1::envelope::Msg::GameRecordChunk(v1::GameRecordChunk {
                game_id: self.game_id.clone(),
                seq,
                data,
                last,
            })),
        }
    }
}

#[cfg(test)]
mod tests;
