//! A payment window's taps, staged on this side of the wire until the player
//! confirms them (`PlayerAction::PayWindow`, `docs/protocol.md` §"Paying a
//! window in one action").
//!
//! A CR 605.3a window — a spell's own cost, an activation's, a tax, a pact,
//! ward — used to be paid one mana ability per round trip, and every tap was
//! real the moment it was clicked: a painland dealt its damage, a City of
//! Brass its point, and a land tapped by mistake stayed tapped. The engine
//! now takes the whole payment as one all-or-nothing action, so the taps
//! wait here. A staged tap is a promise the player can take back until Go;
//! nothing about it reaches the engine before then.
//!
//! What this module decides, and nothing else: what the staged taps are worth
//! (an estimate by the planner's own reading of each source, never more than
//! the engine will make), what is still owed with them counted, what the
//! "Pay now" plan adds to them, and the actions they become on the wire. The
//! engine stays the judge — a payment it refuses comes back as one sentence
//! with the window still open and the staging untouched.

use baylee_core::ids::{GrantedActionId, ObjectId};
use baylee_core::mana::{ManaColor, ManaCost, ManaSymbol};
use baylee_engine::choice::{GrantedActionKind, GrantedActionOffer, LegalActions, PlayerAction};
use baylee_view::ManaPoolView;

use crate::manaplan::{self, Plan, Source, Step, Tap};

/// The taps staged in one payment window, in the order they were staged.
///
/// The order is the order they are sent in: a painland's damage and a mana
/// ability's trigger happen in it, so it is the player's and never sorted.
/// One step per permanent — a permanent taps once, and staging it again
/// takes it back.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Staging {
    steps: Vec<Step>,
    /// Granted mana actions staged beside the taps (`TakeGrantedAction`,
    /// a fixed `AddMana` paid with life), in staging order; sent after the
    /// taps. See [`Self::toggle_granted`].
    granted: Vec<GrantedMana>,
}

/// One granted special action that adds a fixed amount of mana, staged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GrantedMana {
    /// The offer's identity.
    pub id: GrantedActionId,
    /// The mana it adds.
    pub color: ManaColor,
    /// How much of it.
    pub amount: u16,
}

impl GrantedMana {
    /// The offer as something a payment can stage: a fixed `AddMana` whose
    /// cost is not mana. A mana-costed one would spend the pool the payment
    /// is counting, so it is taken as it always was.
    #[must_use]
    pub fn of(offer: &GrantedActionOffer) -> Option<Self> {
        let GrantedActionKind::AddMana { color, amount } = offer.effect else {
            return None;
        };
        matches!(offer.cost, baylee_cards_dsl::SpecialActionCost::Life(_)).then_some(Self {
            id: offer.id,
            color,
            amount,
        })
    }
}

impl Staging {
    /// The staged taps, in staging order.
    #[must_use]
    pub fn steps(&self) -> &[Step] {
        &self.steps
    }

    /// The staged granted mana actions, in staging order.
    #[must_use]
    pub fn granted(&self) -> &[GrantedMana] {
        &self.granted
    }

    /// Whether nothing is staged.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.steps.is_empty() && self.granted.is_empty()
    }

    /// Stages a granted mana action, or takes it back when it is staged
    /// already. Returns whether it is staged afterwards.
    pub fn toggle_granted(&mut self, granted: GrantedMana) -> bool {
        if let Some(at) = self.granted.iter().position(|g| g.id == granted.id) {
            self.granted.remove(at);
            false
        } else {
            self.granted.push(granted);
            true
        }
    }

    /// Whether `source` is staged.
    #[must_use]
    pub fn holds(&self, source: ObjectId) -> bool {
        self.steps.iter().any(|s| s.source == source)
    }

    /// Stages `step`, or — when its permanent is already staged — takes that
    /// tap back. Returns whether the step is staged afterwards.
    ///
    /// A second click on a staged permanent is always an un-stage, whatever
    /// mode it asks for: a permanent taps once, and the player who wants the
    /// other colour takes the first back and stages again.
    pub fn toggle(&mut self, step: Step) -> bool {
        if let Some(at) = self.steps.iter().position(|s| s.source == step.source) {
            self.steps.remove(at);
            false
        } else {
            self.steps.push(step);
            true
        }
    }

    /// Takes back the tap of `source`, if it is staged. Returns whether it was.
    pub fn unstage(&mut self, source: ObjectId) -> bool {
        let before = self.steps.len();
        self.steps.retain(|s| s.source != source);
        self.steps.len() != before
    }

    /// Takes every tap back.
    pub fn clear(&mut self) {
        self.steps.clear();
        self.granted.clear();
    }

    /// Appends a plan's taps behind what is staged, skipping any permanent
    /// already staged (the plan was made without them, so none should be).
    pub fn extend(&mut self, plan: &Plan) {
        for step in &plan.steps {
            if !self.holds(step.source) {
                self.steps.push(*step);
            }
        }
    }

    /// Drops every staged tap the engine no longer offers, and says whether
    /// any went. A permanent that left, or was tapped by something else, is
    /// not a tap this payment can still make.
    pub fn keep_offered(&mut self, legal: &LegalActions) -> bool {
        let before = self.steps.len() + self.granted.len();
        self.steps.retain(|step| tap_action(step, legal).is_some());
        self.granted
            .retain(|g| legal.granted_actions.iter().any(|offer| offer.id == g.id));
        self.steps.len() + self.granted.len() != before
    }

    /// The pool as it would stand with every staged tap made: `pool` plus
    /// what each step is worth by [`worth`].
    #[must_use]
    pub fn pool_with(&self, pool: &ManaPoolView, sources: &[Source]) -> ManaPoolView {
        let mut out = *pool;
        for step in &self.steps {
            for (color, count) in worth(step, sources) {
                *slot(&mut out, color) = slot(&mut out, color).saturating_add(count);
            }
        }
        for granted in &self.granted {
            *slot(&mut out, granted.color) =
                slot(&mut out, granted.color).saturating_add(u32::from(granted.amount));
        }
        out
    }

    /// What `cost` still asks for once the pool and the staged taps have
    /// paid what they can, in [`manaplan::remainder`]'s shape: empty once it
    /// is covered.
    #[must_use]
    pub fn remaining(
        &self,
        cost: &ManaCost,
        pool: &ManaPoolView,
        sources: &[Source],
    ) -> Vec<ManaSymbol> {
        manaplan::remainder(cost, &self.pool_with(pool, sources))
    }

    /// Whether the pool and the staged taps together pay `cost`, by the
    /// planner's own matching — the condition under which Go is the answer
    /// rather than "Pay now".
    #[must_use]
    pub fn covers(&self, cost: &ManaCost, pool: &ManaPoolView, sources: &[Source]) -> bool {
        manaplan::plan(cost, &self.pool_with(pool, sources), &[]).is_some()
    }

    /// The taps "Pay now" would add: a plan for what the pool and the staged
    /// taps leave owing, over the sources not staged yet. `None` when nothing
    /// here pays the rest; an empty plan when nothing is left to tap.
    #[must_use]
    pub fn suggest(
        &self,
        cost: &ManaCost,
        pool: &ManaPoolView,
        sources: &[Source],
    ) -> Option<Plan> {
        let free: Vec<Source> = sources
            .iter()
            .filter(|s| !self.holds(s.id))
            .cloned()
            .collect();
        manaplan::plan(cost, &self.pool_with(pool, sources), &free)
    }

    /// The `PlayerAction::PayWindow` steps: each tap as the engine offers it,
    /// followed by its colour where the tap asks one ([`Step::color`] is
    /// `Some` exactly then — an answer to a question nobody posed refuses the
    /// whole payment), then each staged granted action as
    /// `TakeGrantedAction`. `None` when anything staged is no longer offered.
    #[must_use]
    pub fn wire(&self, legal: &LegalActions) -> Option<Vec<PlayerAction>> {
        let mut out = Vec::with_capacity(self.steps.len() * 2);
        for step in &self.steps {
            out.push(tap_action(step, legal)?);
            if let Some(color) = step.color {
                out.push(PlayerAction::ChooseColor(color));
            }
        }
        for granted in &self.granted {
            if !legal
                .granted_actions
                .iter()
                .any(|offer| offer.id == granted.id)
            {
                return None;
            }
            out.push(PlayerAction::TakeGrantedAction { id: granted.id });
        }
        Some(out)
    }
}

/// The action for one tap, or `None` when the engine is not offering it.
#[must_use]
pub fn tap_action(step: &Step, legal: &LegalActions) -> Option<PlayerAction> {
    match step.tap {
        Tap::Intrinsic => legal.mana_abilities.contains(&step.source).then_some(
            PlayerAction::ActivateManaAbility {
                source: step.source,
            },
        ),
        Tap::Ability(ability_index) => legal
            .abilities
            .contains(&(step.source, ability_index))
            .then_some(PlayerAction::ActivateAbility {
                source: step.source,
                ability_index,
            }),
    }
}

/// What one staged tap makes, by the planner's reading of its source: the
/// colours and counts it adds to the pool.
///
/// An estimate that never over-counts. A tap whose source this client cannot
/// read (none in `sources`, or a mode the source does not list) is worth
/// nothing here, so the remainder over-states the debt rather than hiding
/// one; the engine still counts what the tap really makes when Go is sent.
#[must_use]
pub fn worth(step: &Step, sources: &[Source]) -> Vec<(ManaColor, u32)> {
    let Some(source) = sources.iter().find(|s| s.id == step.source) else {
        return Vec::new();
    };
    if source.tap == step.tap {
        if source.bundle {
            return source.colors.iter().map(|&c| (c, 1)).collect();
        }
        let color = match (step.color, source.colors.as_slice()) {
            (Some(color), colors) if colors.contains(&color) => color,
            (None, [only]) => *only,
            _ => return Vec::new(),
        };
        return vec![(color, u32::from(source.amount.max(1)))];
    }
    match &source.dear {
        Some(dear) if dear.tap == step.tap => {
            let color = match (step.color, dear.colors.as_slice()) {
                (Some(color), colors) if colors.contains(&color) => color,
                (None, [only]) => *only,
                _ => return Vec::new(),
            };
            vec![(color, 1)]
        }
        _ => Vec::new(),
    }
}

fn slot(pool: &mut ManaPoolView, color: ManaColor) -> &mut u32 {
    match color {
        ManaColor::White => &mut pool.white,
        ManaColor::Blue => &mut pool.blue,
        ManaColor::Black => &mut pool.black,
        ManaColor::Red => &mut pool.red,
        ManaColor::Green => &mut pool.green,
        ManaColor::Colorless => &mut pool.colorless,
    }
}

#[cfg(test)]
mod tests;
