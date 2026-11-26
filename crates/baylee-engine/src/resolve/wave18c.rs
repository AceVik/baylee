//! Wave 18c resolution handlers: basic land type choices, APNAP any-player
//! unless-costs, event/destination/mass countering, same-name/attached
//! destruction, LTB-attached control gain, bottom-library tucking, and
//! forced blocked status.

#[allow(clippy::wildcard_imports)]
use super::*;

use crate::zone::Zone;
use baylee_cards_dsl::effect::{AnyPlayerCost, CounterDestination, CounterScope};
use baylee_cards_dsl::{Duration, Filter, Layer, Modifier};
use baylee_core::generated::subtypes::land;
use baylee_core::ids::{EffectId, SubtypeId};

const BASIC_LAND_TYPES: [SubtypeId; 5] = [
    land::PLAINS,
    land::ISLAND,
    land::SWAMP,
    land::MOUNTAIN,
    land::FOREST,
];

/// Prompt `player` to choose one of the five basic land types.
pub(super) fn choose_basic_land(res: &mut Resolution, player: PlayerId) -> Pending {
    res.awaiting = Some(AwaitingOp::ChooseBasicLandType);
    Pending::ChooseSubtype {
        player,
        options: BASIC_LAND_TYPES.to_vec(),
    }
}

/// Resume a resolution suspended on [`AwaitingOp::ChooseBasicLandType`].
#[must_use]
pub(super) fn resume_basic_land(
    state: &mut GameState,
    res: &mut Resolution,
    subtype: SubtypeId,
) -> Flow {
    let since = state.journal.last_seq();
    let Some(AwaitingOp::ChooseBasicLandType) = res.awaiting.take() else {
        unreachable!("resume_basic_land called without ChooseBasicLandType");
    };
    if BASIC_LAND_TYPES.contains(&subtype) {
        res.subject.chosen_subtype = Some(subtype);
    }
    finish_choice(state, res, since)
}

/// Apply `Modifier::SetLandType` with the basic land type previously chosen in this resolution.
pub(super) fn become_chosen_land(
    state: &mut GameState,
    res: &Resolution,
    target: TargetSpec,
    duration: Duration,
) {
    let Some(subtype) = res.subject.chosen_subtype else {
        return;
    };
    if !BASIC_LAND_TYPES.contains(&subtype)
        || !continuous_subjects::source_duration_starts(state, res, duration)
    {
        return;
    }
    let you = res.controller;
    if matches!(duration, Duration::WhileYouControlSource) && !source_still_yours(state, res, you) {
        return;
    }
    if matches!(duration, Duration::WhileSourceTapped { .. })
        && (!source_still_tapped(state, res)
            || !subjects::source(state, res).is_some_and(|s| subjects::on_battlefield(state, s)))
    {
        return;
    }
    let mut targets: smallvec::SmallVec<[ObjectId; 4]> = smallvec::SmallVec::new();
    match target {
        TargetSpec::ThisObject => {
            if let Some(this) = this_to_affect(state, res) {
                targets.push(this);
            }
        }
        TargetSpec::Object(filter) if !res.targeted && res.targets.is_empty() => {
            for id in state.battlefield_seen() {
                if let Some(obj) = state.object(id)
                    && eval::matches_with_context(filter, state, obj, you, res.rule_context())
                {
                    targets.push(id);
                }
            }
        }
        _ => {
            for &id in &res.targets {
                if let Some(target_ref) = continuous_subjects::target(state, res, id) {
                    targets.push(target_ref.object);
                }
            }
        }
    }
    for id in targets {
        let timestamp = state.next_timestamp();
        state.effects.register(crate::effects::ContinuousEffect {
            id: EffectId::new(0),
            source: Some(res.source),
            controller: you,
            origin: crate::effects::EffectOrigin::Resolution,
            layer: Layer::Type,
            timestamp,
            duration,
            filter: crate::effects::EffectFilter::object(state, id),
            modifier: Modifier::SetLandType(subtype),
        });
    }
}

/// Active player and remaining players in turn order (CR 101.4), skipping players who left.
fn apnap_players(state: &GameState) -> Vec<PlayerId> {
    let seats = state.players.len();
    let active = usize::from(state.turn.active.get());
    (0..seats)
        .filter_map(|offset| {
            let player = state.players[(active + offset) % seats].id;
            (!state.has_left(player)).then_some(player)
        })
        .collect()
}

/// Offer `mana` (evaluated now) to each player in APNAP order starting at `next`.
pub(super) fn any_player_pay_mana_amount_offer(
    state: &mut GameState,
    res: &mut Resolution,
    cost: baylee_core::mana::ManaCost,
    on_payment: bool,
    effects: &'static [Effect],
    next: usize,
) -> Option<Pending> {
    let seats = state.players.len();
    for offset in next..seats {
        let seat = (usize::from(state.turn.active.get()) + offset) % seats;
        let player = state.players[seat].id;
        if state.has_left(player) {
            continue;
        }
        res.awaiting = Some(AwaitingOp::PlayerMayPay {
            player,
            cost,
            outcome: ManaPaymentOutcome::AnyPlayerAmount {
                on_payment,
                effects,
                next: offset + 1,
            },
        });
        return Some(Pending::YesNo {
            player,
            prompt: YesNoPrompt::PayMana { cost },
            source: resolving_ability(state, res),
        });
    }
    if !on_payment {
        return run_nested(state, res, effects);
    }
    None
}

fn any_player_cost_options(
    state: &GameState,
    res: &Resolution,
    player: PlayerId,
    cost: AnyPlayerCost,
) -> Vec<ObjectId> {
    let ctx = res.rule_context();
    match cost {
        AnyPlayerCost::DamageToYou(_) => Vec::new(),
        AnyPlayerCost::Sacrifice { filter, .. } => state
            .battlefield_view()
            .into_iter()
            .filter(|&id| {
                state.object(id).is_some_and(|o| {
                    o.controller == player
                        && state.can_be_sacrificed(id, None)
                        && eval::matches_with_context(filter, state, o, player, ctx)
                })
            })
            .collect(),
        AnyPlayerCost::ExileFromGraveyard { filter, .. } => state
            .zones
            .list(ZoneLocation::Graveyard(player))
            .iter()
            .copied()
            .filter(|&id| {
                state
                    .object(id)
                    .is_some_and(|o| eval::matches_with_context(filter, state, o, player, ctx))
            })
            .collect(),
    }
}

/// Start an APNAP loop asking whether any player wants to pay `cost`.
pub(super) fn any_player_pay_cost_offer(
    state: &mut GameState,
    res: &mut Resolution,
    cost: AnyPlayerCost,
    on_payment: bool,
    effects: &'static [Effect],
) -> Option<Pending> {
    let remaining = apnap_players(state);
    any_player_pay_cost_advance(state, res, remaining, cost, on_payment, effects)
}

/// Offer `cost` to the next eligible player in `remaining`, or run `!on_payment` fallback when exhausted.
pub(super) fn any_player_pay_cost_advance(
    state: &mut GameState,
    res: &mut Resolution,
    mut remaining: Vec<PlayerId>,
    cost: AnyPlayerCost,
    on_payment: bool,
    effects: &'static [Effect],
) -> Option<Pending> {
    while !remaining.is_empty() {
        let asked = remaining.remove(0);
        if state.has_left(asked) {
            continue;
        }
        let can_pay = match cost {
            AnyPlayerCost::DamageToYou(_) => true,
            AnyPlayerCost::Sacrifice { count, .. }
            | AnyPlayerCost::ExileFromGraveyard { count, .. } => {
                count == 0
                    || any_player_cost_options(state, res, asked, cost).len() >= usize::from(count)
            }
        };
        if !can_pay {
            continue;
        }
        res.awaiting = Some(AwaitingOp::AnyPlayerPayCost {
            asked,
            remaining,
            step: 0,
            cost,
            on_payment,
            effects,
        });
        return Some(Pending::YesNo {
            player: asked,
            prompt: YesNoPrompt::Generic,
            source: resolving_ability(state, res),
        });
    }
    if !on_payment {
        return run_nested(state, res, effects);
    }
    None
}

/// Handle `YesNo` answer for [`AwaitingOp::AnyPlayerPayCost`].
pub(super) fn any_player_pay_cost_yesno(
    state: &mut GameState,
    res: &mut Resolution,
    answer: bool,
) -> Flow {
    let Some(AwaitingOp::AnyPlayerPayCost {
        asked,
        remaining,
        step,
        cost,
        on_payment,
        effects,
    }) = res.awaiting.take()
    else {
        unreachable!("matched AnyPlayerPayCost");
    };
    if answer && step == 0 {
        match cost {
            AnyPlayerCost::DamageToYou(damage) => {
                if on_payment {
                    let next = res.pc + 1;
                    res.effects.splice(next..next, effects.iter().copied());
                }
                if let Some(pending) =
                    life::deal_to_players(state, res, &[(asked, u32::from(damage))])
                {
                    return Flow::Wait(pending);
                }
                res.pc += 1;
                return run(state, res);
            }
            AnyPlayerCost::Sacrifice { count, .. }
            | AnyPlayerCost::ExileFromGraveyard { count, .. } => {
                if count == 0 {
                    return if on_payment {
                        super::answers::run_fallback(state, res, effects)
                    } else {
                        res.pc += 1;
                        run(state, res)
                    };
                }
                let options = any_player_cost_options(state, res, asked, cost);
                if options.len() >= usize::from(count) {
                    res.awaiting = Some(AwaitingOp::AnyPlayerPayCost {
                        asked,
                        remaining,
                        step: 1,
                        cost,
                        on_payment,
                        effects,
                    });
                    let prompt = match cost {
                        AnyPlayerCost::Sacrifice { .. } => ChoicePrompt::CostSacrifice,
                        AnyPlayerCost::ExileFromGraveyard { .. } => ChoicePrompt::CostExile,
                        AnyPlayerCost::DamageToYou(_) => unreachable!(),
                    };
                    return Flow::Wait(Pending::ChooseCards {
                        player: asked,
                        options,
                        min: count,
                        max: count,
                        prompt,
                        total: None,
                    });
                }
            }
        }
    }
    let since = state.journal.last_seq();
    if let Some(pending) =
        any_player_pay_cost_advance(state, res, remaining, cost, on_payment, effects)
    {
        return next_choice(state, res, since, pending);
    }
    res.pc += 1;
    run(state, res)
}

/// Handle `ChooseCards` answer for [`AwaitingOp::AnyPlayerPayCost`].
#[allow(clippy::too_many_arguments)]
pub(super) fn any_player_pay_cost_cards(
    state: &mut GameState,
    res: &mut Resolution,
    asked: PlayerId,
    remaining: Vec<PlayerId>,
    step: u8,
    cost: AnyPlayerCost,
    on_payment: bool,
    effects: &'static [Effect],
    chosen: &[ObjectId],
    since: u64,
) -> Flow {
    let expected = match cost {
        AnyPlayerCost::Sacrifice { count, .. }
        | AnyPlayerCost::ExileFromGraveyard { count, .. } => count,
        AnyPlayerCost::DamageToYou(_) => 0,
    };
    let options = any_player_cost_options(state, res, asked, cost);
    let distinct = chosen
        .iter()
        .enumerate()
        .all(|(i, id)| !chosen[..i].contains(id));
    if step == 1
        && expected > 0
        && chosen.len() == usize::from(expected)
        && distinct
        && chosen.iter().all(|id| options.contains(id))
    {
        let mut paid_count = 0u8;
        let batch = crate::graveyard_order::PaymentBatch::new(state);
        for &id in chosen {
            let ok = match cost {
                AnyPlayerCost::Sacrifice { .. } => state.sacrifice_object(id, Cause::Cost).is_ok(),
                AnyPlayerCost::ExileFromGraveyard { .. } => {
                    let owner = state.object(id).map_or(asked, |o| o.owner);
                    state
                        .move_object(
                            id,
                            ZoneLocation::Exile(owner),
                            ZonePosition::Top,
                            Cause::Cost,
                        )
                        .is_ok()
                }
                AnyPlayerCost::DamageToYou(_) => false,
            };
            if ok {
                paid_count = paid_count.saturating_add(1);
            }
        }
        batch.finish(state);
        if paid_count == expected {
            if on_payment {
                return super::answers::run_fallback(state, res, effects);
            }
            return finish_choice(state, res, since);
        }
    }
    if let Some(pending) =
        any_player_pay_cost_advance(state, res, remaining, cost, on_payment, effects)
    {
        return next_choice(state, res, since, pending);
    }
    finish_choice(state, res, since)
}

fn counter_destination_location(
    owner: PlayerId,
    dest: Option<CounterDestination>,
) -> (ZoneLocation, ZonePosition) {
    match dest {
        None => (ZoneLocation::Graveyard(owner), ZonePosition::Top),
        Some(CounterDestination::Hand) => (ZoneLocation::Hand(owner), ZonePosition::Top),
        Some(CounterDestination::TopOfLibrary) => (ZoneLocation::Library(owner), ZonePosition::Top),
        Some(CounterDestination::BottomOfLibrary) => {
            (ZoneLocation::Library(owner), ZonePosition::Bottom)
        }
    }
}

fn move_countered_spell(
    state: &mut GameState,
    target_id: ObjectId,
    you: PlayerId,
    dest: Option<CounterDestination>,
) -> bool {
    if !state.spell_can_be_countered(target_id) {
        return false;
    }
    state
        .journal
        .record(GameEvent::SpellCountered { object: target_id });
    let owner = state.object(target_id).map_or(you, |o| o.owner);
    state.remember_damage_source(target_id);
    if let Some(obj) = state.object_mut(target_id) {
        obj.kind = ObjectKind::Card;
    }
    let (loc, pos) = counter_destination_location(owner, dest);
    let _ = state.move_object(target_id, loc, pos, Cause::Effect);
    true
}

/// Counter the triggering spell or ability (`Rider::EventAbility` or `res.event_object`).
pub(super) fn counter_event_spell_or_ability(state: &mut GameState, res: &mut Resolution) {
    let event_ability = state.object(res.on_stack).and_then(|stack| {
        stack.riders.iter().find_map(|rider| match rider {
            crate::object::Rider::EventAbility(reference) => Some(*reference),
            _ => None,
        })
    });
    if let Some(reference) = event_ability {
        super::zones::counter_ability(state, res, reference.object);
        return;
    }
    let Some(ev) = subjects::event(state, res) else {
        return;
    };
    let Some(obj) = state.object(ev.object) else {
        return;
    };
    if obj.zone != Zone::Stack {
        return;
    }
    match obj.kind {
        ObjectKind::Spell => {
            let _ = move_countered_spell(state, ev.object, res.controller, None);
        }
        ObjectKind::AbilityOnStack => {
            super::zones::counter_ability(state, res, ev.object);
        }
        _ => {}
    }
}

/// Counter target spell and move it to `dest` (`CR 903.9b` commander replacement respected).
pub(super) fn counter_target_spell_to(
    state: &mut GameState,
    res: &mut Resolution,
    dest: CounterDestination,
) -> Option<Pending> {
    let target_id = *res.targets.first()?;
    if !state.spell_can_be_countered(target_id) {
        return None;
    }
    let owner = state.object(target_id).map_or(res.controller, |o| o.owner);
    let (loc, _) = counter_destination_location(owner, Some(dest));
    if let Some(pending) = ask_commander_replace(state, res, &[(target_id, loc)]) {
        return Some(pending);
    }
    let _ = move_countered_spell(state, target_id, res.controller, Some(dest));
    None
}

/// Counter every spell or ability on the stack matching `scope` and `filter`.
pub(super) fn counter_all(
    state: &mut GameState,
    res: &mut Resolution,
    scope: CounterScope,
    filter: &'static Filter,
) {
    let you = res.controller;
    let stack = state.zones.list(ZoneLocation::Stack).clone();
    let matching: Vec<(ObjectId, ObjectKind)> = stack
        .into_iter()
        .filter_map(|id| {
            let obj = state.object(id)?;
            let scope_ok = matches!(
                (scope, obj.kind),
                (
                    CounterScope::Spells | CounterScope::SpellsAndAbilities,
                    ObjectKind::Spell
                ) | (
                    CounterScope::Abilities | CounterScope::SpellsAndAbilities,
                    ObjectKind::AbilityOnStack
                )
            );
            (scope_ok && eval::matches_with_context(filter, state, obj, you, res.rule_context()))
                .then_some((id, obj.kind))
        })
        .collect();
    for (id, kind) in matching {
        match kind {
            ObjectKind::Spell => {
                let _ = move_countered_spell(state, id, you, None);
            }
            ObjectKind::AbilityOnStack => {
                super::zones::counter_ability(state, res, id);
            }
            _ => {}
        }
    }
}

/// Put the resolving source spell on the bottom of its owner's library (CR 903.9b respected).
pub(super) fn put_source_on_bottom_of_library(
    state: &mut GameState,
    res: &mut Resolution,
) -> Option<Pending> {
    let source = subjects::moving_source(state, res)?.object;
    let owner = state.object(source).map_or(res.controller, |o| o.owner);
    if let Some(pending) =
        ask_commander_replace(state, res, &[(source, ZoneLocation::Library(owner))])
    {
        return Some(pending);
    }
    let _ = state.move_object(
        source,
        ZoneLocation::Library(owner),
        ZonePosition::Bottom,
        Cause::Effect,
    );
    None
}

/// Destroy target permanent and every other permanent sharing its name matching `filter`.
pub(super) fn destroy_target_and_same_name(
    state: &mut GameState,
    res: &Resolution,
    filter: &'static Filter,
) {
    let Some(target) = res.targets.first().copied() else {
        return;
    };
    let Some(target_obj) = state.object(target).filter(|o| o.zone == Zone::Battlefield) else {
        return;
    };
    let name = target_obj.characteristics().name;
    let mut doomed: Vec<ObjectId> = Vec::new();
    if name == crate::state::NAMELESS {
        doomed.push(target);
    } else {
        for id in state.battlefield_seen() {
            if let Some(obj) = state.object(id)
                && (id == target
                    || (obj.kind == ObjectKind::Permanent
                        && obj.characteristics().name == name
                        && eval::matches_with_context(
                            filter,
                            state,
                            obj,
                            res.controller,
                            res.rule_context(),
                        )))
            {
                doomed.push(id);
            }
        }
    }
    sba::destroy_all(state, &doomed, false);
}

/// Destroy all permanents matching `filter` attached to the target permanent, optionally destroying target too.
pub(super) fn destroy_all_attached_to_target(
    state: &mut GameState,
    res: &Resolution,
    filter: &'static Filter,
    destroy_target: bool,
) {
    let Some(target) = res.targets.first().copied() else {
        return;
    };
    if state
        .object(target)
        .is_none_or(|o| o.zone != Zone::Battlefield)
    {
        return;
    }
    let mut doomed: Vec<ObjectId> = state
        .battlefield_seen()
        .filter(|&id| {
            state.object(id).is_some_and(|o| {
                o.attached_to == Some(target)
                    && eval::matches_with_context(
                        filter,
                        state,
                        o,
                        res.controller,
                        res.rule_context(),
                    )
            })
        })
        .collect();
    if destroy_target && !doomed.contains(&target) {
        doomed.insert(0, target);
    }
    sba::destroy_all(state, &doomed, false);
}

/// Gain control of permanents matching `filter` that are (or were last-known) attached to the first target.
pub(super) fn gain_control_ltb_attached(
    state: &mut GameState,
    res: &Resolution,
    filter: &'static Filter,
    duration: Duration,
) {
    let you = res.controller;
    if state.has_left(you) || !continuous_subjects::source_duration_starts(state, res, duration) {
        return;
    }
    if matches!(duration, Duration::WhileYouControlSource) && !source_still_yours(state, res, you) {
        return;
    }
    let Some(target) = res.targets.first().copied() else {
        return;
    };
    let candidates: Vec<ObjectId> = if state
        .object(target)
        .is_some_and(|o| o.zone == Zone::Battlefield)
    {
        state
            .battlefield_seen()
            .filter(|&id| {
                state
                    .object(id)
                    .is_some_and(|o| o.attached_to == Some(target))
            })
            .collect()
    } else {
        state
            .ltb_attachments
            .iter()
            .rfind(|(host, _)| *host == target)
            .map_or_else(Vec::new, |(_, worn)| worn.clone())
    };
    let matching: Vec<ObjectId> = candidates
        .into_iter()
        .filter(|&id| {
            state.object(id).is_some_and(|o| {
                o.zone == Zone::Battlefield
                    && !o.status.contains(crate::object::Status::PHASED_OUT)
                    && eval::matches_with_context(filter, state, o, you, res.rule_context())
            })
        })
        .collect();
    for id in matching {
        let timestamp = state.next_timestamp();
        state.effects.register(crate::effects::ContinuousEffect {
            id: EffectId::new(0),
            source: Some(res.source),
            controller: you,
            origin: crate::effects::EffectOrigin::Resolution,
            layer: Layer::Control,
            timestamp,
            duration,
            filter: crate::effects::EffectFilter::object(state, id),
            modifier: Modifier::GainControl,
        });
    }
}

/// Mark targeted or filter-matching attacking creatures as blocked (CR 509.1h).
pub(super) fn becomes_blocked(state: &mut GameState, res: &Resolution, target: TargetSpec) {
    let attackers: smallvec::SmallVec<[ObjectId; 4]> = match target {
        TargetSpec::Object(filter) if !res.targeted => state
            .battlefield_seen()
            .filter(|&id| {
                state.combat.is_attacking(id)
                    && state.object(id).is_some_and(|o| {
                        eval::matches_with_context(
                            filter,
                            state,
                            o,
                            res.controller,
                            res.rule_context(),
                        )
                    })
            })
            .collect(),
        _ => res
            .targets
            .iter()
            .copied()
            .filter(|&id| {
                state.combat.is_attacking(id)
                    && state
                        .object(id)
                        .is_some_and(|o| o.zone == Zone::Battlefield)
            })
            .collect(),
    };
    for attacker in attackers {
        if state.combat.mark_blocked(attacker) {
            state
                .journal
                .record(GameEvent::AttackerBlocked { object: attacker });
        }
    }
}

/// Schedule Plasm Capture's delayed mana trigger for the controller's next first main phase.
pub(super) fn delayed_mana_any_color_at_next_main(state: &mut GameState, res: &Resolution) {
    let amount = u16::try_from(target_mana_value(res, state)).unwrap_or(u16::MAX);
    let source_version = source_version(state, res)
        .or_else(|| state.object(res.source).map(|o| o.version))
        .unwrap_or(0);
    state.delayed.push(crate::state::DelayedTrigger {
        controller: res.controller,
        when: crate::state::DelayedWhen::NextFirstMain,
        action: crate::state::DelayedAction::AddManaAnyColor {
            source: res.source,
            source_version,
            amount,
        },
    });
}
