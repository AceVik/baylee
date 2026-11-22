use crate::{
    EngineRunner, Envelope, LoopDetection, PlayerId, SeatKind, SessionRegistry, curtain, frames,
    seat_frame, seat_frames, v1,
};

impl<D: LoopDetection, R: SessionRegistry, T: crate::recording::Transport> EngineRunner<D, R, T> {
    /// A seat's socket opened. Sends it the payload every later frame refers
    /// to, then either advances the game for it or hands it what it missed.
    pub(crate) fn attach(&mut self, attached: v1::SeatAttached) -> Vec<Envelope> {
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
        let budget = self.pump_budget;
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
            session.retell(player);
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
            session.retell(player);
            let routed = session.pump_slice(budget);
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
    pub(crate) fn spectators_changed(&mut self, changed: v1::SpectatorsChanged) -> Vec<Envelope> {
        self.spectators = changed.count;
        let Some(session) = self.session.as_mut() else {
            return Vec::new();
        };
        session.set_spectators(changed.count);
        let mut out: Vec<Envelope> = seat_frames(
            self.attached.iter().copied(),
            &frames::spectators(changed.count),
        )
        .collect();
        if changed.joined {
            out.extend(
                session
                    .spectator_snapshot()
                    .iter()
                    .map(frames::spectator_frame),
            );
            out.push(frames::spectator_frame(&frames::spectators(changed.count)));
            out.push(frames::spectator_frame(&curtain()));
        } else if changed.count > 0 {
            out.push(frames::spectator_frame(&frames::spectators(changed.count)));
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
    pub(crate) fn detach(&mut self, detached: v1::SeatDetached) -> Vec<Envelope> {
        let Ok(seat) = u8::try_from(detached.seat) else {
            return Vec::new();
        };
        let was_here = self.attached.contains(&seat);
        if seat == 0 && self.is_challenge() {
            self.challenge.interrupt();
            if detached.left {
                return self.closed("challenge abandoned", "challenge abandoned");
            }
        }
        self.attached.retain(|s| *s != seat);
        if self.ranked_duel && detached.left && !self.ended {
            if self.curtain.is_some() {
                return self.closed(
                    "A ranked player left before play began.",
                    "ranked preparation cancelled",
                );
            }
            return self.forfeit_ranked(PlayerId::new(seat));
        }
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
        let budget = self.pump_budget;
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
                session.pump_slice(budget)
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
        let routed = session.pump_slice(budget);
        self.route(&routed)
    }

    /// [`Self::another_player_here`] against the session as it stands.
    pub(crate) fn another_player_here_now(&self, seat: PlayerId) -> bool {
        self.session
            .as_ref()
            .is_some_and(|session| self.another_player_here(session, Some(seat)))
    }

    /// The table was held a whole [`crate::HOLD_SECS`] with nobody at it: the house
    /// concedes for every absent player, and the game ends.
    pub(crate) fn hold_expired(&mut self) -> Vec<Envelope> {
        let Some(session) = self.session.as_ref() else {
            return Vec::new();
        };
        if self.ended || self.another_player_here(session, None) {
            return Vec::new();
        }
        if self.ranked_duel {
            return self.closed(
                "Both ranked players disconnected.",
                "ranked table abandoned",
            );
        }
        if session.is_challenge() {
            return self.closed(
                "challenge connection expired",
                "challenge connection expired",
            );
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

    /// Let the rules engine settle a ranked concession, including the record and
    /// the normal game-over view for the remaining player.
    pub(crate) fn forfeit_ranked(&mut self, seat: PlayerId) -> Vec<Envelope> {
        let Some(routed) = self
            .session
            .as_mut()
            .and_then(|s| s.concede_for_absent(seat))
        else {
            return Vec::new();
        };
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
    pub(crate) fn route(&self, routed: &[(PlayerId, Envelope)]) -> Vec<Envelope> {
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
}
