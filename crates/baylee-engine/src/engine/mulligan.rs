//! The opening mulligans, which every seat answers at once.
//!
//! House rule 4 (`house_rules_tests`). CR 103.5 has the starting player
//! declare first and each other player in turn order after them, and only
//! then do all who chose to mulligan take them at the same time, over and
//! over until nobody does. That makes every seat wait for the ones before
//! it, at every round. Here nobody waits for anybody (#257): every seat is
//! asked its own question from the start, answers it when it likes, and is
//! shown its new hand at once. Turn 1 begins once every seat has kept or
//! left; a seat that has left does not begin it (CR 800.4k).
//!
//! Each seat's shuffles run on its own stream ([`GameRng::for_seat`]), so
//! the hands a seat is dealt depend only on its own answers, never on
//! whether it answered before or after the seat beside it.
//!
//! While the window is open [`Engine::pending`] shows the lowest seat still
//! deciding, so a caller that only knows `pending()` still gets through it
//! one seat at a time; [`Engine::pending_for`] and [`Engine::awaited`] are
//! what a host that asks everyone at once reads. The machine does not run
//! until the window closes: nothing happens in a game before turn 1.

use super::{Engine, EngineError};
use crate::choice::{Pending, PlayerAction};
use crate::event::{Cause, LossReason};
use crate::rng::GameRng;
use crate::sba;
use crate::state::CardLookup;
use crate::zone::{ZoneLocation, ZonePosition};
use baylee_core::ids::{PlayerId, SeatSet};

/// The number of cards a hand is drawn with (CR 103.5, "normally seven").
const HAND: u8 = 7;

/// One seat's place in the opening mulligans.
#[derive(Clone, Debug)]
pub(crate) struct SeatMulligan {
    /// What this seat is being asked, a `Mulligan` or a `MulliganBottom`;
    /// `None` once it has kept or left.
    question: Option<Pending>,
    /// The stream this seat's mulligans shuffle with.
    rng: GameRng,
}

impl SeatMulligan {
    /// Everything about this seat's place that decides what happens next,
    /// folded for [`Engine::snapshot_hash`]. Every field is named, so a new
    /// one does not compile until it is folded or left out on purpose.
    pub(crate) fn fingerprint(&self) -> u64 {
        let Self { question, rng } = self;
        let (asked, taken, free, count) = match question {
            None => (0, 0, false, 0),
            Some(Pending::Mulligan {
                taken,
                next_is_free,
                ..
            }) => (1, *taken, *next_is_free, 0),
            Some(Pending::MulliganBottom { count, .. }) => (2, 0, false, *count),
            Some(other) => unreachable!("a mulligan window asked {other:?}"),
        };
        let word_pos = rng.word_pos();
        [
            asked,
            u64::from(taken),
            u64::from(free),
            u64::from(count),
            word_pos as u64,
            (word_pos >> 64) as u64,
            rng.calls(),
        ]
        .into_iter()
        .fold(0, |acc: u64, v| acc.wrapping_mul(31).wrapping_add(v))
    }
}

/// The window as a game opens: every seat asked whether it keeps.
pub(crate) fn open(table: &GameRng, seats: usize, free_first: bool) -> Vec<SeatMulligan> {
    (0..seats)
        .map(|seat| SeatMulligan {
            question: Some(Pending::Mulligan {
                player: PlayerId::new(seat as u8),
                taken: 0,
                next_is_free: free_first,
            }),
            rng: table.for_seat(seat as u8),
        })
        .collect()
}

impl<L: CardLookup> Engine<L> {
    /// The question `seat` is being asked, if it is being asked one.
    ///
    /// During the opening mulligans that is each seat's own; after them, it
    /// is [`Engine::pending`] for the one seat that is asked, and `None` for
    /// every other seat and once the game is over. A query, not an action:
    /// the game still moves only through [`Engine::apply`].
    #[must_use]
    pub fn pending_for(&self, seat: PlayerId) -> Option<&Pending> {
        match &self.mulligans {
            Some(window) => window.get(seat.get() as usize)?.question.as_ref(),
            None => (self.pending.asked() == Some(seat)).then_some(&self.pending),
        }
    }

    /// Every seat being asked a question right now: all that have not yet
    /// kept during the opening mulligans, the one [`Engine::pending`] names
    /// after them, and nobody once the game is over.
    #[must_use]
    pub fn awaited(&self) -> SeatSet {
        match &self.mulligans {
            Some(window) => window
                .iter()
                .filter_map(|seat| seat.question.as_ref()?.asked())
                .collect(),
            None => self.pending.asked().into_iter().collect(),
        }
    }

    /// An answer while the mulligans are open: a seat's own mulligan
    /// question, or a concession (CR 104.3a). Nothing else is legal before
    /// turn 1.
    pub(super) fn apply_in_mulligans(
        &mut self,
        player: PlayerId,
        action: PlayerAction,
    ) -> Result<(), EngineError> {
        let seat = player.get() as usize;
        let question = self
            .mulligans
            .as_ref()
            .and_then(|window| window.get(seat))
            .and_then(|s| s.question.clone());
        let next = match (question, action) {
            (_, PlayerAction::Concede) => {
                sba::eliminate_player(&mut self.state, player, LossReason::Conceded);
                None
            }
            (Some(Pending::Mulligan { taken, .. }), PlayerAction::MulliganKeep) => {
                let count = self.mulligan_bottom_count(taken);
                (count > 0).then_some(Pending::MulliganBottom { player, count })
            }
            (Some(Pending::Mulligan { taken, .. }), PlayerAction::MulliganTake) => {
                // Until the opening hand would be zero cards, and no further
                // (CR 103.5): past that the bottom question asks for more
                // cards than the hand holds, and nothing can answer it.
                if self.mulligan_bottom_count(taken) >= HAND {
                    return Err(EngineError::IllegalAction(
                        "a hand that would open with zero cards takes no further mulligan",
                    ));
                }
                // Hand goes back, reshuffle, draw 7 (CR 103.5).
                let hand = self.state.zones.list(ZoneLocation::Hand(player)).clone();
                for card in hand {
                    self.state.move_object(
                        card,
                        ZoneLocation::Library(player),
                        ZonePosition::Bottom,
                        Cause::Effect,
                    )?;
                }
                let window = self.mulligans.as_mut().expect("the window is open");
                self.state
                    .shuffle_library_with(player, &mut window[seat].rng);
                self.state.draw_cards(player, usize::from(HAND));
                Some(Pending::Mulligan {
                    player,
                    taken: taken + 1,
                    next_is_free: false,
                })
            }
            (
                Some(Pending::MulliganBottom { count, .. }),
                PlayerAction::ChooseObjects { objects },
            ) => {
                // `[c, c]` has the right length and both halves are in hand;
                // it bottomed one card for two and kept an eighth (CR 103.5).
                if objects.len() != usize::from(count) || super::actions::names_one_twice(&objects)
                {
                    return Err(EngineError::IllegalAction(
                        "must bottom exactly the required number of cards",
                    ));
                }
                if !objects.iter().all(|card| self.in_hand(player, *card)) {
                    return Err(EngineError::IllegalAction("card not in hand"));
                }
                for card in objects {
                    self.state.move_object(
                        card,
                        ZoneLocation::Library(player),
                        ZonePosition::Bottom,
                        Cause::Effect,
                    )?;
                }
                None
            }
            (Some(_), _) => return Err(EngineError::MismatchedAction),
            (None, _) => {
                return Err(EngineError::IllegalAction(
                    "this seat has no question before turn 1",
                ));
            }
        };
        if let Some(window) = self.mulligans.as_mut() {
            window[seat].question = next;
        }
        self.settle_mulligans();
        Ok(())
    }

    /// Publishes the window's state after an answer: the lowest seat still
    /// deciding as [`Engine::pending`], or, once nobody is, turn 1.
    ///
    /// A concession can leave a side alone at the table, and then the game
    /// is over before it began.
    fn settle_mulligans(&mut self) {
        if let Some(result) = self.game_result() {
            self.mulligans = None;
            self.end_game(result);
            return;
        }
        let Some(window) = &self.mulligans else {
            return;
        };
        if let Some(question) = window.iter().find_map(|seat| seat.question.clone()) {
            self.pending = question;
            self.awaiting_answer = true;
            return;
        }
        self.mulligans = None;
        // CR 800.4k: a player who has left does not begin a turn, and the
        // next one in turn order does.
        let first = self.state.turn.active;
        if self.state.players[first.get() as usize].has_lost() {
            self.state.turn.active = self.next_alive_after(first);
        }
        self.begin_turn(true);
        self.awaiting_answer = false;
        self.run_until_choice();
    }
}
