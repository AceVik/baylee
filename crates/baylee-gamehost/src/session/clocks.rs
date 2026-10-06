use super::{SeatKind, Session};
use baylee_ai::HeuristicAgent;
use baylee_core::ids::{PlayerId, SeatSet};
use baylee_core::preset::AIProfile;
use baylee_engine::choice::PlayerAction;

impl Session {
    /// How long a seat may sit on a decision, per the table's house rules
    /// (`0` = no limit).
    ///
    /// The clock itself belongs to the caller. Nothing below this line may
    /// read a wall clock: the rules kernel is deterministic, and a session
    /// that timed itself would replay differently on every machine.
    #[must_use]
    pub const fn decision_timeout_secs(&self) -> u32 {
        self.house_rules.decision_timeout_secs
    }

    /// How long a seat may be gone before the house takes its chair, per the
    /// table's house rules (`0` = wait forever).
    ///
    /// A second clock rather than the same one, because it answers a
    /// different question. A seat with no socket is on no decision clock at
    /// all — nobody should lose on time to a question they never saw — and
    /// that is exactly the state this one measures. It is also independent of
    /// `decision_timeout_secs`: a table that gives its players all the time in
    /// the world still must not sit forever waiting on a closed laptop.
    #[must_use]
    pub const fn reconnect_window_secs(&self) -> u32 {
        self.house_rules.reconnect_window_secs
    }

    /// Tell this session how much of `seat`'s decision clock is left, as read
    /// by whoever owns the clock.
    ///
    /// `seq` is the [`Session::asked_at`] the reading was taken against, and
    /// passing it is not ceremony: the caller reads its clock *before*
    /// handing a frame in, and handling that frame may move the game, so by
    /// the time a view is built the number can already belong to a question
    /// nobody is being asked any more.
    ///
    /// `remaining_ms` of `None` states that **no decision clock is running** —
    /// an untimed table, an AI chair, or a seat waiting out its reconnect
    /// window rather than deciding. That is a different statement from never
    /// having called this at all, and [`Session::decision_remaining_ms`]
    /// treats them differently.
    pub fn set_decision_remaining(&mut self, seat: PlayerId, seq: u64, remaining_ms: Option<u32>) {
        if let Some(slot) = self.clock.get_mut(seat.get() as usize) {
            *slot = Some((seq, remaining_ms));
        }
    }

    /// How long `seat` has left to answer, in milliseconds, or `None` when it
    /// is not being asked or no decision clock is running for it.
    ///
    /// Almost all of this is the anchor check. A reading is only true of the
    /// question it was taken for, so once the seat has been asked a new one
    /// ([`Session::asked_at`]) the reading is discarded and the seat is given the table's
    /// whole allowance instead — which is exactly right, because a question
    /// that has only just been asked has had no time taken off it. Without
    /// that branch the first view of every new question would carry the
    /// *previous* question's leftovers, and a seat would be shown four
    /// seconds to answer something it was asked a moment ago.
    ///
    /// The allowance is refused for a chair the clock does not run for. An AI
    /// chair is on no clock, and a chair the house is holding is waiting on a
    /// player rather than deciding, so neither inherits an allowance merely
    /// because the seat asked before them had one.
    #[must_use]
    pub fn decision_remaining_ms(&self, seat: PlayerId) -> Option<u32> {
        let asked_at = self.asked_at(seat)?;
        if let Some(Some((seq, reading))) = self.clock.get(seat.get() as usize)
            && *seq == asked_at
        {
            return *reading;
        }
        if !self
            .seat_kind(seat)
            .is_some_and(SeatKind::answers_over_socket)
        {
            return None;
        }
        match self.house_rules.decision_timeout_secs {
            0 => None,
            secs => Some(secs.saturating_mul(1_000)),
        }
    }

    /// Every seat that owes an answer: all that are still deciding their
    /// opening mulligans, and after them the one the game is waiting on.
    #[must_use]
    pub fn awaited(&self) -> SeatSet {
        self.engine.awaited()
    }

    /// The seat the table is waiting on, as every view names it: the one
    /// [`Engine::pending`](baylee_engine::engine::Engine::pending) is
    /// addressed to. During the opening mulligans that is the lowest seat
    /// still deciding, and [`Session::awaited`] is every one of them.
    #[must_use]
    pub fn awaiting_seat(&self) -> Option<PlayerId> {
        self.engine.pending().asked()
    }

    /// The [`Session::decision_seq`] at which `seat` was asked the question
    /// it owes now, or `None` when it owes none: what a decision clock for
    /// `seat` is anchored to.
    ///
    /// Not `decision_seq` itself, because during the opening mulligans
    /// several seats are asked at once and each answers in its own time. One
    /// seat's keep moves the game without asking any other seat anything
    /// new, and a clock anchored to the shared count would restart every
    /// other seat's deadline at every keep: free time for whoever waits.
    /// Everywhere else one seat is asked, and its anchor moves exactly when
    /// the shared count does ([`Session::moved`]).
    #[must_use]
    pub fn asked_at(&self, seat: PlayerId) -> Option<u64> {
        if !self.engine.awaited().contains(seat) {
            return None;
        }
        self.asked_at.get(seat.get() as usize).copied()
    }

    /// The seats owing an opening mulligan question: the same set every
    /// view carries as `PlayerView::deciding`.
    pub(super) fn deciding(&self) -> SeatSet {
        crate::view::deciding(&self.engine)
    }

    /// The game moved, by `answered`'s answer or concession: counts the
    /// question and re-anchors every seat that is now asked something new.
    ///
    /// `deciding` is [`Session::deciding`] read before the move. A seat
    /// keeps its anchor only if it owed an opening mulligan question before,
    /// still owes one and did not answer: that seat is asked exactly what it
    /// was asked, and its clock runs on. Every other seat still being asked
    /// is asked anew, and outside the mulligans that is every move, as it
    /// was when the one count was the anchor: a bystander's concession or
    /// draw offer changes the awaited seat's question without that seat
    /// saying anything.
    pub(super) fn moved(&mut self, answered: PlayerId, deciding: SeatSet) {
        self.decisions += 1;
        let still = self.deciding();
        for seat in self.engine.awaited().iter() {
            let holds = seat != answered && deciding.contains(seat) && still.contains(seat);
            if !holds && let Some(at) = self.asked_at.get_mut(seat.get() as usize) {
                *at = self.decisions;
            }
        }
    }

    /// The action to apply when a seat's decision clock runs out: the
    /// question's answer that does nothing
    /// ([`baylee_engine::choice::timeout_answer`]), else the house's
    /// ([`Self::house_action`]) (#258).
    ///
    /// A seat that ran out of time passes, attacks and blocks with nothing,
    /// keeps its hand and declines what declining leaves alone. It used to
    /// be played by the house in full, which cast its spells and sent its
    /// creatures in on its behalf. The client writes the countdown into the
    /// button this answer is, and it can only do that for an answer it can
    /// know. A question with no answer that does nothing (a discard, a
    /// target) is still the house's.
    #[must_use]
    pub fn timeout_action(&self, seat: PlayerId) -> Option<PlayerAction> {
        let pending = self.engine.pending_for(seat)?;
        baylee_engine::choice::timeout_answer(pending).or_else(|| self.house_action(seat))
    }

    /// What the house answers for `seat`, if it is being asked anything.
    ///
    /// The house agent answers rather than a hand-written table of defaults.
    /// It already produces a *legal* answer for every `Pending`, and a
    /// timeout that produced an illegal one would stall the very game it
    /// exists to unstick — the seat would be asked again, time out again,
    /// and the table would never move.
    #[must_use]
    pub fn house_action(&self, player: PlayerId) -> Option<PlayerAction> {
        let pending = self.engine.pending_for(player)?;
        // Teams included, as `stand_in` builds it: without them every other
        // chair reads as an enemy, the seat's own partner among them.
        let agent = HeuristicAgent::new(AIProfile::default()).with_teams(self.teams.clone());
        Some(agent.act(&self.agent_view(player, pending), pending))
    }
}
