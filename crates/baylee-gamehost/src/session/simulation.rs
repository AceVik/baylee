//! A session's game forked for one seat's lookahead: the house's own search
//! (`session/foresight/`) and the training and evaluation harnesses that host
//! their games in-process.
//!
//! [`Session::forked`] wraps the engine's own fork (`Engine::fork_for`,
//! `docs/engine-internals.md` §"Forking a simulation") in a
//! [`SimulationSession`]: the same chairs and the same house, answering
//! through the same views ([`super::table`]), and nothing a hosted table
//! keeps for the people at it — no record, no log, no envelopes, no clocks,
//! no spectators. It is never a seat's path over the gateway, which has no
//! engine to fork.
//!
//! Under the `foresight` feature, which the hosting builds enable, only the
//! house's search makes and plays one. What a harness drives a fork with by
//! hand (`Session::fork_for`, the context views, taking a chair over) is under
//! `simulation`, which no hosting build enables.

use std::collections::BTreeMap;
use std::sync::Arc;

use baylee_ai::intelligence::DeckIntel;
use baylee_core::ids::{ObjectId, PlayerId, SeatSet};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::{DeckKnowledge, Engine, ForkRefusal, PublicPrior};
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
    showing: Vec<SeatSet>,
    shown_once: BTreeMap<(u8, u8), Vec<ObjectId>>,
    hand_sharing: baylee_core::preset::HandSharingRule,
    ai_hand_share_policy: baylee_core::preset::AiHandSharePolicy,
    #[cfg(any(test, feature = "simulation"))]
    seat: PlayerId,
    /// What the house's answers and the engine's steps in this fork have
    /// taken, where a search measures its forks ([`Self::seat_model`]).
    /// Read by no answer.
    timing: Option<ForkTiming>,
    /// The meter of the search this fork belongs to, which every house
    /// answer given here is counted into ([`answer_work`]).
    meter: Option<Arc<baylee_ai::WorkMeter>>,
}

/// The clock a table's searches are measured by: the one its harness gave it
/// (`Session::set_foresight_clock`), such as the CPU time of the thread a game
/// is played on, which another job on the machine does not stretch, or its
/// host's wall guard's (`Session::set_foresight_host`). A measurement only:
/// no answer the bound decides reads it. None reads nothing, so a table
/// nobody gave a clock reads none, as the session reads none anywhere
/// (`LocalHost` runs in a browser too, where `Instant::now` panics).
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct Clock(pub(super) Option<fn() -> std::time::Duration>);

impl Clock {
    /// The time now, from an origin that is the same for every reading;
    /// always zero without a clock.
    pub(super) fn now(self) -> std::time::Duration {
        self.0.map_or(std::time::Duration::ZERO, |clock| clock())
    }
}

/// What a measured fork has taken so far.
#[derive(Clone, Copy, Debug, Default)]
pub(super) struct ForkTiming {
    clock: Clock,
    /// The house's answers: to every other question, and to an attack or a
    /// block.
    pub(super) think: [std::time::Duration; 2],
    /// How many answers of each.
    pub(super) asked: [u32; 2],
    /// The engine applying them.
    pub(super) apply: std::time::Duration,
    /// The permanents and stack objects each answer was given over, summed.
    pub(super) board: u64,
    /// What those answers cost on the search's meter ([`answer_work`]).
    pub(super) work: u64,
    /// The longest single answer, the engine's step included.
    pub(super) longest: std::time::Duration,
}

/// What one house answer in a fork costs on a search's meter
/// (`baylee_ai::WorkMeter`, in nodes of the combat search), the engine's
/// step that applies it included, with `board` permanents and stack objects
/// in the fork: reading the table and moving the game both grow with what is
/// on it.
///
/// Fitted (11.11.2026) to `eval4`'s `foresight.by_board` over 9,776 searched
/// answers at expert's tier, timed by their thread's CPU time: a combat-search
/// node on a full table takes about 0.08 µs, and an answer that is no attack
/// or block, its step included, 41, 57, 77, 109 and 154 µs at the mean on
/// boards from 0, 8, 16, 24 and 32 permanents and stack objects. A line
/// through the two ends, which prices the middle boards a little high:
/// 30 µs and 3.2 µs a permanent at 0.08 µs a unit
/// (`docs/proposals/house-ai-strength.md` §7).
pub(super) const fn answer_work(board: u64) -> u64 {
    375 + 40 * board
}

/// What making one fork costs on a search's meter, in the same unit: 196 to
/// 206 µs at the mean on every size of board in the same profile, so one
/// price for all, which also reads nothing of the game.
pub(super) const FORK_WORK: u64 = 2_500;

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
    /// `decks` goes to the engine as it is. Under [`DeckKnowledge::Prior`]
    /// the fork's house is not told the real lists either: the chair of
    /// `seat` keeps its own, and every other chair's house is told, as its
    /// deck, the cards the prior lists for it (each once) with the
    /// commanders it really has, which are public. A house answering for
    /// another chair in the fork would otherwise play the prior's cards by
    /// the real list, and the search would read that list off its play.
    ///
    /// Takes `&self`: forking, and anything done to the fork, leaves this
    /// session — its engine, hash, record and chairs — as it was.
    ///
    /// # Errors
    /// The engine's [`ForkRefusal`] for a position it cannot fork faithfully.
    #[cfg(any(test, feature = "simulation"))]
    pub fn fork_for(
        &self,
        seat: PlayerId,
        decks: DeckKnowledge<'_>,
        policy_seed: u64,
    ) -> Result<SimulationSession, ForkRefusal> {
        let scouting_decks = match decks {
            DeckKnowledge::Prior(prior) => self
                .scouting_decks
                .iter()
                .zip(0u8..)
                .map(|(real, at)| {
                    let chair = PlayerId::new(at);
                    let cards = if chair == seat {
                        real.cards.clone()
                    } else {
                        prior.cards(chair).iter().map(|&(card, _)| card).collect()
                    };
                    DeckIntel::new(cards, real.commanders.clone())
                })
                .collect(),
            DeckKnowledge::Known | DeckKnowledge::Unknown => Arc::clone(&self.scouting_decks),
        };
        self.forked(seat, decks, policy_seed, scouting_decks)
    }

    /// What the seat of `view` may suppose every other player hides, for a
    /// fair fork ([`DeckKnowledge::Prior`]): `baylee_ai`'s public prior,
    /// built from `view` and that seat's own deck list and from nothing
    /// else. The one builder a house search and a harness's search share.
    ///
    /// `view` must be the seat's own view of this table: the prior is fair
    /// because a view holds no card its seat may not see.
    #[must_use]
    pub fn public_prior(&self, view: &PlayerView) -> PublicPrior {
        let own = self
            .scouting_decks
            .get(usize::from(view.seat.get()))
            .map_or(&[][..], |deck| &deck.cards[..]);
        baylee_ai::foresight::prior::public_prior(view, own)
            .into_iter()
            .fold(PublicPrior::new(), |prior, (seat, cards)| {
                prior.with_player(seat, cards)
            })
    }

    /// [`Self::fork_for`] with the deck analyses its chairs are told given
    /// by the caller: a house search (`session/foresight.rs`) seats its own
    /// model in every chair and tells no chair but its own a deck, and
    /// analysing a prior's thousands of cards for every fork would be most
    /// of what a fork costs.
    pub(super) fn forked(
        &self,
        seat: PlayerId,
        decks: DeckKnowledge<'_>,
        policy_seed: u64,
        scouting_decks: Arc<[DeckIntel]>,
    ) -> Result<SimulationSession, ForkRefusal> {
        let shared = self.hands_told(seat)[0];
        let engine = self
            .engine
            .fork_for_with_shared(seat, decks, policy_seed, shared)?;
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
            scouting_decks,
            fair_scouting: self.fair_scouting,
            seq: self.seq,
            decisions: self.decisions,
            house_answered: self.house_answered.clone(),
            showing: self.showing.clone(),
            shown_once: self.shown_once.clone(),
            hand_sharing: self.house_rules.hand_sharing,
            ai_hand_share_policy: self.ai_hand_share_policy,
            #[cfg(any(test, feature = "simulation"))]
            seat,
            timing: None,
            meter: None,
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
            showing: &self.showing,
            shown_once: &self.shown_once,
            hand_sharing: self.hand_sharing,
            ai_hand_share_policy: self.ai_hand_share_policy,
        }
    }

    /// The deck the house in `chair` is told as its own when it scouts.
    #[cfg(test)]
    pub(super) fn told_deck(&self, chair: PlayerId) -> &[baylee_core::ids::CardIndex] {
        &self.scouting_decks[usize::from(chair.get())].cards
    }

    /// The seat this simulation was forked for.
    #[must_use]
    #[cfg(any(test, feature = "simulation"))]
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
    #[cfg(any(test, feature = "simulation"))]
    pub fn view_for_with_context(&self, seat: PlayerId) -> Option<(Pending, PlayerView)> {
        self.table().view_for_with_context(seat)
    }

    /// As [`Session::view_for_with_payment_context`].
    #[must_use]
    #[cfg(any(test, feature = "simulation"))]
    pub fn view_for_with_payment_context(&self, seat: PlayerId) -> Option<(Pending, PlayerView)> {
        self.table().view_for_with_payment_context(seat)
    }

    /// As [`Session::view_for_with_announcement_context`].
    #[must_use]
    #[cfg(any(test, feature = "simulation"))]
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
    #[cfg(any(test, feature = "simulation"))]
    pub fn observe_view_with_context(&self, seat: PlayerId) -> PlayerView {
        self.table().observe_view_with_context(seat)
    }

    /// As [`Session::observe_view_with_payment_context`].
    #[must_use]
    #[cfg(any(test, feature = "simulation"))]
    pub fn observe_view_with_payment_context(&self, seat: PlayerId) -> PlayerView {
        self.table().observe_view_with_payment_context(seat)
    }

    /// As [`Session::observe_view_with_announcement_context`].
    #[must_use]
    #[cfg(any(test, feature = "simulation"))]
    pub fn observe_view_with_announcement_context(&self, seat: PlayerId) -> PlayerView {
        self.table().observe_view_with_announcement_context(seat)
    }

    /// As [`Session::decision_seq`]: continues the count of the session it
    /// was forked from.
    #[must_use]
    #[cfg(any(test, feature = "simulation"))]
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
    #[cfg(any(test, feature = "simulation"))]
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
    #[cfg(any(test, feature = "simulation"))]
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
    #[cfg(any(test, feature = "simulation"))]
    pub fn house_action(&self, player: PlayerId) -> Option<PlayerAction> {
        self.table().house_action(player)
    }

    /// What kind of chair a seat is here.
    #[must_use]
    pub fn seat_kind(&self, seat: PlayerId) -> Option<&SeatKind> {
        self.seats.get(usize::from(seat.get()))
    }

    /// Takes a house chair over so the harness answers it, as
    /// [`Session::take_over`] does. Returns whether it was a house chair.
    #[cfg(any(test, feature = "simulation"))]
    pub fn take_over(&mut self, seat: PlayerId) -> bool {
        let Some(kind) = self.seats.get_mut(usize::from(seat.get())) else {
            return false;
        };
        let SeatKind::Ai(agent) = kind else {
            return false;
        };
        *kind = SeatKind::Driven(agent.clone());
        true
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
    #[cfg(any(test, feature = "simulation"))]
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
            let board = self.board();
            if let Some(meter) = &self.meter {
                meter.spend(answer_work(board));
            }
            let asked = self.timing.as_ref().map(|timing| timing.clock.now());
            let Some(action) = self.table().house_answer(player, &pending) else {
                return answered;
            };
            let by = self.seats[usize::from(player.get())]
                .is_away()
                .then_some(HouseAnswer::StandIn);
            let thought = self.timing.as_ref().map(|timing| timing.clock.now());
            let moves = self.apply_house(player, action);
            if let (Some(timing), Some(asked), Some(thought)) =
                (self.timing.as_mut(), asked, thought)
            {
                let combat = matches!(
                    pending,
                    Pending::ChooseAttackers { .. } | Pending::ChooseBlockers { .. }
                );
                let now = timing.clock.now();
                timing.think[usize::from(combat)] += thought.saturating_sub(asked);
                timing.asked[usize::from(combat)] += 1;
                timing.apply += now.saturating_sub(thought);
                timing.board += board;
                timing.work += answer_work(board);
                timing.longest = timing.longest.max(now.saturating_sub(asked));
            }
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

impl SimulationSession {
    /// Seats a copy of `model` in every chair, each seeded from `seed` and
    /// the chair, and holds `player`'s for an answer from outside
    /// ([`Self::act_slice`]; [`Self::release`] hands it to its copy): the
    /// fork a house search plays (`session/foresight.rs`).
    ///
    /// Nothing of the table's own chairs is left: not their kinds, levels,
    /// agents or seeds. With `fair`, the fork is held fair as well, so that
    /// no chair is told more than the deck it was forked with
    /// (`Session::forked`).
    ///
    /// Every copy counts its combat searches into `work` and is held to
    /// what that meter has left, so that one search's forks are bounded
    /// between them; and the fork is measured by `clock` from here on
    /// ([`Self::timing`]).
    pub(super) fn seat_model(
        &mut self,
        player: PlayerId,
        model: &baylee_ai::HeuristicAgent,
        seed: u64,
        fair: bool,
        work: &Arc<baylee_ai::WorkMeter>,
        clock: Clock,
    ) {
        self.seats = (0..self.seats.len())
            .map(|at| {
                let agent = model
                    .simulated(house_seed(seed, at))
                    .metered(Arc::clone(work));
                if at == usize::from(player.get()) {
                    SeatKind::Driven(agent)
                } else {
                    SeatKind::Ai(agent)
                }
            })
            .collect();
        if fair {
            self.fair_scouting = true;
        }
        self.meter = Some(Arc::clone(work));
        self.timing = Some(ForkTiming {
            clock,
            ..ForkTiming::default()
        });
    }

    /// What this fork's answers and steps have taken since a search seated
    /// its model in it.
    pub(super) fn timing(&self) -> ForkTiming {
        self.timing.unwrap_or_default()
    }

    /// Takes this fork off its search's meter: what is played in it from
    /// here on is counted nowhere and bounded by nothing. For the worlds a
    /// probe plays on past their leaf, which are no part of the search.
    pub(super) fn unmetered(&mut self) {
        self.meter = None;
        for seat in &mut self.seats {
            let (SeatKind::Ai(agent) | SeatKind::Driven(agent) | SeatKind::StandIn(agent)) = seat
            else {
                continue;
            };
            *agent = agent.clone().metered(Arc::default());
        }
    }

    /// The permanents and stack objects in this fork: what an answer here
    /// is priced by ([`answer_work`]).
    fn board(&self) -> u64 {
        use baylee_engine::zone::ZoneLocation;
        let zones = &self.engine.state().zones;
        (zones.list(ZoneLocation::Battlefield).len() + zones.list(ZoneLocation::Stack).len()) as u64
    }
}
