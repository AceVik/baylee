use super::{Session, by_clock, choice_envelope, clock_answer, state_frames};
use crate::record::Source;
use baylee_core::ids::PlayerId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_protocol::v1::Envelope;
use baylee_view::{HouseAnswer, LogTail, PlayerView};

impl Session {
    /// The view the house answers `player`'s question from, whether it plays
    /// the chair or has only been handed one decision by a clock.
    ///
    /// One builder for both, because what an agent may know is one rule.
    /// Two things a socket's view carries never reach it:
    ///
    /// - **No clock.** `decision_remaining_ms` is made of elapsed wall time,
    ///   and machine speed is not an authorized input to a decision: an
    ///   agent that read it would play the same position differently on a
    ///   slow machine, legally and invisibly (#87). A clock's answer is
    ///   where it would bite first, since that view exists because the
    ///   number ran out.
    /// - **No teammate's hand** (#265). Only [`Session::show_hands`] puts
    ///   one into a view, on the way to a socket, and this never calls it,
    ///   so a chair the house holds for an absent player is played from
    ///   what it was played from before hands could be shown.
    pub(crate) fn agent_view(&self, player: PlayerId, pending: &Pending) -> PlayerView {
        crate::view::player_view(
            self.engine.state(),
            player,
            self.seq,
            Some(pending),
            &crate::view::SeatContext {
                awaiting: crate::view::awaiting_for(&self.engine, player),
                decision_player: pending.asked(),
                controlled_players: self.engine.controlled_players(player),
                deciding: self.deciding(),
                held: self.engine.automation(player).hold.suppresses(),
                owed: crate::view::owed_payment(&self.engine),
                library_reveal_blocked: self.engine.library_reveal_blocked(),
                decision_remaining_ms: None,
                // What a policy answered is told to the seat's player; it is
                // not an input to a decision.
                policy_acts: &[],
            },
            &self.house_answered,
        )
    }

    /// What an agent that is not the house answers `seat`'s current question
    /// from — the question and [`Session::agent_view`]'s view of it — or
    /// `None` when `seat` is asked nothing.
    ///
    /// For an offline player of a chair taken over with
    /// [`Session::take_over`] (the trained AI's self-play in `baylee-train`),
    /// which then answers with [`Session::act`]. It is the house's answering
    /// view, not what a socket is sent: no clock, no policy acts, and never a
    /// teammate's shared hand, because an AI chair is shown nothing (#265).
    #[must_use]
    pub fn view_for(&self, seat: PlayerId) -> Option<(Pending, PlayerView)> {
        let pending = self.engine.pending_for(seat)?.clone();
        let view = self.agent_view(seat, &pending);
        Some((pending, view))
    }

    /// The sequence number a client should report back when it resumes.
    #[must_use]
    pub const fn seq(&self) -> u64 {
        self.seq
    }

    /// How many times the question has changed — the anchor for a decision
    /// clock, and deliberately *not* [`Session::seq`].
    ///
    /// `seq` counts frames, and a frame is produced by anything a seat says,
    /// including the two things a seat may say while it is not the one being
    /// asked: a priority hold and a standing answer. A clock anchored to `seq`
    /// therefore restarts when the *opponent* presses `F6`, or reconnects
    /// (an attach replays every remembered answer), which hands out unlimited
    /// thinking time to whoever spams either. This counts only actions that
    /// moved the game, so an automation setting leaves the clock exactly where
    /// it was.
    #[must_use]
    pub const fn decision_seq(&self) -> u64 {
        self.decisions
    }

    /// What a seat is shown of a table that has not opened yet (#256): its
    /// own view, after the payload when the view reveals a printing the seat
    /// did not hold or the roster moved, and never a question.
    ///
    /// Between [`Session::pump`] and [`Session::snapshot`], and neither will
    /// do. `pump` drives every AI seat until a human is needed, which is a
    /// decision taken before the table is open. `snapshot` does not reveal,
    /// because it rebuilds a state a pump already showed; a seat arriving at
    /// a table nobody has pumped has been shown nothing, and an opponent's
    /// commander would have no printing to draw. And no question, because
    /// nothing may be answered before the curtain is up: a seat that is never
    /// asked cannot answer.
    pub fn show(&mut self, seat: PlayerId) -> Vec<Envelope> {
        self.view_envelopes(seat)
    }

    /// Everything a seat needs to render the game from scratch: its own
    /// view, every log line it has been sent, plus the outstanding choice
    /// when this seat is the one being asked (or the game is over, which
    /// everyone is told about).
    ///
    /// Read-only on purpose. `pump` *advances* the game — it drives every AI
    /// seat until a human is needed — so rebuilding a client through it
    /// would let a reconnect take a turn on the AI's behalf.
    ///
    /// The log is here because a rebuild is asked for when frames were lost
    /// (a lagging socket, a `ResumeGame`), and the lines in them are gone
    /// with the views. A client appends by `LogTail::from`, so the lines it
    /// already holds cost bytes and nothing else.
    #[must_use]
    pub fn snapshot(&self, seat: PlayerId) -> Vec<Envelope> {
        self.rebuild(seat, self.log_sent(seat))
    }

    /// The seat's view and the question it owes, and no log: what a refused
    /// answer is handed back.
    ///
    /// A refusal loses no frame, so the seat holds every line it was sent,
    /// and a snapshot would send it the whole log again for each one — a
    /// few bytes of action answered with every line of the game.
    #[must_use]
    pub fn reask(&self, seat: PlayerId) -> Vec<Envelope> {
        self.rebuild(seat, LogTail::default())
    }

    fn rebuild(&self, seat: PlayerId, log: LogTail) -> Vec<Envelope> {
        let own = self.engine.pending_for(seat);
        let awaiting = crate::view::awaiting_for(&self.engine, seat);
        // Read-only, so no printing is revealed here: this rebuilds a state a
        // `pump` already showed this seat, and the reveal happened there.
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
        let mut out = state_frames(self.seq, &view, log);
        let over = Some(self.engine.pending()).filter(|p| matches!(p, Pending::GameOver(_)));
        if let Some(pending) = own.or(over) {
            out.push(choice_envelope(self.seq, pending));
        }
        out
    }

    /// Answers `ResumeGame{last_seq}`: the snapshot when the client is
    /// behind, nothing when it is already current.
    ///
    /// Returning nothing matters — a client that reconnects without having
    /// missed anything should not be made to re-render the whole table.
    #[must_use]
    pub fn resume(&self, seat: PlayerId, last_seq: u64) -> Vec<Envelope> {
        if last_seq >= self.seq {
            return Vec::new();
        }
        self.snapshot(seat)
    }

    /// Applies a human action, then pumps the AI until a human is needed.
    ///
    /// # Errors
    /// When the action isn't the acting seat's legal answer.
    pub fn act(
        &mut self,
        player: PlayerId,
        action: PlayerAction,
    ) -> Result<Vec<(PlayerId, Envelope)>, String> {
        self.answer(player, action, None)
    }

    /// The decision clock ran out on `seat`: the house answers this one
    /// question for it, with the answer that does nothing where there is one
    /// ([`Self::timeout_action`], [`by_clock`]), and the seat is marked as
    /// answered by the clock until it answers one itself.
    ///
    /// `None` when `seat` is not the one being asked, which is how a timer
    /// that fired after the question moved on is told apart from one that
    /// is still due: one seat's expired clock must never take another seat's
    /// decision. `Some(Err)` when the engine refused the house's answer,
    /// which a caller treats like any refused answer.
    ///
    /// Its own door rather than [`Session::timeout_action`] followed by
    /// [`Session::act`], because through `act` the session cannot tell the
    /// clock's answer from the player's — and the views `act` builds on the
    /// way out are the ones that have to say which it was.
    pub fn answer_by_clock(
        &mut self,
        seat: PlayerId,
        asked_at: u64,
    ) -> Option<Result<Vec<(PlayerId, Envelope)>, String>> {
        if self.asked_at(seat)? != asked_at {
            return None;
        }
        let pending = self.engine.pending_for(seat)?.clone();
        by_clock(
            self,
            &pending,
            |session, action| session.answer(seat, action, Some(HouseAnswer::Clock)),
            |session| session.house_action(seat),
        )
    }

    /// Applies an answer for a socket seat and records who produced it.
    fn answer(
        &mut self,
        player: PlayerId,
        action: PlayerAction,
        by: Option<HouseAnswer>,
    ) -> Result<Vec<(PlayerId, Envelope)>, String> {
        let Some(kind) = self.seats.get(player.get() as usize) else {
            return Err("not a human seat".to_string());
        };
        if !kind.answers_over_socket() {
            return Err("not a human seat".to_string());
        }
        let clock = matches!(by, Some(HouseAnswer::Clock))
            .then(|| clock_answer(self.engine.pending_for(player), &action));
        let by_hand = by.is_none();
        let by = by.filter(|_| !kind.is_ai_chair());
        // Read before the action is spent: an automation setting is the one
        // thing the engine takes from a seat that is not being asked, and it
        // leaves the question standing. See [`Session::decision_seq`].
        let moves_the_game = !action.is_automation_setting();
        let deciding = self.deciding();
        let asked = self.answering(player, &action);
        let source = if clock.is_some() {
            Source::Clock
        } else {
            Source::Seat
        };
        let kept = self.record.is_some().then(|| action.clone());
        if self.engine.apply(player, action).is_err() {
            return Err("illegal action for your seat".to_string());
        }
        if let Some(action) = kept {
            self.recorded(player, source, action);
        }
        // Before the journal is read: the same apply may have run on into
        // this seat's policy answering again, and that one is news.
        if moves_the_game
            && by_hand
            && let Some(window) = self.policy_acts.get_mut(player.get() as usize)
        {
            window.clear();
        }
        self.log_answer(player, &asked, deciding, clock);
        self.seq += 1;
        if moves_the_game {
            self.moved(player, deciding);
            self.house_answered[player.get() as usize] = by;
        }
        Ok(self.pump())
    }
}
