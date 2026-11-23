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
    if source.colors.is_empty() && source.dear.is_none() {
        return;
    }
    let add_fixed_color = |source: &mut Source, color: ManaColor, count: u8| {
        if count == 0 {
            return;
        }
        if !source.bundle && source.colors.len() == 1 && source.dear.is_none() {
            if source.colors[0] == color {
                source.amount = source.amount.saturating_add(count);
            } else {
                let mut expanded = vec![source.colors[0]; usize::from(source.amount)];
                expanded.extend(std::iter::repeat_n(color, usize::from(count)));
                source.colors = expanded;
                source.bundle = true;
                source.amount = source.amount.saturating_add(count);
            }
        } else if source.bundle && source.dear.is_none() {
            source
                .colors
                .extend(std::iter::repeat_n(color, usize::from(count)));
            source.amount = source.amount.saturating_add(count);
        } else {
            source
                .bonus
                .extend(std::iter::repeat_n(color, usize::from(count)));
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
            // CR 719.3a-b: cast restrictions and taxes see the prototype's
            // alternative characteristics, including its colored mana cost.
            obj.colors = prototype.cost.colors();
            obj.mana_value = prototype.cost.cmc();
            obj.power = Some(prototype.power);
            obj.toughness = Some(prototype.toughness);
        } else if mode == baylee_engine::choice::CastModeKind::Bestow {
            apply_bestow_projection(&mut obj);
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

fn any_target_matches_in_view(
    agent: &HeuristicAgent,
    view: &PlayerView,
    target_filter: &Filter,
    source_id: Option<ObjectId>,
    allow_unknown: bool,
) -> bool {
    let matches_in = |obj: &baylee_view::PublicObject, zone: baylee_cards_dsl::ZoneRef| match agent
        .filter_matches(target_filter, view, obj, zone, source_id)
    {
        Some(true) => true,
        None => allow_unknown,
        Some(false) => false,
    };
    view.battlefield
        .iter()
        .any(|o| !o.status.is_phased_out() && matches_in(o, baylee_cards_dsl::ZoneRef::Battlefield))
        || view
            .stack
            .iter()
            .any(|o| matches_in(o, baylee_cards_dsl::ZoneRef::Stack))
}

/// Evaluates a simple battlefield-observable `Amount` on `perm` (used for
/// `Modifier::ActivatedAbilitiesCostLess` such as *Agatha of the Vile Cauldron*).
fn view_modifier_amount(amount: baylee_cards_dsl::Amount, perm: &baylee_view::PublicObject) -> u32 {
    match amount {
        baylee_cards_dsl::Amount::Fixed(n) => n,
        baylee_cards_dsl::Amount::SourcePower => perm
            .power
            .and_then(|p| u32::try_from(p.max(0)).ok())
            .unwrap_or(0),
        baylee_cards_dsl::Amount::CountersOnSource(kind) => {
            let kind = crate::worth::counter_view(kind);
            perm.counters
                .iter()
                .filter(|c| c.kind == kind)
                .map(|c| u32::from(c.count))
                .sum()
        }
        _ => 0,
    }
}

fn ability_def_is_exhaust(def: &AbilityDef) -> bool {
    matches!(
        def,
        AbilityDef::Activated {
            limit: baylee_cards_dsl::ActivationLimit::Exhaust,
            ..
        } | AbilityDef::ActivatedConditional {
            limit: baylee_cards_dsl::ActivationLimit::Exhaust,
            ..
        } | AbilityDef::ModalActivated {
            limit: baylee_cards_dsl::ActivationLimit::Exhaust,
            ..
        }
    )
}

fn activation_reduction_matches_perm(
    agent: &HeuristicAgent,
    view: &PlayerView,
    source_obj: &baylee_view::PublicObject,
    perm: &baylee_view::PublicObject,
    outer: &Filter,
    inner: Option<&Filter>,
) -> bool {
    let outer_ok = if let Some(inner_f) = inner
        && *inner_f != Filter::Any
    {
        matches!(outer, Filter::Any | Filter::This)
            || agent.filter_matches(
                outer,
                view,
                source_obj,
                baylee_cards_dsl::ZoneRef::Battlefield,
                Some(perm.id),
            ) == Some(true)
    } else {
        agent.filter_matches(
            outer,
            view,
            source_obj,
            baylee_cards_dsl::ZoneRef::Battlefield,
            Some(perm.id),
        ) == Some(true)
    };
    outer_ok
        && inner.is_none_or(|inner_f| {
            *inner_f == Filter::Any
                || agent.filter_matches(
                    inner_f,
                    view,
                    source_obj,
                    baylee_cards_dsl::ZoneRef::Battlefield,
                    Some(perm.id),
                ) == Some(true)
        })
}

/// Effective mana cost of activating `def` on `source` after battlefield
/// generic activation reductions (`AbilitiesCostLess`, `ActivatedAbilitiesCostLess`,
/// CR 118.7a: floored reductions before unfloored reductions).
pub(crate) fn effective_activation_mana(
    view: &PlayerView,
    source: ObjectId,
    def: &AbilityDef,
    base: ManaCost,
) -> ManaCost {
    if base.generic_total() == 0 {
        return base;
    }
    let Some(source_obj) = view.object(source) else {
        return base;
    };
    let agent = HeuristicAgent::new(crate::AIProfile::default());
    let is_exhaust = ability_def_is_exhaust(def);
    let mut mana = base;
    for floored in [true, false] {
        for perm in &view.battlefield {
            for ability in active_abilities(perm) {
                let AbilityDef::Static(sa) = ability else {
                    continue;
                };
                match sa.modifier {
                    baylee_cards_dsl::Modifier::AbilitiesCostLess { amount, minimum }
                        if (minimum > 0) == floored
                            && activation_reduction_matches_perm(
                                &agent, view, source_obj, perm, &sa.filter, None,
                            ) =>
                    {
                        let reduction = amount.min(mana.cmc().saturating_sub(minimum));
                        mana = mana.with_less_generic(reduction);
                    }
                    baylee_cards_dsl::Modifier::ActivatedAbilitiesCostLess {
                        filter,
                        amount,
                        min_mana,
                        exhaust_only,
                    } if (min_mana > 0) == floored
                        && (!exhaust_only || is_exhaust)
                        && activation_reduction_matches_perm(
                            &agent,
                            view,
                            source_obj,
                            perm,
                            &sa.filter,
                            Some(filter),
                        ) =>
                    {
                        let amt = view_modifier_amount(amount, perm);
                        let reduction = amt.min(mana.cmc().saturating_sub(u32::from(min_mana)));
                        mana = mana.with_less_generic(reduction);
                    }
                    _ => {}
                }
            }
        }
    }
    mana
}

/// Full mana price of activating `def` on `source` (increases, generic
/// ability reductions, and target-conditional reductions).
pub(crate) fn ability_mana_price(
    view: &PlayerView,
    legal: &LegalActions,
    source: ObjectId,
    def: &AbilityDef,
    base: ManaCost,
) -> ManaCost {
    let increased = base.with_more_generic(legal.activation_increase(source));
    let reduced = effective_activation_mana(view, source, def, increased);
    let target_reduction = ability_target_mana_reduction(view, source);
    reduced.with_less_mana(&target_reduction, false)
}

/// Optimistic colored+generic mana reduction from battlefield
/// `Modifier::ReduceCostIfTargets { abilities: true, .. }` statics matching
/// `source` for the seat currently acting in `view`.
pub(crate) fn ability_target_mana_reduction(view: &PlayerView, source: ObjectId) -> ManaCost {
    let Some(source_obj) = view.object(source) else {
        return ManaCost::ZERO;
    };
    let you = resource_player(view);
    let agent = HeuristicAgent::new(crate::AIProfile::default());
    let mut total = ManaCost::ZERO;
    for perm in &view.battlefield {
        for ability in active_abilities(perm) {
            let AbilityDef::Static(baylee_cards_dsl::StaticAbility {
                filter,
                modifier:
                    baylee_cards_dsl::Modifier::ReduceCostIfTargets {
                        who,
                        spell_filter,
                        target_filter,
                        reduction,
                        abilities: true,
                    },
                ..
            }) = ability
            else {
                continue;
            };
            if *reduction == ManaCost::ZERO || !player_rel_hits_you(*who, perm.controller == you) {
                continue;
            }
            let outer_ok = matches!(filter, Filter::Any | Filter::This)
                || agent.filter_matches(
                    filter,
                    view,
                    source_obj,
                    baylee_cards_dsl::ZoneRef::Battlefield,
                    Some(perm.id),
                ) == Some(true);
            let inner_ok = matches!(spell_filter, Filter::Any)
                || (matches!(spell_filter, Filter::This) && perm.id == source_obj.id)
                || agent.filter_matches(
                    spell_filter,
                    view,
                    source_obj,
                    baylee_cards_dsl::ZoneRef::Battlefield,
                    Some(perm.id),
                ) == Some(true);
            if outer_ok
                && inner_ok
                && any_target_matches_in_view(&agent, view, target_filter, Some(perm.id), true)
            {
                total = total.combine(reduction);
            }
        }
    }
    total
}

#[derive(Default)]
struct SpellManaReductions {
    colored_only: ManaCost,
    mana: ManaCost,
    target_only: ManaCost,
}

fn collect_board_spell_reductions(
    agent: &HeuristicAgent,
    view: &PlayerView,
    obj: &baylee_view::PublicObject,
    allow_unknown_target: bool,
    out: &mut SpellManaReductions,
) {
    let you = resource_player(view);
    for perm in &view.battlefield {
        for ability in active_abilities(perm) {
            let AbilityDef::Static(sa) = ability else {
                continue;
            };
            match sa.modifier {
                baylee_cards_dsl::Modifier::SpellsCostLessMana {
                    cost: less,
                    colored_only,
                } => {
                    if agent.filter_matches(
                        &sa.filter,
                        view,
                        obj,
                        baylee_cards_dsl::ZoneRef::Stack,
                        Some(perm.id),
                    ) == Some(true)
                    {
                        if colored_only {
                            out.colored_only = out.colored_only.combine(&less);
                        } else {
                            out.mana = out.mana.combine(&less);
                        }
                    }
                }
                baylee_cards_dsl::Modifier::ReduceCostIfTargets {
                    who,
                    spell_filter,
                    target_filter,
                    reduction,
                    abilities: false,
                } if reduction != ManaCost::ZERO
                    && player_rel_hits_you(who, perm.controller == you) =>
                {
                    let outer_ok = matches!(sa.filter, Filter::Any | Filter::This)
                        || agent.filter_matches(
                            &sa.filter,
                            view,
                            obj,
                            baylee_cards_dsl::ZoneRef::Stack,
                            Some(perm.id),
                        ) == Some(true);
                    let inner_ok = matches!(spell_filter, Filter::Any)
                        || agent.filter_matches(
                            spell_filter,
                            view,
                            obj,
                            baylee_cards_dsl::ZoneRef::Stack,
                            Some(perm.id),
                        ) == Some(true);
                    if outer_ok
                        && inner_ok
                        && any_target_matches_in_view(
                            agent,
                            view,
                            target_filter,
                            Some(perm.id),
                            allow_unknown_target,
                        )
                    {
                        out.mana = out.mana.combine(&reduction);
                        out.target_only = out.target_only.combine(&reduction);
                    }
                }
                _ => {}
            }
        }
    }
}

fn collect_self_spell_reductions(
    agent: &HeuristicAgent,
    view: &PlayerView,
    id: ObjectId,
    card: CardIdentity,
    obj: &baylee_view::PublicObject,
    allow_unknown_target: bool,
    out: &mut SpellManaReductions,
) {
    let Some(def) = baylee_cards::by_index(card.index) else {
        return;
    };
    for ability in def.abilities_for_face(usize::from(card.face)) {
        let AbilityDef::Static(sa) = ability else {
            continue;
        };
        if sa.source_zone != baylee_cards_dsl::ZoneRef::Stack {
            continue;
        }
        match sa.modifier {
            baylee_cards_dsl::Modifier::SpellsCostLessMana {
                cost: less,
                colored_only,
            } if matches!(sa.filter, Filter::This | Filter::Any) => {
                if colored_only {
                    out.colored_only = out.colored_only.combine(&less);
                } else {
                    out.mana = out.mana.combine(&less);
                }
            }
            baylee_cards_dsl::Modifier::ReduceCostIfTargets {
                who,
                spell_filter,
                target_filter,
                reduction,
                abilities: false,
            } if reduction != ManaCost::ZERO
                && player_rel_hits_you(who, true)
                && (matches!(sa.filter, Filter::This | Filter::Any)
                    || agent.filter_matches(
                        &sa.filter,
                        view,
                        obj,
                        baylee_cards_dsl::ZoneRef::Stack,
                        Some(id),
                    ) == Some(true))
                && (matches!(spell_filter, Filter::This | Filter::Any)
                    || agent.filter_matches(
                        spell_filter,
                        view,
                        obj,
                        baylee_cards_dsl::ZoneRef::Stack,
                        Some(id),
                    ) == Some(true))
                && any_target_matches_in_view(
                    agent,
                    view,
                    target_filter,
                    Some(id),
                    allow_unknown_target,
                ) =>
            {
                out.mana = out.mana.combine(&reduction);
                out.target_only = out.target_only.combine(&reduction);
            }
            _ => {}
        }
    }
}

fn spell_mana_reduction(
    view: &PlayerView,
    id: ObjectId,
    mut card: CardIdentity,
    legal: &LegalActions,
    mode: baylee_engine::choice::CastModeKind,
    increased: ManaCost,
) -> ManaCost {
    let total = legal.spell_reduction(id, mode);
    let offered = legal.castable_once_paid(id);
    if total == 0 && !offered {
        return increased;
    }
    if let baylee_engine::choice::CastModeKind::Face(index) = mode {
        let Ok(index) = u8::try_from(index) else {
            return increased.with_less_generic(total);
        };
        card.face = index;
    }
    let mut reductions = SpellManaReductions::default();
    if mode != baylee_engine::choice::CastModeKind::Disguise
        && let Some(f) = face(card)
    {
        let mut obj = card_as_object(view, Some(id), card, f);
        if mode == baylee_engine::choice::CastModeKind::Prototype
            && let Some(prototype) = f.prototype
        {
            obj.colors = prototype.cost.colors();
            obj.mana_value = prototype.cost.cmc();
            obj.power = Some(prototype.power);
            obj.toughness = Some(prototype.toughness);
        } else if mode == baylee_engine::choice::CastModeKind::Bestow {
            apply_bestow_projection(&mut obj);
        }
        let agent = HeuristicAgent::new(crate::AIProfile::default());
        let allow_unknown_target = total > 0 || offered;
        collect_board_spell_reductions(&agent, view, &obj, allow_unknown_target, &mut reductions);
        collect_self_spell_reductions(
            &agent,
            view,
            id,
            card,
            &obj,
            allow_unknown_target,
            &mut reductions,
        );
    }
    let colored_only_saved = increased.cmc().saturating_sub(
        increased
            .with_less_mana(&reductions.colored_only, true)
            .cmc(),
    );
    let effective_total = if offered
        && reductions.target_only != ManaCost::ZERO
        && colored_only_saved.saturating_add(reductions.mana.cmc()) > total
    {
        total.saturating_add(reductions.target_only.cmc())
    } else {
        total
    };
    if effective_total == 0 {
        return increased;
    }
    apply_colored_spell_reductions(
        increased,
        effective_total,
        &reductions.colored_only,
        &reductions.mana,
    )
}

pub(crate) fn apply_colored_spell_reductions(
    increased: ManaCost,
    total: u32,
    colored_only_red: &ManaCost,
    mana_red: &ManaCost,
) -> ManaCost {
    let after_colored_only = increased.with_less_mana(colored_only_red, true);
    let colored_only_saved = increased.cmc().saturating_sub(after_colored_only.cmc());
    if colored_only_saved > total {
        return increased.with_less_generic(total);
    }
    let remaining_budget = total - colored_only_saved;
    if mana_red.cmc() <= remaining_budget {
        let after_mana = after_colored_only.with_less_mana(mana_red, false);
        let rest = remaining_budget.saturating_sub(mana_red.cmc());
        after_mana.with_less_generic(rest)
    } else {
        after_colored_only.with_less_generic(remaining_budget)
    }
}

/// `cost` less what the graveyard pays for a delve spell (CR 702.66a): a card
/// each from the caster's graveyard, the card being cast aside, off the generic
/// part only. The subtraction `casting::can_cast` makes: a planner without it
/// never taps for a spell the engine would cast.
pub(crate) fn less_delve(view: &PlayerView, id: ObjectId, cost: ManaCost) -> ManaCost {
    let you = resource_player(view);
    let cards = view
        .graveyards
        .get(usize::from(you.get()))
        .map_or(0, |grave| grave.iter().filter(|card| card.id != id).count());
    cost.with_less_generic(u32::try_from(cards).unwrap_or(u32::MAX))
}

/// What casting `id` under `mode` costs: `cost` — the printed, flashback or
/// alternative cost the caller read, a commander's tax included, `{X}` as the
/// caller left it — with the engine's published increases added (coloured
/// ones kept as pips, [`spell_mana_increase`]) and then its published
/// reductions taken off: CR 601.2f determines the total cost increases first,
/// reductions after, colored reductions via [`spell_mana_reduction`] (CR
/// 118.7b–d), and generic reductions stopping at `{0}` (CR 118.7a).
///
/// The one place a planner applies `LegalActions`' price adjustments. Every
/// planner used to add the increases itself and none subtracted anything, so
/// under Goblin Warchief a Goblin two Mountains pay for was not offered and
/// the one offered tapped three (NEW-03, 3.11.2026). `card` is `None` where
/// the caller cannot name the card, which loses only the colour of a
/// coloured increase or reduction.
pub(crate) fn spell_price(
    view: &PlayerView,
    id: ObjectId,
    card: Option<CardIdentity>,
    legal: &LegalActions,
    mode: baylee_engine::choice::CastModeKind,
    cost: ManaCost,
) -> ManaCost {
    let increase = card.map_or_else(
        || ManaCost::from_symbol_generic(legal.spell_increase(id, mode)),
        |card| spell_mana_increase(view, id, card, legal, mode),
    );
    let increased = cost.combine(&increase);
    card.map_or_else(
        || increased.with_less_generic(legal.spell_reduction(id, mode)),
        |card| spell_mana_reduction(view, id, card, legal, mode, increased),
    )
}

fn apply_bestow_projection(obj: &mut baylee_view::PublicObject) {
    use baylee_core::generated::subtypes;
    use baylee_core::types::{SubtypeSet, TypeSet};
    obj.types = obj
        .types
        .difference(TypeSet::CREATURE)
        .union(TypeSet::ENCHANTMENT);
    if !obj.types.contains(TypeSet::KINDRED) {
        obj.subtypes = obj.subtypes.difference(SubtypeSet::ALL_CREATURE);
    }
    obj.subtypes.insert(subtypes::enchantment::AURA);
    obj.power = None;
    obj.toughness = None;
}
