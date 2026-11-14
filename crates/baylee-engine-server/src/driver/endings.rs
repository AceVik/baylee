use crate::{
    EngineRunner, Envelope, LoopDetection, PlayerId, REMAINDER_MS, SeatKind, Session,
    SessionRegistry, v1,
};

impl<D: LoopDetection, R: SessionRegistry> EngineRunner<D, R> {
    /// What the gateway is owed after the game moved: the game's record
    /// (#315) once [`RECORD_CHUNK_BYTES`] of it have gathered, and when the
    /// game is over the rest of it and then `GameEnded`, once.
    ///
    /// The gateway cannot read a `Pending` — it does not link the engine — so
    /// the end of a game has to be said outright. The record goes before it
    /// on the same socket, so a gateway that has read `GameEnded` has read
    /// the whole record.
    pub(crate) fn ending(&mut self) -> Vec<Envelope> {
        let mut out = self.participation_endings();
        out.extend(self.game_ending());
        out.extend(self.remainder_bound());
        out
    }

    /// Whether a seat that answers over a socket still plays: a person's, a
    /// seat bridge's, a developer's. Without one the house plays the rest of
    /// the game alone, at the machine's speed.
    pub(crate) fn sockets_remain(session: &Session<D, R>) -> bool {
        (0..session.seat_count())
            .filter_map(|seat| u8::try_from(seat).ok())
            .map(PlayerId::new)
            .any(|seat| {
                !session.has_left(seat)
                    && session
                        .seat_kind(seat)
                        .is_some_and(SeatKind::answers_over_socket)
            })
    }

    /// The house plays a game on alone once every seat that answers over a
    /// socket has left it, so that the result stands (a player who conceded
    /// lost, one whose house teammate wins won) and the record is whole. It
    /// does so by its heuristics (no search: `HostForesight::needs_a_person`)
    /// and within [`REMAINDER_ANSWERS`] of its answers and [`REMAINDER_MS`]
    /// of time: past either the game is closed as an admin closes one
    /// ([`Self::close`]), no winners and the record ending where it stopped,
    /// so a game everybody left can never keep its engine busy for long. A
    /// seat that comes back starts the count again. Read after each pump
    /// slice, so the house may run past the count by one slice
    /// ([`ATTACHED_PUMP_SLICE`] answers).
    pub(crate) fn remainder_bound(&mut self) -> Vec<Envelope> {
        let Some(session) = self.session.as_ref() else {
            return Vec::new();
        };
        if self.ended || Self::sockets_remain(session) {
            self.remainder_from = None;
            return Vec::new();
        }
        let answers = session.decision_seq();
        let (from, since) = *self.remainder_from.get_or_insert((answers, self.now));
        let alone = answers.saturating_sub(from);
        let ms = self.now.saturating_sub(since);
        if alone < self.remainder_answers && ms < REMAINDER_MS {
            return Vec::new();
        }
        tracing::warn!(
            game_id = self.game_id,
            answers = alone,
            ms,
            "nobody is left at the table and the house did not finish the game within its bound"
        );
        self.closed(
            "nobody is left at the table, and the house did not finish the game",
            "nobody is left",
        )
    }

    /// Only the rules session can release a player's active-game hold.
    /// Emit each terminal seat once, including when other players continue.
    /// A house-held human chair is also released once no human is still playing.
    pub(crate) fn participation_endings(&mut self) -> Vec<Envelope> {
        let Some(session) = self.session.as_ref() else {
            return Vec::new();
        };
        let humans_remain = Self::sockets_remain(session);
        let mut seats = Vec::new();
        for seat in (0..session.seat_count())
            .filter_map(|seat| u8::try_from(seat).ok())
            .map(PlayerId::new)
        {
            let finished = session.has_left(seat)
                || (!humans_remain
                    && matches!(session.seat_kind(seat), Some(SeatKind::StandIn(_))));
            if finished && !self.participation_reported.contains(seat) {
                self.participation_reported.insert(seat);
                seats.push(u32::from(seat.get()));
            }
        }
        if seats.is_empty() {
            Vec::new()
        } else {
            vec![Envelope {
                msg: Some(v1::envelope::Msg::SeatParticipationEnded(
                    v1::SeatParticipationEnded { seats },
                )),
            }]
        }
    }
}
