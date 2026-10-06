use super::{SeatKind, Session, seat_byte, state_frames};
use baylee_ai::policy_seed;
use baylee_core::ids::PlayerId;
use baylee_engine::choice::Pending;
use baylee_protocol::v1;
use baylee_protocol::v1::Envelope;
use baylee_view::{GameStatic, LogTail, SeatIdentity};

impl Session {
    /// The seats that won: one for a solo winner, every seat on the team for
    /// a team win — the dead ones included, because a team wins as a team
    /// (CR 104.2c) — and none at all for a draw.
    ///
    /// It lives here rather than in the engine because a `Victor::Team` names
    /// a team and a client's roster names seats, and the seat roster is what
    /// this session already keeps.
    #[must_use]
    pub fn winning_seats(&self, result: baylee_engine::win::GameResult) -> Vec<PlayerId> {
        let Some(victor) = result.winner else {
            return Vec::new();
        };
        (0..self.seats.len())
            .map(|i| PlayerId::new(i as u8))
            .filter(|seat| {
                victor.includes(
                    *seat,
                    self.teams.get(seat.get() as usize).copied().flatten(),
                )
            })
            .collect()
    }

    /// Names the table for the payload clients are sent.
    ///
    /// The rules kernel has never heard of an account, so a host supplies this
    /// once, after building the session and before the first socket. A seat
    /// nobody names falls back to its number rather than to an empty chair.
    pub fn describe(&mut self, game_id: String, names: Vec<String>) {
        // The public game identifier arrives exactly here, once, before the
        // first socket — so this is where an AI chair stops playing with the
        // no-identifier derivation and starts playing with its own (#87).
        // Every chair that holds an agent is reseeded, including the two that
        // are not currently answering: a seat handed back by `release` or
        // `hand_back` plays on with the agent it kept.
        for (i, kind) in self.seats.iter_mut().enumerate() {
            let seed = policy_seed(&game_id, seat_byte(i));
            match kind {
                SeatKind::Ai(agent) | SeatKind::Driven(agent) | SeatKind::StandIn(agent) => {
                    *agent = agent.clone().with_seed(seed);
                }
                SeatKind::Human => {}
            }
        }
        self.game_id = game_id;
        self.names = names;
    }

    /// The once-per-game payload for a seat: the seat roster, the print table
    /// as far as this seat has earned it, and the view schema version.
    #[must_use]
    pub fn game_static(&self, seat: PlayerId) -> GameStatic {
        let seats = self
            .seats
            .iter()
            .enumerate()
            .map(|(i, kind)| SeatIdentity {
                player: PlayerId::new(i as u8),
                display_name: self
                    .names
                    .get(i)
                    .cloned()
                    .unwrap_or_else(|| format!("Seat {i}")),
                is_ai: kind.is_ai_chair(),
                away: kind.is_away(),
                team: self.teams.get(i).copied().flatten(),
            })
            .collect();
        let shown = self
            .revealed
            .get(seat.get() as usize)
            .map_or(&[][..], Vec::as_slice);
        crate::view::game_static(
            self.game_id.clone(),
            seat,
            seats,
            &self.prints,
            shown,
            &self.house_rules,
        )
    }

    /// [`Session::game_static`] as the envelope a socket sends.
    ///
    /// Every host — the in-process one included — takes this payload off the
    /// wire rather than building it from a preset it happens to have in hand.
    /// A field that only the local path filled in would be missing in exactly
    /// the case nobody tests at a desk.
    ///
    /// Producing the envelope *is* the act of telling the seat, which is why
    /// this takes `&mut self` where [`Session::game_static`] does not: it
    /// clears the seat's pending roster change. Anything that only wants to
    /// look reads `game_static`.
    #[must_use]
    pub fn game_static_envelope(&mut self, seat: PlayerId) -> Envelope {
        if let Some(dirty) = self.roster_dirty.get_mut(seat.get() as usize) {
            *dirty = false;
        }
        // The hands this seat is shown (#265), whose cards may have changed
        // while nobody was on the seat: a chair the house holds is built no
        // view, and what a returning seat is sent after this may be a
        // read-only snapshot, which reveals nothing.
        if self
            .seat_kind(seat)
            .is_some_and(SeatKind::answers_over_socket)
        {
            let [shown, ..] = self.hands_told(seat);
            let prints: Vec<_> = shown
                .iter()
                .flat_map(|owner| crate::view::own_hand(self.engine.state(), owner))
                .map(|card| card.card.print)
                .collect();
            self.reveal(seat, prints);
        }
        let statics = self.game_static(seat);
        Envelope {
            msg: Some(v1::envelope::Msg::GameStatic(v1::GameStaticMsg {
                game_id: self.game_id.clone(),
                view_version: baylee_view::VIEW_VERSION,
                static_json: serde_json::to_vec(&statics).unwrap_or_default(),
            })),
        }
    }

    /// Marks every printing a seat was shown, by its view or its log; true
    /// when any was new.
    fn reveal(
        &mut self,
        seat: PlayerId,
        prints: impl IntoIterator<Item = baylee_core::ids::PrintRef>,
    ) -> bool {
        let Some(shown) = self.revealed.get_mut(seat.get() as usize) else {
            return false;
        };
        let mut grew = false;
        for print in prints {
            if let Some(slot) = shown.get_mut(print.get() as usize)
                && !*slot
            {
                *slot = true;
                grew = true;
            }
        }
        grew
    }

    /// A seat's view and the log lines it has not been sent, preceded by a
    /// fresh opening payload when these are the first to show it one of the
    /// game's printings, or when a chair has changed hands since this seat
    /// was last told the roster.
    ///
    /// The order matters: the entry has to be there before the object or the
    /// line that points at it, or the client draws a card it cannot key an
    /// image on.
    pub(super) fn view_envelopes(&mut self, seat: PlayerId) -> Vec<Envelope> {
        let awaiting = crate::view::awaiting_for(&self.engine, seat);
        let mut view = crate::view::player_view(
            self.engine.state(),
            seat,
            self.seq,
            self.engine.information_pending_for(seat),
            &crate::view::SeatContext {
                awaiting,
                decision_player: crate::view::decision_player_for(&self.engine, seat),
                controlled_players: self.engine.controlled_players(seat),
                deciding: self.deciding(),
                held: self.engine.automation(seat).hold.suppresses(),
                owed: crate::view::owed_payment(&self.engine),
                library_reveal_blocked: self.engine.library_reveal_blocked(),
                decision_remaining_ms: awaiting.and_then(|s| self.decision_remaining_ms(s)),
                policy_acts: &self.policy_acts,
            },
            &self.house_answered,
        );
        view.targeting = crate::view::targeting_context(&self.engine, seat);
        self.show_hands(&mut view, seat);
        let mut out = Vec::new();
        let tail = self.log_tail(seat);
        // Two separate `let`s: `reveal` marks printings as shown, so folding
        // it into an `||` would let the other half short-circuit it away.
        let revealed = self.reveal(seat, view.prints().chain(tail.prints()));
        let roster_moved = self
            .roster_dirty
            .get(seat.get() as usize)
            .copied()
            .unwrap_or(false);
        if revealed || roster_moved {
            out.push(self.game_static_envelope(seat));
        }
        out.extend(state_frames(self.seq, &view, tail));
        out
    }

    /// The log lines `seat` has not been sent yet, as it may know them, and
    /// marks them sent.
    ///
    /// Nothing for a seat nobody answers over a socket. An agent is handed a
    /// view and a question, never a log: a log is a history, and what a
    /// house seat may know of the game is exactly what its view shows now.
    /// A chair the house holds for an absent player has nobody to tell, and
    /// the player is sent the whole log when they are back
    /// ([`Session::retell_log`]).
    pub(super) fn log_tail(&mut self, seat: PlayerId) -> LogTail {
        let at = seat.get() as usize;
        let socket = self
            .seats
            .get(at)
            .is_some_and(SeatKind::answers_over_socket);
        let lines = self.log.len();
        let Some(told) = self.told.get_mut(at).filter(|_| socket) else {
            return LogTail::default();
        };
        let from = (*told).min(lines);
        *told = lines;
        self.log.seal(lines);
        LogTail {
            from: u32::try_from(from).unwrap_or(u32::MAX),
            entries: self.log.told(seat, from, lines),
        }
    }

    /// Every log line `seat` has been sent, from the first, for a rebuild.
    ///
    /// Read-only: nothing is marked sent and no printing is earned, because
    /// both happened when the lines were first sent. Lines not sent yet are
    /// not here either; they come with the next view, as they would have.
    pub(super) fn log_sent(&self, seat: PlayerId) -> LogTail {
        let at = seat.get() as usize;
        if !self
            .seats
            .get(at)
            .is_some_and(SeatKind::answers_over_socket)
        {
            return LogTail::default();
        }
        let sent = self.told.get(at).copied().unwrap_or(0).min(self.log.len());
        LogTail {
            from: 0,
            entries: self.log.told(seat, 0, sent),
        }
    }

    /// `seat`'s next view carries its whole log again, from the first line.
    ///
    /// For a socket that has just attached and will be pumped: it holds none
    /// of the log, and whatever went to the seat while nobody was on it was
    /// dropped on the way. A resync that is sent a [`Session::snapshot`]
    /// needs no call, since the snapshot carries every line already sent.
    pub fn retell_log(&mut self, seat: PlayerId) {
        if let Some(told) = self.told.get_mut(seat.get() as usize) {
            *told = 0;
        }
    }

    /// Tells the session the time, in milliseconds since the Unix epoch: the
    /// game log stamps every line it writes from here on with it (#300),
    /// until it is told another. The session never reads a clock of its own,
    /// because it runs in a browser too, where the standard library has
    /// none; whoever drives it tells it the time before each call that can
    /// write a line.
    pub fn tell_time(&mut self, unix_ms: u64) {
        self.log.tell_time(unix_ms);
        if let Some(record) = self.record.as_mut() {
            record.tell_time(unix_ms);
        }
    }

    /// The engine's determinism hash, which a record checks its replay
    /// against (#315).
    #[must_use]
    pub fn snapshot_hash(&self) -> u64 {
        self.engine.snapshot_hash()
    }

    /// Read-only state access (views).
    #[must_use]
    pub fn state(&self) -> &baylee_engine::state::GameState {
        self.engine.state()
    }

    /// The current pending choice.
    #[must_use]
    pub fn pending(&self) -> &Pending {
        self.engine.pending()
    }
}
