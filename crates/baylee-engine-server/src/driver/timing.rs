use crate::{
    Clock, Deadline, EngineRunner, Envelope, HOLD_SECS, LoopDetection, Pending, PlayerId, SeatKind,
    Session, SessionRegistry,
};

impl<D: LoopDetection, R: SessionRegistry> EngineRunner<D, R> {
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
        if session.needs_pump() {
            return Vec::new();
        }
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
    pub(crate) fn another_player_here(
        &self,
        session: &Session<D, R>,
        seat: Option<PlayerId>,
    ) -> bool {
        self.attached.iter().any(|&other| {
            Some(other) != seat.map(PlayerId::get)
                && session
                    .seat_kind(PlayerId::new(other))
                    .is_some_and(SeatKind::answers_over_socket)
        })
    }

    /// A player's chair still in the game whose socket is gone.
    pub(crate) fn absent_players(&self, session: &Session<D, R>) -> Vec<PlayerId> {
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
    pub(crate) fn hold_clock(&self, session: &Session<D, R>) -> Option<Clock> {
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
    pub(crate) fn clock_for(&self, session: &Session<D, R>, seat: PlayerId) -> Option<Clock> {
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
    pub(crate) fn stand_in_secs(&self, session: &Session<D, R>, seat: PlayerId) -> u32 {
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
        let budget = self.pump_budget;
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
            Deadline::Decide => {
                match session.answer_by_clock_slice(clock.seat, clock.seq, budget) {
                    None => Vec::new(),
                    Some(Ok(routed)) => {
                        let mut out = self.route(&routed);
                        out.extend(self.ending());
                        out
                    }
                    Some(Err(reason)) => self.refused(clock.seat, &reason),
                }
            }
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
                let routed = session.pump_slice(budget);
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
    pub(crate) fn absorb_clock(&mut self) {
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
}
