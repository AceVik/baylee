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

/// What a deadline is waiting for.
///
/// Two clocks rather than one, because they measure different things and only
/// ever one of them at a time: a seat that can see the question is deciding, a
/// seat whose socket is gone is being waited for. They also expire into
/// different actions — one answer versus a change of who is answering.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Deadline {
    /// A seat that can see the question and has not answered it.
    /// `HouseRules::decision_timeout_secs`.
    Decide,
    /// A seat that cannot see the question, because its socket is gone.
    /// `HouseRules::reconnect_window_secs`. On expiry the house takes the
    /// chair, rather than answering once for a player who is not coming back
    /// to the next question either.
    StandIn,
}

/// What a seat owes, and how long it has to pay.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Clock {
    /// The seat being asked.
    pub seat: PlayerId,
    /// Which of the two clocks this is.
    ///
    /// Part of the value, not a second field the caller tracks: the attach
    /// loop re-arms whenever `clock()` stops equalling what it armed, so a
    /// player who reconnects turns a `StandIn` deadline into a `Decide` one
    /// simply by being a different `Clock`.
    pub what: Deadline,
    /// When the seat was asked the question it owes ([`Session::asked_at`],
    /// counted in questions, not `Session::seq` frames). The deadline is
    /// anchored to it, so it restarts when the seat is asked something new
    /// rather than every time something else happens: an opponent's priority
    /// hold or reconnect, which produce frames without moving the game, and
    /// during the opening mulligans another seat's answer, which moves the
    /// game without asking this seat anything new, cannot restart it at all.
    pub seq: u64,
    /// How long the seat has, in seconds.
    pub secs: u32,
}

/// The deadlines armed for the clocks that are running, at most one per
/// seat, each at the moment it was armed plus its allowance.
///
/// Generic over the instant so the arming rule can be tested without a
/// runtime; the attach loop keeps `tokio::time::Instant`s in it.
#[derive(Debug)]
pub struct Armed<T> {
    deadlines: Vec<(Clock, T)>,
}

impl<T> Default for Armed<T> {
    fn default() -> Self {
        Self {
            deadlines: Vec::new(),
        }
    }
}

impl<T: Copy + Ord> Armed<T> {
    /// Brings the deadlines in line with `clocks`. A clock still running
    /// unchanged keeps the deadline it has: that is what stops a seat's time
    /// from starting over every time another seat's frame wakes the loop. A
    /// clock that has stopped or changed loses its deadline, and a new one is
    /// armed at `at(clock)`.
    pub fn sync(&mut self, clocks: &[Clock], at: impl Fn(&Clock) -> T) {
        self.deadlines.retain(|(armed, _)| clocks.contains(armed));
        for clock in clocks {
            if !self.deadlines.iter().any(|(armed, _)| armed == clock) {
                self.deadlines.push((*clock, at(clock)));
            }
        }
    }

    /// The deadline that falls first, if any is armed.
    #[must_use]
    pub fn next(&self) -> Option<(Clock, T)> {
        self.deadlines.iter().copied().min_by_key(|(_, at)| *at)
    }

    /// Forgets a deadline that has fired.
    pub fn fired(&mut self, clock: Clock) {
        self.deadlines.retain(|(armed, _)| *armed != clock);
    }

    /// Each armed clock, with what `left` says remains of it.
    #[must_use]
    pub fn remaining(&self, left: impl Fn(T) -> u32) -> Vec<(Clock, u32)> {
        self.deadlines
            .iter()
            .map(|(clock, at)| (*clock, left(*at)))
            .collect()
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
        session
            .awaited()
            .iter()
            .filter_map(|seat| self.clock_for(session, seat))
            .collect()
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
        {
            (Deadline::StandIn, session.reconnect_window_secs())
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

    /// The clock ran out. Acts for the seat, and only for that seat and the
    /// question it was armed for — the seat may have answered between the
    /// timer firing and this being called, and one seat's expired clock must
    /// never take another seat's decision, nor this seat's next one.
    pub fn timeout(&mut self, clock: Clock) -> Vec<Envelope> {
        if self.curtain.is_some() {
            return Vec::new();
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
            Some(v1::envelope::Msg::SeatDetached(detached)) => {
                self.attached.retain(|s| u32::from(*s) != detached.seat);
                if let (Some(barrier), Ok(seat)) =
                    (self.curtain.as_mut(), u8::try_from(detached.seat))
                {
                    barrier.ready = barrier.ready.iter().filter(|p| p.get() != seat).collect();
                    self.enter_at = None;
                    return self.loading_status();
                }
                Vec::new()
            }
            Some(v1::envelope::Msg::SeatFrame(frame)) => self.seat_frame(&frame),
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
        session.hand_back(player);
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
            .filter(|(p, _)| self.attached.contains(&p.get()))
            .map(|(p, env)| seat_frame(p.get(), env))
            .collect()
    }

    /// A player-facing frame from one seat.
    fn seat_frame(&mut self, frame: &v1::SeatFrame) -> Vec<Envelope> {
        let Ok(seat) = u8::try_from(frame.seat) else {
            return Vec::new();
        };
        let player = PlayerId::new(seat);
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

/// Wraps a player-facing envelope for the seat it belongs to.
#[must_use]
pub fn seat_frame(seat: u8, envelope: &Envelope) -> Envelope {
    Envelope {
        msg: Some(v1::envelope::Msg::SeatFrame(v1::SeatFrame {
            seat: u32::from(seat),
            envelope: prost::Message::encode_to_vec(envelope).into(),
        })),
    }
}

/// The table is open (#256).
#[must_use]
pub fn curtain() -> Envelope {
    Envelope {
        msg: Some(v1::envelope::Msg::Curtain(v1::Curtain {})),
    }
}

/// An error a seat may be shown.
fn error(message: &str) -> Envelope {
    Envelope {
        msg: Some(v1::envelope::Msg::Error(v1::Error {
            code: 1,
            message: message.to_string(),
        })),
    }
}

/// A `GameEnded` that says why.
fn ended(game_id: &str, reason: &str) -> Envelope {
    Envelope {
        msg: Some(v1::envelope::Msg::GameEnded(v1::GameEnded {
            game_id: game_id.to_string(),
            winners: Vec::new(),
            reason: reason.to_string(),
        })),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The one clock a duel with one human seat can have running.
    fn on_clock(runner: &EngineRunner) -> Option<Clock> {
        let clocks = runner.clocks();
        assert!(clocks.len() <= 1, "a duel with one human ran {clocks:?}");
        clocks.first().copied()
    }

    /// `ms` left on every clock running now, as the attach loop reads its
    /// timers before handing a frame in.
    fn reading(runner: &EngineRunner, ms: u32) -> Vec<(Clock, u32)> {
        runner
            .clocks()
            .into_iter()
            .map(|clock| (clock, ms))
            .collect()
    }

    /// The acceptance duel, seat 0 human and seat 1 the house.
    fn duel(timeout_secs: u32) -> GamePreset {
        let text = std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../data/acceptance-decks.txt"),
        )
        .expect("acceptance deck file");
        let a = baylee_cards::decks::load_acceptance(&text, "Allytifact").expect("Allytifact");
        let b = baylee_cards::decks::load_acceptance(&text, "Victory").expect("Victory");
        let mut preset = baylee_cards::decks::preset_for(7, &a, &b);
        preset.seats[0].controller = baylee_core::preset::SeatController::Open;
        preset.house_rules.decision_timeout_secs = timeout_secs;
        preset
    }

    fn setup(runner: &mut EngineRunner, preset: &GamePreset) -> Vec<Envelope> {
        runner.handle(
            Envelope {
                msg: Some(v1::envelope::Msg::GameSetup(v1::GameSetup {
                    game_id: "g1".to_string(),
                    preset_json: serde_json::to_vec(preset).expect("preset serializes"),
                    seat_names: vec!["You".to_string(), "House".to_string()],
                })),
            },
            &[],
        )
    }

    /// The same duel with a reconnect window a test can point at.
    fn duel_window(decision_secs: u32, window_secs: u32) -> GamePreset {
        let mut preset = duel(decision_secs);
        preset.house_rules.reconnect_window_secs = window_secs;
        preset
    }

    fn detach(runner: &mut EngineRunner, seat: u32) -> Vec<Envelope> {
        runner.handle(
            Envelope {
                msg: Some(v1::envelope::Msg::SeatDetached(v1::SeatDetached { seat })),
            },
            &[],
        )
    }

    fn attach(runner: &mut EngineRunner, seat: u32) -> Vec<Envelope> {
        runner.handle(
            Envelope {
                msg: Some(v1::envelope::Msg::SeatAttached(v1::SeatAttached {
                    seat,
                    resync: false,
                })),
            },
            &[],
        )
    }

    /// A seat says it has drawn its table, the way its socket would.
    fn ready(runner: &mut EngineRunner, seat: u32) -> Vec<Envelope> {
        let inner = Envelope {
            msg: Some(v1::envelope::Msg::SeatReady(v1::SeatReady {})),
        };
        runner.handle(
            Envelope {
                msg: Some(v1::envelope::Msg::SeatFrame(v1::SeatFrame {
                    seat,
                    envelope: prost::Message::encode_to_vec(&inner).into(),
                })),
            },
            &[],
        )
    }

    /// A seat that attaches and has drawn its table: what every test about
    /// an open table starts from (#256). The last human seat to sit raises
    /// the curtain.
    fn sit(runner: &mut EngineRunner, seat: u32) -> Vec<Envelope> {
        let mut out = attach(runner, seat);
        out.extend(ready(runner, seat));
        if let Some(at) = runner.entrance_deadline() {
            runner.tell_time(at);
            out.extend(runner.finish_entrance());
        }
        out
    }

    /// One seat's answer, wrapped the way its socket would deliver it.
    fn act(runner: &mut EngineRunner, seat: u32, action: &PlayerAction) -> Vec<Envelope> {
        act_reading(runner, seat, action, &[])
    }

    /// [`act`], with what the attach loop read off its armed clocks.
    fn act_reading(
        runner: &mut EngineRunner,
        seat: u32,
        action: &PlayerAction,
        remaining: &[(Clock, u32)],
    ) -> Vec<Envelope> {
        let inner = Envelope {
            msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
                game_id: "g1".to_string(),
                seat_token: String::new(),
                action_json: serde_json::to_vec(action).expect("action serializes"),
            })),
        };
        runner.handle(
            Envelope {
                msg: Some(v1::envelope::Msg::SeatFrame(v1::SeatFrame {
                    seat,
                    envelope: prost::Message::encode_to_vec(&inner).into(),
                })),
            },
            remaining,
        )
    }

    /// The `(seat, inner message)` of each frame, which is all a test cares
    /// about — the gateway never looks inside one either.
    fn frames(envelopes: &[Envelope]) -> Vec<(u32, v1::envelope::Msg)> {
        envelopes
            .iter()
            .filter_map(|env| match &env.msg {
                Some(v1::envelope::Msg::SeatFrame(frame)) => {
                    let inner = <Envelope as prost::Message>::decode(&frame.envelope[..]).ok()?;
                    Some((frame.seat, inner.msg?))
                }
                _ => None,
            })
            .collect()
    }

    /// A seat's first frame has to be the roster and the print table: every
    /// frame after it points into them, and without the print table a
    /// `PrintRef` names no card at all.
    #[test]
    fn a_seats_first_frame_is_the_payload_the_rest_refers_to() {
        let mut runner = EngineRunner::new();
        assert!(
            setup(&mut runner, &duel(0)).is_empty(),
            "setup says nothing"
        );
        assert!(runner.ready());
        let out = attach(&mut runner, 0);
        let frames = frames(&out);
        assert!(
            matches!(frames.first(), Some((0, v1::envelope::Msg::GameStatic(_)))),
            "expected the opening payload first, got {:?}",
            frames.first().map(|(seat, _)| seat)
        );
    }

    /// Frames for a seat that has not arrived are dropped here rather than one
    /// hop later, which is what keeps that seat's own opening payload first on
    /// its wire when it does arrive.
    #[test]
    fn a_seat_with_no_socket_is_sent_nothing() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &duel(0));
        let out = attach(&mut runner, 0);
        assert!(
            frames(&out).iter().all(|(seat, _)| *seat == 0),
            "the absent seat was sent something"
        );
        // And once it arrives, its own attach hands it the whole state.
        let out = attach(&mut runner, 1);
        let frames = frames(&out);
        assert!(matches!(
            frames.first(),
            Some((1, v1::envelope::Msg::GameStatic(_)))
        ));
        assert!(
            frames.len() > 1,
            "a seat that arrives late is left with only a roster"
        );
    }

    /// A refusal must cost the seat the action and nothing else.
    ///
    /// The network half of the rule `LocalHost` obeys. The client clears its
    /// `Interaction` the moment it submits, so an engine that answers a
    /// refused action with silence leaves that seat holding no question at
    /// all — it can never act again, and the decision clock then hands its
    /// turns to the house agent, which from the table looks like the phase
    /// advancing on its own.
    #[test]
    fn a_refused_action_is_answered_with_the_question_again() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &duel(0));
        sit(&mut runner, 0);
        sit(&mut runner, 1);
        // The opening choice is a mulligan; passing priority is not an answer.
        let out = act(&mut runner, 0, &PlayerAction::PassPriority);
        let refusal = frames(&out);
        assert!(
            refusal
                .iter()
                .any(|(seat, msg)| *seat == 0 && matches!(msg, v1::envelope::Msg::Error(_))),
            "the seat was not told why"
        );
        assert!(
            refusal
                .iter()
                .any(|(seat, msg)| *seat == 0
                    && matches!(msg, v1::envelope::Msg::ChoiceRequest(_))),
            "the seat was not asked again"
        );
        // One seat's mistake is not everyone's business.
        assert!(
            refusal.iter().all(|(seat, _)| *seat == 0),
            "the refusal was broadcast"
        );
        // And the question that came back is the one it still owes an answer
        // to — proof the re-ask did not advance the game while saying no.
        let out = act(&mut runner, 0, &PlayerAction::MulliganKeep);
        assert!(
            frames(&out)
                .iter()
                .any(|(_, msg)| matches!(msg, v1::envelope::Msg::StateDelta(_))),
            "the seat could not play after the refusal"
        );
    }

    /// A refusal answers the seat. It must not also buy it time.
    ///
    /// Re-asking made a refused action produce frames where it used to produce
    /// none, so it is worth saying outright that the clock does not notice: it
    /// is anchored to the sequence number it was armed at, a refusal moves
    /// nothing, and the arming loop leaves an identical `Clock` alone. Without
    /// that, a seat could hold its own decision open indefinitely by sending
    /// illegal actions at it, which is a cheat rather than a bug.
    #[test]
    fn a_refusal_does_not_restart_the_clock() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &duel(30));
        sit(&mut runner, 0);
        sit(&mut runner, 1);
        let before = on_clock(&runner).expect("a seat is on the clock");
        let out = act(&mut runner, 0, &PlayerAction::PassPriority);
        assert!(!out.is_empty(), "the refusal was answered at all");
        assert_eq!(
            on_clock(&runner),
            Some(before),
            "a refused action re-armed the clock"
        );
    }

    /// The acceptance duel with nobody automated, so the seat that is *not*
    /// being asked is still able to say something.
    fn two_humans(timeout_secs: u32) -> GamePreset {
        let mut preset = duel(timeout_secs);
        preset.seats[1].controller = baylee_core::preset::SeatController::Open;
        preset
    }

    /// A human seat's clock starts when the table opens, not when the game
    /// is built (#256).
    ///
    /// It used to be the other way round, and this test pinned it before it
    /// moved: every human chair owes its opening mulligan from the setup on,
    /// so a chair with no socket yet was on its reconnect window from that
    /// moment, and a player whose client took longer than the window to load
    /// lost the chair to the house before drawing the table. Now nobody is on
    /// either clock until the curtain is up, and then the seat that never
    /// arrived starts its whole window, and the seat that is here its whole
    /// allowance.
    #[test]
    fn a_seat_with_no_socket_yet_is_waited_for_from_the_curtain_on() {
        let (zero, one) = (PlayerId::new(0), PlayerId::new(1));
        let mut preset = two_humans(600);
        preset.house_rules.reconnect_window_secs = 60;
        let mut runner = EngineRunner::new();
        setup(&mut runner, &preset);
        let kinds = |runner: &EngineRunner| {
            runner
                .clocks()
                .iter()
                .map(|c| (c.seat, c.what, c.secs))
                .collect::<Vec<_>>()
        };
        assert_eq!(kinds(&runner), [], "nobody has attached, and nobody waits");
        sit(&mut runner, 0);
        assert_eq!(
            kinds(&runner),
            [],
            "the first seat in waits for the others with its clock stopped"
        );
        let out = runner.raise_curtain();
        assert_eq!(
            kinds(&runner),
            [(zero, Deadline::Decide, 600), (one, Deadline::StandIn, 60)],
            "the table opened without seat 1, and its window starts now"
        );
        assert!(
            matches!(
                frames(&out).last(),
                Some((0, v1::envelope::Msg::Curtain(_)))
            ),
            "the seat that is here is told the table is open"
        );
    }

    /// Whose frames say what, in order, for the seats a test cares about.
    fn said(out: &[Envelope]) -> Vec<(u32, &'static str)> {
        frames(out)
            .into_iter()
            .map(|(seat, msg)| {
                let what = match msg {
                    v1::envelope::Msg::GameStatic(_) => "static",
                    v1::envelope::Msg::StateDelta(_) => "view",
                    v1::envelope::Msg::ChoiceRequest(_) => "question",
                    v1::envelope::Msg::Curtain(_) => "curtain",
                    v1::envelope::Msg::TableLoading(_) => "loading",
                    v1::envelope::Msg::Error(_) => "error",
                    _ => "other",
                };
                (seat, what)
            })
            .collect()
    }

    /// The curtain waits for every human seat, and each is shown its table
    /// but asked nothing until the last one is ready (#256). Then every seat
    /// is asked, and told the table is open last, after what it opens on.
    #[test]
    fn the_curtain_goes_up_when_the_last_human_seat_is_ready() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &two_humans(600));
        // Shown a table: the payload, and a view (after a second payload
        // when the view is the first to show a printing), but no question
        // and no curtain.
        let shown = |out: &[Envelope], seat: u32| {
            let order: Vec<_> = said(out)
                .into_iter()
                .filter(|(_, what)| *what != "loading")
                .collect();
            order.first() == Some(&(seat, "static"))
                && order.last() == Some(&(seat, "view"))
                && order
                    .iter()
                    .all(|(s, what)| *s == seat && matches!(*what, "static" | "view"))
        };
        let first = attach(&mut runner, 0);
        assert!(shown(&first, 0), "{:?}", said(&first));
        assert_eq!(
            last_view(&first, 0).expect("a view").decision_remaining_ms,
            None,
            "a table that has not opened shows no clock"
        );
        assert!(!ready(&mut runner, 0).is_empty(), "readiness is broadcast");
        assert!(
            runner.entrance_deadline().is_none(),
            "seat 1 is not here yet"
        );
        let second = attach(&mut runner, 1);
        assert!(shown(&second, 1), "{:?}", said(&second));
        assert!(
            said(&second).iter().all(|(s, _)| *s == 1),
            "seat 0's count has not changed, so it is not told it again: {:?}",
            said(&second)
        );
        assert!(runner.curtain_pending());

        let scheduled = ready(&mut runner, 1);
        assert!(
            frames(&scheduled)
                .iter()
                .all(|(_, m)| matches!(m, v1::envelope::Msg::TableLoading(_)))
        );
        assert!(runner.curtain_pending());
        assert!(runner.clocks().is_empty());
        assert!(
            runner.finish_entrance().is_empty(),
            "an early timer cannot open play"
        );
        runner.tell_time(runner.entrance_deadline().unwrap());
        let up = runner.finish_entrance();
        assert!(!runner.curtain_pending());
        for seat in [0, 1] {
            let theirs: Vec<&str> = said(&up)
                .into_iter()
                .filter(|(s, _)| *s == seat)
                .map(|(_, what)| what)
                .collect();
            assert!(
                theirs.contains(&"question"),
                "seat {seat} is asked: {theirs:?}"
            );
            assert_eq!(theirs.last(), Some(&"curtain"), "seat {seat}: {theirs:?}");
        }
        assert_eq!(
            last_view(&up, 0).expect("a view").decision_remaining_ms,
            Some(600_000),
            "the clock starts whole when the table opens"
        );
    }

    /// An AI chair never holds the curtain, and plays nothing behind it: the
    /// house keeps its opening hand in the pump that raises it (#256).
    #[test]
    fn the_house_plays_nothing_before_the_curtain_is_up() {
        let house = PlayerId::new(1);
        let mut runner = EngineRunner::new();
        setup(&mut runner, &duel(600));
        attach(&mut runner, 0);
        let session = runner.session().expect("a game");
        assert!(
            session.awaited().contains(house),
            "the house has decided already"
        );
        let stuck = session.decision_seq();

        ready(&mut runner, 0);
        assert_eq!(
            runner.session().unwrap().decision_seq(),
            stuck,
            "AI waits throughout the portal"
        );
        runner.tell_time(runner.entrance_deadline().unwrap());
        let up = runner.finish_entrance();
        assert_eq!(said(&up).last(), Some(&(0, "curtain")));
        let session = runner.session().expect("a game");
        assert!(!session.awaited().contains(house), "the house did not keep");
        assert!(session.decision_seq() > stuck);
    }

    #[test]
    fn preparation_timeout_never_opens_an_unready_table() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &two_humans(600));
        attach(&mut runner, 0);
        ready(&mut runner, 0);
        let out = runner.preparation_expired();
        assert!(runner.finished());
        assert!(runner.clocks().is_empty());
        assert!(said(&out).contains(&(0, "error")));
        assert!(!said(&out).iter().any(|(_, what)| *what == "curtain"));
        assert!(
            out.iter()
                .any(|e| matches!(e.msg, Some(v1::envelope::Msg::GameEnded(_))))
        );
    }

    #[test]
    fn a_resynchronized_snapshot_must_earn_readiness_again() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &two_humans(600));
        attach(&mut runner, 0);
        attach(&mut runner, 1);
        ready(&mut runner, 0);
        ready(&mut runner, 1);
        assert!(runner.entrance_deadline().is_some());
        let refreshed = attach(&mut runner, 0);
        assert!(runner.entrance_deadline().is_none());
        assert!(
            said(&refreshed).contains(&(1, "loading")),
            "the other seat hears its departure cancelled"
        );
        assert!(frames(&refreshed).iter().any(|(_, message)| matches!(message, v1::envelope::Msg::TableLoading(s) if s.ready == 1 && s.enter_at_ms == 0)));
        ready(&mut runner, 0);
        assert!(runner.entrance_deadline().is_some());
    }

    #[test]
    fn a_dropped_seat_cancels_departure_and_must_prepare_again() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &two_humans(600));
        attach(&mut runner, 0);
        attach(&mut runner, 1);
        ready(&mut runner, 0);
        ready(&mut runner, 1);
        let old_deadline = runner.entrance_deadline().unwrap();
        let cancelled = detach(&mut runner, 1);
        assert!(frames(&cancelled).iter().any(|(_, message)| matches!(message, v1::envelope::Msg::TableLoading(s) if s.ready == 1 && s.enter_at_ms == 0)));
        runner.tell_time(old_deadline);
        assert!(runner.finish_entrance().is_empty());
        assert!(
            ready(&mut runner, 1).is_empty(),
            "a detached seat cannot be ready"
        );
        attach(&mut runner, 1);
        assert!(runner.entrance_deadline().is_none());
        ready(&mut runner, 1);
        assert!(runner.entrance_deadline().unwrap() > old_deadline);
        assert!(runner.clocks().is_empty());
    }

    /// An action before the curtain is up is dropped without a word: a
    /// refusal would reach the client as a failure, and a correct client has
    /// been asked nothing it could answer (#256).
    #[test]
    fn an_action_before_the_curtain_is_dropped_without_a_word() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &two_humans(600));
        attach(&mut runner, 0);
        let before = runner.session().expect("a game").seq();
        assert!(act(&mut runner, 0, &PlayerAction::MulliganKeep).is_empty());
        assert_eq!(runner.session().expect("a game").seq(), before);
        assert!(
            runner
                .session()
                .expect("a game")
                .awaited()
                .contains(PlayerId::new(0)),
            "the keep went through"
        );
    }

    /// A seat's `SeatReady` counts for that seat, once, and for nothing
    /// after the curtain is up; one from a seat the curtain never waited
    /// for, or one beyond any table, counts for nobody (#256).
    #[test]
    fn a_seat_ready_counts_once_for_its_own_seat() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &two_humans(600));
        attach(&mut runner, 0);
        attach(&mut runner, 1);
        assert!(!ready(&mut runner, 0).is_empty());
        for _ in 0..3 {
            assert!(ready(&mut runner, 0).is_empty());
        }
        assert!(ready(&mut runner, 7).is_empty(), "no seat 7 at this table");
        assert!(
            ready(&mut runner, 200).is_empty(),
            "nor any seat past a set"
        );
        assert!(runner.curtain_pending(), "seat 1 has not said it");
        assert!(!ready(&mut runner, 1).is_empty());
        assert!(
            ready(&mut runner, 0).is_empty(),
            "the curtain is already up"
        );
    }

    /// A seat that asks what it missed before the curtain is up is shown its
    /// table again, and still asked nothing (#256).
    #[test]
    fn a_resume_before_the_curtain_asks_nothing() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &two_humans(600));
        attach(&mut runner, 0);
        let inner = Envelope {
            msg: Some(v1::envelope::Msg::Resume(v1::ResumeGame {
                game_id: "g1".to_string(),
                seat_token: String::new(),
                last_seq: 0,
            })),
        };
        let out = runner.handle(
            Envelope {
                msg: Some(v1::envelope::Msg::SeatFrame(v1::SeatFrame {
                    seat: 0,
                    envelope: prost::Message::encode_to_vec(&inner).into(),
                })),
            },
            &[],
        );
        assert_eq!(said(&out), [(0, "view")]);
    }

    /// The log each frame of `seat`'s carries, in order: the index of its
    /// first line and how many lines it holds.
    fn logs(envelopes: &[Envelope], seat: u32) -> Vec<(u64, usize)> {
        frames(envelopes)
            .into_iter()
            .filter_map(|(s, msg)| match msg {
                v1::envelope::Msg::StateDelta(delta) if s == seat && !delta.log_json.is_empty() => {
                    let tail: serde_json::Value = serde_json::from_slice(&delta.log_json).ok()?;
                    Some((tail["from"].as_u64()?, tail["entries"].as_array()?.len()))
                }
                _ => None,
            })
            .collect()
    }

    /// A loader that drops and dials again before the table is open holds
    /// none of the log its first socket was sent, so its second is told the
    /// log from the first line (#256, #262), as a socket attaching to an open
    /// table is.
    ///
    /// Nothing the runner does before the curtain writes a line today: no
    /// clock runs, no answer is applied, and the opening deal is not logged.
    /// So the line is put there by hand, the one a pre-curtain table would
    /// hold first if a clock ever ran there: the house standing in.
    #[test]
    fn a_seat_that_dials_again_before_the_curtain_is_told_its_log_from_the_start() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &two_humans(600));
        assert!(
            logs(&attach(&mut runner, 0), 0).is_empty(),
            "the runner logs nothing before the curtain"
        );
        detach(&mut runner, 0);
        let session = runner.session.as_mut().expect("set up");
        assert!(
            session.stand_in(PlayerId::new(1)),
            "seat 1 is a human chair"
        );
        assert_eq!(
            logs(&attach(&mut runner, 0), 0),
            [(0, 1)],
            "the socket is told the line"
        );
        detach(&mut runner, 0);
        assert!(runner.curtain_pending(), "still loading");
        assert_eq!(
            logs(&attach(&mut runner, 0), 0),
            [(0, 1)],
            "the second socket was not told the log from its first line"
        );
    }

    /// During the opening mulligans each human seat is on its own clock, and
    /// one seat's answer leaves the other's exactly as it was: the same
    /// `Clock`, which is what keeps the attach loop from arming it again
    /// ([`Armed::sync`]). The old deadline of the seat that answered answers
    /// nothing, and the other seat's still answers for it.
    #[test]
    fn each_seat_deciding_its_mulligan_is_on_its_own_clock() {
        let (zero, one) = (PlayerId::new(0), PlayerId::new(1));
        let mut runner = EngineRunner::new();
        setup(&mut runner, &two_humans(30));
        sit(&mut runner, 0);
        sit(&mut runner, 1);
        let before = runner.clocks();
        assert_eq!(
            before.iter().map(|c| (c.seat, c.what)).collect::<Vec<_>>(),
            [(zero, Deadline::Decide), (one, Deadline::Decide)]
        );
        act(&mut runner, 0, &PlayerAction::MulliganTake);
        let after = runner.clocks();
        assert_eq!(after[1], before[1], "seat 0's take re-armed seat 1's clock");
        assert_ne!(after[0], before[0], "seat 0 is asked anew after its take");
        assert!(
            runner.timeout(before[0]).is_empty(),
            "a stale deadline answered"
        );
        assert!(!runner.timeout(before[1]).is_empty());
        let session = runner.session().expect("a game");
        assert!(!session.awaited().contains(one), "seat 1 did not keep");
        assert_eq!(runner.clocks(), vec![after[0]]);
    }

    /// A deadline whose clock still runs unchanged keeps its moment; one that
    /// changed is armed anew; one that stopped is gone.
    #[test]
    fn a_running_clock_keeps_the_deadline_it_was_armed_with() {
        let clock = |seat: u8, seq: u64| Clock {
            seat: PlayerId::new(seat),
            what: Deadline::Decide,
            seq,
            secs: 30,
        };
        let mut armed = Armed::default();
        armed.sync(&[clock(0, 0), clock(1, 0)], |c| 100 + u64::from(c.secs));
        // Ten seconds on, seat 0 is asked something new and seat 1 is not.
        armed.sync(&[clock(0, 1), clock(1, 0)], |c| 110 + u64::from(c.secs));
        assert_eq!(armed.next(), Some((clock(1, 0), 130)));
        assert_eq!(
            armed.remaining(|at| u32::try_from(at).unwrap_or(u32::MAX)),
            vec![(clock(1, 0), 130), (clock(0, 1), 140)]
        );
        armed.fired(clock(1, 0));
        assert_eq!(armed.next(), Some((clock(0, 1), 140)));
        armed.sync(&[], |_| 0);
        assert_eq!(armed.next(), None);
    }

    /// Both seats keep their opening hands, so that turn 1 has begun and one
    /// seat is being asked.
    fn keep_both(runner: &mut EngineRunner) {
        for seat in [0, 1] {
            act(runner, seat, &PlayerAction::MulliganKeep);
        }
        assert_eq!(
            runner.session().expect("a game").awaited().len(),
            1,
            "turn 1 has not begun"
        );
    }

    /// The clock belongs to the seat being asked, and nobody else may wind it.
    ///
    /// A priority hold is one of the settings the engine takes from a seat
    /// that is not on the clock — an ability's yield or standing answer is
    /// another. Both produce frames without moving the game, so a clock
    /// anchored to the frame counter restarts on every press of `F6` at the
    /// other end of the table: unlimited thinking time for whoever spams it,
    /// which is a cheat rather than a bug.
    #[test]
    fn the_other_seats_hold_does_not_wind_the_clock() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &two_humans(30));
        sit(&mut runner, 0);
        sit(&mut runner, 1);
        // Past the mulligans, which ask both seats at once: this is about
        // the seat that is not being asked.
        keep_both(&mut runner);
        let before = on_clock(&runner).expect("a seat is on the clock");
        let frames_before = runner.session().expect("a game").seq();
        let idle = u32::from(before.seat.get() == 0);
        for _ in 0..3 {
            act(
                &mut runner,
                idle,
                &PlayerAction::SetPriorityHold(
                    baylee_engine::choice::PriorityHold::UntilEndOfTurn { turn: 1 },
                ),
            );
        }
        // Both halves, or the test proves nothing: an unchanged clock is also
        // what a *refused* hold would look like, and a hold that never reached
        // the engine could not be taken back either.
        assert_eq!(
            runner.session().expect("a game").seq(),
            frames_before + 3,
            "the seat that was not being asked could not state a hold at all"
        );
        assert_eq!(
            on_clock(&runner),
            Some(before),
            "the seat not being asked wound the other seat's clock"
        );
    }

    /// The clock is the one thing the rules kernel must not own, and it must
    /// not run against a player who is not there to see it.
    ///
    /// They are on the *other* clock instead: a seat with no socket cannot
    /// lose on time to a question it never saw, but the table must not be
    /// left waiting on it forever either.
    #[test]
    fn nobody_is_on_a_clock_they_cannot_see() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &duel_window(60, 30));
        sit(&mut runner, 0);
        let clock = on_clock(&runner).expect("the seat being asked is here");
        assert_eq!(clock.seat.get(), 0);
        assert_eq!(clock.what, Deadline::Decide);
        assert_eq!(clock.secs, 60);

        detach(&mut runner, 0);
        let clock = on_clock(&runner).expect("the table is still waiting on it");
        assert_eq!(clock.seat.get(), 0);
        assert_eq!(
            clock.what,
            Deadline::StandIn,
            "the player walked away; it is the chair that is on a clock now"
        );
        assert_eq!(clock.secs, 30, "and it is the table's reconnect window");
    }

    /// A table with no decision limit puts nobody on a *decision* clock — and
    /// the two limits are independent, because a table that gives its players
    /// all the time in the world still must not sit forever on a closed
    /// laptop.
    #[test]
    fn no_limit_means_no_clock() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &duel_window(0, 30));
        sit(&mut runner, 0);
        assert_eq!(on_clock(&runner), None);

        detach(&mut runner, 0);
        assert_eq!(
            on_clock(&runner).map(|c| (c.what, c.secs)),
            Some((Deadline::StandIn, 30)),
            "no decision limit is not no reconnect window"
        );
    }

    /// A table that says to wait forever is waited on forever.
    #[test]
    fn a_table_that_never_gives_up_a_chair_never_takes_one() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &duel_window(60, 0));
        attach(&mut runner, 0);
        detach(&mut runner, 0);
        assert_eq!(on_clock(&runner), None);
    }

    /// The bug the reconnect window exists for: seat 0 closes its laptop
    /// while it owes an answer, and nothing moves the game again.
    ///
    /// A stand-in rather than one answer on the seat's behalf, because a
    /// player who is not there for this question is not there for the next
    /// one either — answering once would put the table straight back where it
    /// was, one decision later.
    #[test]
    fn a_chair_nobody_is_sitting_in_goes_to_the_house() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &duel_window(60, 30));
        sit(&mut runner, 0);
        detach(&mut runner, 0);
        let stuck = runner.session().expect("the game is built").decision_seq();

        let clock = on_clock(&runner).expect("the chair is on a clock");
        assert_eq!(clock.what, Deadline::StandIn);
        let _ = runner.timeout(clock);
        assert!(
            runner.session().expect("the game is built").decision_seq() > stuck,
            "the house sat down and the table moved on"
        );
    }

    /// The same race the decision clock has, in the other direction: the
    /// socket may come back between the timer firing and this being called,
    /// and a player who is at the table must not have their chair taken.
    #[test]
    fn a_player_who_gets_back_in_time_keeps_their_chair() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &duel_window(60, 30));
        sit(&mut runner, 0);
        detach(&mut runner, 0);
        let clock = on_clock(&runner).expect("the chair is on a clock");
        sit(&mut runner, 0);

        assert!(
            runner.timeout(clock).is_empty(),
            "the deadline fired for a chair that is occupied again"
        );
        assert_eq!(
            on_clock(&runner).map(|c| c.what),
            Some(Deadline::Decide),
            "and the seat is simply being asked again"
        );
    }

    /// The chair was only ever borrowed, so the socket coming back takes it
    /// straight off the house — no window, no second deadline.
    #[test]
    fn the_house_gives_the_chair_back_when_the_socket_returns() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &duel_window(60, 30));
        sit(&mut runner, 0);
        detach(&mut runner, 0);
        let clock = on_clock(&runner).expect("the chair is on a clock");
        let _ = runner.timeout(clock);
        assert!(
            runner
                .session()
                .expect("the game is built")
                .seat_kind(PlayerId::new(0))
                .is_some_and(SeatKind::is_away),
            "the house is holding seat 0"
        );

        sit(&mut runner, 0);
        assert!(
            !runner
                .session()
                .expect("the game is built")
                .seat_kind(PlayerId::new(0))
                .is_some_and(SeatKind::is_away),
            "the player is back and the chair is theirs"
        );
    }

    /// One seat's expired clock must never take another seat's decision, and
    /// a deadline that fired after the game moved on must take none at all.
    #[test]
    fn a_stale_deadline_answers_for_nobody() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &duel(60));
        sit(&mut runner, 0);
        let clock = on_clock(&runner).expect("someone is being asked");
        let stale = Clock {
            seq: clock.seq + 1,
            ..clock
        };
        assert!(
            runner.timeout(stale).is_empty(),
            "a deadline armed for an older question answered the current one"
        );
        let other_seat = Clock {
            seat: PlayerId::new(1),
            ..clock
        };
        assert!(
            runner.timeout(other_seat).is_empty(),
            "one seat's clock answered for another"
        );
        assert!(
            !runner.timeout(clock).is_empty(),
            "the seat's own deadline did nothing"
        );
    }

    /// The frames the clock's answer goes out in say the clock gave it. The
    /// runner answers through the session's clock door rather than the one
    /// a player's answer takes, which is the only way the session can tell.
    #[test]
    fn the_frames_a_timeout_sends_say_the_clock_answered() {
        use baylee_gamehost::view::wire::HouseAnswer;
        let mut runner = EngineRunner::new();
        setup(&mut runner, &duel(60));
        sit(&mut runner, 0);
        let clock = on_clock(&runner).expect("someone is being asked");
        assert_eq!(clock.what, Deadline::Decide);
        let out = runner.timeout(clock);
        let view = last_view(&out, clock.seat.get().into()).expect("the seat is sent the table");
        assert_eq!(
            view.seat(clock.seat).and_then(|s| s.house_answered),
            Some(HouseAnswer::Clock)
        );
    }

    /// The game's record reaches the gateway whole (#315): in pieces as it
    /// grows, the last of them before `GameEnded`, and the pieces together
    /// replay the game the runner played.
    #[test]
    fn the_record_goes_to_the_gateway_before_the_game_ends() {
        let me = PlayerId::new(0);
        let mut runner = EngineRunner::new();
        let mut out = setup(&mut runner, &duel(0));
        out.extend(sit(&mut runner, 0));
        for _ in 0..5_000 {
            if runner.finished() {
                break;
            }
            let action = runner
                .session()
                .and_then(|s| s.house_action(me))
                .expect("the player is asked while the game goes on");
            let step = act(&mut runner, 0, &action);
            let sent = step
                .iter()
                .any(|e| matches!(e.msg, Some(v1::envelope::Msg::GameRecordChunk(_))));
            if !sent {
                // Nothing held back that was due: what is left waiting is
                // under the bound.
                let pending = runner.session().unwrap().record_pending();
                assert!(pending < RECORD_CHUNK_BYTES, "{pending} bytes held back");
            }
            out.extend(step);
        }
        assert!(runner.finished(), "the game ended");
        let pieces: Vec<&v1::GameRecordChunk> = out
            .iter()
            .filter_map(|e| match &e.msg {
                Some(v1::envelope::Msg::GameRecordChunk(c)) => Some(c),
                _ => None,
            })
            .collect();
        assert!(
            pieces.len() > 1,
            "a whole game is sent in more than one piece"
        );
        let seqs: Vec<u32> = pieces.iter().map(|c| c.seq).collect();
        assert_eq!(
            seqs,
            (0..u32::try_from(pieces.len()).unwrap()).collect::<Vec<_>>()
        );
        let last = pieces.iter().filter(|c| c.last).count();
        assert_eq!(last, 1, "one last piece");
        assert!(pieces.last().unwrap().last, "the last piece is last");
        let kinds: Vec<bool> = out
            .iter()
            .filter_map(|e| match &e.msg {
                Some(v1::envelope::Msg::GameRecordChunk(_)) => Some(false),
                Some(v1::envelope::Msg::GameEnded(_)) => Some(true),
                _ => None,
            })
            .collect();
        assert_eq!(
            kinds.last(),
            Some(&true),
            "GameEnded comes after the record"
        );
        assert_eq!(kinds.iter().filter(|&&k| k).count(), 1);

        let stored: Vec<u8> = pieces.iter().flat_map(|c| c.data.iter().copied()).collect();
        let mut record = Vec::new();
        std::io::Read::read_to_end(
            &mut flate2::read::MultiGzDecoder::new(&stored[..]),
            &mut record,
        )
        .expect("the pieces are one gzip stream");
        let replayed = baylee_gamehost::record::replay(&record).expect("the record replays");
        assert!(replayed.ended);
        assert_eq!(
            replayed.engine.snapshot_hash(),
            runner.session().unwrap().snapshot_hash()
        );
        let text = String::from_utf8(record).unwrap();
        assert!(!text.contains("\"You\""), "the record names no seat");

        // Each piece but the last waited for the bound, and each is a gzip
        // member of its own.
        for piece in &pieces {
            let mut inside = Vec::new();
            std::io::Read::read_to_end(
                &mut flate2::read::GzDecoder::new(&piece.data[..]),
                &mut inside,
            )
            .expect("a gzip member");
            if !piece.last {
                assert!(inside.len() >= RECORD_CHUNK_BYTES, "piece {}", piece.seq);
            }
        }
    }

    /// The bound is inclusive: exactly [`RECORD_CHUNK_BYTES`] is a piece.
    #[test]
    fn a_piece_is_due_at_exactly_its_bound() {
        assert!(!record_due(0));
        assert!(!record_due(RECORD_CHUNK_BYTES - 1));
        assert!(record_due(RECORD_CHUNK_BYTES));
        assert!(record_due(RECORD_CHUNK_BYTES + 1));
    }

    /// The gateway's ask for the record as it stands (#323).
    fn flush(runner: &mut EngineRunner, nonce: u64) -> Vec<Envelope> {
        runner.handle(
            Envelope {
                msg: Some(v1::envelope::Msg::FlushRecord(v1::FlushRecord {
                    game_id: "g1".to_string(),
                    nonce,
                })),
            },
            &[],
        )
    }

    /// The record pieces among `out`, in order.
    fn pieces(out: &[Envelope]) -> Vec<v1::GameRecordChunk> {
        out.iter()
            .filter_map(|e| match &e.msg {
                Some(v1::envelope::Msg::GameRecordChunk(c)) => Some(c.clone()),
                _ => None,
            })
            .collect()
    }

    /// One gzip member, unpacked; panics on anything else, trailing bytes
    /// included, so a piece that needs its neighbour to be read fails here.
    fn member(data: &[u8]) -> Vec<u8> {
        let mut gz = flate2::read::GzDecoder::new(data);
        let mut inside = Vec::new();
        std::io::Read::read_to_end(&mut gz, &mut inside).expect("a gzip member");
        assert!(gz.into_inner().is_empty(), "one member, nothing after it");
        inside
    }

    /// The record the pieces make, each unpacked on its own.
    fn unpacked(pieces: &[v1::GameRecordChunk]) -> Vec<u8> {
        pieces.iter().flat_map(|p| member(&p.data)).collect()
    }

    /// Plays the house's answers for seat 0, `steps` of them.
    fn play_a_little(runner: &mut EngineRunner, steps: usize) -> Vec<Envelope> {
        let me = PlayerId::new(0);
        let mut out = Vec::new();
        for _ in 0..steps {
            let action = runner
                .session()
                .and_then(|s| s.house_action(me))
                .expect("the player is asked while the game goes on");
            out.extend(act(runner, 0, &action));
        }
        out
    }

    /// What answers a seat, as its socket declares it, wrapped the way the
    /// socket sends it.
    fn declare(runner: &mut EngineRunner, seat: u32, model: &str) -> Vec<Envelope> {
        let inner = Envelope {
            msg: Some(v1::envelope::Msg::SeatMind(v1::SeatMind {
                kind: v1::seat_mind::Kind::LlmApi as i32,
                provider: "anthropic".into(),
                model: model.into(),
                effort: "high".into(),
                level: String::new(),
            })),
        };
        runner.handle(
            Envelope {
                msg: Some(v1::envelope::Msg::SeatFrame(v1::SeatFrame {
                    seat,
                    envelope: prost::Message::encode_to_vec(&inner).into(),
                })),
            },
            &[],
        )
    }

    /// A seat's declared mind goes into the record, before the curtain as
    /// after, and to nobody at the table: it answers no frame. One shaped
    /// like a key is dropped, and the record holds no trace of it. A swap
    /// mid-game is written where it came; the record still replays.
    #[test]
    fn a_declared_mind_goes_into_the_record_and_to_no_seat() {
        let key = format!("claude-opus-5-5sk-ant-{}", "Z".repeat(30));
        let mut runner = EngineRunner::new();
        setup(&mut runner, &duel(0));
        attach(&mut runner, 0);
        assert!(runner.curtain_pending());
        assert!(declare(&mut runner, 0, &key).is_empty());
        assert!(declare(&mut runner, 0, "claude-opus-5-5").is_empty());
        ready(&mut runner, 0);
        runner.tell_time(runner.entrance_deadline().expect("the entrance"));
        runner.finish_entrance();
        play_a_little(&mut runner, 4);
        assert!(declare(&mut runner, 0, "claude-sonnet-5-5").is_empty());
        play_a_little(&mut runner, 4);
        let record = unpacked(&pieces(&flush(&mut runner, 1)));
        let text = String::from_utf8(record.clone()).unwrap();
        assert!(!text.contains("ZZZZ"), "the key-shaped one is not written");
        let models: Vec<String> = text
            .lines()
            .filter_map(|l| {
                match serde_json::from_str::<baylee_gamehost::record::Line>(l).ok()? {
                    baylee_gamehost::record::Line::DeclaredMind { seat: 0, mind, .. } => {
                        Some(mind.model)
                    }
                    _ => None,
                }
            })
            .collect();
        assert_eq!(models, ["claude-opus-5-5", "claude-sonnet-5-5"]);
        let replayed = baylee_gamehost::record::replay(&record).expect("the record replays");
        assert_eq!(
            replayed.engine.snapshot_hash(),
            runner.session().unwrap().snapshot_hash()
        );
    }

    /// A new socket is credited with nothing the last one declared: one that
    /// answers before it says what it is ends the declaration where its first
    /// answer stands, and one that says it again is written again. A resync,
    /// the same socket that lagged, changes nothing.
    #[test]
    fn a_new_socket_that_answers_undeclared_ends_the_last_declaration() {
        use baylee_gamehost::record::{Line, MindKind, Source};
        let mut runner = EngineRunner::new();
        setup(&mut runner, &duel(0));
        attach(&mut runner, 0);
        declare(&mut runner, 0, "claude-opus-5-5");
        ready(&mut runner, 0);
        runner.tell_time(runner.entrance_deadline().expect("the entrance"));
        runner.finish_entrance();
        play_a_little(&mut runner, 3);
        runner.handle(
            Envelope {
                msg: Some(v1::envelope::Msg::SeatAttached(v1::SeatAttached {
                    seat: 0,
                    resync: true,
                })),
            },
            &[],
        );
        play_a_little(&mut runner, 2);
        detach(&mut runner, 0);
        attach(&mut runner, 0);
        play_a_little(&mut runner, 2);
        declare(&mut runner, 0, "claude-opus-5-5");
        play_a_little(&mut runner, 2);
        let record = unpacked(&pieces(&flush(&mut runner, 1)));
        let lines: Vec<Line> = record
            .split(|&b| b == b'\n')
            .filter(|l| !l.is_empty())
            .map(|l| serde_json::from_slice(l).expect("a line"))
            .collect();
        let said: Vec<(usize, MindKind)> = lines
            .iter()
            .enumerate()
            .filter_map(|(at, l)| match l {
                Line::DeclaredMind { seat: 0, mind, .. } => Some((at, mind.kind)),
                _ => None,
            })
            .collect();
        assert_eq!(
            said.iter().map(|(_, kind)| *kind).collect::<Vec<_>>(),
            [MindKind::LlmApi, MindKind::Undeclared, MindKind::LlmApi],
            "{said:?}"
        );
        let undeclared = said[1].0;
        let seat_inputs_before = lines[..undeclared]
            .iter()
            .filter(|l| {
                matches!(
                    l,
                    Line::Input {
                        seat: 0,
                        by: Source::Seat,
                        ..
                    }
                )
            })
            .count();
        assert_eq!(
            seat_inputs_before, 5,
            "the first socket's answers, resync included"
        );
        assert!(matches!(
            lines.get(undeclared + 1),
            Some(Line::Input {
                seat: 0,
                by: Source::Seat,
                ..
            })
        ));
        baylee_gamehost::record::replay(&record).expect("the record replays");
    }

    /// A flush sends what has gathered as one piece that is not `last`, and
    /// then the acknowledgement with the gateway's nonce, in that order on
    /// the socket; the piece replays to exactly the game as it stands. It
    /// moves nothing: no frame for a seat, the same hash, the same question.
    #[test]
    fn a_flush_sends_what_has_gathered_and_then_says_so() {
        let mut runner = EngineRunner::new();
        let mut out = setup(&mut runner, &duel(0));
        out.extend(sit(&mut runner, 0));
        out.extend(play_a_little(&mut runner, 6));
        assert!(
            pieces(&out).is_empty(),
            "a few answers are under one size-due piece"
        );
        let session = runner.session().unwrap();
        assert!(session.record_pending() > 0, "the record has gathered");
        let (hash, asked) = (session.snapshot_hash(), session.asked_at(PlayerId::new(0)));

        let flushed = flush(&mut runner, 7);
        assert_eq!(flushed.len(), 2, "{flushed:?}");
        let Some(v1::envelope::Msg::GameRecordChunk(piece)) = &flushed[0].msg else {
            panic!("the piece first: {flushed:?}");
        };
        assert_eq!(
            (piece.game_id.as_str(), piece.seq, piece.last),
            ("g1", 0, false)
        );
        assert_eq!(
            flushed[1].msg,
            Some(v1::envelope::Msg::RecordFlushed(v1::RecordFlushed {
                game_id: "g1".to_string(),
                nonce: 7,
                pieces: 1,
            })),
            "then the acknowledgement, after the piece"
        );
        let session = runner.session().unwrap();
        assert_eq!(session.record_pending(), 0, "nothing left waiting");
        assert_eq!(session.snapshot_hash(), hash, "the game did not move");
        assert_eq!(session.asked_at(PlayerId::new(0)), asked);

        let replayed =
            baylee_gamehost::record::replay(&member(&piece.data)).expect("the piece replays");
        assert!(!replayed.ended);
        assert_eq!(
            replayed.engine.snapshot_hash(),
            hash,
            "to the moment of the flush"
        );
    }

    /// Every flush is answered, because the gateway waits on the answer:
    /// with nothing waiting, before the game is built and after it ended
    /// the answer is the acknowledgement alone, never an empty piece.
    #[test]
    fn a_flush_with_nothing_waiting_is_answered_with_the_acknowledgement_alone() {
        let ack = |nonce: u64, pieces: u32, game_id: &str| Envelope {
            msg: Some(v1::envelope::Msg::RecordFlushed(v1::RecordFlushed {
                game_id: game_id.to_string(),
                nonce,
                pieces,
            })),
        };
        let mut runner = EngineRunner::new();
        assert_eq!(
            flush(&mut runner, 1),
            [ack(1, 0, "")],
            "before the game is built"
        );

        setup(&mut runner, &duel(0));
        sit(&mut runner, 0);
        play_a_little(&mut runner, 2);
        let first = flush(&mut runner, 2);
        assert_eq!(pieces(&first).len(), 1);
        assert_eq!(
            flush(&mut runner, 3),
            [ack(3, 1, "g1")],
            "nothing gathered since the last flush"
        );

        let out = act(&mut runner, 0, &PlayerAction::Concede);
        assert!(runner.finished());
        assert_eq!(pieces(&out).last().map(|p| p.last), Some(true));
        assert_eq!(
            flush(&mut runner, 4),
            [ack(4, 2, "g1")],
            "after the last piece there is nothing more to send"
        );
    }

    /// Record that waits [`RECORD_FLUSH_MS`] goes without being asked for,
    /// counted from when it began to wait, not from the last piece; not
    /// while the curtain is down, and never a moment early.
    #[test]
    fn the_record_goes_by_time_once_it_has_waited_long_enough() {
        const T0: u64 = 1_700_000_000_000;
        let mut runner = EngineRunner::new();
        runner.tell_time(T0);
        assert_eq!(runner.record_deadline(), None, "no game, no record");
        setup(&mut runner, &duel(0));
        assert!(runner.curtain_pending());
        assert_eq!(runner.record_deadline(), None, "not while seats load");
        runner.tell_time(T0 + RECORD_FLUSH_MS);
        assert!(runner.flush_record_due().is_empty());

        runner.tell_time(T0);
        sit(&mut runner, 0);
        let due = T0 + RECORD_FLUSH_MS;
        assert_eq!(
            runner.record_deadline(),
            Some(due),
            "the header has waited since the game was built"
        );
        runner.tell_time(due - 1);
        assert!(runner.flush_record_due().is_empty(), "never early");
        runner.tell_time(due);
        let sent = pieces(&runner.flush_record_due());
        assert_eq!(sent.len(), 1);
        assert!(!sent[0].last);
        assert_eq!(runner.record_deadline(), None, "nothing waits now");
        assert!(
            runner.flush_record_due().is_empty(),
            "a stale timer does nothing"
        );

        // Bytes that begin to wait at T1 are due at T1 + RECORD_FLUSH_MS,
        // however much more joins them before then.
        let t1 = due + 5 * RECORD_FLUSH_MS;
        runner.tell_time(t1);
        play_a_little(&mut runner, 1);
        assert_eq!(runner.record_deadline(), Some(t1 + RECORD_FLUSH_MS));
        runner.tell_time(t1 + RECORD_FLUSH_MS / 2);
        play_a_little(&mut runner, 1);
        assert_eq!(runner.record_deadline(), Some(t1 + RECORD_FLUSH_MS));

        // A flush the gateway asked for starts the wait over.
        flush(&mut runner, 9);
        assert_eq!(runner.record_deadline(), None);

        act(&mut runner, 0, &PlayerAction::Concede);
        assert!(runner.finished());
        assert_eq!(runner.record_deadline(), None, "the game is over");
    }

    /// A tiny deterministic generator, so the property below is the same
    /// every run.
    struct XorShift(u64);

    impl XorShift {
        fn below(&mut self, n: u64) -> u64 {
            self.0 ^= self.0 << 13;
            self.0 ^= self.0 >> 7;
            self.0 ^= self.0 << 17;
            self.0 % n
        }
    }

    /// Flushes at any moment change nothing but where the pieces break
    /// (#323). One game played twice, answer for answer and millisecond for
    /// millisecond: once sending only size-due pieces, once also flushing at
    /// random moments and on the timer. Each piece is a gzip member on its
    /// own; at every flush the pieces so far replay to exactly the game as
    /// it stands; and the two records are the same bytes.
    #[test]
    fn flushes_at_any_moment_change_nothing_but_where_the_pieces_break() {
        let me = PlayerId::new(0);
        for seed in [0x9e37_79b9_7f4a_7c15_u64, 0x2545_f491_4f6c_dd1d] {
            let mut rng = XorShift(seed);
            let mut plain = EngineRunner::new();
            let mut flushed = EngineRunner::new();
            let (mut out_plain, mut out_flushed) = (Vec::new(), Vec::new());
            for (runner, out) in [
                (&mut plain, &mut out_plain),
                (&mut flushed, &mut out_flushed),
            ] {
                runner.tell_time(1_700_000_000_000);
                out.extend(setup(runner, &duel(0)));
                out.extend(sit(runner, 0));
            }
            let mut now = plain.now;
            let (mut asked, mut checked) = (0, 0);
            for _ in 0..5_000 {
                if flushed.finished() {
                    break;
                }
                let action = flushed
                    .session()
                    .and_then(|s| s.house_action(me))
                    .expect("the player is asked while the game goes on");
                now += rng.below(20_000);
                plain.tell_time(now);
                flushed.tell_time(now);
                out_plain.extend(act(&mut plain, 0, &action));
                out_flushed.extend(act(&mut flushed, 0, &action));
                out_flushed.extend(flushed.flush_record_due());
                assert_eq!(
                    plain.session().unwrap().snapshot_hash(),
                    flushed.session().unwrap().snapshot_hash()
                );
                if !flushed.finished() && rng.below(25) == 0 {
                    asked += 1;
                    out_flushed.extend(flush(&mut flushed, asked));
                    let so_far = unpacked(&pieces(&out_flushed));
                    let replayed =
                        baylee_gamehost::record::replay(&so_far).expect("the record replays");
                    assert_eq!(
                        replayed.engine.snapshot_hash(),
                        flushed.session().unwrap().snapshot_hash(),
                        "flush {asked} reaches the moment it was asked at"
                    );
                    checked += 1;
                }
            }
            assert!(plain.finished() && flushed.finished(), "the game ended");
            assert!(checked >= 5, "only {checked} flushes checked");
            let (a, b) = (pieces(&out_plain), pieces(&out_flushed));
            assert!(
                b.len() > a.len() + checked,
                "{} pieces against {}",
                b.len(),
                a.len()
            );
            for list in [&a, &b] {
                let seqs: Vec<u32> = list.iter().map(|p| p.seq).collect();
                assert_eq!(
                    seqs,
                    (0..u32::try_from(list.len()).unwrap()).collect::<Vec<_>>()
                );
                assert_eq!(list.iter().filter(|p| p.last).count(), 1);
                assert!(list.last().unwrap().last);
            }
            assert_eq!(unpacked(&a), unpacked(&b), "the same record");
            let stored: Vec<u8> = b.iter().flat_map(|p| p.data.iter().copied()).collect();
            let mut whole = Vec::new();
            std::io::Read::read_to_end(
                &mut flate2::read::MultiGzDecoder::new(&stored[..]),
                &mut whole,
            )
            .expect("the pieces stored in order are one gzip stream");
            assert_eq!(whole, unpacked(&a));
        }
    }

    /// A game can end with nothing left to send (the last take drained it):
    /// the last piece is then an empty gzip member, which a reader of the
    /// pieces in order passes over, and `seq` still counts on.
    #[test]
    fn an_empty_last_piece_still_ends_the_record() {
        let mut runner = EngineRunner::new();
        let piece = |env: Envelope| match env.msg {
            Some(v1::envelope::Msg::GameRecordChunk(c)) => c,
            other => panic!("{other:?}"),
        };
        let first = piece(runner.record_chunk(b"a line\n", false));
        let last = piece(runner.record_chunk(b"", true));
        assert_eq!((first.seq, last.seq), (0, 1));
        assert!(last.last && !last.data.is_empty());
        let mut record = Vec::new();
        std::io::Read::read_to_end(
            &mut flate2::read::MultiGzDecoder::new(&[first.data, last.data].concat()[..]),
            &mut record,
        )
        .expect("one gzip stream");
        assert_eq!(record, b"a line\n");
    }

    /// A preset that does not describe a game has to say so. The gateway
    /// cannot read a `Pending` and would otherwise hold a table open forever
    /// waiting for a game that was never built.
    #[test]
    fn a_game_that_cannot_start_says_so_rather_than_hanging() {
        let mut runner = EngineRunner::new();
        let out = runner.handle(
            Envelope {
                msg: Some(v1::envelope::Msg::GameSetup(v1::GameSetup {
                    game_id: "g1".to_string(),
                    preset_json: b"not a preset".to_vec(),
                    seat_names: Vec::new(),
                })),
            },
            &[],
        );
        assert!(
            matches!(
                out.first().map(|e| &e.msg),
                Some(Some(v1::envelope::Msg::GameEnded(_)))
            ),
            "a broken setup was swallowed"
        );
        assert!(!runner.ready());
    }

    /// Everything before the game exists is ignored rather than acted on: a
    /// seat frame that arrives first is a race, not an attack, and neither
    /// deserves a panic.
    #[test]
    fn frames_before_the_game_exists_do_nothing() {
        let mut runner = EngineRunner::new();
        assert!(attach(&mut runner, 0).is_empty());
        assert!(
            runner
                .handle(
                    Envelope {
                        msg: Some(v1::envelope::Msg::SeatFrame(v1::SeatFrame {
                            seat: 0,
                            envelope: bytes::Bytes::new(),
                        })),
                    },
                    &[]
                )
                .is_empty()
        );
        assert!(!runner.finished());
    }

    /// A second `GameSetup` must not rebuild a game that is already being
    /// played — the seats would silently be handed a different one.
    #[test]
    fn a_game_is_built_once() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &duel(0));
        let seq_before = runner.session().expect("a game").seq();
        assert!(setup(&mut runner, &duel(0)).is_empty());
        assert_eq!(
            runner.session().expect("still the same game").seq(),
            seq_before
        );
    }
    // ------------------------------------------------- the decision clock

    /// The last view `seat` was sent.
    ///
    /// The view rather than the field, so that "was sent no view at all" and
    /// "was sent a view carrying no clock" stay two different answers: the
    /// first is a missing `Option`, the second is a present view whose field
    /// is `None`, and a test that conflated them would pass on silence.
    fn last_view(envelopes: &[Envelope], seat: u32) -> Option<baylee_gamehost::PlayerView> {
        frames(envelopes)
            .into_iter()
            .filter_map(|(s, msg)| match msg {
                v1::envelope::Msg::StateDelta(delta) if s == seat => {
                    serde_json::from_slice::<baylee_gamehost::PlayerView>(&delta.view_json).ok()
                }
                _ => None,
            })
            .next_back()
    }

    /// A question nobody has spent time on is worth the whole allowance, and
    /// `blitz` is what proves the threshold is flat: at thirty seconds the
    /// number is on from the very first question rather than appearing part
    /// way through.
    #[test]
    fn a_question_just_asked_is_shown_the_whole_allowance() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &duel(30));
        let out = sit(&mut runner, 0);
        assert_eq!(
            last_view(&out, 0)
                .expect("seat 0 was sent a view")
                .decision_remaining_ms,
            Some(30_000),
            "the opening question of a blitz table showed no clock"
        );
    }

    /// The reading the caller took is what the seat is shown — the case the
    /// parameter exists for, and the one a reconnecting player depends on.
    #[test]
    fn a_reading_taken_for_this_question_is_what_the_seat_is_shown() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &duel(600));
        sit(&mut runner, 0);
        // A socket returning mid-question: the game has not moved, so this
        // is the same question with four seconds left on it.
        let out = runner.handle(
            Envelope {
                msg: Some(v1::envelope::Msg::SeatAttached(v1::SeatAttached {
                    seat: 0,
                    resync: true,
                })),
            },
            &reading(&runner, 4_000),
        );
        assert_eq!(
            last_view(&out, 0)
                .expect("seat 0 was sent a view")
                .decision_remaining_ms,
            Some(4_000),
            "a seat that reconnected was shown the allowance it started with, \
             not the time it has left"
        );
    }

    /// The anchor check, and the one that will silently regress if someone
    /// later simplifies the `decision_seq` comparison away: once the question
    /// has moved, the previous question's leftovers must not run against it.
    #[test]
    fn a_view_built_after_the_question_moved_shows_the_whole_allowance() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &duel(600));
        sit(&mut runner, 0);
        let before = runner.session().expect("a game").decision_seq();
        // Four seconds left on *this* question, and then an answer that
        // moves the game on to the next one. The reading has to be a partial
        // one or the test proves nothing: with the whole allowance on both
        // sides, a stale value and a correct one are the same number.
        let inner = Envelope {
            msg: Some(v1::envelope::Msg::PlayerAction(v1::PlayerActionMsg {
                game_id: "g1".to_string(),
                seat_token: String::new(),
                action_json: serde_json::to_vec(&PlayerAction::MulliganKeep)
                    .expect("action serializes"),
            })),
        };
        let out = runner.handle(
            Envelope {
                msg: Some(v1::envelope::Msg::SeatFrame(v1::SeatFrame {
                    seat: 0,
                    envelope: prost::Message::encode_to_vec(&inner).into(),
                })),
            },
            &reading(&runner, 4_000),
        );
        assert_ne!(
            runner.session().expect("a game").decision_seq(),
            before,
            "this test needs an action that actually asks a new question"
        );
        let seen = last_view(&out, 0).expect("seat 0 was sent a view");
        assert_eq!(
            seen.decision_remaining_ms,
            Some(600_000),
            "the new question inherited the old one's remainder"
        );
    }

    /// An untimed table has no number, and `0` would be a seat with no time
    /// left rather than a seat with no limit.
    #[test]
    fn an_untimed_table_shows_no_number_at_all() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &duel(0));
        let out = attach(&mut runner, 0);
        assert_eq!(
            last_view(&out, 0)
                .expect("seat 0 was sent a view")
                .decision_remaining_ms,
            None,
            "an untimed table drew a countdown"
        );
    }

    /// A seat whose socket is gone is on the stand-in clock, which is not a
    /// decision clock: the other seat must not be shown a countdown against
    /// a player who is not deciding.
    #[test]
    fn a_seat_waiting_out_its_reconnect_window_is_on_no_decision_clock() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &two_humans(600));
        sit(&mut runner, 0);
        sit(&mut runner, 1);
        // Past the mulligans, which ask both seats at once: this is about
        // the seat that is not being asked.
        keep_both(&mut runner);
        detach(&mut runner, 0);
        // Seat 1 is still here and still being sent views; seat 0, which the
        // table is waiting on, is on the reconnect window instead.
        let out = sit(&mut runner, 1);
        assert_eq!(
            on_clock(&runner).map(|c| c.what),
            Some(Deadline::StandIn),
            "this test needs seat 0 to be on the reconnect window"
        );
        assert_eq!(
            last_view(&out, 1)
                .expect("seat 1 was sent a view")
                .decision_remaining_ms,
            None,
            "a seat with no socket was drawn a decision countdown"
        );
    }

    /// The clock is public: the seat that is *not* being asked is told how
    /// long the seat that is has left. A table where one player is running
    /// out of time and nobody else can see it reads the pause as rudeness.
    ///
    /// From turn 1 on, so past the mulligans, where every seat is asked its
    /// own question and told its own remainder (the next test).
    #[test]
    fn the_other_seat_is_told_the_awaited_seats_remainder() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &two_humans(600));
        sit(&mut runner, 0);
        sit(&mut runner, 1);
        act(&mut runner, 0, &PlayerAction::MulliganKeep);
        let out = act(&mut runner, 1, &PlayerAction::MulliganKeep);
        let awaited = runner.session().expect("a game").awaiting_seat();
        assert_eq!(
            awaited.map(PlayerId::get),
            Some(0),
            "this test needs seat 0 to be the one being asked"
        );
        assert_eq!(
            last_view(&out, 1)
                .expect("seat 1 was sent a view")
                .decision_remaining_ms,
            Some(600_000),
            "seat 1 was not told the clock seat 0 is on"
        );
    }

    /// During the opening mulligans each deciding seat is on its own clock
    /// and is told its own remainder. A seat that has kept is told none: its
    /// view waits on nobody (`PlayerView::awaiting`), so a number there
    /// would name nobody's clock.
    #[test]
    fn in_the_mulligans_each_seat_is_told_its_own_remainder() {
        let mut runner = EngineRunner::new();
        setup(&mut runner, &two_humans(600));
        sit(&mut runner, 0);
        sit(&mut runner, 1);
        let [zero, one] = runner.clocks()[..] else {
            panic!("both seats are deciding");
        };
        // Seat 0 takes with nine seconds left and seat 1 has four: seat 0 is
        // asked anew, and seat 1 is still on the question it had.
        let out = act_reading(
            &mut runner,
            0,
            &PlayerAction::MulliganTake,
            &[(zero, 9_000), (one, 4_000)],
        );
        let told = |out: &[Envelope], seat: u32| {
            let view = last_view(out, seat).expect("a view");
            (view.awaiting.map(PlayerId::get), view.decision_remaining_ms)
        };
        assert_eq!(told(&out, 0), (Some(0), Some(600_000)));
        assert_eq!(told(&out, 1), (Some(1), Some(4_000)));

        // Seat 1 keeps with nine seconds gone from seat 0's new question,
        // which is still the question seat 0 owes: the reading stays with it.
        let [zero, one] = runner.clocks()[..] else {
            panic!("both seats are deciding");
        };
        let out = act_reading(
            &mut runner,
            1,
            &PlayerAction::MulliganKeep,
            &[(zero, 591_000), (one, 3_000)],
        );
        assert_eq!(told(&out, 1), (None, None), "seat 1 has kept");
        assert_eq!(told(&out, 0), (Some(0), Some(591_000)));
        let deciding = last_view(&out, 1).expect("a view").deciding;
        assert_eq!(deciding.iter().map(PlayerId::get).collect::<Vec<_>>(), [0]);
    }

    /// A seat's setting about itself, wrapped the way its socket sends it
    /// (#265).
    fn set(runner: &mut EngineRunner, seat: u32, setting: SeatSetting) -> Vec<Envelope> {
        let inner = Envelope {
            msg: Some(v1::envelope::Msg::SeatSetting(v1::SeatSettingMsg {
                setting_json: serde_json::to_vec(&setting).expect("setting serializes"),
            })),
        };
        runner.handle(
            Envelope {
                msg: Some(v1::envelope::Msg::SeatFrame(v1::SeatFrame {
                    seat,
                    envelope: prost::Message::encode_to_vec(&inner).into(),
                })),
            },
            &[],
        )
    }

    /// Two players on one team, against the house.
    fn partners() -> GamePreset {
        let mut preset = two_humans(600);
        let mut house = preset.seats[1].clone();
        house.controller =
            baylee_core::preset::SeatController::Ai(baylee_core::preset::AIProfile::default());
        house.team = Some(2);
        preset.seats[0].team = Some(1);
        preset.seats[1].team = Some(1);
        preset.seats.push(house);
        preset
    }

    /// A seat's setting is heard before the curtain is up as well as after
    /// it (#265): it is no decision and runs no clock. Either way it sends
    /// the views it changed and asks nothing, and a setting that is refused
    /// is answered to the seat that sent it and to nobody else.
    #[test]
    fn a_seat_setting_is_heard_before_the_curtain_and_after_and_asks_nothing() {
        let only_views = |out: &[Envelope]| {
            !out.is_empty()
                && said(out)
                    .iter()
                    .all(|(_, what)| matches!(*what, "static" | "view"))
        };
        let hands = |out: &[Envelope], seat: u32| {
            last_view(out, seat)
                .expect("the teammate is sent a view")
                .shared_hands
                .iter()
                .map(|h| h.player.get())
                .collect::<Vec<_>>()
        };
        let mut runner = EngineRunner::new();
        setup(&mut runner, &partners());
        attach(&mut runner, 0);
        attach(&mut runner, 1);
        assert!(runner.curtain_pending());
        let partner: baylee_core::ids::SeatSet = [PlayerId::new(1)].into_iter().collect();

        let shown = set(&mut runner, 0, SeatSetting::ShareHand(partner));
        assert!(only_views(&shown), "{:?}", said(&shown));
        assert_eq!(hands(&shown, 1), [0]);

        ready(&mut runner, 0);
        ready(&mut runner, 1);
        runner.tell_time(runner.entrance_deadline().unwrap());
        runner.finish_entrance();
        assert!(!runner.curtain_pending());
        let withdrawn = set(
            &mut runner,
            0,
            SeatSetting::ShareHand(baylee_core::ids::SeatSet::new()),
        );
        assert!(only_views(&withdrawn), "{:?}", said(&withdrawn));
        assert_eq!(hands(&withdrawn, 1), [0; 0]);

        let house: baylee_core::ids::SeatSet = [PlayerId::new(2)].into_iter().collect();
        let refused = set(&mut runner, 0, SeatSetting::ShareHand(house));
        assert_eq!(said(&refused), [(0, "error")]);
    }

    /// What a seat's mind said, wrapped the way its socket sends it.
    fn reasoning_from(runner: &mut EngineRunner, seat: u32, claimed: u32) -> Vec<Envelope> {
        let inner = Envelope {
            msg: Some(v1::envelope::Msg::AiLog(v1::AiLog {
                note: r#"{"chose":"a1 Pass"}"#.to_string(),
                thinking: "my hand is two Islands and a Counterspell".to_string(),
                seat: claimed,
            })),
        };
        runner.handle(
            Envelope {
                msg: Some(v1::envelope::Msg::SeatFrame(v1::SeatFrame {
                    seat,
                    envelope: prost::Message::encode_to_vec(&inner).into(),
                })),
            },
            &[],
        )
    }

    /// Every `AiLog` in `out`: the seat it goes to, and the seat it names.
    fn reasonings(out: &[Envelope]) -> Vec<(u32, u32)> {
        frames(out)
            .into_iter()
            .filter_map(|(to, msg)| match msg {
                v1::envelope::Msg::AiLog(said) => Some((to, said.seat)),
                _ => None,
            })
            .collect()
    }

    /// Three attached seats, two of them a team and the third the other
    /// side, each with a socket, so every seat is one that could be sent a
    /// reasoning.
    fn three_at_the_table() -> EngineRunner {
        let mut preset = partners();
        preset.seats[2].controller = baylee_core::preset::SeatController::Open;
        let mut runner = EngineRunner::new();
        setup(&mut runner, &preset);
        for seat in 0..3 {
            sit(&mut runner, seat);
        }
        runner
    }

    /// In a debug build the AI log is open to the table (the owner's
    /// decision, `docs/protocol.md` §"An AI seat's reasoning"): a seat's
    /// reasoning reaches every other attached seat, the other side and a
    /// teammate not shown the hand included, never back to the sender, and
    /// it names the seat that really sent it, whatever the sender wrote.
    #[cfg(debug_assertions)]
    #[test]
    fn in_a_debug_build_a_seats_reasoning_reaches_every_other_seat_in_its_name() {
        let mut runner = three_at_the_table();
        let claimed = reasoning_from(&mut runner, 0, 2);
        assert_eq!(
            reasonings(&claimed),
            [(1, 0), (2, 0)],
            "both other seats, told it is seat 0's though it claimed seat 2"
        );
        let opponent = reasoning_from(&mut runner, 2, 0);
        assert_eq!(reasonings(&opponent), [(0, 2), (1, 2)]);
    }

    /// In a release build, the one deployed and the one CI's `test-release`
    /// runs, nobody is sent a seat's reasoning: the engine drops it,
    /// whoever sent it and whatever the table shares.
    #[cfg(not(debug_assertions))]
    #[test]
    fn in_a_release_build_nobody_is_sent_a_seats_reasoning() {
        let mut runner = three_at_the_table();
        let partner: baylee_core::ids::SeatSet = [PlayerId::new(1)].into_iter().collect();
        set(&mut runner, 0, SeatSetting::ShareHand(partner));
        for (from, claimed) in [(0, 0), (0, 2), (1, 1), (2, 2)] {
            let out = reasoning_from(&mut runner, from, claimed);
            assert_eq!(reasonings(&out), [], "seat {from}'s reasoning went out");
            assert_eq!(said(&out), [], "seat {from}'s reasoning sent something");
        }
    }
}
