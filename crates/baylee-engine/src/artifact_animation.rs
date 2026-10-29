//! Shared cross-layer artifact animation and Song's immediate departure lifetime.
use baylee_cards_dsl::{AbilityDef, Duration, Layer, Modifier};
use baylee_core::ids::{EffectId, ObjectId};

use crate::{
    effects::{ContinuousEffect, EffectFilter, EffectOrigin},
    object::Status,
    state::GameState,
    text_changes::{RuleContext, TextOrigin},
    zone::Zone,
};

const SONG: Modifier =
    Modifier::AnimateNoncreatureArtifactWithoutAbilitiesUntilEndOfTurnAfterDeparture;

pub(crate) const fn animates(modifier: Modifier) -> bool {
    matches!(modifier, Modifier::AnimateNoncreatureArtifact | SONG)
}

pub(crate) fn continues_in(modifier: Modifier, layer: Layer) -> bool {
    (layer == Layer::PtSet
        && (animates(modifier) || modifier == Modifier::CopyLastExiledCreaturePTAndSubtypes))
        || (layer == Layer::Ability && modifier == SONG)
}

/// This is a continuing static effect, not a leave trigger or a resolving
/// instruction: its recipients remain dynamic after departure (CR 611.3a).
pub(crate) fn retain_departing_song(state: &mut GameState, source: ObjectId) {
    let Some(object) = state.object(source) else {
        return;
    };
    if object.zone != Zone::Battlefield {
        return;
    }
    // Ability loss in layer 6 cannot undo a layer-4 effect (CR 613.6).
    // Basic-land text loss in layer 4 can; match the static synchronizer.
    let active =
        !object.status.contains(Status::PHASED_OUT) && !object.characteristics().rules_text_lost;
    let mut definitions = if active {
        printed_effects(state, source)
    } else {
        Vec::new()
    };
    if definitions.is_empty() && !state.effects.may_continue_song() {
        return;
    }
    if active {
        definitions.extend(state.effects.iter().filter_map(|grant| {
            let effect = crate::effects::granted_static_for(state, grant, object)?;
            (effect.modifier == SONG)
                .then(|| (effect, TextOrigin::Frozen(state.effect_text(grant))))
        }));
    }
    // Current acquisitions are authoritative: a copy may have changed its
    // rules within this incarnation before the next registration sweep.
    // Removal also covers parked records; phasing never creates a continuation.
    state.effects.remove_where(|effect| {
        effect.source == Some(source) && effect.origin.is_static() && effect.modifier == SONG
    });
    for (mut effect, origin) in definitions {
        let text = origin.resolve(&state.text_changes);
        effect.source = None;
        effect.origin = EffectOrigin::Resolution;
        effect.duration = Duration::UntilEndOfTurn;
        let id = state.effects.register(effect);
        state
            .effect_text_overrides
            .push((id, TextOrigin::Frozen(text)));
    }
}

/// A monotone hint also sees grants whose child has not been materialized yet.
pub(crate) fn contains_song(modifier: Modifier) -> bool {
    match modifier {
        SONG => true,
        Modifier::GrantStatic { modifier, .. } => contains_song(*modifier),
        _ => false,
    }
}

fn printed_effects(state: &GameState, source: ObjectId) -> Vec<(ContinuousEffect, TextOrigin)> {
    let object = state.object(source).expect("departing permanent");
    let reference = state.source_identity(source).expect("departing identity");
    let list = object.printed_ability_list(state.bases.as_ref());
    list.abilities
        .iter()
        .enumerate()
        .filter_map(|(index, definition)| {
            let AbilityDef::Static(ability) = definition else {
                return None;
            };
            if ability.modifier != SONG || !object.zone.matches_ref(ability.source_zone) {
                return None;
            }
            let origin = TextOrigin::Ability {
                source: reference,
                index: u32::try_from(index).expect("ability index fits"),
                base: list.base_text(index),
            };
            let context = RuleContext {
                source,
                text: origin.resolve(&state.text_changes),
                resolved_name: None,
                reference: None,
            };
            if ability.condition.is_some_and(|condition| {
                !crate::eval::condition_holds_with_context(
                    state,
                    object.controller,
                    context,
                    condition,
                )
            }) {
                return None;
            }
            Some((
                ContinuousEffect {
                    id: EffectId::new(0),
                    source: Some(source),
                    controller: object.controller,
                    origin: EffectOrigin::Static,
                    layer: ability.layer,
                    timestamp: object.timestamp,
                    duration: Duration::WhileSourceOnBattlefield,
                    filter: EffectFilter::Dsl(&ability.filter),
                    modifier: SONG,
                },
                origin,
            ))
        })
        .collect()
}
