use crate::record::Source;
use crate::session::{Session, clock_answer};
use baylee_core::ids::PlayerId;
use baylee_engine::choice::PlayerAction;
use baylee_protocol::v1::Envelope;
use baylee_view::HouseAnswer;

impl<D: baylee_engine::engine::LoopDetection, R: crate::session::SessionRegistry> Session<D, R> {
    /// Applies an answer for a socket seat and records who produced it.
    pub(super) fn answer(
        &mut self,
        player: PlayerId,
        action: PlayerAction,
        by: Option<HouseAnswer>,
        answers: usize,
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
        if self.is_challenge() && !self.challenge_allows_seat_action(&action) {
            return Err("the frozen opponent is still deciding".to_owned());
        }
        let seat = player.get() as usize;
        if !moves_the_game
            && self
                .settings_made
                .get(seat)
                .is_some_and(|made| *made >= self.settings_allowed())
        {
            return Err("too many settings in one game".to_string());
        }
        let deciding = self.deciding();
        let asked = self.answering(player, &action);
        let source = if clock.is_some() {
            Source::Clock
        } else {
            Source::Seat
        };
        let kept = self.record.is_some().then(|| action.clone());
        let setting_was = (!moves_the_game).then(|| self.engine.automation(player).clone());
        let state_was = (
            self.engine.state().journal.last_seq(),
            self.engine.awaited(),
            self.engine
                .state()
                .zones
                .list(baylee_engine::zone::ZoneLocation::Stack)
                .len(),
        );
        self.last_attempt = Some((player, source, action.clone()));
        let applied = self.engine.apply(player, action);
        self.last_attempt = None;
        if applied.is_err() {
            return Err("illegal action for your seat".to_string());
        }
        // A setting that says what already stands is no news to anybody, and
        // a seat may say it hundreds of times a second: answered with nothing,
        // as an unchanged seat setting is, and no line in the record. A hold
        // that ran the game on (or passed priority to the next seat) and
        // expired in the same step is not unchanged.
        let game_moved = (
            self.engine.state().journal.last_seq(),
            self.engine.awaited(),
            self.engine
                .state()
                .zones
                .list(baylee_engine::zone::ZoneLocation::Stack)
                .len(),
        ) != state_was;
        if setting_was.as_ref() == Some(self.engine.automation(player))
            && !game_moved
            && !self.house_owes()
        {
            return Ok(Vec::new());
        }
        if !moves_the_game && let Some(made) = self.settings_made.get_mut(seat) {
            *made += 1;
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
        if moves_the_game || game_moved {
            self.moved(player, deciding);
        }
        if moves_the_game {
            self.house_answered[player.get() as usize] = by;
        }
        // A setting the game did not move for changes one seat's table, so
        // it costs nobody else a view. One that let the game run on is
        // everybody's, and so is a pump the house still owes a step of.
        if !moves_the_game && !game_moved && !self.house_owes() {
            return Ok(self.own_frames(player));
        }
        Ok(self.pump_slice(answers))
    }
}
