use super::{MAX_DECLARATIONS, SeatKind, Session, seat_byte};
use crate::record::{ChairChange, Mind};
use baylee_ai::HeuristicAgent;
use baylee_core::ids::{PlayerId, SeatSet};
use baylee_core::preset::AIProfile;
use baylee_engine::choice::Pending;
use baylee_protocol::v1::Envelope;
use baylee_view::{LogEvent, PlayerView, SeatSetting, SharedHand};

impl Session {
    /// A socket opened for `seat` (not a resync of one already open): what
    /// its client declares next is the seat's mind, and if it answers
    /// before declaring anything, the record says the earlier declaration
    /// no longer holds ([`MindKind::Undeclared`]).
    pub fn socket_opened(&mut self, seat: PlayerId) {
        if let Some(unsaid) = self.unsaid.get_mut(seat.get() as usize) {
            *unsaid = true;
        }
    }

    /// What `seat`'s own client says answers it (`v1::SeatMind`): a person,
    /// the house, a script, or a language model and which. Written into the
    /// record when it differs from what the seat declared last, and shown
    /// to nobody: no view, roster or log line changes, and the game does not
    /// move (`docs/protocol.md` §"Who answers a seat, as it says").
    ///
    /// Self-declared and unverified, so only its shape is checked
    /// ([`Mind::from_wire`]): nothing in it can be an address, a key or a
    /// prompt. Taken before the curtain as after. A session with no record
    /// checks it and keeps nothing.
    ///
    /// # Errors
    /// When the declaration's shape is refused, `seat` is not a seat here,
    /// or the seat has declared [`MAX_DECLARATIONS`] times already.
    pub fn declare_mind(
        &mut self,
        seat: PlayerId,
        said: &baylee_protocol::v1::SeatMind,
    ) -> Result<(), &'static str> {
        let mind = Mind::from_wire(said)?;
        let at = seat.get() as usize;
        if at >= self.seats.len() {
            return Err("no such seat");
        }
        let Some(record) = self.record.as_mut() else {
            return Ok(());
        };
        self.unsaid[at] = false;
        if self.minds[at].as_ref() == Some(&mind) {
            return Ok(());
        }
        if self.declarations[at] >= MAX_DECLARATIONS {
            return Err("this seat has declared its mind too often");
        }
        self.declarations[at] += 1;
        record.mind(seat, mind.clone());
        self.minds[at] = Some(mind);
        Ok(())
    }

    /// Writes down a change of who answers `seat`.
    fn chair_changed(&mut self, seat: PlayerId, change: ChairChange) {
        if let Some(record) = self.record.as_mut() {
            record.chair(seat, change);
        }
    }

    /// The seats that are played over a socket, in seat order.
    ///
    /// Humans, and any AI seat whose controls have been taken — the two are
    /// the same thing to everything that sends views, which is why the name
    /// is about the socket and not about who is on the other end of it.
    #[must_use]
    pub fn human_seats(&self) -> Vec<PlayerId> {
        self.seats
            .iter()
            .enumerate()
            .filter(|(_, k)| k.answers_over_socket())
            .map(|(i, _)| PlayerId::new(i as u8))
            .collect()
    }

    /// What kind of chair a seat is, or `None` when it is not a seat here.
    ///
    /// The three answers are not interchangeable to a caller deciding whether
    /// someone may sit down: an AI chair can be taken over, a human chair can
    /// be joined, and a chair already being driven must be refused — and
    /// [`Session::take_over`] alone cannot tell the last two apart, because
    /// both are already answering over a socket.
    #[must_use]
    pub fn seat_kind(&self, seat: PlayerId) -> Option<&SeatKind> {
        self.seats.get(seat.get() as usize)
    }

    /// Takes the controls of an AI seat, so it answers over a socket instead.
    ///
    /// Returns whether it took: a seat that is already human, already driven,
    /// or is not a seat at all is refused rather than silently accepted,
    /// because a caller that thinks it is driving a chair it is not would sit
    /// waiting for a question the house AI has already answered.
    ///
    /// The game is not disturbed. Whatever the agent has already played
    /// stands, and the next question addressed to this seat simply goes out
    /// over the wire rather than into [`HeuristicAgent::act`].
    pub fn take_over(&mut self, seat: PlayerId) -> bool {
        let Some(kind) = self.seats.get_mut(seat.get() as usize) else {
            return false;
        };
        let SeatKind::Ai(agent) = kind else {
            return false;
        };
        *kind = SeatKind::Driven(agent.clone());
        self.roster_changed();
        self.chair_changed(seat, ChairChange::TakenOver);
        true
    }

    /// Hands a driven seat back to the house AI it was taken from.
    ///
    /// Returns whether it was being driven. The agent comes back with it —
    /// it was kept rather than dropped precisely so that a developer who
    /// disconnects mid-game leaves a playable opponent behind instead of a
    /// table that stops at the next question nobody is there to answer.
    pub fn release(&mut self, seat: PlayerId) -> bool {
        let Some(kind) = self.seats.get_mut(seat.get() as usize) else {
            return false;
        };
        let SeatKind::Driven(agent) = kind else {
            return false;
        };
        *kind = SeatKind::Ai(agent.clone());
        self.roster_changed();
        self.house_takes_hands(seat);
        self.chair_changed(seat, ChairChange::Released);
        true
    }

    /// The house plays `seat` from now on, which is shown no teammate's hand
    /// and shows its own to every teammate who asks (#265).
    ///
    /// Withdrawn here rather than only left out of the next view: a chair
    /// that is taken over again later must not find a share waiting for it
    /// that its teammate made to somebody else.
    fn house_takes_hands(&mut self, seat: PlayerId) {
        let at = seat.get() as usize;
        for (owner, shown) in self.showing.iter_mut().enumerate() {
            if owner != at {
                *shown = shown.iter().filter(|&s| s != seat).collect();
            }
            if let Some(asked) = self.asking.get_mut(owner)
                && owner != at
            {
                *asked = asked.iter().filter(|&s| s != seat).collect();
            }
        }
        let waiting = std::mem::take(&mut self.asking[at]);
        self.showing[at] = self.showing[at].iter().chain(waiting.iter()).collect();
    }

    /// The house sits down at a player's chair, because nobody is answering
    /// on it.
    ///
    /// Returns whether it took: only a human chair can be stood in for. An AI
    /// chair has nobody to wait for, and a driven one is the dev harness's,
    /// which hands itself back when its socket drops.
    ///
    /// *When* to call this is not a question this crate can answer — it needs
    /// a wall clock, and nothing below the transport may read one. The caller
    /// owns the deadline; `HouseRules::reconnect_window_secs` is how long the
    /// table said to wait.
    pub fn stand_in(&mut self, seat: PlayerId) -> bool {
        let teams = self.teams.clone();
        let Some(kind) = self.seats.get_mut(seat.get() as usize) else {
            return false;
        };
        if !matches!(kind, SeatKind::Human) {
            return false;
        }
        // The same agent an AI chair at this table would get, teams included
        // — a stand-in that did not know who its allies were would play the
        // seat's own team as an enemy.
        *kind = SeatKind::StandIn(HeuristicAgent::new(AIProfile::default()).with_teams(teams));
        self.roster_changed();
        self.log.note(LogEvent::StandIn { player: seat });
        self.chair_changed(seat, ChairChange::StoodIn);
        true
    }

    /// The player came back; the chair is theirs again.
    ///
    /// Returns whether the house was holding it. The agent is dropped rather
    /// than kept — unlike [`Session::release`], there is nothing to hand back
    /// *to*: the chair was never an AI chair, and a fresh stand-in is built
    /// if the player leaves again.
    pub fn hand_back(&mut self, seat: PlayerId) -> bool {
        let Some(kind) = self.seats.get_mut(seat.get() as usize) else {
            return false;
        };
        if !matches!(kind, SeatKind::StandIn(_)) {
            return false;
        }
        *kind = SeatKind::Human;
        self.roster_changed();
        self.log.note(LogEvent::Returned { player: seat });
        self.chair_changed(seat, ChairChange::HandedBack);
        true
    }

    /// What a seat says about itself (#265): which teammates it shows its
    /// hand to, or asking or declining to be shown one.
    ///
    /// Returns the frames for every seat whose view it changed, and nothing
    /// at all when it changed nothing, so a client that repeats its setting
    /// costs no view. It moves nothing in the game: no action reaches the
    /// engine, the journal and the snapshot hash stay as they were, and no
    /// decision clock is touched. No question is re-sent either, so a
    /// teammate's share never resets a seat halfway through an answer.
    ///
    /// Teammates may review each other's hands at any time (CR 808.5, CR
    /// 809.7, CR 810.5); this is that permission, used by choice. A request
    /// to a chair the house AI plays is accepted at once. A request to a
    /// chair the house is holding for an absent player waits for the
    /// player: the house does not decide about a person's cards.
    ///
    /// # Errors
    /// When the seat is not answered over a socket or has left the game, or
    /// the game is over; when a seat it names is not a teammate still in the
    /// game, or is a chair the house AI plays and so is shown no hand; and
    /// when it asks again in the turn it was turned down.
    pub fn seat_setting(
        &mut self,
        seat: PlayerId,
        setting: SeatSetting,
    ) -> Result<Vec<(PlayerId, Envelope)>, String> {
        let at = seat.get() as usize;
        if !self
            .seat_kind(seat)
            .is_some_and(SeatKind::answers_over_socket)
        {
            return Err("only a seat answered over a socket says anything about itself".into());
        }
        if !self.seated(seat) {
            return Err("this seat is no longer in the game".into());
        }
        let mates = self.teammates(seat);
        let before: Vec<_> = self
            .human_seats()
            .into_iter()
            .map(|s| self.hands_told(s))
            .collect();
        match setting {
            SeatSetting::ShareHand(shown) => {
                if let Some(other) = shown.iter().find(|&o| !mates.contains(o)) {
                    return Err(format!("seat {other} is not a teammate still in the game"));
                }
                if let Some(house) = shown.iter().find(|&o| !self.may_be_shown(o)) {
                    return Err(format!(
                        "seat {house} is played by the house and is shown no hand"
                    ));
                }
                self.showing[at] = shown;
                // Showing a teammate the hand answers their request.
                self.asking[at] = self.asking[at]
                    .iter()
                    .filter(|&o| !shown.contains(o))
                    .collect();
            }
            SeatSetting::RequestHand(owner) => {
                if !mates.contains(owner) {
                    return Err(format!("seat {owner} is not a teammate still in the game"));
                }
                let key = (owner.get(), seat.get());
                let turn = self.engine.state().turn.number;
                let o = owner.get() as usize;
                if self.showing[o].contains(seat) || self.asking[o].contains(seat) {
                    // Already shown, or already asked: nothing to change.
                } else if self.declined.get(&key) == Some(&turn) {
                    return Err(format!(
                        "seat {owner} turned this request down this turn; ask again next turn"
                    ));
                } else if matches!(self.seats[o], SeatKind::Ai(_)) {
                    self.showing[o].insert(seat);
                } else {
                    self.asking[o].insert(seat);
                }
            }
            SeatSetting::DeclineHand(asker) => {
                if self.asking[at].contains(asker) {
                    self.asking[at] = self.asking[at].iter().filter(|&o| o != asker).collect();
                    self.declined
                        .insert((seat.get(), asker.get()), self.engine.state().turn.number);
                }
            }
        }
        let changed: Vec<PlayerId> = self
            .human_seats()
            .into_iter()
            .zip(before)
            .filter(|(s, was)| self.hands_told(*s) != *was)
            .map(|(s, _)| s)
            .collect();
        if changed.is_empty() {
            return Ok(Vec::new());
        }
        self.seq += 1;
        let mut out = Vec::new();
        for s in changed {
            let envelopes = self.view_envelopes(s);
            out.extend(envelopes.into_iter().map(|env| (s, env)));
        }
        Ok(out)
    }

    /// Whether `seat` is still in a game that is still going.
    fn seated(&self, seat: PlayerId) -> bool {
        (seat.get() as usize) < self.seats.len()
            && !self.engine.state().has_left(seat)
            && !matches!(self.engine.pending(), Pending::GameOver(_))
    }

    /// The other seats on `seat`'s team that are still in the game. None for
    /// a seat on no team: it has no teammates.
    fn teammates(&self, seat: PlayerId) -> SeatSet {
        let Some(team) = self.teams.get(seat.get() as usize).copied().flatten() else {
            return SeatSet::new();
        };
        (0..self.seats.len())
            .map(|i| PlayerId::new(seat_byte(i)))
            .filter(|&other| {
                other != seat
                    && self.teams.get(other.get() as usize).copied().flatten() == Some(team)
                    && self.seated(other)
            })
            .collect()
    }

    /// Whether a chair may be shown a teammate's hand: every chair but one
    /// the house AI plays. A chair the house is holding for an absent player
    /// keeps what the player was shown, and its agent is handed none of it
    /// ([`Session::show_hands`] is only on the way to a socket).
    fn may_be_shown(&self, seat: PlayerId) -> bool {
        !matches!(self.seat_kind(seat), Some(SeatKind::Ai(_)) | None)
    }

    /// Whether `owner`'s hand is shown to `viewer` right now.
    fn shows_hand(&self, owner: PlayerId, viewer: PlayerId) -> bool {
        self.seated(owner)
            && self.teammates(owner).contains(viewer)
            && self.may_be_shown(viewer)
            && self.showing[owner.get() as usize].contains(viewer)
    }

    /// Whose hands `seat` is shown, and the three sets beside them: what a
    /// setting is compared by to decide whose view it changed.
    pub(super) fn hands_told(&self, seat: PlayerId) -> [SeatSet; 4] {
        if !self.seated(seat) {
            return [SeatSet::new(); 4];
        }
        let mates = self.teammates(seat);
        [
            mates.iter().filter(|&o| self.shows_hand(o, seat)).collect(),
            self.showing[seat.get() as usize]
                .iter()
                .filter(|&o| mates.contains(o) && self.may_be_shown(o))
                .collect(),
            self.asking[seat.get() as usize]
                .iter()
                .filter(|&o| mates.contains(o))
                .collect(),
            mates
                .iter()
                .filter(|&o| self.asking[o.get() as usize].contains(seat))
                .collect(),
        ]
    }

    /// Puts into a view a socket is sent what its seat is shown of its
    /// teammates' hands, and where its own showing and asking stand (#265).
    ///
    /// Only on the way to a socket. The views agents answer from are built
    /// elsewhere and never pass through here, so a seat the house plays is
    /// handed exactly what it was handed before hands could be shown.
    pub(super) fn show_hands(&self, view: &mut PlayerView, seat: PlayerId) {
        if !self
            .seat_kind(seat)
            .is_some_and(SeatKind::answers_over_socket)
        {
            return;
        }
        let [shown, sharing, requests, requested] = self.hands_told(seat);
        view.shared_hands = shown
            .iter()
            .map(|owner| SharedHand {
                player: owner,
                cards: crate::view::own_hand(self.engine.state(), owner),
            })
            .collect();
        view.hand_shared_with = sharing;
        view.hand_requests = requests;
        view.hand_requested = requested;
    }

    /// A chair changed hands, so every seat's roster is out of date.
    ///
    /// The roster travels in [`GameStatic`], which is sent once when a socket
    /// attaches — so before this existed, a chair could change hands and the
    /// table was simply never told. Every seat is marked, not just the one
    /// that changed: the roster lists all of them.
    fn roster_changed(&mut self) {
        self.roster_dirty.iter_mut().for_each(|d| *d = true);
    }
}
