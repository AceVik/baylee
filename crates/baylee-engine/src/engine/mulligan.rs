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
    /// What this seat is being asked, a `Mulligan`, `MulliganBottom`, or
    /// an opening-hand action (CR 103.6); `None` once it has kept or left.
    question: Option<Pending>,
    /// The stream this seat's mulligans shuffle with.
    rng: GameRng,
    /// Whether this seat's kept opening hand has been scanned for CR 103.6 actions.
    opening_hand_scanned: bool,
    /// Remaining opening-hand cards with CR 103.6 actions to offer this seat.
    opening_hand_queue: std::collections::VecDeque<baylee_core::ids::ObjectId>,
    /// The opening-hand card currently being asked about.
    opening_hand_current: Option<baylee_core::ids::ObjectId>,
}

impl SeatMulligan {
    /// Everything about this seat's place that decides what happens next,
    /// folded for [`Engine::snapshot_hash`]. Every field is named, so a new
    /// one does not compile until it is folded or left out on purpose.
    pub(crate) fn fingerprint(&self) -> u64 {
        let Self {
            question,
            rng,
            opening_hand_scanned,
            opening_hand_queue,
            opening_hand_current,
        } = self;
        let (asked, taken, free, count) = match question {
            None => (0, 0, false, 0),
            Some(Pending::Mulligan {
                taken,
                next_is_free,
                ..
            }) => (1, *taken, *next_is_free, 0),
            Some(Pending::MulliganBottom { count, .. }) => (2, 0, false, *count),
            Some(Pending::YesNo { .. }) => (3, 0, false, 0),
            Some(Pending::ChooseCards { min, max, .. }) => (4, *min, false, *max),
            Some(other) => unreachable!("a mulligan window asked {other:?}"),
        };
        let word_pos = rng.word_pos();
        let queue_hash = crate::state::structural_fingerprint(&(
            opening_hand_scanned,
            opening_hand_queue,
            opening_hand_current,
        ));
        [
            asked,
            u64::from(taken),
            u64::from(free),
            u64::from(count),
            word_pos as u64,
            (word_pos >> 64) as u64,
            rng.calls(),
            queue_hash,
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
                // A hand of seven bottoms nothing yet (CR 103.5).
                can_take: true,
            }),
            rng: table.for_seat(seat as u8),
            opening_hand_scanned: false,
            opening_hand_queue: std::collections::VecDeque::new(),
            opening_hand_current: None,
        })
        .collect()
}

impl<L: CardLookup> Engine<L> {
    /// Whether a seat that has taken `taken` mulligans may take another:
    /// until its hand would open with no cards (CR 103.5), past which the
    /// bottom question asks for more cards than the hand holds and nothing
    /// can answer it. What `Pending::Mulligan::can_take` states.
    pub(crate) fn may_mulligan(&self, taken: u8) -> bool {
        self.mulligan_bottom_count(taken) < HAND
    }

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
            None => (self.decision_actor() == Some(seat)).then_some(&self.pending),
        }
    }

    /// Private information from a choice remains visible to its resource
    /// player and to the actor controlling that player. This does not confer
    /// authority to answer; hosts use `pending_for` for answer packets.
    #[must_use]
    pub fn information_pending_for(&self, seat: PlayerId) -> Option<&Pending> {
        if self.mulligans.is_some() {
            return self.pending_for(seat);
        }
        (self.pending.asked() == Some(seat) || self.decision_actor() == Some(seat))
            .then_some(&self.pending)
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
            None => self.decision_actor().into_iter().collect(),
        }
    }

    /// An answer while the mulligans are open: a seat's own mulligan
    /// question, or a concession (CR 104.3a). Nothing else is legal before
    /// turn 1.
    #[allow(clippy::too_many_lines)]
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
        // The seat's own question states what it takes, as every question
        // does (`Pending::answer_fault`); a concession answers none of them
        // and is always legal.
        if !matches!(action, PlayerAction::Concede)
            && let Some(fault) = question.as_ref().and_then(|q| q.answer_fault(&action))
        {
            return Err(fault.into());
        }
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
                // (CR 103.5): the question says so (`can_take`), and the
                // check above refused a mulligan past it.
                debug_assert!(self.may_mulligan(taken), "can_take was checked");
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
                    next_is_free: taken + 1 < self.house_rules.free_mulligan_count(),
                    can_take: self.may_mulligan(taken + 1),
                })
            }
            (Some(Pending::MulliganBottom { .. }), PlayerAction::ChooseObjects { objects }) => {
                // The count, and no card named twice, were the question's
                // (`answer_fault`): `[c, c]` had the right length and both
                // halves in hand, and bottomed one card for two and kept an
                // eighth (CR 103.5). That they are in the hand is the
                // engine's, the question offering the whole hand.
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
            (Some(Pending::YesNo { .. }), PlayerAction::YesNo(yes)) => {
                let caverns = self
                    .mulligans
                    .as_mut()
                    .and_then(|w| w.get_mut(seat))
                    .and_then(|s| s.opening_hand_current.take());
                if yes
                    && let Some(caverns) = caverns
                    && self.in_hand(player, caverns)
                {
                    if let Some(obj) = self.state.object_mut(caverns) {
                        obj.kind = crate::object::ObjectKind::Permanent;
                        obj.set_controller(player);
                    }
                    self.state.move_object(
                        caverns,
                        ZoneLocation::Battlefield,
                        ZonePosition::Top,
                        Cause::Effect,
                    )?;
                    if self
                        .state
                        .object(caverns)
                        .is_some_and(|o| o.zone == crate::zone::Zone::Battlefield)
                    {
                        crate::replacement::put_counters_by(
                            &mut self.state,
                            player,
                            caverns,
                            baylee_cards_dsl::counters::LUCK,
                            1,
                        );
                    }
                    let options = self.state.zones.list(ZoneLocation::Hand(player)).clone();
                    if options.is_empty() {
                        None
                    } else {
                        Some(Pending::ChooseCards {
                            player,
                            options,
                            min: 1,
                            max: 1,
                            prompt: crate::choice::ChoicePrompt::Generic,
                            total: None,
                        })
                    }
                } else {
                    None
                }
            }
            (Some(Pending::ChooseCards { .. }), PlayerAction::ChooseObjects { objects }) => {
                if !objects.iter().all(|card| self.in_hand(player, *card)) {
                    return Err(EngineError::IllegalAction("card not in hand"));
                }
                for card in objects {
                    self.state.move_object(
                        card,
                        ZoneLocation::Exile(player),
                        ZonePosition::Top,
                        Cause::Effect,
                    )?;
                    if let Some(window) = self.mulligans.as_mut() {
                        window[seat].opening_hand_queue.retain(|&c| c != card);
                    }
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
        // A mulligan draws a new hand and a concession takes a player's
        // objects out of the game (CR 800.4a), while the loop that refreshes
        // the projection is not running yet: whoever is asked next, or told
        // the game is over, is shown the board as it now is. That includes
        // the effects of the statics that left with a conceder's permanents,
        // which only the sync drops.
        self.sync_static_effects();
        self.state.refresh_characteristics();
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
        // CR 103.6a: after all mulligans are complete, players may take
        // opening-hand actions in turn order starting with the starting player.
        let starting = self.state.turn.active;
        let seats = self.state.players.len();
        for offset in 0..seats {
            let seat_idx = (usize::from(starting.get()) + offset) % seats;
            let p = PlayerId::new(seat_idx as u8);
            if self.state.has_left(p) {
                continue;
            }
            let scanned = self
                .mulligans
                .as_ref()
                .is_some_and(|w| w[seat_idx].opening_hand_scanned);
            if !scanned {
                let mut candidates = std::collections::VecDeque::new();
                if p != starting {
                    for &card in self.state.zones.list(ZoneLocation::Hand(p)) {
                        let has_caverns = self.state.object(card).is_some_and(|o| {
                            o.abilities(&self.lookup).iter().any(|a| {
                                matches!(
                                    a,
                                    baylee_cards_dsl::AbilityDef::Replacement(
                                        baylee_cards_dsl::ReplacementRule::OpeningHandBattlefieldWithLuckCounterExilingHandCard,
                                    )
                                )
                            })
                        });
                        if has_caverns {
                            candidates.push_back(card);
                        }
                    }
                }
                if let Some(w) = self.mulligans.as_mut() {
                    w[seat_idx].opening_hand_scanned = true;
                    w[seat_idx].opening_hand_queue = candidates;
                }
            }
            while let Some(caverns) = self
                .mulligans
                .as_mut()
                .and_then(|w| w[seat_idx].opening_hand_queue.pop_front())
            {
                if !self.in_hand(p, caverns) {
                    continue;
                }
                let source = self
                    .state
                    .object(caverns)
                    .and_then(|o| o.card)
                    .map(|c| baylee_core::ids::AbilityRef::new(c.index, 0));
                let q = Pending::YesNo {
                    player: p,
                    prompt: crate::choice::YesNoPrompt::Generic,
                    source,
                };
                if let Some(w) = self.mulligans.as_mut() {
                    w[seat_idx].opening_hand_current = Some(caverns);
                    w[seat_idx].question = Some(q.clone());
                }
                self.pending = q;
                self.awaiting_answer = true;
                return;
            }
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
