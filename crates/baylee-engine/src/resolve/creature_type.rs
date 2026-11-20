//! Creature-type choices made during spell or ability resolution
//! (CR 205.3m, CR 205.1b, CR 608.2d, CR 612.1, CR 612.3).

use super::{AwaitingOp, Flow, GameState, Pending, Resolution, run, subjects};
use baylee_cards_dsl::{Duration, Filter, Layer, Modifier};
use baylee_core::{generated::subtypes::ALL_CREATURE_TYPES, ids::SubtypeId};

/// Suspend resolution to ask the controller for a creature type, optionally
/// excluding one forbidden subtype (such as `Wall`).
pub(super) fn choose(res: &mut Resolution, exclude: Option<SubtypeId>) -> Pending {
    let options = ALL_CREATURE_TYPES
        .iter()
        .filter(|&subtype| Some(subtype) != exclude)
        .collect();
    res.awaiting = Some(AwaitingOp::ChooseCreatureType { exclude });
    Pending::ChooseSubtype {
        player: res.controller,
        options,
    }
}

/// Resume a resolution suspended on [`AwaitingOp::ChooseCreatureType`].
pub(super) fn resume(state: &mut GameState, res: &mut Resolution, subtype: SubtypeId) -> Flow {
    subjects::begin_resume(state, res);
    let Some(AwaitingOp::ChooseCreatureType { exclude }) = res.awaiting.take() else {
        panic!("creature-type answer requires a suspended creature-type choice");
    };
    // CR 205.3m / CR 608.2d: only a legal, non-excluded creature type is kept.
    // CR 612.1 / CR 612.3: a word chosen during resolution is not part of the
    // printed rules text, so text-changing effects do not alter it.
    if ALL_CREATURE_TYPES.contains(subtype) && Some(subtype) != exclude {
        res.subject.chosen_subtype = Some(subtype);
    }
    res.pc += 1;
    let flow = run(state, res);
    subjects::flush(state, res);
    flow
}

/// Apply the creature type chosen earlier in this resolution to the permanent(s)
/// selected by `filter`, locking affected objects at resolution (CR 611.2c).
pub(super) fn become_chosen(
    state: &mut GameState,
    res: &Resolution,
    filter: &'static Filter,
    additive: bool,
    duration: Duration,
) {
    let Some(subtype) = res.subject.chosen_subtype else {
        return;
    };
    if !super::continuous_subjects::source_duration_starts(state, res, duration) {
        return;
    }
    let you = res.controller;
    let modifier = if additive {
        Modifier::AddSubtype(subtype)
    } else {
        Modifier::ReplaceCreatureTypes(subtype)
    };
    let filters = match filter {
        Filter::This => {
            let Some(this) = super::this_to_affect(state, res) else {
                return;
            };
            smallvec::smallvec![crate::effects::EffectFilter::object(state, this)]
        }
        Filter::AttachedToBySource => {
            let Some((host, version)) = super::attached_host(state, res) else {
                return;
            };
            smallvec::smallvec![crate::effects::EffectFilter::ObjectIs(host, version)]
        }
        other => {
            super::bound_now_with_context(state, other, &modifier, you, res.rule_context(), None)
        }
    };
    let timestamp = state.next_timestamp();
    for effect_filter in filters {
        if !crate::permanent_guards::permits_resolution_effect(state, effect_filter, modifier, you)
            || !state.source_duration_holds(duration, res.source, you, effect_filter)
        {
            continue;
        }
        let id = state.effects.register(crate::effects::ContinuousEffect {
            id: baylee_core::ids::EffectId::new(0),
            source: Some(res.source),
            controller: you,
            origin: crate::effects::EffectOrigin::Resolution,
            layer: Layer::Type,
            timestamp,
            duration,
            filter: effect_filter,
            modifier,
        });
        // CR 612.1 / CR 612.3: freeze an identity text map on this continuous
        // effect so the generic resolution hook does not bind `res.text` to a
        // creature type that was chosen on resolution rather than printed.
        state.effect_text_overrides.push((
            id,
            crate::text_changes::TextOrigin::Frozen(crate::text_changes::TextChangeMap::IDENTITY),
        ));
    }
}
