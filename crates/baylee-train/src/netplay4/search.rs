//! Belief-consistent lookahead over Playernet v4, for training and
//! evaluation harnesses that host their games in-process
//! (`docs/trained-ai.md` §"Lookahead search").
//!
//! At a priority decision in a main phase or combat, the policy's own
//! answer and its next best legal answers are each tried in worlds forked
//! from the table for the deciding seat ([`Session::fork_for`]): every card
//! the seat cannot see is dealt again from a policy seed, the house plays
//! every other chair, and the net plays the seat on from its own memory at
//! the decision. Each world ends at the game's end, when the seat holds
//! priority in a settled main phase, when its next own turn ends, or after
//! `H` decisions of its own (mana abilities and forced answers do not
//! count), and is scored by the outcome or by the net's value for the seat's
//! side. The policy's answer is played unless the [`Selection4`] finds
//! another one better: by a paired margin over the same worlds, by the
//! policy's prior weighed against the worlds, or (v1) by the best mean.
//! With [`SearchConfig4::with_targets`] (v3) a cast is also tried at each
//! of its targets, which the engine asks for where it cannot fork.
//!
//! Budgets are counts, never time; every seed is derived from the deal's
//! policy seed, the decision and the world; so a search is a function of
//! its inputs. A net on a shared batched session runs the rollouts at once
//! on twins of itself, tallied in the order they would run one by one. It
//! is never a seat's path over the gateway.

use std::str::FromStr;

use anyhow::Context as _;
use baylee_core::ids::PlayerId;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_engine::engine::{DeckKnowledge, ForkRefusal, PublicPrior};
use baylee_gamehost::{Session, SimulationSession};
use baylee_view::{Phase, PlayerView};
use serde_json::{Value, json};

use super::{Answer4, NetPlayer4, PlayerSnapshot4, Ranked4, RootDecision4, Script4};
use crate::deckgen::Rng;
use crate::features4::{PendingKind4, SourceKind4, StepProvenance4, TeacherTarget4, side_utility};

/// The search's contract name, for run metadata and reports.
pub const SEARCH_CONTRACT: &str = "belief_lookahead_v2";
/// The contract of a search whose casts are tried at each of their targets
/// ([`SearchConfig4::with_targets`]); otherwise [`SEARCH_CONTRACT`]'s.
pub const SEARCH_CONTRACT_TARGETS: &str = "belief_lookahead_v3";
/// The contract of a search that also takes combat declarations as roots
/// ([`SearchConfig4::with_combat`]), with or without targets.
pub const SEARCH_CONTRACT_COMBAT: &str = "belief_lookahead_v4";
/// The contract of a search with fair worlds, turn horizons or macro
/// candidates ([`SearchConfig4::with_fair`], [`SearchConfig4::with_turns`],
/// [`SearchConfig4::with_macros`]), whatever else it has.
pub const SEARCH_CONTRACT_FORESIGHT: &str = "belief_lookahead_v5";

/// How a search chooses among its candidates once their worlds are played.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
#[serde(tag = "rule", rename_all = "snake_case")]
pub enum Selection4 {
    /// The best mean over complete rounds, ties to the policy's answer, then
    /// candidate order (v1).
    Mean,
    /// The policy's answer stays unless another candidate's mean improvement
    /// over it, world by world, exceeds `max(margin, z · se)`, `se` being the
    /// standard error of those paired differences. With fewer than two
    /// rounds there is no standard error, and only `z = 0` can override.
    /// Of the candidates that pass, the largest mean improvement wins.
    Paired {
        /// The smallest mean improvement that counts, in value units.
        margin: f64,
        /// Standard errors the mean improvement must also exceed.
        z: f64,
    },
    /// The PUCT exploitation score with equal visits: a candidate's mean
    /// plus `c · prior / (1 + n)`, its prior the policy's probability of it
    /// (softmax of the raw logits over every legal answer) and `n` the
    /// complete rounds. The policy's preference fades as worlds accumulate.
    Prior {
        /// Weight of the prior.
        c: f64,
    },
}

/// The default: [`Selection4::Prior`] with `c = 1`. On s1 it left land drops
/// and casts alone where [`Selection4::Paired`] replaced one land drop in
/// seven with a pass: the value head prefers the pass there consistently,
/// and with greedy rollouts and leaves inside the turn the worlds often
/// vary not at all (a standard error of 0), so only the margin was left to
/// guard (`docs/trained-ai.md` §"Lookahead search").
impl Default for Selection4 {
    fn default() -> Self {
        Self::Prior { c: 1.0 }
    }
}

impl Selection4 {
    /// Checks that every parameter is a finite, non-negative number.
    ///
    /// # Errors
    /// When one is not.
    pub fn validate(self) -> anyhow::Result<Self> {
        let parameters: &[f64] = match &self {
            Self::Mean => &[],
            Self::Paired { margin, z } => &[*margin, *z],
            Self::Prior { c } => &[*c],
        };
        anyhow::ensure!(
            parameters.iter().all(|x| x.is_finite() && *x >= 0.0),
            "search selection parameters must be finite and non-negative"
        );
        Ok(self)
    }
}

/// One searched decision's budgets (counts only, never time) and how it
/// chooses.
#[derive(Clone, Copy, Debug, PartialEq, serde::Serialize)]
pub struct SearchConfig4 {
    candidates: usize,
    particles: usize,
    horizon: usize,
    selection: Selection4,
    threads: usize,
    targets: usize,
    neural_opponent: bool,
    combat: usize,
    combat_only: bool,
    decks: Decks4,
    turns: usize,
    macros: bool,
    house: HouseInWorlds4,
}

/// What a rollout does where its next answer would be the house's.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum HouseInWorlds4 {
    /// Plays it, as the evaluator does at the table, and counts it.
    Answers,
    /// Stops there and scores the position before it.
    Stops,
}

/// What a search's worlds deal the hidden cards from.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum Decks4 {
    /// The real lists, which the harness asserts known ([`KnownDecks`]).
    Known,
    /// The seat's public prior ([`PriorDecks`]).
    Prior,
}

impl SearchConfig4 {
    /// The most candidates one decision compares.
    pub const MAX_CANDIDATES: usize = 32;
    /// The most worlds one candidate is tried in.
    pub const MAX_PARTICLES: usize = 256;
    /// The most decisions of the searching seat one rollout counts.
    pub const MAX_HORIZON: usize = 10_000;
    /// The most rollouts one searched decision runs at once.
    pub const MAX_THREADS: usize = 256;
    /// Rollouts one searched decision runs at once unless told otherwise.
    pub const DEFAULT_THREADS: usize = 16;
    /// The most targets one cast is tried at.
    pub const MAX_TARGETS: usize = 8;
    /// The most declarations one combat root compares.
    pub const MAX_COMBAT: usize = 16;
    /// The most turn boundaries one rollout plays to.
    pub const MAX_TURNS: usize = 16;

    /// `candidates` answers (K, the policy's own first), each tried in
    /// `particles` worlds (P), each rolled out at most `horizon` decisions
    /// of the searching seat past the answer (H; `0` scores the answer's own
    /// position), chosen among by the default [`Selection4`].
    ///
    /// # Errors
    /// When K or P is zero or any budget is past its maximum.
    pub fn new(candidates: usize, particles: usize, horizon: usize) -> anyhow::Result<Self> {
        anyhow::ensure!(
            (1..=Self::MAX_CANDIDATES).contains(&candidates),
            "search candidates must be 1..={}",
            Self::MAX_CANDIDATES
        );
        anyhow::ensure!(
            (1..=Self::MAX_PARTICLES).contains(&particles),
            "search particles must be 1..={}",
            Self::MAX_PARTICLES
        );
        anyhow::ensure!(
            horizon <= Self::MAX_HORIZON,
            "search horizon must be at most {}",
            Self::MAX_HORIZON
        );
        Ok(Self {
            candidates,
            particles,
            horizon,
            selection: Selection4::default(),
            threads: Self::DEFAULT_THREADS,
            targets: 0,
            neural_opponent: false,
            combat: 0,
            combat_only: false,
            decks: Decks4::Known,
            turns: 0,
            macros: false,
            house: HouseInWorlds4::Answers,
        })
    }

    /// The same search in fair worlds: every card the seat cannot see is
    /// dealt from what its own view lets it suppose ([`PriorDecks`]), never
    /// from the real lists. The caller forks from a [`PriorDecks`] then; this
    /// says so in the run's metadata.
    #[must_use]
    pub const fn with_fair(self, fair: bool) -> Self {
        let decks = if fair { Decks4::Prior } else { Decks4::Known };
        Self { decks, ..self }
    }

    /// Whether the worlds are dealt from a public prior.
    #[must_use]
    pub const fn fair(&self) -> bool {
        matches!(self.decks, Decks4::Prior)
    }

    /// The same search, its rollouts played to the `turns`-th turn boundary
    /// past the root (2: through the next player's turn in a duel) instead
    /// of to the seat's first settled pass or the end of its next own turn.
    /// The horizon H still caps the seat's decisions. `0` (the default)
    /// keeps v2's stops.
    ///
    /// # Errors
    /// When `turns` is past [`Self::MAX_TURNS`].
    pub fn with_turns(self, turns: usize) -> anyhow::Result<Self> {
        anyhow::ensure!(
            turns <= Self::MAX_TURNS,
            "search turns must be at most {}",
            Self::MAX_TURNS
        );
        Ok(Self { turns, ..self })
    }

    /// Turn boundaries a rollout plays to; `0` for v2's stops.
    #[must_use]
    pub const fn turns(&self) -> usize {
        self.turns
    }

    /// The same search with macro candidates at priority (`search/macros.rs`):
    /// the policy's answer, then a pass, each spell the planner can pay for
    /// (its taps and the cast as one candidate), each ability and each
    /// land, read off the engine's offer instead of the policy's ranking.
    #[must_use]
    pub const fn with_macros(self, macros: bool) -> Self {
        Self { macros, ..self }
    }

    /// Whether priority roots compare macro candidates.
    #[must_use]
    pub const fn macros(&self) -> bool {
        self.macros
    }

    /// The same search with no house answer in a world: where a rollout's
    /// next answer would be the house's (the player's own fallback for a
    /// question it does not support, a repeated view or a failed decode, or
    /// the fork's house standing in for a refused answer), the rollout stops
    /// and the position before it is scored (`leaves.house_stop`). For a
    /// search whose worlds feed labels: the house is no label source.
    /// Otherwise such answers are played and counted
    /// ([`SearchReport4::house_answers`]).
    #[must_use]
    pub const fn with_house_free(self, house_free: bool) -> Self {
        let house = if house_free {
            HouseInWorlds4::Stops
        } else {
            HouseInWorlds4::Answers
        };
        Self { house, ..self }
    }

    /// Whether a rollout stops where a house answer would be played.
    #[must_use]
    pub const fn house_free(&self) -> bool {
        matches!(self.house, HouseInWorlds4::Stops)
    }

    /// The same search, with every other chair in a rollout answered by the
    /// searching player's net from that chair's simulated view (with empty
    /// prior deck knowledge) instead of releasing other chairs to the house.
    #[must_use]
    pub const fn with_neural_opponent(self, neural_opponent: bool) -> Self {
        Self {
            neural_opponent,
            ..self
        }
    }

    /// Whether other chairs in a rollout are answered by the net instead of
    /// the house.
    #[must_use]
    pub const fn neural_opponent(&self) -> bool {
        self.neural_opponent
    }

    /// The same search, trying a candidate that casts a spell or activates
    /// an ability at each of up to `targets` of its targets: when the
    /// seat's next question after it is a single target, the candidate
    /// becomes one per target the policy ranks there, its own first
    /// (`docs/trained-ai.md` §"Lookahead search"). `0` (the default) leaves
    /// targets to the policy, as v2 does.
    ///
    /// # Errors
    /// When `targets` is past [`Self::MAX_TARGETS`].
    pub fn with_targets(self, targets: usize) -> anyhow::Result<Self> {
        anyhow::ensure!(
            targets <= Self::MAX_TARGETS,
            "search targets must be at most {}",
            Self::MAX_TARGETS
        );
        Ok(Self { targets, ..self })
    }

    /// The most targets one cast is tried at; `0` when targets are left to
    /// the policy.
    #[must_use]
    pub const fn targets(&self) -> usize {
        self.targets
    }

    /// The same search, also taking a declaration of attackers or blockers
    /// as a root (v4, `search/combat.rs`): the policy's declaration and up
    /// to `combat - 1` others built from it are compared. `0` (the default)
    /// leaves declarations to the policy.
    ///
    /// # Errors
    /// When `combat` is past [`Self::MAX_COMBAT`].
    pub fn with_combat(self, combat: usize) -> anyhow::Result<Self> {
        anyhow::ensure!(
            combat <= Self::MAX_COMBAT,
            "search combat declarations must be at most {}",
            Self::MAX_COMBAT
        );
        Ok(Self { combat, ..self })
    }

    /// The most declarations a combat root compares; `0` when declarations
    /// are left to the policy.
    #[must_use]
    pub const fn combat(&self) -> usize {
        self.combat
    }

    /// The same search taking combat declarations alone as roots when
    /// `combat_only` (priority decisions answered by the policy, counted as
    /// [`SearchSkip4::Roots`]): a teacher for attacks and blocks whose
    /// budget goes to them.
    #[must_use]
    pub const fn with_combat_only(self, combat_only: bool) -> Self {
        Self {
            combat_only,
            ..self
        }
    }

    /// Whether combat declarations are the only roots.
    #[must_use]
    pub const fn combat_only(&self) -> bool {
        self.combat_only
    }

    /// The contract this search keeps.
    #[must_use]
    pub const fn contract(&self) -> &'static str {
        if self.fair() || self.turns > 0 || self.macros {
            SEARCH_CONTRACT_FORESIGHT
        } else if self.combat > 0 {
            SEARCH_CONTRACT_COMBAT
        } else if self.targets > 0 {
            SEARCH_CONTRACT_TARGETS
        } else {
            SEARCH_CONTRACT
        }
    }

    /// The same search, with at most `threads` rollouts at once. They run at
    /// once only for a player that lends rollout players of its own
    /// ([`SearchPlayer4::rollout_player`]: a net on a shared batched
    /// session); `1` runs them one by one whatever the player.
    ///
    /// # Errors
    /// When `threads` is zero or past [`Self::MAX_THREADS`].
    pub fn with_threads(self, threads: usize) -> anyhow::Result<Self> {
        anyhow::ensure!(
            (1..=Self::MAX_THREADS).contains(&threads),
            "search threads must be 1..={}",
            Self::MAX_THREADS
        );
        Ok(Self { threads, ..self })
    }

    /// The most rollouts one searched decision runs at once.
    #[must_use]
    pub const fn threads(&self) -> usize {
        self.threads
    }

    /// The same budgets, chosen among by `selection`.
    ///
    /// # Errors
    /// When a selection parameter is negative or not finite.
    pub fn with_selection(self, selection: Selection4) -> anyhow::Result<Self> {
        Ok(Self {
            selection: selection.validate()?,
            ..self
        })
    }

    /// How the candidates are chosen among.
    #[must_use]
    pub const fn selection(&self) -> Selection4 {
        self.selection
    }

    /// K: answers compared, the policy's own first.
    #[must_use]
    pub const fn candidates(&self) -> usize {
        self.candidates
    }

    /// P: worlds each answer is tried in.
    #[must_use]
    pub const fn particles(&self) -> usize {
        self.particles
    }

    /// H: decisions of the searching seat a rollout counts past the answer
    /// (neither forced answers nor mana abilities count).
    #[must_use]
    pub const fn horizon(&self) -> usize {
        self.horizon
    }

    /// What a run records about its search, so a reader knows exactly what
    /// produced every searched answer.
    #[must_use]
    pub fn metadata(&self) -> Value {
        let mut metadata = json!({
            "contract": self.contract(),
            "candidates": self.candidates,
            "particles": self.particles,
            "horizon": self.horizon,
            "horizon_unit": "searching_seat_decisions_neither_forced_nor_mana_abilities",
            "roots": "priority_in_main_phases_and_combat",
            "candidate_order": "policy_answer_then_raw_logit_desc_then_menu_order",
            "decks": "known_to_the_harness",
            "worlds": "session_fork_redealt_from_purpose_tagged_policy_seeds",
            "rollout": if self.neural_opponent {
                "net_for_every_chair_other_seats_from_world_view_only"
            } else {
                "net_for_the_searching_seat_house_for_every_other_chair"
            },
            "stop": "game_over_or_settled_priority_or_end_of_next_own_turn_or_horizon",
            "settled": "seat_would_pass_priority_in_a_main_phase_after_the_roots_phase_empty_stack_no_cast_no_payment_no_floating_mana",
            "leaf": "terminal_side_utility_else_k0_observation_value_of_the_seat",
            "prior": "softmax_of_raw_logits_over_every_legal_answer",
            "selection": self.selection,
            "default_selection": Selection4::default(),
            "threads": self.threads,
            "parallel": "rollouts_at_once_on_twin_players_of_a_shared_batched_net_tallied_in_serial_order",
            "provenance": "search_without_behavior_logprob",
        });
        if self.targets > 0 {
            metadata["targets"] = json!(self.targets);
            metadata["target_candidates"] = json!(
                "a_cast_or_activation_whose_next_question_in_world_0_is_a_single_target_\
                 tried_at_the_policys_own_target_then_its_next_by_raw_logit"
            );
            metadata["target_prior"] = json!("root_prior_times_softmax_at_the_target_question");
            metadata["target_follow"] =
                json!("the_chosen_target_answers_the_seats_next_target_question_at_the_table");
        }
        if self.combat > 0 {
            metadata["combat"] = json!(self.combat);
            metadata["combat_roots"] =
                json!("declarations_of_attackers_and_blockers_whose_answer_is_the_policys_own");
            metadata["combat_candidates"] = json!(
                "the_policys_declaration_then_none_all_in_and_each_creatures_part_changed_alone_\
                 those_world_0_refuses_dropped"
            );
            metadata["combat_prior"] = json!("one_for_the_policys_declaration_zero_for_the_others");
            if self.combat_only {
                metadata["roots"] = json!("declarations_of_attackers_and_blockers_only");
            }
        }
        if self.fair() {
            metadata["decks"] =
                json!("public_prior_from_the_seats_view_and_own_list_never_another_players_list");
        }
        if self.turns > 0 {
            metadata["turns"] = json!(self.turns);
            metadata["stop"] = json!("game_over_or_the_turn_boundary_after_turns_turns_or_horizon");
        }
        metadata["house_in_worlds"] = json!(self.house);
        if self.macros {
            metadata["roots"] = json!(
                "priority_at_the_start_of_a_chain_where_the_policy_acts_or_in_the_seats_main_phase_\
                 or_with_another_players_object_on_top_of_the_stack"
            );
            metadata["candidate_order"] = json!(
                "policy_answer_then_pass_then_each_spell_the_planner_pays_for_then_castable_spells_\
                 then_lands_then_abilities_one_per_card"
            );
            metadata["prior"] = json!("one_for_the_policys_answer_zero_for_the_others");
        }
        metadata
    }
}

impl FromStr for SearchConfig4 {
    type Err = anyhow::Error;

    /// `K,P,H`, e.g. `4,2,8`.
    fn from_str(text: &str) -> anyhow::Result<Self> {
        let parts: Vec<&str> = text.split(',').map(str::trim).collect();
        let [k, p, h] = parts.as_slice() else {
            anyhow::bail!("search budgets are K,P,H (candidates, particles, horizon)");
        };
        Self::new(
            k.parse().context("search candidates K")?,
            p.parse().context("search particles P")?,
            h.parse().context("search horizon H")?,
        )
    }
}

/// Which of a session's views a player reads, by its encoder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ViewContext4 {
    /// [`Session::view_for`] and [`Session::observe_view`].
    Plain,
    /// The `*_with_context` views.
    Choice,
    /// The `*_with_payment_context` views.
    Payment,
    /// The `*_with_announcement_context` views.
    Announcement,
}

impl ViewContext4 {
    /// The question `seat` owes in `fork` and its view of it.
    #[must_use]
    pub fn view_for(
        self,
        fork: &SimulationSession,
        seat: PlayerId,
    ) -> Option<(Pending, PlayerView)> {
        match self {
            Self::Plain => fork.view_for(seat),
            Self::Choice => fork.view_for_with_context(seat),
            Self::Payment => fork.view_for_with_payment_context(seat),
            Self::Announcement => fork.view_for_with_announcement_context(seat),
        }
    }

    /// What `seat` observes of `fork` now.
    #[must_use]
    pub fn observe(self, fork: &SimulationSession, seat: PlayerId) -> PlayerView {
        match self {
            Self::Plain => fork.observe_view(seat),
            Self::Choice => fork.observe_view_with_context(seat),
            Self::Payment => fork.observe_view_with_payment_context(seat),
            Self::Announcement => fork.observe_view_with_announcement_context(seat),
        }
    }
}

/// What a search needs of the player it searches for. [`NetPlayer4`] is
/// the one that plays; a test stands in for it without a model.
pub trait SearchPlayer4 {
    /// Everything per game the player changes while it plays.
    type Snapshot;

    /// The player's per-game state as it stands.
    fn snapshot(&self) -> Self::Snapshot;
    /// Undoes everything done since `snapshot` was taken.
    fn restore(&mut self, snapshot: &Self::Snapshot);
    /// Which views the player reads.
    fn view_context(&self) -> ViewContext4;
    /// Takes in a view, as the host has the player do before every action.
    ///
    /// # Errors
    /// When the player cannot observe (its model failed).
    fn observe(&mut self, view: &PlayerView) -> anyhow::Result<()>;
    /// The player's value for `seat`'s side from `seat`'s latest observation.
    fn observed_value(&self, seat: PlayerId) -> Option<f32>;
    /// Decides as in play, up to noting the answer, with the ranking of
    /// the legal answers when the answer is the policy's own.
    fn decide(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> RootDecision4;
    /// Finishes `decision` with `action` as its answer.
    fn conclude(&mut self, view: &PlayerView, decision: &RootDecision4, action: &PlayerAction);
    /// Answers a question as in play.
    fn answer(&mut self, view: &PlayerView, pending: &Pending, teams: &[Option<u8>]) -> Answer4;
    /// Restarts the player's sampling stream from `seed`, keeping the game.
    fn reseed_sampling(&mut self, seed: u64);
    /// Counts a decision recorded under `from` as searched instead.
    fn attribute_to_search(&mut self, pending: &Pending, from: SourceKind4);
    /// Clears any other seat's game state before a neural-opponent rollout so
    /// another chair in a simulated world reads only what that world shows it.
    fn reset_other_seats_for_rollout(&mut self, searching_seat: PlayerId) {
        let _ = searching_seat;
    }
    /// Whether [`Self::rollout_player`] lends players, so that a search can
    /// run its rollouts at once.
    fn lends_rollout_players(&self) -> bool {
        false
    }
    /// A player of the player's own, configured as it is and in `snapshot`'s
    /// state, to play one rollout on another thread: it answers every
    /// question as the player would from that state. `None` when the player
    /// lends none.
    fn rollout_player(&self, snapshot: &Self::Snapshot) -> Option<Self>
    where
        Self: Sized,
    {
        let _ = snapshot;
        None
    }
}

impl SearchPlayer4 for NetPlayer4 {
    type Snapshot = PlayerSnapshot4;

    fn snapshot(&self) -> PlayerSnapshot4 {
        Self::snapshot(self)
    }
    fn restore(&mut self, snapshot: &PlayerSnapshot4) {
        Self::restore(self, snapshot);
    }
    fn view_context(&self) -> ViewContext4 {
        if self.uses_announcement_context() {
            ViewContext4::Announcement
        } else if self.uses_payment_context() {
            ViewContext4::Payment
        } else if self.is_encoder7() {
            ViewContext4::Choice
        } else {
            ViewContext4::Plain
        }
    }
    fn observe(&mut self, view: &PlayerView) -> anyhow::Result<()> {
        Self::observe(self, view)
    }
    fn observed_value(&self, seat: PlayerId) -> Option<f32> {
        Self::observed_value(self, seat)
    }
    fn decide(
        &mut self,
        view: &PlayerView,
        pending: &Pending,
        teams: &[Option<u8>],
    ) -> RootDecision4 {
        Self::decide(self, view, pending, teams, true)
    }
    fn conclude(&mut self, view: &PlayerView, decision: &RootDecision4, action: &PlayerAction) {
        Self::conclude(self, view, decision, action);
    }
    fn answer(&mut self, view: &PlayerView, pending: &Pending, teams: &[Option<u8>]) -> Answer4 {
        self.act_detailed(view, pending, teams)
    }
    fn reseed_sampling(&mut self, seed: u64) {
        Self::reseed_sampling(self, seed);
    }
    fn attribute_to_search(&mut self, pending: &Pending, from: SourceKind4) {
        let row = &mut self.coverage.counts[PendingKind4::of(pending) as usize];
        row[from as usize] = row[from as usize].saturating_sub(1);
        row[SourceKind4::Search as usize] += 1;
    }
    fn reset_other_seats_for_rollout(&mut self, searching_seat: PlayerId) {
        Self::reset_other_seats_for_rollout(self, searching_seat);
    }
    fn lends_rollout_players(&self) -> bool {
        self.is_on_shared_net()
    }
    fn rollout_player(&self, snapshot: &PlayerSnapshot4) -> Option<Self> {
        self.twin_on_shared_net(snapshot)
    }
}

/// The table a search forks its worlds from.
pub trait ForkSource4 {
    /// A world forked for `seat` from `policy_seed`.
    ///
    /// # Errors
    /// The engine's refusal of the position.
    fn fork(&self, seat: PlayerId, policy_seed: u64) -> Result<SimulationSession, ForkRefusal>;
    /// The table's [`Session::decision_seq`], which seeds the worlds.
    fn decision_seq(&self) -> u64;
}

/// A session whose every deck list the harness asserts the searching seat
/// may know ([`DeckKnowledge::Known`]): the decks an evaluation chose, or
/// published lists. The worlds are dealt from the real remaining decks.
pub struct KnownDecks<'a>(pub &'a Session);

impl ForkSource4 for KnownDecks<'_> {
    fn fork(&self, seat: PlayerId, policy_seed: u64) -> Result<SimulationSession, ForkRefusal> {
        self.0.fork_for(seat, DeckKnowledge::Known, policy_seed)
    }
    fn decision_seq(&self) -> u64 {
        self.0.decision_seq()
    }
}

/// A session forked into fair worlds ([`DeckKnowledge::Prior`]): every card
/// the searching seat cannot see is dealt from the public prior its own view
/// and its own deck list give ([`Session::public_prior`], the builder the
/// house's foresight uses), never from another player's list. Two tables that
/// differ only in what the seat cannot see fork into the same worlds.
pub struct PriorDecks<'a> {
    table: &'a Session,
    prior: PublicPrior,
}

impl<'a> PriorDecks<'a> {
    /// `table` forked for the seat of `view`, which must be that seat's own
    /// view of `table`.
    #[must_use]
    pub fn new(table: &'a Session, view: &PlayerView) -> Self {
        Self {
            table,
            prior: table.public_prior(view),
        }
    }
}

impl ForkSource4 for PriorDecks<'_> {
    fn fork(&self, seat: PlayerId, policy_seed: u64) -> Result<SimulationSession, ForkRefusal> {
        self.table
            .fork_for(seat, DeckKnowledge::Prior(&self.prior), policy_seed)
    }
    fn decision_seq(&self) -> u64 {
        self.table.decision_seq()
    }
}

/// Why a decision was answered without a search.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchSkip4 {
    /// The question is not ordinary priority.
    NotPriority,
    /// Priority outside a main phase and combat.
    Phase,
    /// The answer is not the policy's own choice: forced, unsupported,
    /// guarded against a repeated view, or a fallback.
    NotPolicy,
    /// A candidate was refused in its world before one round completed.
    CandidateRefused,
    /// A priority decision in a search that takes combat declarations
    /// alone as roots ([`SearchConfig4::combat_only`]).
    Roots,
    /// Priority in the middle of a chain (mana floating, a spell being cast,
    /// a payment owed), in a search with macro candidates.
    MidChain,
    /// Nothing to compare the policy's answer with.
    NoAlternative,
}

impl SearchSkip4 {
    /// A stable name for counts.
    #[must_use]
    pub const fn key(self) -> &'static str {
        match self {
            Self::NotPriority => "not_priority",
            Self::Phase => "phase",
            Self::NotPolicy => "not_policy",
            Self::CandidateRefused => "candidate_refused",
            Self::Roots => "roots",
            Self::MidChain => "mid_chain",
            Self::NoAlternative => "no_alternative",
        }
    }
}

/// What one searched decision found.
#[derive(Clone, Debug, PartialEq, serde::Serialize)]
pub struct SearchReport4 {
    /// Candidates compared (at most K; fewer when fewer answers are legal).
    pub candidates: usize,
    /// The candidates in candidate order, the policy's own answer first.
    pub actions: Vec<PlayerAction>,
    /// Each candidate's answer to the target question after it, when the
    /// search tries its targets ([`SearchConfig4::with_targets`]); `None`
    /// where the policy answers it.
    pub targets: Vec<Option<PlayerAction>>,
    /// What each candidate's taps pay for, when it is a planned cast or
    /// activation ([`SearchConfig4::with_macros`]); `None` for a single answer.
    pub scripts: Vec<Option<PlayerAction>>,
    /// Complete rounds: worlds in which every candidate was rolled out.
    pub rounds: usize,
    /// The chosen candidate's place in candidate order (`0`: the policy's).
    pub chosen: usize,
    /// Whether the search played another answer than the policy's.
    pub changed: bool,
    /// Each candidate's mean score over the complete rounds.
    pub means: Vec<f64>,
    /// Each candidate's mean improvement over the policy's answer, world by
    /// world (`0` for the policy's answer itself).
    pub diffs: Vec<f64>,
    /// The standard error of each improvement; `None` with fewer than two
    /// rounds, and for the policy's answer.
    pub ses: Vec<Option<f64>>,
    /// Each candidate's prior: the policy's probability of it.
    pub priors: Vec<f64>,
    /// The score the selection ranked each candidate by.
    pub scores: Vec<f64>,
    /// Rollouts run, those of an incomplete round included.
    pub rollouts: u64,
    /// Actions taken in rollouts past their candidates, every chair's.
    pub rollout_steps: u64,
    /// Decisions of the searching seat that counted toward the horizon.
    pub seat_decisions: u64,
    /// House answers played in rollouts: a chair's answer the player gave
    /// as the house's, or the fork's house standing in for a refused one.
    pub house_answers: u64,
    /// How the rollouts ended.
    pub leaves: LeafCounts4,
    /// Whether the rollouts ran at once, on rollout players of the player's
    /// own ([`SearchPlayer4::rollout_player`]), rather than one by one.
    pub parallel: bool,
}

/// How rollouts ended, by kind. All but `terminal` are scored by the value
/// head.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize)]
pub struct LeafCounts4 {
    /// The game ended.
    pub terminal: u64,
    /// `H = 0`: the candidate's own position.
    pub immediate: u64,
    /// The searching seat held priority in a settled position.
    pub settled: u64,
    /// The horizon's decisions were spent.
    pub horizon: u64,
    /// The searching seat's next own turn ended.
    pub turn_end: u64,
    /// The turn boundary of [`SearchConfig4::with_turns`] was reached.
    pub boundary: u64,
    /// Nobody could answer, an answer was refused, or the action cap was
    /// reached.
    pub stopped: u64,
    /// Of `stopped`: the next answer was the house's and the search plays
    /// none ([`SearchConfig4::with_house_free`]).
    pub house_stop: u64,
}

impl LeafCounts4 {
    /// Adds `other`'s counts to these.
    pub fn add(&mut self, other: &Self) {
        self.terminal += other.terminal;
        self.immediate += other.immediate;
        self.settled += other.settled;
        self.horizon += other.horizon;
        self.turn_end += other.turn_end;
        self.boundary += other.boundary;
        self.stopped += other.stopped;
        self.house_stop += other.house_stop;
    }

    /// Rollouts scored by the value head.
    #[must_use]
    pub const fn value(&self) -> u64 {
        self.immediate + self.settled + self.horizon + self.turn_end + self.boundary + self.stopped
    }
}

/// How a decision was answered.
#[derive(Clone, Debug, PartialEq)]
pub enum SearchOutcome4 {
    /// Searched.
    Searched(Box<SearchReport4>),
    /// Answered by the policy without searching.
    Skipped(SearchSkip4),
    /// Answered by the policy: the table refused to fork.
    Refused(ForkRefusal),
}

/// A searched decision's answer and how it was found.
#[derive(Clone, Debug)]
pub struct SearchedAnswer4 {
    /// The answer to play. `SourceKind4::Search` and no behavior
    /// probability when searched; the policy's own answer otherwise.
    pub answer: Answer4,
    /// How it was found.
    pub outcome: SearchOutcome4,
    /// The target the chosen candidate was tried at, for the seat's next
    /// question at the table to be answered with ([`follow_decision`]);
    /// `None` when the policy answers it.
    pub follow: Option<PlayerAction>,
    /// The rest of the chosen candidate's planned cast or activation, whose
    /// first tap is the answer: the seat's next questions at the table are
    /// answered from it ([`follow_script`]). `None` for a single answer.
    pub script: Option<Script4>,
}

/// A candidate: an answer at the root and, when the search tries it at a
/// target, the answer to the single-target question that follows it.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Candidate4 {
    /// The answer at the root, as the policy ranked it.
    pub(super) root: Ranked4,
    /// The target it is tried at; `None` for the policy to choose.
    pub(super) target: Option<TargetFollow4>,
    /// The planned cast or activation `root` is the first tap of; `None`
    /// for a single answer.
    pub(super) script: Option<Script4>,
}

/// A target a candidate is tried at.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct TargetFollow4 {
    /// The answer to the target question.
    pub(super) action: PlayerAction,
    /// The policy's probability of it at that question.
    pub(super) prior: f64,
}

impl Candidate4 {
    /// The candidate the policy finishes itself.
    const fn plain(root: Ranked4) -> Self {
        Self {
            root,
            target: None,
            script: None,
        }
    }
}

/// Purpose tags of the seeds a search derives.
const SEARCH: u64 = u64::from_be_bytes(*b"search\0\0");
const WORLD: u64 = u64::from_be_bytes(*b"world\0\0\0");
const ROLLOUT: u64 = u64::from_be_bytes(*b"rollout\0");

/// `SplitMix64` over `seed`, a purpose and two indices: one independent
/// seed per purpose and position.
fn derive(seed: u64, purpose: u64, a: u64, b: u64) -> u64 {
    let first = Rng::new(seed ^ purpose).next();
    let second = Rng::new(first ^ a).next();
    Rng::new(second ^ b.rotate_left(17)).next()
}

/// A game's search seed, from the deal's policy seed (never the deal's
/// shuffle seed).
#[must_use]
pub fn search_seed(policy_seed: u64) -> u64 {
    derive(policy_seed, SEARCH, 0, 0)
}

/// The policy seed of world `particle` at the decision `decision_seq`.
fn world_seed(search_seed: u64, decision_seq: u64, particle: usize) -> u64 {
    derive(search_seed, WORLD, decision_seq, particle as u64)
}

/// Why `pending` is not a root this version searches.
fn root_skip(
    view: &PlayerView,
    pending: &Pending,
    decision: &RootDecision4,
    k: usize,
) -> Option<SearchSkip4> {
    if !matches!(pending, Pending::Priority { .. }) {
        return Some(SearchSkip4::NotPriority);
    }
    if matches!(view.phase, Phase::Beginning | Phase::Ending) {
        return Some(SearchSkip4::Phase);
    }
    let Some(ranking) = decision.ranking.as_deref() else {
        return Some(SearchSkip4::NotPolicy);
    };
    let own = &decision.answer.action;
    if k > 1
        && (view.owed.is_some()
            || is_mana_action(view, own)
            || (matches!(own, PlayerAction::PassPriority)
                && !ranking
                    .iter()
                    .any(|r| is_strategic_alternative(view, own, &r.action))))
    {
        return Some(SearchSkip4::NotPolicy);
    }
    None
}

/// Whether `action` is a mana ability: a basic land type's
/// (`ActivateManaAbility`) or one a card prints (`ActivateAbility` on a
/// mana ability, CR 605.1).
pub(super) fn is_mana_action(view: &PlayerView, action: &PlayerAction) -> bool {
    match action {
        PlayerAction::ActivateManaAbility { .. } => true,
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        } => crate::netplay4::tactics::is_mana_ability_entry(view, *source, *ability_index),
        _ => false,
    }
}

/// Whether `candidate` is a strategic alternative to `own` at `view`:
/// neither `own` itself, nor a mana ability, nor cancelling a cast, nor
/// passing priority in place of a land drop or while mana is floating.
fn is_strategic_alternative(
    view: &PlayerView,
    own: &PlayerAction,
    candidate: &PlayerAction,
) -> bool {
    if candidate == own
        || matches!(candidate, PlayerAction::CancelCast)
        || is_mana_action(view, candidate)
    {
        return false;
    }
    if matches!(candidate, PlayerAction::PassPriority) {
        let floating = view
            .seat(view.seat)
            .is_some_and(|s| s.mana_pool.total() > 0);
        if matches!(own, PlayerAction::PlayLand { .. })
            || (floating && !matches!(own, PlayerAction::CastSpell { .. }))
        {
            return false;
        }
    }
    true
}

/// The policy's own answer, then the next `k - 1` strategic legal answers
/// by its ranking.
fn candidates(view: &PlayerView, decision: &RootDecision4, k: usize) -> Vec<Ranked4> {
    let own = &decision.answer.action;
    let ranking = decision.ranking.as_deref().unwrap_or_default();
    let first = ranking
        .iter()
        .find(|r| r.action == *own)
        .cloned()
        .unwrap_or_else(|| Ranked4 {
            index: usize::from(decision.answer.target.single_idx),
            action: own.clone(),
            logit: f32::NEG_INFINITY,
        });
    let mut out = vec![first];
    out.extend(
        ranking
            .iter()
            .filter(|r| is_strategic_alternative(view, own, &r.action))
            .take(k.saturating_sub(1))
            .cloned(),
    );
    if k > 1
        && matches!(
            own,
            PlayerAction::CastSpell { .. } | PlayerAction::ActivateAbility { .. }
        )
        && !out
            .iter()
            .any(|r| matches!(r.action, PlayerAction::PassPriority))
        && let Some(pass) = ranking.iter().find(|r| {
            matches!(r.action, PlayerAction::PassPriority)
                && is_strategic_alternative(view, own, &r.action)
        })
    {
        if out.len() < k {
            out.push(pass.clone());
        } else {
            out[k - 1] = pass.clone();
        }
    }
    out
}

/// The side utility of a finished game for `seat`, as the evaluator scores
/// it: `1` for a win of its side, `0` for a loss, a draw shared among the
/// sides at the table.
#[must_use]
pub fn terminal_utility(seat: u8, teams: &[Option<u8>], winners: &[u8], seats: usize) -> f32 {
    let surviving: Vec<u8> = if winners.is_empty() {
        (0..seats).filter_map(|s| u8::try_from(s).ok()).collect()
    } else {
        winners.to_vec()
    };
    side_utility(seat, teams, &surviving, winners)
}

/// The policy's answer, finished as play finishes it, and how it came about.
fn unsearched<P: SearchPlayer4>(
    player: &mut P,
    view: &PlayerView,
    decision: RootDecision4,
    outcome: SearchOutcome4,
) -> SearchedAnswer4 {
    player.conclude(view, &decision, &decision.answer.action);
    SearchedAnswer4 {
        answer: decision.answer,
        outcome,
        follow: None,
        script: None,
    }
}

/// Why `pending` is not a root of `config`'s search, by the kind of root it
/// would be: a declaration, or priority with ranked or macro candidates.
fn skip_reason(
    view: &PlayerView,
    pending: &Pending,
    decision: &RootDecision4,
    config: SearchConfig4,
) -> Option<SearchSkip4> {
    if config.combat > 0 && combat::is_declaration(pending) {
        (decision.answer.provenance.source != SourceKind4::Net).then_some(SearchSkip4::NotPolicy)
    } else if config.combat_only {
        Some(SearchSkip4::Roots)
    } else if config.macros {
        macros::root_skip(view, pending, decision)
    } else {
        root_skip(view, pending, decision, config.candidates)
    }
}

/// Answers `seat`'s question `pending` (`view` is its view of it) with a
/// lookahead where this version searches, and as `player` would without one
/// everywhere else.
///
/// `player` must have observed `view`'s table, as the host has it do before
/// every answer. Searched: K candidates (the policy's answer first), P
/// complete rounds of worlds forked from `table` with seeds derived from
/// `search_seed`, the decision and the world, each candidate rolled out at
/// most H decisions of the seat; the [`Selection4`] picks the answer. Every
/// world is the same for every candidate (one seed per world), so
/// candidates are compared on the same deals. With `config.targets() > 0`
/// a candidate that casts or activates is tried at each of its targets
/// (`search/targets.rs`), and the chosen one comes back as
/// [`SearchedAnswer4::follow`] for the target question that follows it at
/// the table ([`follow_decision`]). Up to `config.threads()`
/// rollouts run at once when `player` lends rollout players
/// ([`SearchPlayer4::rollout_player`]); the report is the one the rollouts
/// would give one by one. `player` ends exactly as if it had answered the
/// chosen action itself (only its coverage counts the decision as
/// searched); `table` is not changed.
///
/// # Errors
/// When the player fails to observe in a rollout (its model failed).
pub fn search_decision<P: SearchPlayer4 + Send, T: ForkSource4>(
    player: &mut P,
    table: &T,
    view: &PlayerView,
    pending: &Pending,
    teams: &[Option<u8>],
    config: SearchConfig4,
    search_seed: u64,
) -> anyhow::Result<SearchedAnswer4> {
    let decision = player.decide(view, pending, teams);
    // A declaration of attackers or blockers (v4): searched when the
    // declaration is the policy's own, its others built from it.
    let declaration = config.combat > 0 && combat::is_declaration(pending);
    if let Some(reason) = skip_reason(view, pending, &decision, config) {
        return Ok(unsearched(
            player,
            view,
            decision,
            SearchOutcome4::Skipped(reason),
        ));
    }
    let root = player.snapshot();
    let first_world = world_seed(search_seed, table.decision_seq(), 0);
    let expanded = if declaration {
        combat::playable(
            table,
            view.seat,
            pending,
            &decision.answer.action,
            config.combat,
            first_world,
        )
        .map_err(Stop::Refused)
    } else if config.macros {
        macros::playable(
            table,
            player.view_context(),
            view,
            pending,
            &decision,
            config.candidates,
            first_world,
        )
        .map_err(Stop::Refused)
    } else {
        let ranked = candidates(view, &decision, config.candidates);
        targets::Probe {
            view,
            teams,
            decision: &decision,
            root: &root,
            targets: config.targets,
            seed: first_world,
        }
        .expand(player, table, ranked)
    };
    let candidates = match expanded {
        // A declaration the policy alone may make (or the table refuses), or
        // nothing the table takes beside the policy's answer at priority.
        Ok(candidates) if (declaration || config.macros) && candidates.len() < 2 => {
            let skip = SearchOutcome4::Skipped(if declaration {
                SearchSkip4::CandidateRefused
            } else {
                SearchSkip4::NoAlternative
            });
            return Ok(unsearched(player, view, decision, skip));
        }
        Ok(candidates) => candidates,
        Err(Stop::Refused(refusal)) => {
            player.restore(&root);
            return Ok(unsearched(
                player,
                view,
                decision,
                SearchOutcome4::Refused(refusal),
            ));
        }
        Err(Stop::Failed(error)) => return Err(error),
    };
    let rounds = Rounds {
        view,
        teams,
        decision: &decision,
        candidates: &candidates,
        root: &root,
    }
    .play(player, table, config, search_seed);
    player.restore(&root);
    let played = match rounds {
        Ok(Some(played)) => played,
        Ok(None) => {
            let skip = SearchOutcome4::Skipped(SearchSkip4::CandidateRefused);
            return Ok(unsearched(player, view, decision, skip));
        }
        Err(Stop::Refused(refusal)) => {
            let refused = SearchOutcome4::Refused(refusal);
            return Ok(unsearched(player, view, decision, refused));
        }
        Err(Stop::Failed(error)) => return Err(error),
    };
    Ok(choose_and_answer(
        player,
        view,
        pending,
        decision,
        &candidates,
        played,
        config.selection,
        declaration || config.macros,
    ))
}

/// The candidate the selection picks over the played worlds, concluded as
/// the player's answer and recorded as the search's.
#[allow(
    clippy::too_many_arguments,
    reason = "The searched decision's parts, handed on whole"
)]
fn choose_and_answer<P: SearchPlayer4>(
    player: &mut P,
    view: &PlayerView,
    pending: &Pending,
    decision: RootDecision4,
    candidates: &[Candidate4],
    played: Played,
    selection: Selection4,
    own_prior: bool,
) -> SearchedAnswer4 {
    let Played { mut report, worlds } = played;
    let declaration = combat::is_declaration(pending);
    report.priors = if own_prior {
        // No ranking over declarations or macro candidates: the policy's own
        // answer is its whole prior.
        (0..candidates.len())
            .map(|at| if at == 0 { 1.0 } else { 0.0 })
            .collect()
    } else {
        select::priors(&decision, candidates)
    };
    let choice = select::choose(selection, &worlds, &report.priors);
    report.chosen = choice.chosen;
    report.means = choice.means;
    report.diffs = choice.diffs;
    report.ses = choice.ses;
    report.scores = choice.scores;
    report.changed = report.chosen != 0;
    let chosen = &candidates[report.chosen];
    let pick = &chosen.root;
    player.conclude(view, &decision, &pick.action);
    player.attribute_to_search(pending, decision.answer.provenance.source);
    let own = decision.answer;
    let target = if !report.changed {
        own.target
    } else if declaration {
        // A declaration's target is its mask over the subset or pair head's
        // candidates, which convert4 encodes from the recorded declaration.
        TeacherTarget4::default()
    } else {
        TeacherTarget4 {
            single_idx: u16::try_from(pick.index).unwrap_or(u16::MAX),
            ..TeacherTarget4::default()
        }
    };
    SearchedAnswer4 {
        answer: Answer4 {
            action: pick.action.clone(),
            value: own.value,
            provenance: StepProvenance4 {
                source: SourceKind4::Search,
                temperature: own.provenance.temperature,
                thinking_steps: own.provenance.thinking_steps,
                behavior_logprob: 0.0,
                has_logprob: false,
            },
            target,
        },
        outcome: SearchOutcome4::Searched(Box::new(report)),
        follow: chosen.target.as_ref().map(|target| target.action.clone()),
        script: chosen.script.clone(),
    }
}

/// Answers `seat`'s target question `pending` (`view` is its view of it)
/// with `follow`, the target a search chose with the cast or activation
/// just made ([`SearchedAnswer4::follow`]), and `true`; when `follow` is not
/// an answer the policy ranks there, with the policy's own answer and
/// `false`.
///
/// `player` must have observed `view`'s table. A followed answer is
/// recorded with `SourceKind4::Search` and the policy's menu index of the
/// target, so a search-distilled net learns it; the player ends as if it
/// had chosen the target itself (only its coverage counts the question as
/// searched).
pub fn follow_decision<P: SearchPlayer4>(
    player: &mut P,
    view: &PlayerView,
    pending: &Pending,
    teams: &[Option<u8>],
    follow: &PlayerAction,
) -> (Answer4, bool) {
    let decision = player.decide(view, pending, teams);
    let ranked = matches!(pending, Pending::ChooseTargets { .. })
        .then(|| {
            decision
                .ranking
                .as_deref()
                .unwrap_or_default()
                .iter()
                .find(|ranked| ranked.action == *follow)
                .map(|ranked| ranked.index)
        })
        .flatten();
    let Some(index) = ranked else {
        player.conclude(view, &decision, &decision.answer.action);
        return (decision.answer, false);
    };
    player.conclude(view, &decision, follow);
    player.attribute_to_search(pending, decision.answer.provenance.source);
    let own = decision.answer;
    let answer = Answer4 {
        action: follow.clone(),
        value: own.value,
        provenance: StepProvenance4 {
            source: SourceKind4::Search,
            temperature: own.provenance.temperature,
            thinking_steps: own.provenance.thinking_steps,
            behavior_logprob: 0.0,
            has_logprob: false,
        },
        target: TeacherTarget4 {
            single_idx: u16::try_from(index).unwrap_or(u16::MAX),
            ..TeacherTarget4::default()
        },
    };
    (answer, true)
}

/// How a seat's question was answered while a chosen planned cast or
/// activation stood ([`SearchedAnswer4::script`]).
#[derive(Clone, Debug)]
pub enum ScriptFollow4 {
    /// From the script: its next tap, a colour a tap asks for, or the
    /// action the taps pay for, after which (`ends`) the script is over.
    Answered {
        /// The answer to play, recorded as the search's.
        answer: Answer4,
        /// Whether it was the script's last.
        ends: bool,
    },
    /// The script broke (a tap the table would not take, the moment passed,
    /// the action not offered once paid): it is over and the question is
    /// the player's.
    Over,
    /// Not a question of the script's: the player answers and it stands.
    Other,
}

/// Answers `seat`'s question `pending` (`view` is its view of it) from
/// `script`, the rest of the planned cast or activation a search chose
/// ([`SearchedAnswer4::script`]), where the question is the script's.
///
/// `player` must have observed `view`'s table. A scripted answer is recorded
/// with `SourceKind4::Search` and the policy's menu index of it where the
/// policy ranks it; the player ends as if it had chosen the answer itself
/// (only its coverage counts the question as searched).
pub fn follow_script<P: SearchPlayer4>(
    player: &mut P,
    view: &PlayerView,
    pending: &Pending,
    teams: &[Option<u8>],
    script: &mut Script4,
) -> ScriptFollow4 {
    let step = script.next(view, pending);
    let Some(action) = step.action().cloned() else {
        return if step.ends() {
            ScriptFollow4::Over
        } else {
            ScriptFollow4::Other
        };
    };
    let decision = player.decide(view, pending, teams);
    player.conclude(view, &decision, &action);
    player.attribute_to_search(pending, decision.answer.provenance.source);
    let index = decision
        .ranking
        .as_deref()
        .unwrap_or_default()
        .iter()
        .find(|ranked| ranked.action == action)
        .map_or(u16::MAX, |ranked| {
            u16::try_from(ranked.index).unwrap_or(u16::MAX)
        });
    let own = decision.answer;
    ScriptFollow4::Answered {
        answer: Answer4 {
            action,
            value: own.value,
            provenance: StepProvenance4 {
                source: SourceKind4::Search,
                temperature: own.provenance.temperature,
                thinking_steps: own.provenance.thinking_steps,
                behavior_logprob: 0.0,
                has_logprob: false,
            },
            target: TeacherTarget4 {
                single_idx: index,
                ..TeacherTarget4::default()
            },
        },
        ends: step.ends(),
    }
}

mod rollout;
use rollout::{Played, Rounds, Stop};
mod combat;
mod macros;
mod select;
mod targets;

#[cfg(test)]
mod foresight_tests;
#[cfg(test)]
mod tests;
