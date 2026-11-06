//! Target questions: the side the engine's reading of the effect names, what a
//! gate on the target lets through, and which of the other side's permanents a
//! kill should go to.

use baylee_cards_dsl::{AbilityDef, Effect, Filter, KeywordSet};
use baylee_core::ids::{ObjectId, PlayerId, TargetRef};
use baylee_core::types::{TargetSide, TypeSet};
use baylee_engine::choice::{Pending, PlayerAction};
use baylee_view::{PlayerView, PublicObject};

use super::super::lookup_card_identity;
use super::read::{actor, has, toughness_left, worth};
use super::{RuleGroup, TacticRules4};
use crate::features4::{Candidate4, EncodedCandidates4, public_object_matches_basic_filter};
use crate::netplay4::choices::active_source_fixed_damage;

/// How much more a permanent must be worth before a kill may not go past it.
const AIM_MARGIN: i32 = 4;

/// What a candidate of a target question names.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Named {
    Object(ObjectId),
    Player(PlayerId),
    Nothing,
}

impl Named {
    fn of(cand: &Candidate4) -> Self {
        if let Some(id) = cand.object_id {
            return Self::Object(id);
        }
        if let Some(player) = cand.player_id {
            return Self::Player(player);
        }
        match &cand.action {
            Some(PlayerAction::ChooseTargets { objects, players }) => {
                match (objects.as_slice(), players.as_slice()) {
                    ([id], []) => Self::Object(*id),
                    ([], [player]) => Self::Player(*player),
                    _ => Self::Nothing,
                }
            }
            Some(PlayerAction::ChoosePlayer(player)) => Self::Player(*player),
            _ => Self::Nothing,
        }
    }

    fn object(self, view: &PlayerView) -> Option<&PublicObject> {
        let Self::Object(id) = self else { return None };
        view.object(id)
    }

    /// What this names on the battlefield or the stack: a card in a graveyard is
    /// on nobody's side.
    fn in_play(self, view: &PlayerView) -> Option<&PublicObject> {
        let Self::Object(id) = self else { return None };
        view.battlefield
            .iter()
            .chain(view.stack.iter())
            .find(|object| object.id == id)
    }

    /// Whether this is `me` or something of `me`'s: `None` when no side reads off it.
    fn is_mine(self, view: &PlayerView, me: PlayerId) -> Option<bool> {
        match self {
            Self::Player(player) => Some(player == me),
            Self::Object(_) => self.in_play(view).map(|object| object.controller == me),
            Self::Nothing => None,
        }
    }
}

/// The effects of the card asking, every mode of it: the question does not say
/// which mode was chosen, so a reading must hold for all of them.
fn asked_effects(view: &PlayerView, source: ObjectId) -> Vec<&'static [Effect]> {
    let Some(identity) = lookup_card_identity(view, source) else {
        return Vec::new();
    };
    let Some(card) = baylee_cards::by_index(identity.index) else {
        return Vec::new();
    };
    let mut parts = Vec::new();
    for ability in card.abilities_for_face(usize::from(identity.face)) {
        match ability {
            AbilityDef::Spell { effects, .. }
            | AbilityDef::Activated { effects, .. }
            | AbilityDef::ActivatedConditional { effects, .. }
            | AbilityDef::Triggered { effects, .. } => parts.push(*effects),
            AbilityDef::ModalSpell { modes, .. }
            | AbilityDef::ModalActivated { modes, .. }
            | AbilityDef::ModalTriggered { modes, .. } => {
                parts.extend(modes.iter().map(|mode| mode.effects));
            }
            _ => {}
        }
    }
    parts
}

/// The gate every effect of a spell stands behind, if there is one: Pyroblast
/// does nothing unless its target is blue, in either mode.
fn common_gate(parts: &[&'static [Effect]]) -> Option<&'static Filter> {
    let mut gate = None;
    for part in parts {
        let [Effect::IfTargetMatches { filter, .. }] = part else {
            return None;
        };
        gate = Some(*filter);
    }
    gate
}

/// What `me`'s own kills already on the stack name: a second one there is wasted.
pub(super) fn doomed(view: &PlayerView, me: PlayerId) -> Vec<ObjectId> {
    view.stack
        .iter()
        .filter(|item| item.controller == me && kills_outright(&asked_effects(view, item.id)))
        .flat_map(|item| &item.targets)
        .filter_map(|target| match target {
            TargetRef::Object(named) => Some(named.object),
            TargetRef::Player(_) => None,
        })
        .collect()
}

/// Whether a kill of this kind ends what it names whatever its size.
fn kills_outright(parts: &[&'static [Effect]]) -> bool {
    parts.iter().any(|part| {
        part.iter()
            .any(|effect| matches!(effect, Effect::Destroy { .. } | Effect::Exile { .. }))
    })
}

fn survives(object: &PublicObject, damage: Option<i32>) -> bool {
    has(object, KeywordSet::INDESTRUCTIBLE)
        || damage.is_some_and(|amount| toughness_left(object) > amount)
}

impl TacticRules4 {
    /// Which of `named` a group refuses, and which group.
    pub(super) fn refuse_named(
        &self,
        named: &[Named],
        view: &PlayerView,
        pending: &Pending,
        active_source: Option<(ObjectId, Option<u32>)>,
    ) -> Vec<(usize, RuleGroup)> {
        if !matches!(pending, Pending::ChooseTargets { .. }) {
            return Vec::new();
        }
        let me = actor(view);
        let side = view.targeting.as_ref().and_then(|asked| asked.side);
        let source = view
            .targeting
            .as_ref()
            .map(|asked| asked.source.id)
            .or(active_source.map(|active| active.0));
        let parts = source.map_or_else(Vec::new, |id| asked_effects(view, id));
        let gate = common_gate(&parts);
        let doomed = doomed(view, me);
        let mut struck = Vec::new();
        for (index, target) in named.iter().enumerate() {
            if self.on(RuleGroup::Side)
                && matches!(
                    (side, target.is_mine(view, me)),
                    (Some(TargetSide::Harmful), Some(true))
                        | (Some(TargetSide::Helpful), Some(false))
                )
            {
                struck.push((index, RuleGroup::Side));
            }
            if self.on(RuleGroup::Void)
                && let (Some(gate), Some(object)) = (gate, target.object(view))
                && !public_object_matches_basic_filter(me, object, gate)
            {
                struck.push((index, RuleGroup::Void));
            }
            if self.on(RuleGroup::Void)
                && side == Some(TargetSide::Harmful)
                && let Named::Object(id) = target
                && doomed.contains(id)
            {
                struck.push((index, RuleGroup::Void));
            }
        }
        if self.on(RuleGroup::Trick)
            && side == Some(TargetSide::Helpful)
            && let Some(pump) = source.and_then(|id| super::trick::pump_of_card(view, id))
        {
            let turned = super::trick::turned_by(view, me, pump);
            if !turned.is_empty() {
                for (index, target) in named.iter().enumerate() {
                    if !matches!(target, Named::Object(id) if turned.contains(id)) {
                        struck.push((index, RuleGroup::Trick));
                    }
                }
            }
        }
        if self.on(RuleGroup::Aim) && side == Some(TargetSide::Harmful) {
            let damage = active_source_fixed_damage(view, active_source);
            if damage.is_some() || kills_outright(&parts) {
                aim(named, view, me, damage, &mut struck);
            }
        }
        struck
    }

    /// The candidates of a target question that a group refuses, and which.
    pub(super) fn strike_targets(
        &self,
        cands: &EncodedCandidates4,
        view: &PlayerView,
        pending: &Pending,
        active_source: Option<(ObjectId, Option<u32>)>,
    ) -> Vec<(usize, RuleGroup)> {
        let named: Vec<Named> = cands.candidates.iter().map(Named::of).collect();
        self.refuse_named(&named, view, pending, active_source)
    }
}

/// Past the permanent most worth killing a kill goes only to something close to
/// it. Nothing the effect would not end counts, so a Bolt looks at what it kills.
fn aim(
    named: &[Named],
    view: &PlayerView,
    me: PlayerId,
    damage: Option<i32>,
    struck: &mut Vec<(usize, RuleGroup)>,
) {
    let killable = |target: &Named| {
        target
            .in_play(view)
            .filter(|object| object.controller != me && object.types.intersects(TypeSet::CREATURE))
            .filter(|object| !survives(object, damage))
    };
    let Some(top) = named.iter().filter_map(killable).map(worth).max() else {
        return;
    };
    // A land is no answer while something that fights is in reach.
    for (index, target) in named.iter().enumerate() {
        if target.in_play(view).is_some_and(|object| {
            object.controller != me
                && object.types.contains(TypeSet::LAND)
                && !object.types.contains(TypeSet::CREATURE)
        }) {
            struck.push((index, RuleGroup::Aim));
        }
    }
    for (index, target) in named.iter().enumerate() {
        if killable(target).is_some_and(|object| worth(object) + AIM_MARGIN < top) {
            struck.push((index, RuleGroup::Aim));
        }
    }
}
