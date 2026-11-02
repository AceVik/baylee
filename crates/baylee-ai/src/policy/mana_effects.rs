//! Battlefield effects that change the mana a tap makes or a spell costs.

use baylee_cards_dsl::{AbilityDef, Effect, Filter, ManaSource};
use baylee_client_core::decision::resource_player;
use baylee_client_core::manaplan::Source;
use baylee_core::ids::ObjectId;
use baylee_core::mana::{ManaColor, ManaCost};
use baylee_engine::choice::LegalActions;
use baylee_view::{CardIdentity, PlayerView};

use super::{card_as_object, effects_self_harm, face, player_rel_hits_you};
use crate::HeuristicAgent;

/// These readers inspect triggers and rules effects, which stop when their
/// abilities are removed. Phased-out permanents remain in the view for drawing
/// but affect nothing in play (CR 702.26b).
fn active_abilities(object: &baylee_view::PublicObject) -> &'static [AbilityDef] {
    if object.status.is_phased_out()
        || object.status.has_no_abilities()
        || object.status.is_face_down()
    {
        &[]
    } else {
        crate::activate::printed_list(object)
    }
}

fn tappers_filter_matches(
    view: &PlayerView,
    filter: &Filter,
    perm: &baylee_view::PublicObject,
    object: &baylee_view::PublicObject,
) -> bool {
    if matches!(filter, Filter::AttachedToBySource) {
        return perm.attached_to == Some(object.id);
    }
    HeuristicAgent::new(crate::AIProfile::default()).filter_matches(
        filter,
        view,
        object,
        baylee_cards_dsl::ZoneRef::Battlefield,
        Some(perm.id),
    ) == Some(true)
}

pub(super) fn tap_trigger_harm(view: &PlayerView, object: &baylee_view::PublicObject) -> i32 {
    let mut harm = 0_i32;
    let you = resource_player(view);
    for perm in &view.battlefield {
        let perm_is_you = perm.controller == you;
        for ability in active_abilities(perm) {
            match ability {
                AbilityDef::Triggered {
                    trigger: baylee_cards_dsl::Trigger::BecomesTapped(filter),
                    effects,
                    ..
                } if (perm.id == object.id && matches!(filter, Filter::This))
                    || tappers_filter_matches(view, filter, perm, object) =>
                {
                    harm = harm.saturating_add(effects_self_harm(effects, perm_is_you).max(0));
                }
                AbilityDef::Triggered {
                    trigger: baylee_cards_dsl::Trigger::TappedForMana { by, filter },
                    effects,
                    ..
                } if player_rel_hits_you(*by, perm_is_you)
                    && tappers_filter_matches(view, filter, perm, object) =>
                {
                    harm = harm.saturating_add(effects_self_harm(effects, perm_is_you).max(0));
                }
                _ => {}
            }
        }
    }
    harm
}

/// Whether tapping `object` sets off a non-mana trigger (City of Brass,
/// Psychic Venom, Blight, Manabarbs, Lifetap).
pub(crate) fn taps_set_off_a_trigger(
    view: &PlayerView,
    object: &baylee_view::PublicObject,
) -> bool {
    if active_abilities(object).iter().any(|ability| {
        matches!(
            ability,
            AbilityDef::Triggered {
                trigger: baylee_cards_dsl::Trigger::BecomesTapped(filter),
                ..
            } if matches!(filter, Filter::This)
        )
    }) {
        return true;
    }
    for perm in &view.battlefield {
        let same_side = perm.controller == object.controller;
        for ability in active_abilities(perm) {
            match ability {
                AbilityDef::Triggered {
                    trigger: baylee_cards_dsl::Trigger::BecomesTapped(filter),
                    ..
                } if (perm.id == object.id && matches!(filter, Filter::This))
                    || tappers_filter_matches(view, filter, perm, object) =>
                {
                    return true;
                }
                AbilityDef::Triggered {
                    trigger: baylee_cards_dsl::Trigger::TappedForMana { by, filter },
                    effects,
                    ..
                } if player_rel_hits_you(*by, same_side)
                    && tappers_filter_matches(view, filter, perm, object)
                    && !effects.iter().all(|e| {
                        matches!(
                            e,
                            Effect::AddMana { .. }
                                | Effect::AddManaFor { .. }
                                | Effect::AddManaLikeEvent { .. }
                        )
                    }) =>
                {
                    return true;
                }
                _ => {}
            }
        }
    }
    false
}

pub(super) fn tap_destroys_or_locks(view: &PlayerView, object: &baylee_view::PublicObject) -> bool {
    if active_abilities(object).iter().any(|ability| {
        matches!(
            ability,
            AbilityDef::Static(baylee_cards_dsl::StaticAbility {
                modifier: baylee_cards_dsl::Modifier::DoesNotUntap,
                ..
            })
        )
    }) {
        return true;
    }
    for perm in &view.battlefield {
        for ability in active_abilities(perm) {
            if let AbilityDef::Triggered {
                trigger: baylee_cards_dsl::Trigger::BecomesTapped(filter),
                effects,
                ..
            } = ability
                && tappers_filter_matches(view, filter, perm, object)
                && effects.iter().any(|e| {
                    matches!(
                        e,
                        Effect::Destroy { .. }
                            | Effect::DestroyEventThenMayReattach { .. }
                            | Effect::SacrificeObject { .. }
                    )
                })
            {
                return true;
            }
        }
    }
    false
}

/// Applies bonus mana from triggered mana abilities (CR 605.1b: Wild Growth,
/// Mana Flare, Vernal Bloom, Gauntlet of Might) when tapping `object`.
pub(crate) fn apply_triggered_mana_bonus(
    view: &PlayerView,
    object: &baylee_view::PublicObject,
    source: &mut Source,
) {
    if source.dear.is_some() || source.colors.is_empty() {
        return;
    }
    let add_fixed_color = |source: &mut Source, color: ManaColor, count: u8| {
        if count == 0 {
            return;
        }
        if !source.bundle && source.colors.len() == 1 {
            if source.colors[0] == color {
                source.amount = source.amount.saturating_add(count);
            } else {
                let mut expanded = vec![source.colors[0]; usize::from(source.amount)];
                expanded.extend(std::iter::repeat_n(color, usize::from(count)));
                source.colors = expanded;
                source.bundle = true;
                source.amount = source.amount.saturating_add(count);
            }
        } else if source.bundle {
            source
                .colors
                .extend(std::iter::repeat_n(color, usize::from(count)));
            source.amount = source.amount.saturating_add(count);
        }
    };
    for perm in &view.battlefield {
        let same_side = perm.controller == object.controller;
        for ability in active_abilities(perm) {
            let AbilityDef::Triggered {
                trigger: baylee_cards_dsl::Trigger::TappedForMana { by, filter },
                effects,
                ..
            } = ability
            else {
                continue;
            };
            if !player_rel_hits_you(*by, same_side)
                || !tappers_filter_matches(view, filter, perm, object)
            {
                continue;
            }
            for effect in *effects {
                match effect {
                    Effect::AddManaLikeEvent { amount } => {
                        let count = u8::try_from(*amount).unwrap_or(u8::MAX);
                        if source.bundle {
                            if let Some(&first) = source.colors.first() {
                                source
                                    .colors
                                    .extend(std::iter::repeat_n(first, usize::from(count)));
                                source.amount = source.amount.saturating_add(count);
                            }
                        } else {
                            source.amount = source.amount.saturating_add(count);
                        }
                    }
                    Effect::AddManaFor {
                        who,
                        color,
                        amount: n,
                    } if player_rel_hits_you(*who, same_side) => {
                        let count = u8::try_from(*n).unwrap_or(0);
                        add_fixed_color(source, *color, count);
                    }
                    Effect::AddMana {
                        source: ManaSource::Fixed(color),
                        amount: baylee_cards_dsl::Amount::Fixed(n),
                        restriction: None,
                        ..
                    } if same_side => {
                        let count = u8::try_from(*n).unwrap_or(0);
                        add_fixed_color(source, *color, count);
                    }
                    _ => {}
                }
            }
        }
    }
}

pub(crate) fn spell_mana_increase(
    view: &PlayerView,
    id: ObjectId,
    mut card: CardIdentity,
    legal: &LegalActions,
    mode: baylee_engine::choice::CastModeKind,
) -> ManaCost {
    let total = legal.spell_increase(id, mode);
    if total == 0 {
        return ManaCost::ZERO;
    }
    if let baylee_engine::choice::CastModeKind::Face(index) = mode {
        let Ok(index) = u8::try_from(index) else {
            return ManaCost::from_symbol_generic(total);
        };
        card.face = index;
    }
    let mut colored = ManaCost::ZERO;
    if mode != baylee_engine::choice::CastModeKind::Disguise
        && let Some(f) = face(card)
    {
        let mut obj = card_as_object(view, Some(id), card, f);
        if mode == baylee_engine::choice::CastModeKind::Prototype
            && let Some(prototype) = f.prototype
        {
            // CR 718.3a-b: cast restrictions and taxes see the prototype's
            // alternative characteristics, including its colored mana cost.
            obj.colors = prototype.cost.colors();
            obj.mana_value = prototype.cost.cmc();
            obj.power = Some(prototype.power);
            obj.toughness = Some(prototype.toughness);
        }
        let agent = HeuristicAgent::new(crate::AIProfile::default());
        for perm in &view.battlefield {
            for ability in active_abilities(perm) {
                let AbilityDef::Static(baylee_cards_dsl::StaticAbility {
                    filter,
                    modifier: baylee_cards_dsl::Modifier::SpellsCostMoreMana(extra),
                    ..
                }) = ability
                else {
                    continue;
                };
                if agent.filter_matches(
                    filter,
                    view,
                    &obj,
                    baylee_cards_dsl::ZoneRef::Stack,
                    Some(perm.id),
                ) == Some(true)
                    && colored.cmc().saturating_add(extra.cmc()) <= total
                {
                    colored = colored.combine(extra);
                }
            }
        }
    }
    let rest = total.saturating_sub(colored.cmc());
    colored.with_more_generic(rest)
}
