//! A session's game forked for one seat's lookahead, for training and
//! evaluation harnesses that host their games in-process.
//!
//! [`Session::fork_for`] wraps the engine's own fork (`Engine::fork_for`,
//! `docs/engine-internals.md` §"Forking a simulation") in a
//! [`SimulationSession`]: the same chairs and the same house, answering
//! through the same views ([`super::table`]), and nothing a hosted table
//! keeps for the people at it — no record, no log, no envelopes, no clocks,
//! no spectators. It is never a seat's path over the gateway, which has no
//! engine to fork.

use std::sync::Arc;

use baylee_ai::intelligence::DeckIntel;
use baylee_core::ids::{PlayerId, SeatSet};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::{DeckKnowledge, Engine, ForkRefusal};
use baylee_view::{HouseAnswer, PlayerView};

use super::table::Table;
use super::{RegistryLookup, SeatKind, Session};

/// A disposable copy of a [`Session`]'s game for one seat's lookahead, made
/// by [`Session::fork_for`].
///
/// It is played through the calls an in-process harness plays a session
/// with ([`Self::view_for`] and its context variants, [`Self::observe_view`]
/// and its variants, [`Self::act_slice`], [`Self::pump_at_most`],
/// [`Self::house_action`]), and its house answers exactly as the table's
/// does. Advancing it never touches the session it was forked from.
pub struct SimulationSession {
    engine: Engine<RegistryLookup>,
    seats: Vec<SeatKind>,
    teams: Vec<Option<u8>>,
    scouting_decks: Arc<[DeckIntel]>,
    fair_scouting: bool,
    seq: u64,
    decisions: u64,
    house_answered: Vec<Option<HouseAnswer>>,
    seat: PlayerId,
}

/// The `SplitMix64` stream constant, the step between seeds of one purpose.
const GOLDEN: u64 = 0x9E37_79B9_7F4A_7C15;
/// Purpose tag of a fork's house seeds.
const HOUSE: u64 = u64::from_be_bytes(*b"house\0\0\0");

/// The seed of the house agent in chair `seat` of a fork made from
/// `policy_seed`.
///
/// A function of the policy seed and the chair alone, never of the real
/// table's game id or deal: the house in a fork plays its near-equal
/// choices on the searcher's stream, as the fork's shuffles do.
fn house_seed(policy_seed: u64, seat: usize) -> u64 {
    let mut z = policy_seed ^ HOUSE ^ (seat as u64 + 1).wrapping_mul(GOLDEN);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

impl Session {
    /// A disposable copy of this game for `seat`'s lookahead
    /// ([`SimulationSession`]), for training and evaluation harnesses that
    /// own the real session; a seat playing over the gateway has none.
    ///
    /// The engine is [`Engine::fork_for`]'s: every card `seat` cannot see is
    /// dealt again from `policy_seed`, and the fork rolls on its own random
    /// stream. Every chair keeps its kind — a driven chair stays driven, so
    /// the harness answers it — and every house agent is the chair's own
    /// with its seed replaced by one derived from `policy_seed`. The agents'
    /// count of refitted answers (`HeuristicAgent::fallbacks`) stays shared
    /// with the table's, as every copy of an agent shares it.
    ///
    /// Takes `&self`: forking, and anything done to the fork, leaves this
    /// session — its engine, hash, record and chairs — as it was.
    ///
    /// # Errors
    /// The engine's [`ForkRefusal`] for a position it cannot fork faithfully.
    pub fn fork_for(
        &self,
        seat: PlayerId,
        decks: DeckKnowledge,
        policy_seed: u64,
    ) -> Result<SimulationSession, ForkRefusal> {
        let engine = self.engine.fork_for(seat, decks, policy_seed)?;
        let seats = self
            .seats
            .iter()
            .enumerate()
            .map(|(at, kind)| {
                let seed = house_seed(policy_seed, at);
                match kind {
                    SeatKind::Human => SeatKind::Human,
                    SeatKind::Ai(agent) => SeatKind::Ai(agent.clone().with_seed(seed)),
                    SeatKind::Driven(agent) => SeatKind::Driven(agent.clone().with_seed(seed)),
                    SeatKind::StandIn(agent) => SeatKind::StandIn(agent.clone().with_seed(seed)),
                }
            })
            .collect();
        Ok(SimulationSession {
            engine,
            seats,
            teams: self.teams.clone(),
            scouting_decks: Arc::clone(&self.scouting_decks),
            fair_scouting: self.fair_scouting,
            seq: self.seq,
            decisions: self.decisions,
            house_answered: self.house_answered.clone(),
            seat,
        })
    }
}

impl SimulationSession {
    fn table(&self) -> Table<'_> {
        Table {
            engine: &self.engine,
            seats: &self.seats,
            teams: &self.teams,
            scouting_decks: &self.scouting_decks,
            fair_scouting: self.fair_scouting,
            seq: self.seq,
            house_answered: &self.house_answered,
        }
    }

    /// The seat this simulation was forked for.
    #[must_use]
    pub const fn seat(&self) -> PlayerId {
        self.seat
    }

    /// As [`Session::view_for`].
    #[must_use]
    pub fn view_for(&self, seat: PlayerId) -> Option<(Pending, PlayerView)> {
        self.table().view_for(seat)
    }

    /// As [`Session::view_for_with_context`].
    #[must_use]
    pub fn view_for_with_context(&self, seat: PlayerId) -> Option<(Pending, PlayerView)> {
        self.table().view_for_with_context(seat)
    }

    /// As [`Session::view_for_with_payment_context`].
    #[must_use]
    pub fn view_for_with_payment_context(&self, seat: PlayerId) -> Option<(Pending, PlayerView)> {
        self.table().view_for_with_payment_context(seat)
    }

    /// As [`Session::view_for_with_announcement_context`].
    #[must_use]
    pub fn view_for_with_announcement_context(
        &self,
        seat: PlayerId,
    ) -> Option<(Pending, PlayerView)> {
        self.table().view_for_with_announcement_context(seat)
    }

    /// As [`Session::observe_view`].
    #[must_use]
    pub fn observe_view(&self, seat: PlayerId) -> PlayerView {
        self.table().observe_view(seat)
    }

    /// As [`Session::observe_view_with_context`].
    #[must_use]
    pub fn observe_view_with_context(&self, seat: PlayerId) -> PlayerView {
        self.table().observe_view_with_context(seat)
    }

    /// As [`Session::observe_view_with_payment_context`].
    #[must_use]
    pub fn observe_view_with_payment_context(&self, seat: PlayerId) -> PlayerView {
        self.table().observe_view_with_payment_context(seat)
    }

    /// As [`Session::observe_view_with_announcement_context`].
    #[must_use]
    pub fn observe_view_with_announcement_context(&self, seat: PlayerId) -> PlayerView {
        self.table().observe_view_with_announcement_context(seat)
    }

    /// As [`Session::decision_seq`]: continues the count of the session it
    /// was forked from.
    #[must_use]
    pub const fn decision_seq(&self) -> u64 {
        self.decisions
    }

    /// The current pending choice.
    #[must_use]
    pub fn pending(&self) -> &Pending {
        self.engine.pending()
    }

    /// Every seat that owes an answer ([`Session::awaited`]).
    #[must_use]
    pub fn awaited(&self) -> SeatSet {
        self.engine.awaited()
    }

    /// Read-only state access.
    #[must_use]
    pub fn state(&self) -> &baylee_engine::state::GameState {
        self.engine.state()
    }

    /// The fork's determinism hash.
    #[must_use]
    pub fn snapshot_hash(&self) -> u64 {
        self.engine.snapshot_hash()
    }

    /// As [`Session::winning_seats`].
    #[must_use]
    pub fn winning_seats(&self, result: baylee_engine::win::GameResult) -> Vec<PlayerId> {
        self.table().winning_seats(result)
    }

    /// As [`Session::house_action`].
    #[must_use]
    pub fn house_action(&self, player: PlayerId) -> Option<PlayerAction> {
        self.table().house_action(player)
    }

    /// What kind of chair a seat is here.
    #[must_use]
    pub fn seat_kind(&self, seat: PlayerId) -> Option<&SeatKind> {
        self.seats.get(usize::from(seat.get()))
    }

    /// Hands a driven chair back to the agent it kept, as
    /// [`Session::release`] does: the fork's house plays it from here on.
    /// Returns whether it was driven.
    pub fn release(&mut self, seat: PlayerId) -> bool {
        let Some(kind) = self.seats.get_mut(usize::from(seat.get())) else {
            return false;
        };
        let SeatKind::Driven(agent) = kind else {
            return false;
        };
        *kind = SeatKind::Ai(agent.clone());
        true
    }

    /// As [`Session::needs_pump`]: whether the house owes an answer.
    #[must_use]
    pub fn needs_pump(&self) -> bool {
        self.engine.awaited().iter().any(|seat| {
            !self
                .seats
                .get(usize::from(seat.get()))
                .is_some_and(SeatKind::answers_over_socket)
                && self.engine.pending_for(seat).is_some()
        })
    }

    /// Applies an answer for a chair answered from outside (a driven or
    /// human chair), then lets the house answer at most `answers` questions
    /// ([`Self::pump_at_most`]); returns how many it answered.
    ///
    /// # Errors
    /// When `player` is not such a chair, or the engine refuses the answer;
    /// a refused answer leaves the fork as it was.
    pub fn act_slice(
        &mut self,
        player: PlayerId,
        action: PlayerAction,
        answers: usize,
    ) -> Result<usize, String> {
        if !self
            .seat_kind(player)
            .is_some_and(SeatKind::answers_over_socket)
        {
            return Err("not a seat answered from outside".to_string());
        }
        let moves = !action.is_automation_setting();
        if self.engine.apply(player, action).is_err() {
            return Err("illegal action for this seat".to_string());
        }
        self.seq += 1;
        if moves {
            self.decisions += 1;
            self.house_answered[usize::from(player.get())] = None;
        }
        Ok(self.pump_at_most(answers))
    }

    /// Lets the house answer at most `answers` questions, in seat order, and
    /// returns how many it answered: as [`Session::pump_at_most`], without
    /// the envelopes.
    ///
    /// Stops early once the game is over or a chair answered from outside is
    /// asked. An answer the engine refuses is replaced as the table replaces
    /// it (the answer that does nothing, else the default house agent); if
    /// that is refused too, the step is spent and nothing moved.
    pub fn pump_at_most(&mut self, answers: usize) -> usize {
        let mut answered = 0;
        while answered < answers {
            let awaited = self.engine.awaited();
            let house_seat = awaited.iter().find(|seat| {
                !self
                    .seats
                    .get(usize::from(seat.get()))
                    .is_some_and(SeatKind::answers_over_socket)
                    && self.engine.pending_for(*seat).is_some()
            });
            let Some(player) = house_seat else {
                return answered;
            };
            let Some(pending) = self.engine.pending_for(player).cloned() else {
                return answered;
            };
            let Some(action) = self.table().house_answer(player, &pending) else {
                return answered;
            };
            let by = self.seats[usize::from(player.get())]
                .is_away()
                .then_some(HouseAnswer::StandIn);
            let moves = self.apply_house(player, action);
            self.seq += 1;
            if moves {
                self.decisions += 1;
                self.house_answered[usize::from(player.get())] = by;
            }
            answered += 1;
        }
        answered
    }

    /// Applies the house's answer, or what the table answers instead when
    /// the engine refuses it; returns whether the game moved.
    fn apply_house(&mut self, player: PlayerId, action: PlayerAction) -> bool {
        let moves = !action.is_automation_setting();
        let refused = action.clone();
        if self.engine.apply(player, action).is_ok() {
            return moves;
        }
        let Some(pending) = self.engine.pending_for(player) else {
            return false;
        };
        let Some(fallback) = self.table().refusal_fallback(player, pending, &refused) else {
            return false;
        };
        self.engine.apply(player, fallback).is_ok()
    }
}
