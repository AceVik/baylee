//! Project one object's characteristics through the prepared layer plan.

use super::{
    LayerPlan, applies, apply, apply_pt_counters, next_type_effect, prohibited_keywords, pt_count,
    update_intrinsic_mana_summary,
};
use crate::{
    eval,
    object::{Characteristics, GameObject},
    state::GameState,
};
use baylee_cards_dsl::{KeywordSet, LAYERS, Layer, Modifier};
use baylee_core::{
    ids::PlayerId,
    types::{SubtypeSet, TypeSet},
};
use smallvec::SmallVec;

pub(crate) fn recompute_cow<'a>(
    state: &GameState,
    obj: &'a GameObject,
    plan: &LayerPlan,
) -> (std::borrow::Cow<'a, Characteristics>, PlayerId, bool) {
    recompute_observing(state, obj, plan, |_, _, _| {})
}

#[allow(clippy::too_many_lines)]
pub(super) fn recompute_observing<'a>(
    state: &GameState,
    obj: &'a GameObject,
    plan: &LayerPlan,
    mut applied: impl FnMut(Layer, &super::ContinuousEffect, &Characteristics),
) -> (std::borrow::Cow<'a, Characteristics>, PlayerId, bool) {
    // CR 406.3a: a face-down card in exile has no characteristics, including
    // its printed characteristic-defining abilities.
    if obj.zone == crate::zone::Zone::Exile && obj.status.contains(crate::object::Status::FACE_DOWN)
    {
        return (
            std::borrow::Cow::Borrowed(&obj.base),
            obj.base_controller,
            false,
        );
    }
    let mut c: std::borrow::Cow<'a, Characteristics> = std::borrow::Cow::Borrowed(&obj.base);
    // CR 202.3e: on the stack, X is the number chosen for it.
    let want_x = if obj.zone == crate::zone::Zone::Stack {
        obj.x_value
    } else {
        0
    };
    if c.x_on_stack != want_x {
        c.to_mut().x_on_stack = want_x;
    }
    // "As long as this permanent's dash cost was paid, it has haste"
    // (CR 702.109a): an ability of the permanent itself, so it is there
    // before any effect applies and an effect that removes abilities
    // (layer 6) removes it too.
    if obj.riders.contains(&crate::object::Rider::Dashed) && !c.keywords.contains(KeywordSet::HASTE)
    {
        c.to_mut().keywords = c.keywords.union(KeywordSet::HASTE);
    }
    // Layer 2 starts from the *base* controller, not from whatever the
    // last refresh projected: an effect that has since ended must leave
    // no trace.
    let mut controller = obj.base_controller;
    let mut read_board = false;
    // CR 613.6: an effect that began in layer 4 continues on the same
    // objects in layer 7b. This belongs to this projection, never the game
    // state: the next refresh starts from the copiable values again.
    let mut animations: SmallVec<[u32; 4]> = SmallVec::new();
    let all = state.effects.as_slice();
    let mut prohibited = KeywordSet::EMPTY;
    for layer in LAYERS {
        if layer == Layer::Text {
            // Text changes affect the type line and printed/copied keywords
            // here, before type-setting effects and later ability grants.
            let text = eval::live_context(state, obj.id).text;
            if text != crate::text_changes::TextChangeMap::IDENTITY {
                let old_types = c.subtypes;
                let new_types = text.land_types(old_types);
                let new_keywords = text.keywords(c.keywords);
                if new_types != old_types || new_keywords != c.keywords {
                    let c_mut = c.to_mut();
                    c_mut.subtypes = new_types;
                    if new_types != old_types {
                        update_intrinsic_mana_summary(state, obj, c_mut, old_types);
                    }
                    c_mut.keywords = new_keywords;
                }
            }
        }
        if !plan.layer(layer).is_empty() {
            let dynamic_types = layer == Layer::Type && plan.conditional_animation;
            let mut remaining: SmallVec<[u32; 16]> = if dynamic_types {
                plan.layer(layer).into()
            } else {
                SmallVec::new()
            };
            for ordered_idx in plan.candidates(layer, obj.id) {
                let idx = if dynamic_types {
                    next_type_effect(state, obj, &c, all, &mut remaining)
                } else {
                    ordered_idx
                };
                let fx = &all[idx as usize];
                // CR 613.1: each layer sees the characteristics as modified by
                // every earlier layer, so the filter is evaluated against the
                // in-progress projection — an "all creatures get +1/+1" anthem
                // has to see a land that layer 4 just animated.
                let animation = crate::artifact_animation::animates(fx.modifier);
                let applies_now = if animation && layer != Layer::Type {
                    animations.contains(&idx)
                } else {
                    applies(state, fx, obj, &c)
                        && (!animation || !c.types.contains(TypeSet::CREATURE))
                };
                if applies_now {
                    prohibited = prohibited.union(prohibited_keywords(&fx.modifier));
                    if animation && layer == Layer::Type {
                        animations.push(idx);
                    }
                    #[cfg(test)]
                    crate::ability_log::static_applied(fx);
                    let c_mut = c.to_mut();
                    apply(
                        c_mut,
                        &mut controller,
                        fx,
                        state,
                        obj,
                        &mut read_board,
                        layer,
                    );
                    // A prohibited grant must not be visible to later filters either.
                    c_mut.keywords = c_mut.keywords.difference(prohibited);
                    applied(layer, fx, c_mut);
                }
            }
        }
        // CR 604.3: a characteristic-defining P/T works in every zone. On
        // the battlefield it is a registered static and applied above; off
        // it, nothing is registered, so the card applies its own here.
        if layer == Layer::PtCda
            && let Some(modifier) = state.off_battlefield_pt_cda(obj)
        {
            let context = eval::live_context(state, obj.id);
            match modifier {
                Modifier::CharacteristicPT {
                    count,
                    toughness_plus,
                } => {
                    let n = pt_count(state, obj, &c, controller, count, context, &mut read_board);
                    let c_mut = c.to_mut();
                    c_mut.power = Some(n);
                    c_mut.toughness = Some(n.saturating_add(i16::from(toughness_plus)));
                }
                Modifier::CharacteristicPTSeparate { power, toughness } => {
                    let p = pt_count(state, obj, &c, controller, power, context, &mut read_board);
                    let t = pt_count(
                        state,
                        obj,
                        &c,
                        controller,
                        toughness,
                        context,
                        &mut read_board,
                    );
                    let c_mut = c.to_mut();
                    c_mut.power = Some(p);
                    c_mut.toughness = Some(t);
                }
                _ => unreachable!("registered characteristic definition"),
            }
        }
        if layer == Layer::PtCounters && !obj.counters.is_empty() {
            apply_pt_counters(&mut c, obj, state);
        }
    }
    // Changeling (CR 702.73): every creature type, in eight word ORs.
    if c.keywords.contains(KeywordSet::CHANGELING)
        && !c.subtypes.contains_all(SubtypeSet::ALL_CREATURE)
    {
        c.to_mut().subtypes = c.subtypes.union(SubtypeSet::ALL_CREATURE);
    }
    if state.effects.may_guard_permanents() {
        controller = crate::permanent_guards::permitted_controller(state, obj, &c, controller);
        read_board = true;
    }
    (c, controller, read_board)
}
