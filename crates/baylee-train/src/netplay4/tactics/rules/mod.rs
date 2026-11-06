//! Rule groups over the tactical layer, each with its own switch.
//!
//! The first ladder against language models named mistakes one ply of the seat's
//! own action and the other side's visible reply can see. Each group here takes
//! one class of them, read from the seat's view and the structure of the cards'
//! rules: never a card's name, nothing the view does not show. A group either
//! strikes an option that does nothing or helps only the other side, or reads the
//! other side's visible swing. With no group on, the layer plays as it always did.

mod attack;
mod block;
mod options;
mod read;
mod targets;
#[cfg(test)]
mod tests;

use std::borrow::Cow;

use baylee_core::ids::ObjectId;
use baylee_engine::choice::{LegalActions, Pending, PlayerAction};
use baylee_view::PlayerView;

use super::select_single_choice;
use crate::deckgen::Rng;
use crate::features4::EncodedCandidates4;

/// One class of mistake, switched on alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RuleGroup {
    /// An option that changes nothing: a gate no target opens, an amount of zero,
    /// a land animated while it can do neither.
    Void,
    /// A target on the wrong side of the table, as the engine reads the effect.
    Side,
    /// A sweep that costs the seat more than the other side.
    Mass,
    /// A lord that lifts the other side's team more than the seat's.
    Gift,
    /// An attack that loses the race to the visible swing back.
    Race,
    /// An attacker left unblocked that does something when it connects.
    Trip,
    /// A kill spent on a small target with a far bigger one in reach.
    Aim,
    /// A turn ended with a proactive spell the mana in play could have paid.
    Spend,
    /// A usable instant held through the other side's spell, attack or end step.
    React,
    /// An even trade left unmade: a blocker that kills what is worth as much.
    Trade,
    /// A creature left at home that nothing could block, at no risk.
    Press,
    /// A planeswalker the attack goes past.
    Walker,
}

const GROUPS: [(RuleGroup, &str); 12] = [
    (RuleGroup::Void, "void"),
    (RuleGroup::Side, "side"),
    (RuleGroup::Mass, "mass"),
    (RuleGroup::Gift, "gift"),
    (RuleGroup::Race, "race"),
    (RuleGroup::Trip, "trip"),
    (RuleGroup::Aim, "aim"),
    (RuleGroup::Spend, "spend"),
    (RuleGroup::React, "react"),
    (RuleGroup::Trade, "trade"),
    (RuleGroup::Press, "press"),
    (RuleGroup::Walker, "walker"),
];

/// A spell (no index) or an ability taken off the menu, and by which group.
type Struck = (ObjectId, Option<u32>, RuleGroup);

/// The groups a net plays by, and what they changed.
#[derive(Clone, Copy, Debug, Default)]
pub struct TacticRules4 {
    on: [bool; GROUPS.len()],
    fired: [u64; GROUPS.len()],
}

impl TacticRules4 {
    /// The groups a comma list names; `all` names every one.
    ///
    /// # Errors
    /// A word that names no group.
    pub fn parse(list: &str) -> anyhow::Result<Self> {
        let mut rules = Self::default();
        for word in list.split(',').filter(|word| !word.is_empty()) {
            if word == "all" {
                rules.on = [true; GROUPS.len()];
                continue;
            }
            let index = GROUPS
                .iter()
                .position(|(_, name)| *name == word)
                .ok_or_else(|| anyhow::anyhow!("no rule group {word}"))?;
            rules.on[index] = true;
        }
        Ok(rules)
    }

    const fn on(&self, group: RuleGroup) -> bool {
        self.on[group as usize]
    }

    /// Whether any group is on.
    #[must_use]
    pub fn any(&self) -> bool {
        self.on.iter().any(|on| *on)
    }

    const fn count(&mut self, group: RuleGroup) {
        self.fired[group as usize] += 1;
    }

    /// Answers each group changed since this was last asked, by group's name.
    pub fn take_fired(&mut self) -> Vec<(&'static str, u64)> {
        let fired = GROUPS
            .iter()
            .zip(self.fired)
            .map(|((_, name), count)| (*name, count))
            .collect();
        self.fired = [0; GROUPS.len()];
        fired
    }

    /// What the groups take off the menu at this priority: the legal actions
    /// without them, and each struck spell or ability with its group.
    fn strike_priority(
        &self,
        view: &PlayerView,
        legal: &LegalActions,
    ) -> Option<(LegalActions, Vec<Struck>)> {
        if view.owed.is_some() {
            return None;
        }
        let mut struck = Vec::new();
        for &card in legal.castable.iter().chain(&legal.payable) {
            if let Some(group) = self.strike_spell(view, card) {
                struck.push((card, None, group));
            }
        }
        for &(source, index) in &legal.abilities {
            if let Some(group) = self.strike_ability(view, source, index) {
                struck.push((source, Some(index), group));
            }
        }
        if struck.is_empty() {
            return None;
        }
        let spell = |id: &ObjectId| !struck.iter().any(|item| item.0 == *id && item.1.is_none());
        let mut left = legal.clone();
        left.castable.retain(spell);
        left.payable.retain(spell);
        left.abilities.retain(|(source, index)| {
            !struck
                .iter()
                .any(|item| item.0 == *source && item.1 == Some(*index))
        });
        Some((left, struck))
    }
}

impl TacticRules4 {
    /// What the groups say of an answer a seat gave, for a reader of its
    /// journal: each group that would have changed it, and how. An attack and a
    /// block are read as declared, with the logits they no longer have.
    #[must_use]
    pub fn audit(
        &self,
        view: &PlayerView,
        pending: &Pending,
        action: &PlayerAction,
    ) -> Vec<(&'static str, String)> {
        let name = |group: RuleGroup| GROUPS[group as usize].1;
        let mut found = Vec::new();
        match (pending, action) {
            (Pending::Priority { .. }, PlayerAction::CastSpell { card, .. }) => {
                if let Some(group) = self.strike_spell(view, *card) {
                    found.push((name(group), String::new()));
                }
            }
            (
                Pending::Priority { .. },
                PlayerAction::ActivateAbility {
                    source,
                    ability_index,
                },
            ) => {
                if let Some(group) = self.strike_ability(view, *source, *ability_index) {
                    found.push((name(group), String::new()));
                }
            }
            (
                Pending::ChooseTargets {
                    options,
                    player_options,
                    ..
                },
                PlayerAction::ChooseObjects { objects }
                | PlayerAction::ChooseTargets { objects, .. },
            ) => {
                let named: Vec<targets::Named> = options
                    .iter()
                    .map(|id| targets::Named::Object(*id))
                    .chain(player_options.iter().map(|id| targets::Named::Player(*id)))
                    .collect();
                for (index, group) in self.refuse_named(&named, view, pending, None) {
                    if index < options.len() && objects.contains(&options[index]) {
                        found.push((name(group), format!("{:?}", options[index])));
                    }
                }
            }
            (Pending::ChooseAttackers { .. }, PlayerAction::DeclareAttackers { attackers }) => {
                let going: Vec<ObjectId> = attackers.iter().map(|pair| pair.0).collect();
                if self.on(RuleGroup::Press) {
                    let free = attack::free_swings(view, pending, &going);
                    if !free.is_empty() {
                        found.push((name(RuleGroup::Press), format!("{free:?}")));
                    }
                }
                if self.on(RuleGroup::Race) {
                    let staying = attack::decide(view, pending, going);
                    if !staying.is_empty() {
                        found.push((name(RuleGroup::Race), format!("{staying:?}")));
                    }
                }
            }
            (Pending::ChooseBlockers { .. }, PlayerAction::DeclareBlockers { blockers }) => {
                if self.on(RuleGroup::Trip) {
                    let open = block::left_open(view, pending, blockers, block::stops_a_connection);
                    if !open.is_empty() {
                        found.push((name(RuleGroup::Trip), format!("{open:?}")));
                    }
                }
                if self.on(RuleGroup::Trade) {
                    let open = block::left_open(view, pending, blockers, block::is_a_fair_trade);
                    if !open.is_empty() {
                        found.push((name(RuleGroup::Trade), format!("{open:?}")));
                    }
                }
            }
            _ => {}
        }
        found
    }
}

impl TacticRules4 {
    /// What a pass is turned to by the groups that do not let a turn go by.
    fn instead_of_pass(
        &mut self,
        cands: &EncodedCandidates4,
        logits: &[f32],
        view: &PlayerView,
        pending: &Pending,
        chosen: usize,
    ) -> Option<usize> {
        let Pending::Priority { legal, .. } = pending else {
            return None;
        };
        if !matches!(
            cands.candidates.get(chosen)?.action,
            Some(PlayerAction::PassPriority)
        ) {
            return None;
        }
        if self.on(RuleGroup::Spend)
            && let Some(better) = super::guard::spend(cands, logits, view, legal)
        {
            self.count(RuleGroup::Spend);
            return Some(better);
        }
        if self.on(RuleGroup::React)
            && let Some(better) = super::guard::react(cands, logits, view, legal)
        {
            self.count(RuleGroup::React);
            return Some(better);
        }
        None
    }
}

/// `action`'s group, if it is one of the struck.
fn struck_by(action: Option<&PlayerAction>, struck: &[Struck]) -> Option<RuleGroup> {
    struck.iter().find_map(|(id, index, group)| {
        let hit = match action? {
            PlayerAction::CastSpell { card, .. } => card == id && index.is_none(),
            PlayerAction::ActivateAbility {
                source,
                ability_index,
            } => source == id && *index == Some(*ability_index),
            _ => false,
        };
        hit.then_some(*group)
    })
}

/// A question as the rules leave it: its logits, and at priority what is left
/// on the menu. The groups behind it, the first of them counted when the answer moves.
struct Reviewed<'a> {
    logits: Vec<f32>,
    pending: Cow<'a, Pending>,
    group: RuleGroup,
}

fn review_single<'a>(
    rules: &TacticRules4,
    cands: &EncodedCandidates4,
    logits: &[f32],
    view: &PlayerView,
    pending: &'a Pending,
    active_source: Option<(ObjectId, Option<u32>)>,
) -> Option<Reviewed<'a>> {
    let mut left = logits[..cands.candidates.len()].to_vec();
    if let Pending::Priority { legal, .. } = pending {
        let (remaining, struck) = rules.strike_priority(view, legal)?;
        for (cand, logit) in cands.candidates.iter().zip(left.iter_mut()) {
            if struck_by(cand.action.as_ref(), &struck).is_some() {
                *logit = f32::NEG_INFINITY;
            }
        }
        let mut filtered = pending.clone();
        if let Pending::Priority { legal, .. } = &mut filtered {
            **legal = remaining;
        }
        return Some(Reviewed {
            logits: left,
            pending: Cow::Owned(filtered),
            group: struck[0].2,
        });
    }
    let struck = rules.strike_targets(cands, view, pending, active_source);
    let group = struck.first()?.1;
    let mut trial = left.clone();
    for (index, _) in &struck {
        trial[*index] = f32::NEG_INFINITY;
    }
    // A question must be answered: with nothing else to name, it stands.
    if !trial.iter().any(|logit| logit.is_finite()) {
        return None;
    }
    left = trial;
    Some(Reviewed {
        logits: left,
        pending: Cow::Borrowed(pending),
        group,
    })
}

/// [`select_single_choice`] under the rule groups.
#[allow(
    clippy::too_many_arguments,
    reason = "The tactical selection's own arguments and the rules beside them"
)]
pub(in crate::netplay4) fn single_choice(
    rules: &mut TacticRules4,
    cands: &EncodedCandidates4,
    logits: &[f32],
    view: &PlayerView,
    pending: &Pending,
    active_source: Option<(ObjectId, Option<u32>)>,
    temperature: f32,
    rng: &mut Rng,
) -> (usize, f32) {
    let reviewed = rules
        .any()
        .then(|| review_single(rules, cands, logits, view, pending, active_source))
        .flatten();
    let (left, asked) = reviewed.as_ref().map_or((logits, pending), |review| {
        (&review.logits[..], &*review.pending)
    });
    // At temperature zero the plain answer costs nothing to know, and what a
    // group changed is what its count should say.
    let plain = (reviewed.is_some() && temperature <= 1e-4).then(|| {
        select_single_choice(
            cands,
            logits,
            view,
            pending,
            active_source,
            temperature,
            rng,
        )
        .0
    });
    let (mut chosen, logprob) =
        select_single_choice(cands, left, view, asked, active_source, temperature, rng);
    if let Some(review) = &reviewed
        && plain.is_some_and(|plain| plain != chosen)
    {
        rules.count(review.group);
    }
    if let Some(better) = rules.instead_of_pass(cands, left, view, asked, chosen) {
        chosen = better;
    }
    (chosen, logprob)
}

/// A subset question under the rule groups: an attack or several targets.
pub(in crate::netplay4) fn subset(
    rules: &mut TacticRules4,
    cands: &EncodedCandidates4,
    logits: &mut [f32],
    view: &PlayerView,
    pending: &Pending,
    active_source: Option<(ObjectId, Option<u32>)>,
) {
    if !rules.any() {
        return;
    }
    if rules.on(RuleGroup::Race) {
        let staying = attack::stay_home(cands, logits, view, pending);
        if !staying.is_empty() {
            attack::apply(cands, logits, &staying);
            rules.count(RuleGroup::Race);
        }
    }
    if rules.on(RuleGroup::Press) {
        let going = attack::going(cands, logits, pending);
        let free = attack::free_swings(view, pending, &going);
        if !free.is_empty() {
            attack::send(cands, logits, &free);
            rules.count(RuleGroup::Press);
        }
    }
    if rules.on(RuleGroup::Walker) && attack::at_walkers(cands, logits, view, pending) {
        rules.count(RuleGroup::Walker);
    }
    for (index, group) in rules.strike_targets(cands, view, pending, active_source) {
        if logits[index] > -20.0 {
            logits[index] = -20.0;
            rules.count(group);
        }
    }
}

/// A block question under the rule groups.
pub(in crate::netplay4) fn pairs(
    rules: &mut TacticRules4,
    cands: &EncodedCandidates4,
    logits: &mut [f32],
    view: &PlayerView,
    pending: &Pending,
) {
    if rules.on(RuleGroup::Trip) && block::stop_connections(cands, logits, view, pending) {
        rules.count(RuleGroup::Trip);
    }
    if rules.on(RuleGroup::Trade) && block::take_trades(cands, logits, view, pending) {
        rules.count(RuleGroup::Trade);
    }
}
