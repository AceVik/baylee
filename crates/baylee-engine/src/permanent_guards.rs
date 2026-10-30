//! Permissions for new attachments and actual control transitions.
use crate::{
    effects::{ContinuousEffect, EffectFilter, EffectOrigin},
    object::{Characteristics, GameObject},
    state::GameState,
    text_changes::RuleContext,
    zone::Zone,
};
use baylee_cards_dsl::{AbilityDef, Modifier};
use baylee_core::ids::{ObjectId, PlayerId};

fn guards(
    state: &GameState,
    object: &GameObject,
    characteristics: &Characteristics,
    effect: &ContinuousEffect,
) -> bool {
    if object.zone != Zone::Battlefield || !crate::effects::source_is_present(state, effect) {
        return false;
    }
    let source = effect.source.and_then(|id| state.object(id));
    if effect.origin.is_static()
        && source.is_some_and(|source| source.status.contains(crate::object::Status::PHASED_OUT))
    {
        return false;
    }
    let controller = if effect.origin.is_static() {
        source.map_or(effect.controller, |source| source.controller)
    } else {
        effect.controller
    };
    let context = RuleContext {
        source: effect.source.unwrap_or(object.id),
        text: state.effect_text(effect),
        resolved_name: None,
        reference: None,
    };
    // Status can change between two instructions before the normal static sweep.
    // Read the actual ability's live condition without recursively projecting it.
    if effect.origin == EffectOrigin::Static
        && !source.is_some_and(|source| {
            source
            .abilities(state.bases.as_ref())
            .iter()
            .any(|ability| {
                matches!(ability, AbilityDef::Static(ability)
                if ability.modifier == effect.modifier
                && source.zone.matches_ref(ability.source_zone)
                && ability.condition.is_none_or(|condition| {
                    crate::eval::condition_holds_with_context(state, controller, context, condition)
                }))
            })
        })
    {
        return false;
    }
    match effect.filter {
        EffectFilter::ObjectIs(..) => effect.filter.names(object),
        EffectFilter::Dsl(filter) => crate::eval::matches_projected_with_context(
            filter,
            state,
            object,
            characteristics,
            controller,
            context,
        ),
        EffectFilter::DslControlledBy(filter, seats) => {
            seats.contains(object.controller)
                && crate::eval::matches_projected_with_context(
                    filter,
                    state,
                    object,
                    characteristics,
                    controller,
                    context,
                )
        }
        EffectFilter::DslAbout(filter, id, version) => crate::eval::matches_projected_with_context(
            filter,
            state,
            object,
            characteristics,
            controller,
            crate::text_changes::RuleContext {
                resolved_name: None,

                reference: crate::effects::still_the_object(state, id, version),
                ..context
            },
        ),
    }
}

/// Existing attachments remain legal; a stack spell's intended host is not one.
pub(crate) fn permits_new_enchantment(state: &GameState, host: ObjectId, aura: ObjectId) -> bool {
    if !state.effects.may_guard_permanents() {
        return true;
    }
    if state
        .object(aura)
        .is_some_and(|object| object.zone == Zone::Battlefield && object.attached_to == Some(host))
    {
        return true;
    }
    let Some(object) = state.object(host) else {
        return false;
    };
    !state.effects.iter().any(|effect| {
        matches!(
            effect.modifier,
            Modifier::CantBecomeEnchanted | Modifier::CantBeEnchanted
        ) && guards(state, object, object.characteristics(), effect)
    })
}

/// Check a newly resolving control instruction before registering its effect.
pub(crate) fn can_gain_control(state: &GameState, id: ObjectId, player: PlayerId) -> bool {
    state.object(id).is_some_and(|object| {
        !state.has_left(player)
            && permitted_controller(state, object, object.characteristics(), player) == player
    })
}

/// Only the final proposed transition matters, not historical layer-2 writes.
pub(crate) fn permitted_controller(
    state: &GameState,
    object: &GameObject,
    characteristics: &Characteristics,
    proposed: PlayerId,
) -> PlayerId {
    if proposed == object.controller || !state.effects.may_guard_permanents() {
        return proposed;
    }
    if state.effects.iter().any(|effect| {
        effect.modifier == Modifier::OtherPlayersCantGainControl
            && if effect.origin.is_static() {
                effect
                    .source
                    .and_then(|id| state.object(id))
                    .map_or(effect.controller, |source| source.controller)
                    != proposed
            } else {
                effect.controller != proposed
            }
            && guards(state, object, characteristics, effect)
    }) {
        object.controller
    } else {
        proposed
    }
}

/// A refused resolving effect must not become a dormant later control change.
pub(crate) fn permits_resolution_effect(
    state: &GameState,
    filter: EffectFilter,
    modifier: Modifier,
    controller: PlayerId,
) -> bool {
    if modifier != Modifier::GainControl {
        return true;
    }
    match filter {
        EffectFilter::ObjectIs(id, version) => state.object(id).is_some_and(|object| {
            object.version == version && can_gain_control(state, id, controller)
        }),
        // Control-changing resolution effects must have frozen their recipient set.
        EffectFilter::Dsl(_) | EffectFilter::DslAbout(..) | EffectFilter::DslControlledBy(..) => {
            false
        }
    }
}
