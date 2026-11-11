//! What the house brings to a search through copies of the game
//! (`AIProfile::FORESIGHT`): which answers are worth trying, what a position
//! is worth once a copy has been played on, and which of the tried answers
//! to give.
//!
//! The copies themselves are the host's (`baylee-gamehost`'s
//! `session/foresight.rs`, under its `simulation` feature): this crate never
//! holds an engine. Everything here reads a [`PlayerView`] and the question
//! asked, as every other answer of the house does, so a level that is fair
//! at a table is fair in its search: the candidates come from the offer the
//! seat was made, and a leaf is valued from the view the seat would be shown
//! of the copy.

pub mod prior;

use crate::{AIProfile, HeuristicAgent};
use baylee_core::ids::{Defender, ObjectId, PlayerId};
use baylee_core::types::TypeSet;
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::{Phase, PlayerView};

/// The life total past which more life is priced as nothing.
const LIFE_CAP: i32 = 30;

/// A seat's own answer at a root and the others worth trying beside it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Candidates {
    /// The answers, the level's own first; never fewer than one.
    pub answers: Vec<PlayerAction>,
}

/// What became of one candidate in one world.
pub type Outcome = Option<f64>;

impl HeuristicAgent {
    /// The seed this agent's near-equal choices are keyed by.
    #[must_use]
    pub const fn seed(&self) -> u64 {
        self.seed
    }

    /// The house player a search seats in every chair of a simulated game
    /// (`seed` for its near-equal choices): this agent's sides and sight on
    /// the heuristics of the strongest fair level, whatever level searches,
    /// with a memory and a fallback count of its own, so that nothing it
    /// does in a copy of the game reaches the agent at the table.
    ///
    /// One model for every level and every chair: a search asks what a
    /// good house player would do in each seat, never what the player who
    /// sits there does. A level's own heuristics give its own answer, which
    /// stands unless the search finds better; its budget says how hard the
    /// search looks.
    #[must_use]
    pub fn simulated(&self, seed: u64) -> Self {
        let profile = AIProfile {
            sight: self.profile.sight,
            ..AIProfile::EXPERT
        };
        Self {
            profile,
            teams: std::sync::Arc::clone(&self.teams),
            seed,
            strategy: crate::intelligence::Strategy::default(),
            fallbacks: std::sync::Arc::default(),
            stall: crate::StallMemory::default(),
        }
    }

    /// Forgets the last action this agent gave ([`crate::stall`]): the host
    /// played another answer in its place, so the position coming back says
    /// nothing about this one.
    pub fn forget_given(&self) {
        self.stall.clear();
    }

    /// A hash of everything `view` shows its seat, the same for the same
    /// position whenever it is asked: what a search keys its worlds by.
    #[must_use]
    pub fn position(view: &PlayerView) -> u64 {
        crate::stall::position(view)
    }

    /// The answers worth trying at `pending`, `own` (this level's answer)
    /// first, at most `most` of them; one answer alone where the question
    /// is not one a search takes.
    ///
    /// A search takes three questions. *Priority* with nothing begun, in the
    /// seat's own main phase or where it means to act or has an opponent's
    /// spell to answer (`settled`), no mana floating and no cast in
    /// progress: passing, each land and each spell and ability the offer
    /// lists, one per card. *Attackers*: the level's declaration, none, all,
    /// and the declaration with one creature more or one fewer. *Blockers*:
    /// the level's blocks, none, and the blocks with one more or one fewer.
    /// Every candidate is checked against the question as an answer is
    /// (`Pending::answer_fault`).
    #[must_use]
    pub fn foresight_candidates(
        &self,
        view: &PlayerView,
        pending: &Pending,
        own: &PlayerAction,
        most: usize,
    ) -> Candidates {
        let mut answers = vec![own.clone()];
        let others = match pending {
            Pending::Priority { player, legal } if settled(view, *player, own) => {
                priority_candidates(view, legal)
            }
            Pending::ChooseAttackers {
                attackers,
                defenders,
                ..
            } => attack_candidates(attackers, defenders, own),
            Pending::ChooseBlockers { blockers, .. } => block_candidates(blockers, own),
            _ => Vec::new(),
        };
        for answer in others {
            if answers.len() >= most {
                break;
            }
            if !answers.contains(&answer) && pending.answer_fault(&answer).is_none() {
                answers.push(answer);
            }
        }
        Candidates { answers }
    }
}

/// Which of the tried answers to give: the first (the level's own) unless
/// another's mean over the worlds both finished beats it by more than
/// `margin`; of those, the best mean, the earlier on a tie.
///
/// `outcomes[candidate][world]` is `None` where the engine refused the
/// candidate or the world could not be made. A candidate is compared with
/// the level's own answer over the worlds both have, so the two are set
/// against the same deals.
#[must_use]
pub fn choose(outcomes: &[Vec<Outcome>], margin: f64) -> usize {
    let Some(own) = outcomes.first() else {
        return 0;
    };
    let mut best = (0, margin);
    for (at, tried) in outcomes.iter().enumerate().skip(1) {
        let pairs: Vec<(f64, f64)> = own
            .iter()
            .zip(tried)
            .filter_map(|(a, b)| a.zip(*b))
            .collect();
        if pairs.is_empty() {
            continue;
        }
        #[allow(clippy::cast_precision_loss)] // a handful of worlds
        let gain = pairs.iter().map(|(a, b)| b - a).sum::<f64>() / pairs.len() as f64;
        if gain > best.1 {
            best = (at, gain);
        }
    }
    best.0
}

/// Whether a search takes `player`'s priority here.
///
/// In their own main phase with nothing on the stack; outside it, where a
/// seat is asked at every step, only where the level means to act or an
/// opponent's spell or ability waits on the stack for an answer. Either
/// way with nothing begun: no cast or payment in progress and no mana
/// floating, which are the middle of an answer already given.
fn settled(view: &PlayerView, player: PlayerId, own: &PlayerAction) -> bool {
    let own_main = view.active == player
        && matches!(view.phase, Phase::FirstMain | Phase::SecondMain)
        && view.stack.is_empty();
    let window = !matches!(own, PlayerAction::PassPriority)
        || view
            .stack
            .last()
            .is_some_and(|top| top.controller != player);
    (own_main || window)
        && view.decision_player.is_none_or(|asked| asked == player)
        && view.casting.is_none()
        && view.announced_cast.is_none()
        && view.owed.is_none()
        && view
            .seat(player)
            .is_some_and(|seat| seat.mana_pool.total() == 0)
}

/// Passing, each land, each spell and each ability of the offer, one per
/// card: two copies of a card in hand are one candidate.
fn priority_candidates(
    view: &PlayerView,
    legal: &baylee_engine::choice::LegalActions,
) -> Vec<PlayerAction> {
    let card_of = |id: ObjectId| {
        view.hand
            .iter()
            .find(|card| card.id == id)
            .map(|card| (card.card.index, card.mana_value))
    };
    let mut out = Vec::new();
    if legal.can_pass {
        out.push(PlayerAction::PassPriority);
    }
    let mut seen = Vec::new();
    let mut spells: Vec<(u32, ObjectId)> = Vec::new();
    for &id in &legal.castable {
        let (card, mana_value) = card_of(id).map_or((None, 0), |(card, mv)| (Some(card), mv));
        if card.is_some() && seen.contains(&card) {
            continue;
        }
        seen.push(card);
        spells.push((mana_value, id));
    }
    // The dearest first: where the budget is short, the plays that use the
    // turn's mana are the ones compared.
    spells.sort_by_key(|&(mana_value, id)| (std::cmp::Reverse(mana_value), id));
    out.extend(
        spells
            .into_iter()
            .map(|(_, card)| PlayerAction::CastSpell { card }),
    );
    let mut lands = Vec::new();
    for &id in &legal.lands {
        let card = card_of(id).map(|(card, _)| card);
        if card.is_some() && lands.contains(&card) {
            continue;
        }
        lands.push(card);
        out.push(PlayerAction::PlayLand { card: id });
    }
    let mana: &[ObjectId] = &legal.mana_abilities;
    for &(source, ability_index) in &legal.abilities {
        let is_land = view
            .battlefield
            .iter()
            .find(|object| object.id == source)
            .is_some_and(|object| object.types.contains(TypeSet::LAND));
        // A land's listed ability beside its mana ability is rarely a play
        // of its own; the level's own answer still names one where it is.
        if is_land && mana.contains(&source) {
            continue;
        }
        out.push(PlayerAction::ActivateAbility {
            source,
            ability_index,
        });
    }
    out
}

/// None, all, and the level's declaration with one attacker fewer or more.
fn attack_candidates(
    attackers: &[ObjectId],
    defenders: &[Defender],
    own: &PlayerAction,
) -> Vec<PlayerAction> {
    let PlayerAction::DeclareAttackers { attackers: chosen } = own else {
        return Vec::new();
    };
    let Some(&target) = chosen
        .first()
        .map(|(_, defender)| defender)
        .or_else(|| defenders.first())
    else {
        return Vec::new();
    };
    let declare =
        |attackers: Vec<(ObjectId, Defender)>| PlayerAction::DeclareAttackers { attackers };
    let mut out = vec![
        declare(attackers.iter().map(|&id| (id, target)).collect()),
        declare(Vec::new()),
    ];
    for &id in attackers {
        if !chosen.iter().any(|(attacker, _)| *attacker == id) {
            let mut more = chosen.clone();
            more.push((id, target));
            out.push(declare(more));
        }
    }
    for at in 0..chosen.len() {
        let mut fewer = chosen.clone();
        fewer.remove(at);
        out.push(declare(fewer));
    }
    out
}

/// No block, and the level's blocks with one fewer or one more.
fn block_candidates(
    options: &[baylee_engine::choice::BlockOption],
    own: &PlayerAction,
) -> Vec<PlayerAction> {
    let PlayerAction::DeclareBlockers { blockers: chosen } = own else {
        return Vec::new();
    };
    let declare = |blockers: Vec<(ObjectId, ObjectId)>| PlayerAction::DeclareBlockers { blockers };
    let mut out = vec![declare(Vec::new())];
    for option in options {
        if chosen.iter().any(|(blocker, _)| *blocker == option.blocker) {
            continue;
        }
        for &attacker in &option.attackers {
            let mut more = chosen.clone();
            more.push((option.blocker, attacker));
            out.push(declare(more));
        }
    }
    for at in 0..chosen.len() {
        let mut fewer = chosen.clone();
        fewer.remove(at);
        out.push(declare(fewer));
    }
    out
}

/// How many numbers [`HeuristicAgent::foresight_features`] gives.
pub const FEATURES: usize = 14;

/// What each feature counts, in order.
pub const FEATURE_NAMES: [&str; FEATURES] = [
    "life_low",
    "life_mid",
    "life_high",
    "poison",
    "hand",
    "lands",
    "creatures",
    "power",
    "toughness",
    "keywords",
    "others",
    "mana_value",
    "active",
    "race",
];

impl HeuristicAgent {
    /// The position `view` shows as numbers a leaf value is fitted on: each
    /// one this seat's side less the other side, counted from what the view
    /// shows and nothing else.
    ///
    /// Life in three bands (the first seven points, the next seven, and up
    /// to [`LIFE_CAP`]), poison, cards in hand, lands, creatures with their
    /// power, toughness and keyword count, other permanents, the mana value
    /// of everything that is not a land, and whose turn it is.
    #[must_use]
    pub fn foresight_features(&self, view: &PlayerView) -> [f64; FEATURES] {
        let me = view.seat;
        let mut f = [0.0; FEATURES];
        let sign = |seat: PlayerId| if self.hostile(seat, me) { -1.0 } else { 1.0 };
        for seat in view.seats.iter().filter(|seat| !seat.has_lost()) {
            let s = sign(seat.player);
            let life = seat.life.clamp(0, LIFE_CAP);
            f[0] += s * f64::from(life.min(7));
            f[1] += s * f64::from((life - 7).clamp(0, 7));
            f[2] += s * f64::from((life - 14).max(0));
            f[3] -= s * f64::from(seat.poison);
            f[4] += s * f64::from(seat.hand_count);
        }
        for object in &view.battlefield {
            let s = sign(object.controller);
            let creature = object.types.contains(TypeSet::CREATURE);
            if object.types.contains(TypeSet::LAND) && !creature {
                f[5] += s;
                continue;
            }
            if creature {
                f[6] += s;
                f[7] += s * f64::from(object.power.unwrap_or(0).max(0));
                f[8] += s * f64::from(object.toughness.unwrap_or(0).max(0));
                f[9] += s * f64::from(object.keywords.count_ones());
            } else {
                f[10] += s;
            }
            f[11] += s * f64::from(object.mana_value);
        }
        f[12] = sign(view.active);
        // The race: how much of the other side's life each side's creatures
        // take in one attack, capped at twice over.
        let (mut power, mut life) = ([0.0_f64; 2], [0.0_f64; 2]);
        for seat in view.seats.iter().filter(|seat| !seat.has_lost()) {
            life[usize::from(self.hostile(seat.player, me))] += f64::from(seat.life.max(1));
        }
        for object in &view.battlefield {
            if object.types.contains(TypeSet::CREATURE) {
                power[usize::from(self.hostile(object.controller, me))] +=
                    f64::from(object.power.unwrap_or(0).max(0));
            }
        }
        if life[0] > 0.0 && life[1] > 0.0 {
            f[13] = (power[0] / life[1]).min(2.0) - (power[1] / life[0]).min(2.0);
        }
        f
    }
}

/// The leaf value's weights, one per feature ([`FEATURE_NAMES`]): the value
/// of a position is the logistic of their sum over
/// [`HeuristicAgent::foresight_features`].
///
/// Fitted by `fit_leaf` (`baylee-train`) on 209,236 positions of 4,096
/// games the house played against itself on the train split (11.11.2026):
/// log loss 0.558 on a fifth of those games held out whole (0.559 for
/// weights fitted without them; 0.576 on another fifth, so read such a
/// figure against others of the same split only),
/// against 0.693 for calling every position even.
///
/// These fourteen counts stand after a round that tried to better them
/// (`docs/proposals/house-ai-strength.md` §6a): weights fitted on the
/// searching house's own games, 49 features, weights that change with the
/// turn and boosted trees call a game better, down to 0.526, and none of
/// them wins the search a game more.
pub const LEAF: [f64; FEATURES] = [
    0.11003, // life_low
    0.13678, // life_mid
    0.02447, // life_high
    0.00000, // poison
    0.08784, // hand
    0.00000, // lands
    0.09487, // creatures
    0.12111, // power
    0.06628, // toughness
    0.14149, // keywords
    0.00000, // others
    0.01382, // mana_value
    0.15621, // active
    0.41514, // race
];

impl HeuristicAgent {
    /// What the position `view` shows is worth to its seat's side, between
    /// 0 (lost) and 1 (won): the fitted value ([`LEAF`]) of what the view
    /// shows. A side with nobody left has lost.
    #[must_use]
    pub fn foresight_value(&self, view: &PlayerView) -> f64 {
        let me = view.seat;
        let alive = |hostile: bool| {
            view.seats
                .iter()
                .any(|seat| !seat.has_lost() && self.hostile(seat.player, me) == hostile)
        };
        if !alive(false) {
            return 0.0;
        }
        if !alive(true) {
            return 1.0;
        }
        let z: f64 = self
            .foresight_features(view)
            .iter()
            .zip(LEAF)
            .map(|(x, w)| x * w)
            .sum();
        1.0 / (1.0 + (-z).exp())
    }
}
