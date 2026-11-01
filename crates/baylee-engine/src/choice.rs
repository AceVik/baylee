//! The choice contract: the only way the game ever advances.
//!
//! The engine never asks open questions — every [`ChoiceRequest`] carries
//! the complete set of legal answers, precomputed. Humans, AIs, and network
//! clients all answer through the same [`PlayerAction`] type.
//!
//! # Automation
//!
//! A seat can tell the engine *not* to ask it about things (see
//! [`Automation`]). This lives in the engine rather than in a client for
//! two reasons. It has to see the same board the rules do — "stop when
//! that trigger reaches the top of the stack" is a question about the
//! stack, not about a UI. And it has to be journaled: an auto-pass that a
//! client invented would replay as a different game, whereas a
//! [`PlayerAction::SetPriorityHold`] in the journal replays exactly.

use crate::win::GameResult;
use baylee_core::ids::{AbilityRef, ObjectId, PlayerId};

mod answers;
mod arrange;
mod automation;
mod damage;
mod granted;
mod mana;
mod prompts;
pub use arrange::{
    ArrangePile, ArrangePlace, ArrangePrompt, arrangement_fault, default_arrangement,
};
pub use damage::{DamageChoiceId, DamageEffectKind, DamageEffectOption, DamagePartView};
pub use granted::{GrantedActionKind, GrantedActionOffer};
pub use mana::{ManaAbilityChoice, ManaChoiceId};

pub use answers::{AnswerFault, timeout_answer};
pub use automation::{PriorityHold, SeatAutomation, StandingAnswer};
pub use prompts::{
    CastModeDesc, CastModeKind, ChoicePrompt, NumberPrompt, TargetPrompt, YesNoPrompt,
};

/// One creature that may block, and the attackers it may be assigned to.
///
/// It blocks one of them or none (CR 509.1a: "for each of the chosen
/// creatures, the defending player chooses one creature for it to block"),
/// so a declaration names each blocker at most once. Nothing in the pool
/// lets a creature block more than one attacker; the day something does,
/// the number goes here.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct BlockOption {
    /// The creature that may block.
    pub blocker: ObjectId,
    /// The attackers this creature may legally block.
    pub attackers: Vec<ObjectId>,
}

/// A creature that may block more than one attacker, and how many: CR
/// 509.1a gives each blocker one, and an effect raises that ("can block an
/// additional creature each combat", "can block any number of creatures").
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct BlockCapacity {
    /// The creature.
    pub blocker: ObjectId,
    /// How many attackers it may block, never fewer than two; `None` for
    /// any number.
    pub most: Option<u8>,
}

/// A creature that may attack only some of what the attack question
/// offers, and what those are: a restriction about the pair (CR 508.1c,
/// "can't attack unless defending player controls an Island").
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct AttackLimit {
    /// The creature.
    pub creature: ObjectId,
    /// What it may attack, a part of the question's `defenders` and never
    /// empty: a creature that may attack nothing is not offered.
    pub defenders: Vec<baylee_core::ids::Defender>,
}

/// How many creatures may block one attacker, where that is not "any
/// number, one or more" (CR 509.1b, the restrictions half).
///
/// The bound holds only for an attacker that is blocked at all: the
/// creatures declared to block it number **zero, or between
/// `min_blockers` and `max_blockers` inclusive**. Menace is `2..=u32::MAX`
/// (CR 702.111b, "can't be blocked except by two or more creatures"), and
/// `min_blockers: 2` never means that the attacker must be blocked: an
/// attacker nobody blocks keeps every such restriction.
///
/// Listed in [`Pending::ChooseBlockers::bounds`] for the attackers that have
/// one; an attacker not listed is bounded by nothing but the pairings.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub struct AttackerBound {
    /// The attacking creature.
    pub attacker: ObjectId,
    /// If blocked, by at least this many creatures.
    pub min_blockers: u32,
    /// If blocked, by at most this many; `u32::MAX` for no bound.
    pub max_blockers: u32,
}

/// How many block requirements ask one creature to block one attacker
/// (CR 509.1c): each lure on the attacker ("all creatures able to block it
/// do so") and each "blocks each attacking creature if able" on the blocker.
/// A declaration obeys the sum over its pairs, and the engine refuses one
/// that obeys fewer than [`Pending::ChooseBlockers::obeying`] does.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub struct BlockDemand {
    /// The creature asked to block.
    pub blocker: ObjectId,
    /// The attacker it is asked to block.
    pub attacker: ObjectId,
    /// How many requirements ask it, never 0.
    pub count: u32,
}

/// What each option of a [`Pending::ChooseCards`] weighs in a
/// [`CardTotal`], as a UI hint: the weights themselves ride with the total.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub enum Measure {
    /// Each card's power, as it is now (crew, CR 702.122a; saddle,
    /// CR 702.171a).
    Power,
    /// Each card's toughness.
    Toughness,
    /// Each card's mana value.
    ManaValue,
}

/// A bound on the sum of the chosen cards' weights: crew's "tap any number
/// of other untapped creatures you control with total power N or greater"
/// (CR 702.122a) is `at_least: Some(N)` over each creature's power.
///
/// The weights ride in the question rather than being read off the view,
/// so the check is the question's own: `weights[i]` is what `options[i]`
/// counts for, as the engine counts it. A power below zero counts below
/// zero (CR 107.1b: "if a calculation or comparison needs to use a
/// negative value, it does so"), which a client summing the view's powers
/// and flooring them at zero would get wrong.
#[derive(Clone, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
pub struct CardTotal {
    /// What is summed (UI hint).
    pub of: Measure,
    /// Each option's weight, in the order of the question's `options`.
    pub weights: Vec<i32>,
    /// The least the chosen cards may add up to, inclusive.
    pub at_least: Option<i32>,
    /// The most the chosen cards may add up to, inclusive.
    pub at_most: Option<i32>,
}

impl CardTotal {
    /// What `chosen` adds up to; a card not among `options` adds nothing
    /// (the membership check is the question's, not the total's).
    #[must_use]
    pub fn sum(&self, options: &[ObjectId], chosen: &[ObjectId]) -> i64 {
        chosen
            .iter()
            .filter_map(|c| options.iter().position(|o| o == c))
            .filter_map(|i| self.weights.get(i))
            .map(|w| i64::from(*w))
            .sum()
    }

    /// Whether `chosen` keeps both bounds.
    #[must_use]
    pub fn fault(&self, options: &[ObjectId], chosen: &[ObjectId]) -> Option<AnswerFault> {
        let sum = self.sum(options, chosen);
        if self.at_least.is_some_and(|least| sum < i64::from(least)) {
            return Some(AnswerFault::TotalTooLow);
        }
        if self.at_most.is_some_and(|most| sum > i64::from(most)) {
            return Some(AnswerFault::TotalTooHigh);
        }
        None
    }

    /// Whether some choice of between `min` and `max` of the weights can
    /// keep each bound, asked of each bound on its own: the most `k` cards
    /// can reach is the sum of the `k` heaviest, and the least the sum of
    /// the `k` lightest. For a total with both bounds that is necessary and
    /// not sufficient; nothing in the pool asks one.
    fn reachable(&self, min: u8, max: u8) -> bool {
        let mut sorted: Vec<i64> = self.weights.iter().map(|w| i64::from(*w)).collect();
        sorted.sort_unstable();
        let counts = usize::from(min)..=usize::from(max).min(sorted.len());
        let heaviest = |k: usize| sorted.iter().rev().take(k).sum::<i64>();
        let lightest = |k: usize| sorted.iter().take(k).sum::<i64>();
        let least_ok = self
            .at_least
            .is_none_or(|least| counts.clone().any(|k| heaviest(k) >= i64::from(least)));
        let most_ok = self
            .at_most
            .is_none_or(|most| counts.clone().any(|k| lightest(k) <= i64::from(most)));
        least_ok && most_ok
    }
}

/// The serde default of [`Pending::Mulligan::can_take`]: every question
/// sent before the field existed allowed a mulligan.
const fn yes() -> bool {
    true
}

/// What the game is currently waiting for.
#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub enum Pending {
    /// A resolving instruction requires another mana ability to be activated.
    ChooseManaAbility {
        /// Player whose lands and resources are used.
        player: PlayerId,
        /// Exact instruction step.
        choice: ManaChoiceId,
        /// Legal activations; declining is not an answer.
        options: Vec<ManaAbilityChoice>,
    },
    /// Choose an exact incarnation as a damage source (CR 609.7a).
    ChooseDamageSource {
        /// Player creating the source-dependent effect.
        player: PlayerId,
        /// Identity of this exact offered choice.
        choice: baylee_core::ids::SourceChoiceId,
        /// Legal sources, including separately remembered incarnations.
        options: Vec<baylee_core::ids::DamageSourceRef>,
    },
    /// Choose which competing prevention or replacement modifies damage
    /// next (CR 616.1). This is not priority or a targeting decision.
    ChooseDamageEffect {
        /// Affected player or affected permanent's controller.
        player: PlayerId,
        /// Identity of this exact decision.
        choice: DamageChoiceId,
        /// Damage the offered effects can modify.
        damage: Vec<DamagePartView>,
        /// Legal effects, each with its own stable identity.
        options: Vec<DamageEffectOption>,
    },
    /// Divide finite prevention among simultaneous damage (CR 615.7).
    AllocatePrevention {
        /// Affected player or affected permanent's controller.
        player: PlayerId,
        /// Identity of this exact decision.
        choice: DamageChoiceId,
        /// The effect already selected to apply.
        effect: DamageEffectOption,
        /// Damage shares that can receive prevention.
        damage: Vec<DamagePartView>,
        /// Exact prevention to distribute; each part permits 0..=amount.
        total: u32,
    },
    /// A mulligan decision (London; first is free per house rules).
    Mulligan {
        /// Deciding player.
        player: PlayerId,
        /// Mulligans already taken by this player.
        taken: u8,
        /// Whether the next mulligan is free (house rule, first only).
        next_is_free: bool,
        /// Whether a mulligan may be taken at all. Not once the hand would
        /// open with no cards (CR 103.5: each mulligan puts one more card on
        /// the bottom, and a free one none), because the bottom question
        /// would then ask for more cards than the hand holds.
        /// `PlayerAction::MulliganTake` is refused while this is `false`.
        #[serde(default = "yes")]
        can_take: bool,
    },
    /// A player must choose cards to put on the bottom after keeping.
    MulliganBottom {
        /// Deciding player.
        player: PlayerId,
        /// How many cards to bottom.
        count: u8,
    },
    /// A player has priority.
    Priority {
        /// The player holding priority.
        player: PlayerId,
        /// Everything they may legally do right now.
        legal: Box<LegalActions>,
    },
    /// Declare attackers (combat).
    ChooseAttackers {
        /// Attacking player.
        player: PlayerId,
        /// Which creatures may attack (CR 508.1a): untapped, unsick, no
        /// defender, and past whatever the card itself demands.
        ///
        /// Enumerated here for the same reason as the defenders below: a
        /// client that filtered its own board by "untapped and not sick"
        /// would offer Wall of Omens as an attacker and would miss every
        /// "can't attack unless…" a card prints.
        attackers: Vec<ObjectId>,
        /// What may be attacked: each surviving opponent, plus every
        /// planeswalker they control. Carried in the request because a
        /// client cannot derive "which permanents are planeswalkers I may
        /// attack" from the view without re-implementing CR 506.2.
        defenders: Vec<baylee_core::ids::Defender>,
        /// Individually eligible creatures that attack if able (CR 508.1d).
        /// `minimum_required` gives the maximum simultaneously satisfiable
        /// count after applying group restrictions.
        #[serde(default)]
        required: Vec<ObjectId>,
        /// Offered creatures that may attack only as the sole attacker.
        #[serde(default)]
        alone: Vec<ObjectId>,
        /// Minimum number of required candidates to declare; absent means all of them.
        #[serde(default)]
        minimum_required: Option<u32>,
        /// The offered creatures that may attack only some of `defenders`.
        /// A creature not named here may attack any of them.
        #[serde(default)]
        limits: Vec<AttackLimit>,
    },
    /// Declare blockers (combat).
    ChooseBlockers {
        /// Defending player.
        player: PlayerId,
        /// Attacking player.
        attacker: PlayerId,
        /// Which creatures may block which attackers. Evasion is a pairing
        /// question — flying, menace, protection, "can't be blocked by" —
        /// so the offer is a pairing, not two flat lists.
        blockers: Vec<BlockOption>,
        /// The offered creatures that may block more than one attacker. A
        /// creature not named here blocks one at most (CR 509.1a).
        #[serde(default)]
        capacity: Vec<BlockCapacity>,
        /// One legal declaration that obeys as many block requirements as
        /// the engine requires ("all creatures able to block enchanted
        /// creature do so", "it blocks each attacking creature if able";
        /// CR 509.1c), as `(blocker, attacker)` pairs out of `blockers`.
        /// Empty when no requirement is in force. Not a list of blocks that
        /// must be made: any declaration obeying as many requirements is as
        /// legal, and one obeying fewer is refused. It is the answer a seat
        /// that does not choose gets.
        #[serde(default)]
        obeying: Vec<(ObjectId, ObjectId)>,
        /// The pairs out of `blockers` a requirement asks for, with how many
        /// ask, which is how `obeying` and an answer are both counted.
        /// Empty when no requirement is in force. A blocker its controller
        /// must pay for is in none (CR 509.1c: not required to pay).
        #[serde(default)]
        demands: Vec<BlockDemand>,
        /// The attackers whose blockers are counted, and the count each
        /// allows (menace, CR 702.111b). A restriction on the whole
        /// declaration rather than on a pair (CR 509.1b), which is why it is
        /// not in the pairings: one creature may block a menace attacker,
        /// and one creature alone may not.
        #[serde(default)]
        bounds: Vec<AttackerBound>,
    },
    /// Discard down to maximum hand size (cleanup).
    DiscardChoice {
        /// Discarding player.
        player: PlayerId,
        /// How many cards to discard.
        count: u8,
    },
    /// Legend rule: choose which copy to keep (CR 704.5j).
    LegendChoice {
        /// Choosing player.
        player: PlayerId,
        /// The duplicated permanents (choose exactly one to keep).
        options: Vec<ObjectId>,
    },
    /// Choose cards from a server-side filtered set (searches, browses).
    ChooseCards {
        /// Choosing player.
        player: PlayerId,
        /// The legal options (already filtered — e.g. only Islands/Swamps).
        options: Vec<ObjectId>,
        /// Minimum to choose.
        min: u8,
        /// Maximum to choose.
        max: u8,
        /// Why (UI hint).
        prompt: ChoicePrompt,
        /// A bound on what the chosen cards add up to, where the question
        /// has one: crew's total power (CR 702.122a). `None` for every
        /// question that counts only cards.
        #[serde(default)]
        total: Option<CardTotal>,
    },
    /// Choose targets for a spell/ability being cast/activated.
    ChooseTargets {
        /// Choosing player.
        player: PlayerId,
        /// The legal object targets.
        options: Vec<ObjectId>,
        /// The legal *player* targets, for "any target" (CR 115.4).
        ///
        /// Empty for every spec that targets objects alone, which is why
        /// [`PlayerAction::ChooseObjects`] still answers this prompt: the
        /// two lists are one choice, and `min`/`max` count across both.
        player_options: Vec<PlayerId>,
        /// Minimum to choose (0 for "up to" targets).
        min: u32,
        /// Maximum to choose.
        max: u32,
        /// Why (UI hint).
        reason: TargetPrompt,
    },
    /// Choose one of the offered subtypes, on entry or during resolution.
    ChooseSubtype {
        /// Choosing player.
        player: PlayerId,
        /// Eligible subtypes, such as the five basic land types.
        options: Vec<baylee_core::ids::SubtypeId>,
    },
    /// Choose a card name ("as this enters, choose a card name" — Pithing
    /// Needle), answered with [`PlayerAction::ChooseCardName`].
    ///
    /// No list rides with the question. Any card's name may be chosen, of
    /// any of its faces (CR 201.4, 201.4b–f), and the pool is the whole of
    /// what a game can mean by one (a token's name only counts when a card
    /// has it too), so the options are the card pool itself: a few thousand
    /// faces every client and agent already has, which a list here would
    /// send again with every frame that asks. The engine checks the answer
    /// against the pool it plays with.
    ChooseCardName {
        /// Choosing player.
        player: PlayerId,
        /// Required printed card types; empty permits any card name.
        types: baylee_core::types::TypeSet,
        /// Printed card types a named face must not have; empty excludes nothing.
        excluded_types: baylee_core::types::TypeSet,
    },
    /// Choose one card type as a permanent enters (CR 205.2a).
    ChooseCardType {
        /// Choosing player.
        player: PlayerId,
        /// Every legal card type, in stable rules order.
        options: Vec<baylee_core::types::TypeSet>,
    },
    /// Choose a mana color (choice-restricted mana abilities).
    ChooseColor {
        /// Choosing player.
        player: PlayerId,
        /// Allowed colors.
        options: Vec<baylee_core::mana::ManaColor>,
    },
    /// A yes/no decision (shockland life payment, optional effects).
    YesNo {
        /// Deciding player.
        player: PlayerId,
        /// What is being decided.
        prompt: YesNoPrompt,
        /// The ability asking, when one can be named.
        ///
        /// This is the key a standing answer is stored under, and the
        /// label a client puts on the prompt. `None` for questions that
        /// come from the game itself rather than from a card (a draw
        /// offer, a mulligan-adjacent choice).
        source: Option<AbilityRef>,
    },
    /// Choose how to cast a spell (normal / alternative cost / mode).
    ChooseCastMode {
        /// Casting player.
        player: PlayerId,
        /// What the question is about: the card being cast or played, or the
        /// permanent whose modal trigger is picking a mode.
        ///
        /// A handle and not a name, for the reason the engine carries no card
        /// text at all — the same rule that makes an ability an
        /// [`baylee_core::ids::AbilityRef`]. Two land faces of a pathway
        /// (CR 712.12) differ in nothing a [`CastModeDesc`] carries: same
        /// kind, same empty cost, different printed name. Without this the
        /// client draws two identical buttons and the choice is blind.
        object: ObjectId,
        /// The legal cast options.
        options: Vec<CastModeDesc>,
    },
    /// Choose a number: the value of X, how many times to pay a replicate
    /// cost, or a share of a division. `reason` says which.
    ChooseNumber {
        /// Choosing player.
        player: PlayerId,
        /// Minimum value.
        min: u32,
        /// Maximum value.
        max: u32,
        /// What the number is (UI hint). A frame without it is asking for X.
        #[serde(default)]
        reason: NumberPrompt,
    },
    /// Choose a target player.
    ChoosePlayer {
        /// Choosing player.
        player: PlayerId,
        /// Candidate players.
        options: Vec<PlayerId>,
    },
    /// Put cards into places, each place in an order: "the rest on the
    /// bottom in any order", "put them back in any order". Answered with
    /// [`PlayerAction::Arrange`]: every card exactly once, pile by pile,
    /// library piles listed top to bottom.
    Arrange {
        /// Choosing player.
        player: PlayerId,
        /// The cards being arranged, in the order they were looked at (top
        /// of the library first).
        cards: Vec<ObjectId>,
        /// Where they may go; answered pile by pile, in this order.
        piles: Vec<ArrangePile>,
        /// Why the question is asked (UI hint).
        prompt: ArrangePrompt,
    },
    /// Choose one of the piles another player separated (Fact or Fiction):
    /// the chosen pile goes into the hand and the others into the
    /// graveyard. Answered with [`PlayerAction::ChooseMode`], the pile's
    /// position; a pile may be empty and is still a pile to choose.
    ChoosePile {
        /// Choosing player.
        player: PlayerId,
        /// The piles, in the order the separation gave them.
        piles: Vec<Vec<ObjectId>>,
        /// `None`: the chosen pile goes into the hand (Fact or Fiction).
        /// `Some(attacker)`: a label for that attacking creature, which the
        /// chosen pile's creatures (and fliers) alone may block this combat
        /// (Raging River: "left" is the first pile, "right" the second).
        #[serde(default, skip_serializing_if = "Option::is_none")]
        label: Option<ObjectId>,
    },
    /// The game is over.
    GameOver(GameResult),
}
impl Pending {
    /// The seat this question is asked of; `None` once the game is over.
    ///
    /// During the opening mulligans several seats are asked at once, and
    /// this names only the one [`crate::engine::Engine::pending`] shows; ask
    /// [`crate::engine::Engine::awaited`] for all of them.
    #[must_use]
    pub const fn asked(&self) -> Option<PlayerId> {
        match self {
            Self::Mulligan { player, .. }
            | Self::ChooseManaAbility { player, .. }
            | Self::ChooseDamageSource { player, .. }
            | Self::ChooseDamageEffect { player, .. }
            | Self::AllocatePrevention { player, .. }
            | Self::MulliganBottom { player, .. }
            | Self::Priority { player, .. }
            | Self::ChooseAttackers { player, .. }
            | Self::ChooseBlockers { player, .. }
            | Self::DiscardChoice { player, .. }
            | Self::LegendChoice { player, .. }
            | Self::ChooseCards { player, .. }
            | Self::ChooseTargets { player, .. }
            | Self::ChooseSubtype { player, .. }
            | Self::ChooseCardName { player, .. }
            | Self::ChooseCardType { player, .. }
            | Self::ChooseColor { player, .. }
            | Self::YesNo { player, .. }
            | Self::ChooseCastMode { player, .. }
            | Self::ChooseNumber { player, .. }
            | Self::ChoosePlayer { player, .. }
            | Self::ChoosePile { player, .. }
            | Self::Arrange { player, .. } => Some(*player),
            Self::GameOver(_) => None,
        }
    }
}

/// One exact existing target, without hidden card identity.
pub use baylee_core::ids::TargetRef;

/// The ability index of a *granted* activated ability.
///
/// An ability a continuous effect handed to a permanent is not printed on
/// that permanent's card, so it has no position in the card's ability list to
/// be named by. It is offered under this index instead, and
/// [`PlayerAction::ActivateAbility`] takes it back the same way.
///
/// Named rather than written out because a client reads it: `u32::MAX` in a
/// client would look like a bug, and `index + 1` for a label is one —
/// arithmetic on a synthetic index overflows, which is exactly what happened
/// to the ability chooser.
///
/// This is slot 0; [`granted_ability`] names the rest. There used to be only
/// this one, which was not a simplification but a bug: Urza's Saga grants
/// *itself* two abilities — chapter I's `{T}: Add {C}` and chapter II's
/// `{2}, {T}: Create a Construct` — and with one slot the Construct was never
/// offered, on a card marked `Coverage::Implemented`.
pub const GRANTED_ABILITY: u32 = granted_ability(0);

/// Tagged identity space, with the top 256 indices reserved for special actions.
const MAX_GRANTED_IDS: u32 = 0x7fff_ff00;

/// Encode a deterministic registry identity, never a recipient-list position.
/// The high bit distinguishes these identities from printed ability indices.
///
/// # Panics
/// Panics if the registry has exhausted its reserved identity space.
#[must_use]
pub const fn granted_ability(n: u32) -> u32 {
    assert!(n < MAX_GRANTED_IDS, "granted ability identity exhausted");
    0x8000_0000 | n
}

/// Which granted slot `index` names, if it names one.
///
/// The decoder half of [`granted_ability`], and the one place that partition
/// is written — an engine that decoded a slot differently from how the offer
/// encoded it would run a different ability than the player pressed.
#[must_use]
pub const fn granted_slot(index: u32) -> Option<u32> {
    let n = index.wrapping_sub(0x8000_0000);
    if n < MAX_GRANTED_IDS { Some(n) } else { None }
}

/// The ability index of a prepared permanent's linked cast (Emeritus of Woe).
///
/// Same reason as [`GRANTED_ABILITY`]: what is being activated is not an
/// ability the card prints. Below the granted block, which is why it moved
/// when that block grew — these indices are per-session, chosen fresh in
/// every `LegalActions`, so nothing outside a running game holds one.
pub const PREPARED_CAST: u32 = u32::MAX - 8;
/// A face-up special action, carried by the existing permanent action menu.
pub const TURN_FACE_UP: u32 = PREPARED_CAST - 1;

/// How many unlock slots there are: one per half of a Room.
pub const UNLOCK_SLOTS: u32 = 2;

/// The index unlocking `half` of a Room (0 the left, 1 the right) is
/// offered under: CR 709.5e's special action (CR 116.2m), carried by the
/// permanent's action menu as turning one face up is.
#[must_use]
pub const fn unlock_door(half: u8) -> u32 {
    // `u32::from` is not const.
    #[allow(clippy::cast_lossless)]
    let half = (half & 1) as u32;
    TURN_FACE_UP - 1 - half
}

/// Which half `index` unlocks, if it names an unlock: the decoder half of
/// [`unlock_door`], and the one place that partition is read.
#[must_use]
pub const fn door_to_unlock(index: u32) -> Option<u8> {
    let n = (TURN_FACE_UP - 1).wrapping_sub(index);
    // Below `UNLOCK_SLOTS`, so it fits.
    #[allow(clippy::cast_possible_truncation)]
    if n < UNLOCK_SLOTS {
        Some(n as u8)
    } else {
        None
    }
}

/// Whether `index` is a special action (CR 116.2): turning a permanent face
/// up, or unlocking a door. Neither is an activated ability, so what stops
/// activating one stops neither (Pithing Needle, CR 602.5; split second,
/// CR 702.61a).
#[must_use]
pub const fn is_special_action(index: u32) -> bool {
    index == TURN_FACE_UP || door_to_unlock(index).is_some()
}

// The indices in this module are **not** `AbilityRef` indices, and that is
// the distinction to keep before adding another one here. They name a slot in
// one `LegalActions` — chosen fresh every time it is built, held by nothing
// outside the game it was built for — while an `AbilityRef` is stored against
// an *account* and replayed into the next game. The two spaces both count
// down from `u32::MAX` and would collide if either were read as the other,
// which is why an ability the engine puts on the stack carries
// `AbilityRef::SYNTHETIC` rather than the slot it was offered under.
//
// CR 903.9a's "put your commander into the command zone?" is asked under
// `AbilityRef::COMMANDER_ZONE` for that reason: it is a question a player
// answers once and for good, so it is a wire constant and belongs in
// `baylee-core` beside the other reserved handles.

/// Everything a player may legally do with priority (precomputed).
#[derive(Clone, Debug, Default, serde::Serialize, serde::Deserialize)]
pub struct LegalActions {
    /// Reusable special actions granted by resolved effects until cleanup.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub granted_actions: Vec<GrantedActionOffer>,
    /// Passing is always legal.
    pub can_pass: bool,
    /// Playable lands in hand.
    pub lands: Vec<ObjectId>,
    /// Castable cards in hand (timing + mana verified).
    pub castable: Vec<ObjectId>,
    /// Cards in hand or the command zone that may be cast now and paid for
    /// while they are cast (CR 601.2g): every rule `castable` asks holds
    /// except that the mana is not floating yet. Offered only while the seat
    /// has a mana ability to activate. `CastSpell` for one runs the cast's
    /// choices, then opens a payment window (`PlayerView::owed`); passing
    /// in it with the cost unpaid reverses the cast (CR 601.2h, 732.1).
    /// Whether the seat's sources can make that mana is not checked: that
    /// is the planner's question. Never in `castable` as well.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub payable: Vec<ObjectId>,
    /// Public rules-effect increases, excluding commander tax, for mana planning.
    /// Form-specific entries override Normal; alternatives use Normal's color.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spell_increases: Vec<(ObjectId, CastModeKind, u32)>,
    /// Generic activation increases on controlled permanents. Simple mana
    /// planners must not count a taxed mana ability as free production.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub activation_increases: Vec<(ObjectId, u32)>,
    /// The CR 305.6 shortcut, and **not** every mana ability on the table.
    ///
    /// A permanent is named here when it taps for mana it has by virtue of a
    /// *basic land type* (`casting::intrinsic_mana`), or when a continuous
    /// effect granted it a mana ability. Those two have no printed ability to
    /// point at, so they are answered with `ActivateManaAbility { source }`,
    /// which carries no index.
    ///
    /// A mana ability a card actually **prints** — Sol Ring's `{T}: Add
    /// {C}{C}`, a nonbasic land's own tap, Deathrite Shaman's — is an
    /// ordinary entry in [`Self::abilities`] as `(source, index)` and is
    /// pressed with `ActivateAbility`. It is a mana ability in the rules
    /// (CR 605.1) and never appears in this list.
    ///
    /// Six engine tests were written against the broader reading of this
    /// field in one batch and all six failed the same way, which is why the
    /// narrower one is spelled out here rather than left to be rediscovered.
    pub mana_abilities: Vec<ObjectId>,
    /// Activated abilities available on controlled permanents:
    /// `(source, ability_index)`.
    pub abilities: Vec<(ObjectId, u32)>,
    /// Battlefield abilities legal except for their floating mana payment:
    /// `(source, ability_index, mana_cost)`. These are planning hints, never
    /// accepted actions. A client may make mana, then recheck `abilities`.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub unpaid_abilities: Vec<(ObjectId, u32, baylee_core::mana::ManaCost)>,
    /// Cards suspendable from hand.
    pub suspendable: Vec<ObjectId>,
    /// Cards with foretell that may be exiled face down for {2} right now.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub foretellable: Vec<ObjectId>,
}

impl LegalActions {
    /// Additional generic mana for an activation of this permanent.
    #[must_use]
    pub fn activation_increase(&self, source: ObjectId) -> u32 {
        self.activation_increases
            .iter()
            .find(|(id, _)| *id == source)
            .map_or(0, |(_, n)| *n)
    }

    /// Additional generic mana for a prospective cast, before reductions.
    #[must_use]
    pub fn spell_increase(&self, card: ObjectId, kind: CastModeKind) -> u32 {
        if matches!(kind, CastModeKind::PlayLandFace(_)) {
            return 0;
        }
        self.spell_increases
            .iter()
            .find(|(id, mode, _)| *id == card && *mode == kind)
            .or_else(|| {
                self.spell_increases
                    .iter()
                    .find(|(id, mode, _)| *id == card && *mode == CastModeKind::Normal)
            })
            .map_or(0, |(_, _, n)| *n)
    }

    /// Whether passing is the only thing this seat could do.
    ///
    /// Mana abilities are excluded on purpose: the engine pays from the
    /// pool itself, so a seat that can only make mana it cannot spend has
    /// no decision to make.
    #[must_use]
    pub fn nothing_but_passing(&self) -> bool {
        self.granted_actions.is_empty()
            && self.lands.is_empty()
            && self.castable.is_empty()
            && self.abilities.is_empty()
            && self.suspendable.is_empty()
            && self.foretellable.is_empty()
    }

    /// Whether there is any mana to make here.
    ///
    /// Read only inside a CR 605.3a payment window, where the engine has
    /// already narrowed `abilities` to mana abilities — outside one, an
    /// entry there is any activated ability at all and this would answer
    /// the wrong question. It exists so the engine can decline to open a
    /// window with nothing in it: a seat with no untapped source would
    /// otherwise be asked to make mana and have only the answer it just
    /// gave.
    #[must_use]
    pub fn has_mana_source(&self) -> bool {
        !self.mana_abilities.is_empty()
            || !self.abilities.is_empty()
            || self
                .granted_actions
                .iter()
                .any(|offer| matches!(offer.effect, GrantedActionKind::AddMana { .. }))
    }
}

/// A player's answer to a [`Pending`] request.
///
/// Comparable so that callers can assert on an answer without formatting it:
/// clients build actions from user input and test that they built the right
/// one, and a host can deduplicate a resent action after a reconnect.
#[derive(Clone, PartialEq, Eq, Debug, serde::Serialize, serde::Deserialize)]
pub enum PlayerAction {
    /// Activate one exact offered land mana ability during an instruction.
    ChooseManaAbility {
        /// The instruction step being answered.
        choice: ManaChoiceId,
        /// Exact source incarnation.
        source: baylee_core::ids::DamageSourceRef,
        /// Printed/granted ability index, or intrinsic mana when absent.
        ability_index: Option<u32>,
    },
    /// Choose one exact damage-source incarnation from the pending offer.
    ChooseDamageSource {
        /// Identity of the source decision being answered.
        choice: baylee_core::ids::SourceChoiceId,
        /// Exact offered object and incarnation.
        source: baylee_core::ids::DamageSourceRef,
    },
    /// Apply one offered damage prevention/replacement effect.
    ChooseDamageEffect {
        /// The exact pending damage decision being answered.
        choice: DamageChoiceId,
        /// Stable ID from the offered effect options.
        effect: u32,
    },
    /// Distribute the exact requested prevention among damage parts.
    AllocatePrevention {
        /// The exact pending allocation being answered.
        choice: DamageChoiceId,
        /// Pairs of stable part ID and prevention amount. Omitted parts
        /// receive zero; duplicate or unknown parts are refused.
        allocation: Vec<(u32, u32)>,
    },
    /// Take one effect-granted special action, paying its cost explicitly.
    TakeGrantedAction {
        /// The exact currently offered permission.
        id: baylee_core::ids::GrantedActionId,
    },
    /// Keep the current hand (mulligan).
    MulliganKeep,
    /// Take a mulligan (redraw, then bottom later).
    MulliganTake,
    /// Pass priority.
    PassPriority,
    /// Play a land from hand.
    PlayLand {
        /// The land card in hand.
        card: ObjectId,
    },
    /// Cast a spell from hand (S2: no modes/targets/X; auto-payment).
    CastSpell {
        /// The card in hand.
        card: ObjectId,
    },
    /// Activate a mana ability (tap a mana source).
    ActivateManaAbility {
        /// The mana source permanent.
        source: ObjectId,
    },
    /// Activate an ability of a permanent.
    ActivateAbility {
        /// The source permanent.
        source: ObjectId,
        /// Index into the card's abilities.
        ability_index: u32,
    },
    /// Declare attackers with what each one attacks.
    DeclareAttackers {
        /// `(attacker, defender)` pairs. The defender is a player or one
        /// of that player's planeswalkers (CR 506.2).
        attackers: Vec<(ObjectId, baylee_core::ids::Defender)>,
    },
    /// Declare blockers.
    DeclareBlockers {
        /// `(blocker, blocked attacker)` pairs.
        blockers: Vec<(ObjectId, ObjectId)>,
    },
    /// Choose objects (mulligan bottoming, discards, legend rule).
    ChooseObjects {
        /// The chosen objects.
        objects: Vec<ObjectId>,
    },
    /// Choose targets where players and objects are one set ("any target",
    /// CR 115.4).
    ///
    /// [`PlayerAction::ChooseObjects`] answers the same prompt when nothing
    /// but objects is being chosen — which is every prompt a spell that
    /// says "target creature" raises. This variant exists because a player
    /// has no [`ObjectId`] to be named by, not because targeting split in
    /// two.
    ChooseTargets {
        /// The chosen object targets.
        objects: Vec<ObjectId>,
        /// The chosen player targets.
        players: Vec<PlayerId>,
    },
    /// Use these targets for a bounded, currently waiting series of the
    /// same triggered ability. Each choice is validated again; this never
    /// establishes a standing policy for future triggers.
    ChooseTargetBatch {
        /// Object targets for each occurrence.
        objects: Vec<ObjectId>,
        /// Player targets for each occurrence.
        players: Vec<PlayerId>,
        /// Maximum number of occurrences the player confirmed, including
        /// the current one. New arrivals cannot increase this number.
        count: u32,
    },
    /// Suspend a card from hand with time counters.
    Suspend {
        /// The card to suspend.
        card: ObjectId,
    },
    /// Pay {2} and exile a card with foretell from hand face down. This is
    /// a special action, available with priority during its owner's turn.
    Foretell {
        /// The card to foretell.
        card: ObjectId,
    },
    /// Answer a [`Pending::Arrange`]: one list per pile, in the pile order
    /// the question gave, library piles listed top to bottom.
    Arrange {
        /// The cards in each pile.
        piles: Vec<Vec<ObjectId>>,
    },
    /// Choose one card type.
    ChooseCardType(baylee_core::types::TypeSet),
    /// Choose a mana color.
    ChooseColor(baylee_core::mana::ManaColor),
    /// Choose a creature type (Roaming Throne & co.).
    ChooseSubtype(baylee_core::ids::SubtypeId),
    /// Choose a card name: face `face` of the card `card` (Pithing Needle,
    /// CR 201.4). Refused for a card the pool does not have and for a face
    /// it does not print.
    ChooseCardName {
        /// The card whose name it is.
        card: baylee_core::ids::CardIndex,
        /// Which of its faces, since each face's name may be chosen
        /// (CR 201.4b, 201.4d, 201.4f).
        face: u8,
    },
    /// Choose a cast option (index into `ChooseCastMode::options`).
    ChooseMode(usize),
    /// Choose a number (X values).
    ChooseNumber(u32),
    /// Choose a target player.
    ChoosePlayer(PlayerId),
    /// Answer a yes/no decision.
    YesNo(bool),
    /// Concede the game.
    Concede,
    /// Take back the spell this seat is casting (`Engine::cancellable_cast`,
    /// `PlayerView::casting`): legal in every question of a cast it began
    /// from priority, its own choices and its payment window alike. The cast
    /// is reversed as CR 732.1 reverses one: the card stays where it was, the
    /// costs chosen for it are not paid, and the mana abilities activated
    /// for it are reversed with it. The seat keeps priority (CR 732.2).
    CancelCast,
    /// Offer a draw to every other player still in the game (CR 104.4i).
    OfferDraw,
    /// Change when this seat wants to be offered priority.
    ///
    /// Legal at any time, including while another seat is being asked
    /// something: it changes nothing about the game, only about who gets
    /// interrupted. It is journaled so a replay auto-passes in exactly the
    /// places the live game did.
    SetPriorityHold(PriorityHold),
    /// Automatically pass this seat's priority for one specific ability.
    ///
    /// Also legal at any time, for the same reason.
    SetAbilityYield {
        /// The individual ability whose priority windows may be skipped.
        ability: AbilityRef,
        /// Enable auto-passing; false restores manual responses.
        enabled: bool,
    },
    /// Replace both settings atomically, before the engine resumes automation.
    SetAbilityPolicy {
        /// The individual card ability.
        ability: AbilityRef,
        /// Pass priority over this ability.
        pass: bool,
        /// Optional standing answer; `None` restores asking.
        answer: Option<StandingAnswer>,
    },
    /// Pay the open payment window in one go: `steps`, in order, then the
    /// payment the window was opened for (`PlayerView::owed`). All or
    /// nothing.
    ///
    /// Legal only while this seat holds priority inside its own payment
    /// window: a spell's cost (CR 601.2g-h), an activation's (CR 602.2b), a
    /// "pay or …" price (CR 605.3a). Each step is an action the seat could
    /// have sent alone in that window — a mana ability
    /// ([`Self::ActivateManaAbility`], [`Self::ActivateAbility`],
    /// [`Self::TakeGrantedAction`]) or the answer to a question one of them
    /// asks (a colour, a mode, a number, yes or no, objects, a player) —
    /// and it is checked and applied exactly as it would be alone, so its
    /// side effects (a painland's damage, a triggered mana ability) happen
    /// in the listed order. After the last step the window must be back on
    /// its own priority, and then it settles as passing would.
    ///
    /// If any step is refused, a question is left open, or the pool then
    /// does not pay what is owed, the whole action is refused and the game
    /// is exactly as it was before it arrived: nothing is tapped, made or
    /// paid, and the journal is unchanged. Mana floating before the window
    /// counts toward the payment; no steps at all pays from the pool alone.
    PayWindow {
        /// The mana abilities and their answers, in the order applied.
        steps: Vec<PlayerAction>,
    },
}

impl PlayerAction {
    /// Whether this action may be one step of a [`Self::PayWindow`]:
    /// making mana, or answering what making it asked.
    #[must_use]
    pub const fn is_payment_step(&self) -> bool {
        matches!(
            self,
            Self::ActivateManaAbility { .. }
                | Self::ActivateAbility { .. }
                | Self::TakeGrantedAction { .. }
                | Self::ChooseColor(_)
                | Self::ChooseMode(_)
                | Self::ChooseNumber(_)
                | Self::YesNo(_)
                | Self::ChooseObjects { .. }
                | Self::ChoosePlayer(_)
        )
    }

    /// Whether the action only changes a seat's automation settings.
    ///
    /// These never touch the game: the engine applies them and re-offers
    /// whatever it was already asking, so they neither reset the priority
    /// round nor count as taking an action.
    #[must_use]
    pub const fn is_automation_setting(&self) -> bool {
        matches!(
            self,
            Self::SetPriorityHold(_) | Self::SetAbilityYield { .. } | Self::SetAbilityPolicy { .. }
        )
    }
}

#[cfg(test)]
mod choice_tests;

#[cfg(test)]
mod fit_to_options_tests;
