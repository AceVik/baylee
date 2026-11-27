//! Resolution-time animation effects:
//! - [`Effect::BecomeAura`] (Licids and animate-to-Aura effects, CR 116.2a, CR 303.4)
//! - [`Effect::SetDynamicPT`] (dynamic P/T animation with signed offsets and single-sided P or T)

use super::{
    Resolution, TargetSpec, attachments, continuous_subjects, counters, matches_in_resolution,
    resolving_ability, subjects, zones,
};
use crate::choice::{GrantedActionKind, GrantedActionOffer};
use crate::effects::{ContinuousEffect, EffectFilter, EffectOrigin};
use crate::granted::EndableEffectGroup;
use crate::state::GameState;
use baylee_cards_dsl::{Amount, Duration, Modifier, SpecialActionCost, SpecialActionTiming};
use baylee_core::generated::subtypes;
use baylee_core::ids::{DamageSourceRef, EffectId, GrantedActionId};
use baylee_core::mana::ManaCost;
use baylee_core::types::TypeSet;
use smallvec::SmallVec;

static AURA_SUBTYPES: [baylee_core::ids::SubtypeId; 1] = [subtypes::enchantment::AURA];

/// Execute [`baylee_cards_dsl::Effect::BecomeAura`]: turn the resolving source
/// permanent into a non-creature Aura enchantment with Layer 6 `GrantEnchant`,
/// optionally strip the resolving printed ability (`Modifier::LoseAbility`),
/// optionally grant a priority special action (`EndContinuousEffects`) to end
/// the effect group, refresh characteristics, and attach to `target`.
pub(super) fn become_aura(
    state: &mut GameState,
    res: &Resolution,
    target: TargetSpec,
    enchant: TargetSpec,
    lose_this_ability: bool,
    revert_cost: Option<ManaCost>,
) {
    let Some(source) =
        subjects::source(state, res).filter(|identity| subjects::on_battlefield(state, *identity))
    else {
        return;
    };
    let you = res.controller;
    let filter = EffectFilter::ObjectIs(source.object, source.version);
    let timestamp = state.next_timestamp();
    let mut group_effects: SmallVec<[EffectId; 4]> = SmallVec::new();

    let register = |state: &mut GameState, modifier: Modifier| -> EffectId {
        state.effects.register(ContinuousEffect {
            id: EffectId::new(0),
            source: Some(res.source),
            controller: you,
            origin: EffectOrigin::Resolution,
            layer: modifier.layer(),
            timestamp,
            duration: Duration::Indefinitely,
            filter,
            modifier,
        })
    };

    group_effects.push(register(
        state,
        Modifier::BecomeTypes {
            types: TypeSet::ENCHANTMENT,
            subtypes: &AURA_SUBTYPES,
        },
    ));
    group_effects.push(register(state, Modifier::GrantEnchant(enchant)));

    if lose_this_ability
        && let Some(index) = state
            .object(res.on_stack)
            .and_then(|o| o.ability)
            .map(|loc| loc.index)
        && crate::choice::granted_slot(index).is_none()
    {
        group_effects.push(register(state, Modifier::LoseAbility(index)));
    }

    if let Some(cost) = revert_cost
        && !state.has_left(you)
    {
        let offer = GrantedActionOffer {
            id: GrantedActionId::new(0),
            source,
            ability: resolving_ability(state, res),
            timing: SpecialActionTiming::Priority,
            cost: SpecialActionCost::Mana(cost),
            effect: GrantedActionKind::EndContinuousEffects { target: source },
        };
        let action = state.grant_action(you, offer);
        state.endable_effect_groups.push(EndableEffectGroup {
            action,
            target: source,
            effects: group_effects,
        });
    }

    // Project the new Aura type and GrantEnchant before attachment legality runs.
    state.refresh_characteristics();
    attachments::attach(state, res, target, false);
}

/// Execute [`baylee_cards_dsl::Effect::SetDynamicPT`]: evaluate dynamic `power`
/// and/or `toughness` amounts with signed offsets and lock Layer 7b continuous
/// effects on the resolved subject(s).
#[allow(clippy::too_many_arguments)]
pub(super) fn set_dynamic_pt(
    state: &mut GameState,
    res: &Resolution,
    subject: TargetSpec,
    power: Option<Amount>,
    toughness: Option<Amount>,
    power_offset: i16,
    toughness_offset: i16,
    duration: Duration,
) {
    if !continuous_subjects::source_duration_starts(state, res, duration) {
        return;
    }
    let you = res.controller;
    let computed_power = power
        .map(|amt| counters::signed_amount(&amt, state, you, res).saturating_add(power_offset));
    let computed_toughness = toughness
        .map(|amt| counters::signed_amount(&amt, state, you, res).saturating_add(toughness_offset));
    let modifier = match (computed_power, computed_toughness) {
        (Some(p), Some(t)) => Modifier::SetPT(p, t),
        (Some(p), None) => Modifier::SetPower(p),
        (None, Some(t)) => Modifier::SetToughness(t),
        (None, None) => return,
    };

    let targets: SmallVec<[DamageSourceRef; 4]> = match subject {
        // Explicit ThisObject always resolves to the ability's source even when
        // the ability also targeted another permanent (Halfdane, Riptide Mangler,
        // Sentinel, Sworn Defender, Exuberant Wolfbear).
        TargetSpec::ThisObject => subjects::source(state, res)
            .filter(|r| subjects::is_current(state, *r))
            .into_iter()
            .collect(),
        TargetSpec::EventObject => subjects::event(state, res)
            .filter(|r| subjects::is_current(state, *r))
            .into_iter()
            .collect(),
        TargetSpec::Object(filter) => {
            if res.targeted {
                res.targets
                    .iter()
                    .copied()
                    .filter_map(|id| {
                        let r = continuous_subjects::target(state, res, id)?;
                        let obj = state.object(id)?;
                        matches_in_resolution(filter, state, obj, you, res, Some(you)).then_some(r)
                    })
                    .collect()
            } else {
                state
                    .battlefield_seen()
                    .filter_map(|id| {
                        let obj = state.object(id)?;
                        matches_in_resolution(filter, state, obj, you, res, Some(you)).then_some(
                            DamageSourceRef {
                                object: id,
                                version: obj.version,
                            },
                        )
                    })
                    .collect()
            }
        }
        other => zones::spec_object(state, res, other)
            .and_then(|id| {
                continuous_subjects::target(state, res, id).or_else(|| state.source_identity(id))
            })
            .filter(|r| subjects::is_current(state, *r))
            .into_iter()
            .collect(),
    };

    let timestamp = state.next_timestamp();
    for reference in targets {
        let filter = EffectFilter::ObjectIs(reference.object, reference.version);
        if !crate::permanent_guards::permits_resolution_effect(state, filter, modifier, you)
            || !state.source_duration_holds(duration, res.source, you, filter)
        {
            continue;
        }
        state.effects.register(ContinuousEffect {
            id: EffectId::new(0),
            source: Some(res.source),
            controller: you,
            origin: EffectOrigin::Resolution,
            layer: modifier.layer(),
            timestamp,
            duration,
            filter,
            modifier,
        });
    }
}
