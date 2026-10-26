//! View-only tactical guards and color-aware mana selection for [`super::NetPlayer4`].

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, Amount, Condition, CostPart, Effect, Filter, ManaRestriction,
    ManaSource, PlayerRel, StepKind, TargetSpec, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::land as land_subtypes;
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::mana::{ManaColor, ManaCost, ManaPayment, ManaSymbol};
use baylee_core::types::TypeSet;
use baylee_engine::choice::{LegalActions, Pending, PlayerAction, granted_slot};
use baylee_view::{CardIdentity, ObjectStatus, Phase, PlayerView, PublicObject, RulesFace, Step};

use crate::deckgen::Rng;
use crate::features4::{
    EncodedCandidates4, public_object_matches_basic_filter, untapped_mana_production,
};

use super::choices::mask_non_priority_single_choice;
use super::sample_categorical;

/// Selects a single candidate index from `cands` (`HeadKind4::SingleChoice`),
/// applying view-only mana-sink masking, main-phase land/proactive guards,
/// target-intent friendly-fire protection, and color-aware mana source redirection.
#[allow(
    clippy::too_many_lines,
    reason = "Applies view-only priority, target-intent, and mana/land redirection guards"
)]
pub(super) fn select_single_choice(
    cands: &EncodedCandidates4,
    cand_logits: &[f32],
    view: &PlayerView,
    pending: &Pending,
    active_source: Option<(ObjectId, Option<u32>)>,
    temperature: f32,
    rng: &mut Rng,
) -> (usize, f32) {
    let n = cands.candidates.len();
    if n == 0 {
        return (0, 0.0);
    }
    let actor = view.decision_player.unwrap_or(view.seat);
    let pool_total = view
        .seat(actor)
        .map_or(0, |s| u32::try_from(s.mana_pool.total()).unwrap_or(0));
    let mut masked_logits = cand_logits[..n].to_vec();

    if n > 1 {
        if let Pending::Priority { legal, .. } = pending {
            let has_sink = has_reachable_mana_sink(view, legal);
            let play_land_first = should_play_land_first(view, legal);
            let tap_self_src = if view.owed.is_some() {
                active_source_requires_tap_self(view, active_source)
            } else {
                None
            };
            let mut trial = masked_logits.clone();
            for (i, cand) in cands.candidates.iter().enumerate() {
                let Some(ref act) = cand.action else {
                    continue;
                };
                if matches!(act, PlayerAction::CancelCast)
                    || is_duplicate_basic_mana_ability(view, legal, act)
                    || tap_self_src.is_some_and(|src| match act {
                        PlayerAction::ActivateManaAbility { source }
                        | PlayerAction::ActivateAbility { source, .. } => *source == src,
                        _ => false,
                    })
                    || ((!has_sink || play_land_first) && is_mana_action(view, act))
                {
                    trial[i] = f32::NEG_INFINITY;
                } else if let PlayerAction::CastSpell { card } = act {
                    if !is_spell_worth_casting_now(view, *card)
                        || !is_x_spell_ready_from_castable(view, *card, pool_total)
                    {
                        trial[i] = f32::NEG_INFINITY;
                    }
                } else if let PlayerAction::ActivateAbility {
                    source,
                    ability_index,
                } = act
                {
                    if is_mana_ability_entry(view, *source, *ability_index) {
                        if !is_mana_ability_usable_now(view, legal, *source, *ability_index) {
                            trial[i] = f32::NEG_INFINITY;
                        }
                    } else if !is_activated_ability_worth_activating_now(
                        view,
                        *source,
                        *ability_index,
                    ) {
                        trial[i] = f32::NEG_INFINITY;
                    }
                }
            }
            if trial.iter().any(|l| l.is_finite()) {
                masked_logits = trial;
            }
        } else {
            let has_non_cancel = cands
                .candidates
                .iter()
                .any(|c| !matches!(c.action, Some(PlayerAction::CancelCast)));
            if has_non_cancel {
                for (i, cand) in cands.candidates.iter().enumerate() {
                    if matches!(cand.action, Some(PlayerAction::CancelCast)) {
                        masked_logits[i] = f32::NEG_INFINITY;
                    }
                }
            }

            if matches!(
                pending,
                Pending::ChooseTargets { .. } | Pending::ChoosePlayer { .. }
            ) {
                let intent = classify_active_target_intent(view, actor, active_source);
                if intent != TargetIntent::Neutral {
                    let mut trial = masked_logits.clone();
                    for (i, cand) in cands.candidates.iter().enumerate() {
                        let Some(ref act) = cand.action else {
                            continue;
                        };
                        match intent {
                            TargetIntent::Hostile => {
                                if is_friendly_target_action(view, actor, act) {
                                    trial[i] = f32::NEG_INFINITY;
                                }
                            }
                            TargetIntent::Beneficial => {
                                if is_hostile_target_action(view, actor, act) {
                                    trial[i] = f32::NEG_INFINITY;
                                }
                            }
                            TargetIntent::Neutral => {}
                        }
                    }
                    if trial.iter().any(|l| l.is_finite()) {
                        masked_logits = trial;
                    }
                }
            } else {
                mask_non_priority_single_choice(
                    cands,
                    &mut masked_logits,
                    view,
                    pending,
                    active_source,
                );
            }
        }
    }

    let logits = &masked_logits[..n];
    let (mut chosen_idx, lp) = sample_categorical(logits, temperature, rng);

    if n > 1
        && let Pending::Priority { legal, .. } = pending
    {
        let actor = view.decision_player.unwrap_or(view.seat);
        let pool_total = view
            .seat(actor)
            .map_or(0, |s| u32::try_from(s.mana_pool.total()).unwrap_or(0));
        let has_sink = has_reachable_mana_sink(view, legal);
        let play_land_first = should_play_land_first(view, legal);

        if chosen_idx == 0
            && matches!(
                cands.candidates[0].action,
                Some(PlayerAction::PassPriority)
            )
        {
            let max_l = logits.iter().copied().fold(f32::NEG_INFINITY, f32::max);
            let sum_exp: f32 = logits
                .iter()
                .filter(|&&l| l.is_finite())
                .map(|&l| (l - max_l).exp())
                .sum();
            let p_pass = if sum_exp > 0.0 && logits[0].is_finite() {
                (logits[0] - max_l).exp() / sum_exp
            } else {
                1.0
            };

            let owed_unmet = matches!(
                view.owed,
                Some(ManaPayment::Fixed(cost)) if pool_total < cost.cmc()
            );
            let floating_with_castable = pool_total > 0
                && legal.castable.iter().any(|&id| {
                    is_spell_worth_casting_now(view, id)
                        && is_x_spell_ready_from_castable(view, id, pool_total)
                });
            let floating_mid_tap = pool_total > 0 && !floating_with_castable && has_sink;
            let main_phase_proactive = view.owed.is_none()
                && view.active == actor
                && view.stack.is_empty()
                && matches!(view.phase, Phase::FirstMain | Phase::SecondMain)
                && (play_land_first || has_proactive_main_phase_play(view, legal))
                && p_pass < 0.90;

            if p_pass < 0.50
                || owed_unmet
                || floating_with_castable
                || floating_mid_tap
                || main_phase_proactive
            {
                let has_unreached_payable = has_reachable_payable_spell(view, legal);
                let prefer_non_mana = floating_with_castable
                    && !has_unreached_payable
                    && (1..n).any(|i| {
                        logits[i].is_finite()
                            && cands.candidates[i]
                                .action
                                .as_ref()
                                .is_some_and(|a| !is_mana_action(view, a))
                    });
                if let Some((best_act, _)) = (1..n)
                    .filter(|&i| logits[i].is_finite())
                    .filter(|&i| {
                        !matches!(
                            cands.candidates[i].action,
                            Some(PlayerAction::PassPriority | PlayerAction::CancelCast)
                        )
                    })
                    .filter(|&i| {
                        !prefer_non_mana
                            || cands.candidates[i]
                                .action
                                .as_ref()
                                .is_some_and(|a| !is_mana_action(view, a))
                    })
                    .map(|i| (i, logits[i]))
                    .max_by(|a, b| a.1.total_cmp(&b.1))
                {
                    chosen_idx = best_act;
                }
            }
        }

        if cands.candidates[chosen_idx]
            .action
            .as_ref()
            .is_some_and(|a| is_mana_action(view, a))
        {
            chosen_idx = redirect_mana_source_for_colors(cands, logits, view, legal, chosen_idx);
        } else if matches!(
            cands.candidates[chosen_idx].action,
            Some(PlayerAction::PlayLand { .. })
        ) {
            chosen_idx = redirect_land_for_hand_colors(cands, logits, view, chosen_idx);
        }
    }

    let out_lp = if temperature <= 1e-4 {
        0.0
    } else {
        let raw_slice = &cand_logits[..n];
        let scale = 1.0 / temperature;
        let max_l = raw_slice.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let sum_exp: f32 = raw_slice
            .iter()
            .filter(|&&l| l.is_finite())
            .map(|&l| ((l - max_l) * scale).exp())
            .sum();
        if sum_exp > 0.0 && raw_slice[chosen_idx].is_finite() {
            ((raw_slice[chosen_idx] - max_l) * scale) - sum_exp.ln()
        } else {
            lp
        }
    };

    (chosen_idx, out_lp)
}

fn is_duplicate_basic_mana_ability(
    view: &PlayerView,
    legal: &LegalActions,
    action: &PlayerAction,
) -> bool {
    if let PlayerAction::ActivateAbility {
        source,
        ability_index: 0,
    } = action
    {
        legal.mana_abilities.contains(source) && is_mana_ability_entry(view, *source, 0)
    } else {
        false
    }
}

pub(super) fn is_mana_action(view: &PlayerView, action: &PlayerAction) -> bool {
    match action {
        PlayerAction::ActivateManaAbility { .. } => true,
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        } => is_mana_ability_entry(view, *source, *ability_index),
        _ => false,
    }
}

pub(super) fn is_mana_ability_entry(
    view: &PlayerView,
    source: ObjectId,
    ability_index: u32,
) -> bool {
    let Some(obj) = view.object(source) else {
        return false;
    };
    if let Some(slot) = granted_slot(ability_index) {
        return obj
            .grant(slot)
            .is_some_and(|g| g.mana_ability || g.mana.is_some())
            || obj.granted_mana_for(slot).is_some();
    }
    if obj
        .board_mana
        .as_ref()
        .is_some_and(|bm| bm.index == ability_index)
    {
        return true;
    }
    let rf = obj.rules.or_else(|| {
        obj.card.map(|c| RulesFace {
            card: c.index,
            face: c.face,
        })
    });
    if let Some(rf) = rf
        && let Some(def) = baylee_cards::by_index(rf.card)
        && let Some(ab) = def
            .abilities_for_face(usize::from(rf.face))
            .get(ability_index as usize)
    {
        return match ab {
            AbilityDef::Activated {
                cost,
                effects,
                targets,
                ..
            } => baylee_cards_dsl::mana_rule::activated_mana_ability(
                cost,
                effects,
                targets.is_some(),
            ),
            AbilityDef::ModalActivated { cost, modes, .. } => {
                baylee_cards_dsl::mana_rule::modal_activated_mana_ability(cost, modes)
            }
            _ => false,
        };
    }
    if let Some(tok_id) = obj.token
        && let Some(tdef) = baylee_cards::tokens::by_token_id(tok_id)
        && let Some(ab) = tdef.abilities.get(ability_index as usize)
    {
        return match ab {
            AbilityDef::Activated {
                cost,
                effects,
                targets,
                ..
            } => baylee_cards_dsl::mana_rule::activated_mana_ability(
                cost,
                effects,
                targets.is_some(),
            ),
            AbilityDef::ModalActivated { cost, modes, .. } => {
                baylee_cards_dsl::mana_rule::modal_activated_mana_ability(cost, modes)
            }
            _ => false,
        };
    }
    false
}

fn should_play_land_first(view: &PlayerView, legal: &LegalActions) -> bool {
    let actor = view.decision_player.unwrap_or(view.seat);
    if view.owed.is_some()
        || view.active != actor
        || !view.stack.is_empty()
        || !matches!(view.phase, Phase::FirstMain | Phase::SecondMain)
        || legal.lands.is_empty()
    {
        return false;
    }
    let pool_total = view
        .seat(actor)
        .map_or(0, |s| u32::try_from(s.mana_pool.total()).unwrap_or(0));
    if pool_total > 0 {
        return false;
    }
    let has_ready_zero_cost_castable = legal.castable.iter().any(|&id| {
        is_spell_worth_casting_now(view, id) && is_x_spell_ready_from_castable(view, id, 0)
    });
    if has_ready_zero_cost_castable {
        return false;
    }
    legal.lands.iter().any(|&id| is_true_land_card(view, id)) || legal.payable.is_empty()
}

fn is_x_spell_ready_from_castable(view: &PlayerView, id: ObjectId, pool_total: u32) -> bool {
    let Some((cost, tax)) = lookup_spell_cost(view, id) else {
        return true;
    };
    if !cost.has_variable() {
        return true;
    }
    let min_x1_cmc = cost.with_x(1).cmc().saturating_add(tax);
    pool_total >= min_x1_cmc
}

pub(super) fn is_true_land_card(view: &PlayerView, id: ObjectId) -> bool {
    let Some(ci) = lookup_card_identity(view, id) else {
        return false;
    };
    let Some(def) = baylee_cards::by_index(ci.index) else {
        return false;
    };
    def.faces
        .get(usize::from(ci.face))
        .is_some_and(|f| f.types.contains(TypeSet::LAND))
}

fn active_source_requires_tap_self(
    view: &PlayerView,
    active_source: Option<(ObjectId, Option<u32>)>,
) -> Option<ObjectId> {
    let (src_id, Some(ab_idx)) = active_source? else {
        return None;
    };
    let ci = lookup_card_identity(view, src_id)?;
    let def = baylee_cards::by_index(ci.index)?;
    let ab = def
        .abilities_for_face(usize::from(ci.face))
        .get(ab_idx as usize)?;
    match ab {
        AbilityDef::Activated { cost, .. } | AbilityDef::ModalActivated { cost, .. } => {
            if cost.parts.iter().any(|p| matches!(p, CostPart::TapSelf)) {
                Some(src_id)
            } else {
                None
            }
        }
        _ => None,
    }
}

#[allow(
    clippy::too_many_lines,
    reason = "Checks owed payment, castable/payable spells, and reachable non-mana permanent abilities"
)]
fn has_reachable_mana_sink(view: &PlayerView, legal: &LegalActions) -> bool {
    let actor = view.decision_player.unwrap_or(view.seat);
    let pool_total = view
        .seat(actor)
        .map_or(0, |s| u32::try_from(s.mana_pool.total()).unwrap_or(0));
    if let Some(ref payment) = view.owed {
        return match payment {
            ManaPayment::Fixed(cost) => {
                pool_total < cost.cmc()
                    || unmet_color_deficit(view, legal)
                        .iter()
                        .any(|&d| d > 0)
            }
            ManaPayment::AnyAmount { .. } => true,
        };
    }
    // Never tap lands in your own Untap, Upkeep, or Draw step on an empty stack:
    // waiting until FirstMain keeps mana for main-phase plays and avoids Draw-step mana burn.
    if view.active == actor
        && matches!(view.step, Step::Untap | Step::Upkeep | Step::Draw)
        && view.stack.is_empty()
    {
        return false;
    }
    if !legal.suspendable.is_empty() || !legal.granted_actions.is_empty() {
        return true;
    }
    let untapped_mana = untapped_mana_production(view, actor);
    let total_reachable = pool_total.saturating_add(untapped_mana);

    // 1. Any X-spell already in legal.castable when untapped mana sources remain
    if untapped_mana > 0
        && legal.castable.iter().any(|&id| {
            lookup_spell_cost(view, id).is_some_and(|(cost, tax)| {
                cost.has_variable() && cost.with_x(1).cmc().saturating_add(tax) <= total_reachable
            }) && is_spell_worth_casting_now(view, id)
        })
    {
        return true;
    }

    // 2. Any spell in legal.payable (not yet in legal.castable) whose cost <= total_reachable
    let board_colors = available_non_sac_colors(view, actor);
    for &id in &legal.payable {
        if legal.castable.contains(&id) {
            continue;
        }
        if !is_spell_worth_casting_now(view, id) {
            continue;
        }
        if let Some((cost, tax)) = lookup_spell_cost(view, id) {
            let min_cmc = if cost.has_variable() {
                cost.with_x(1).cmc()
            } else {
                cost.with_x(0).cmc()
            };
            let req = min_cmc.saturating_add(tax);
            if req <= total_reachable && cost_colors_reachable(&cost, &board_colors) {
                return true;
            }
        } else {
            return true;
        }
    }

    // 3. Any flashback spell in actor's graveyard whose cost <= total_reachable
    let own_main = view.active == actor
        && view.stack.is_empty()
        && matches!(view.phase, Phase::FirstMain | Phase::SecondMain);
    if let Some(gy) = view.graveyards.get(usize::from(actor.get())) {
        for obj in gy {
            if let Some(fb_cost) = obj.flashback {
                let is_instant = obj.types.contains(TypeSet::INSTANT);
                let req = if fb_cost.has_variable() {
                    fb_cost.with_x(1).cmc()
                } else {
                    fb_cost.with_x(0).cmc()
                };
                if (own_main || is_instant)
                    && !legal.castable.contains(&obj.id)
                    && req <= total_reachable
                    && is_spell_worth_casting_now(view, obj.id)
                {
                    return true;
                }
            }
        }
    }

    // 4. Non-mana activated abilities on actor's battlefield permanents that require mana
    let my_creatures = view
        .battlefield_of(actor)
        .filter(|o| o.types.contains(TypeSet::CREATURE))
        .count();
    for obj in view.battlefield_of(actor) {
        if obj.status.contains(ObjectStatus::PHASED_OUT) {
            continue;
        }
        let rf = obj.rules.or_else(|| {
            obj.card.map(|c| RulesFace {
                card: c.index,
                face: c.face,
            })
        });
        let Some(rf) = rf else { continue };
        let Some(def) = baylee_cards::by_index(rf.card) else {
            continue;
        };
        for (ab_idx, ab) in def.abilities_for_face(usize::from(rf.face)).iter().enumerate() {
            if ab.is_mana_ability() {
                continue;
            }
            if !is_activated_ability_worth_activating_now(view, obj.id, ab_idx as u32) {
                continue;
            }
            let (cost, timing, condition) = match ab {
                AbilityDef::Activated { cost, timing, .. } => (cost, *timing, None),
                AbilityDef::ActivatedConditional {
                    cost,
                    timing,
                    condition,
                    ..
                } => (cost, *timing, Some(condition)),
                AbilityDef::ModalActivated {
                    cost,
                    timing,
                    condition,
                    ..
                } => (cost, *timing, condition.as_ref()),
                _ => continue,
            };
            if cost.mana == ManaCost::ZERO {
                continue;
            }
            let already_legal = legal.abilities.contains(&(obj.id, ab_idx as u32));
            let in_unpaid = legal
                .unpaid_abilities
                .iter()
                .any(|&(id, idx, _)| id == obj.id && idx == ab_idx as u32);
            if !already_legal && !in_unpaid {
                continue;
            }
            if already_legal && !cost.mana.has_variable() {
                continue;
            }
            if cost.parts.contains(&CostPart::TapSelf)
                && (obj.status.contains(ObjectStatus::TAPPED) || obj.summoning_sick)
            {
                continue;
            }
            if matches!(timing, ActivationTiming::SorcerySpeed) && !own_main {
                continue;
            }
            if condition.is_some_and(|c| condition_closed(view, actor, c)) {
                continue;
            }
            // Do not tap lands just to sacrifice a land unless we control >= 2 creatures in main/combat
            if obj.types.contains(TypeSet::LAND)
                && cost.parts.contains(&CostPart::SacrificeSelf)
                && (my_creatures < 2 || !own_main)
            {
                continue;
            }
            let req_cmc = if cost.mana.has_variable() {
                cost.mana.with_x(1).cmc()
            } else {
                cost.mana.with_x(0).cmc()
            };
            if req_cmc <= total_reachable && (pool_total < req_cmc || cost.mana.has_variable()) {
                return true;
            }
        }
    }

    false
}

fn has_proactive_main_phase_play(view: &PlayerView, legal: &LegalActions) -> bool {
    let actor = view.decision_player.unwrap_or(view.seat);
    let pool_total = view
        .seat(actor)
        .map_or(0, |s| u32::try_from(s.mana_pool.total()).unwrap_or(0));
    let total_reachable = pool_total.saturating_add(untapped_mana_production(view, actor));

    for &id in legal.castable.iter().chain(legal.payable.iter()) {
        if !is_spell_worth_casting_now(view, id) {
            continue;
        }
        if let Some((cost, tax)) = lookup_spell_cost(view, id) {
            let min_cmc = if cost.has_variable() {
                cost.with_x(1).cmc()
            } else {
                cost.with_x(0).cmc()
            };
            if min_cmc.saturating_add(tax) > total_reachable {
                continue;
            }
        }
        let Some(ci) = lookup_card_identity(view, id) else {
            continue;
        };
        let Some(def) = baylee_cards::by_index(ci.index) else {
            continue;
        };
        let Some(face) = def.faces.get(usize::from(ci.face)) else {
            continue;
        };
        let proactive_types = TypeSet::CREATURE
            .union(TypeSet::PLANESWALKER)
            .union(TypeSet::ARTIFACT)
            .union(TypeSet::ENCHANTMENT)
            .union(TypeSet::BATTLE)
            .union(TypeSet::SORCERY);
        if face.types.intersects(proactive_types) {
            return true;
        }
    }
    false
}

pub(super) fn is_spell_worth_casting_now(view: &PlayerView, id: ObjectId) -> bool {
    if !is_spell_worth_casting_now_inner(view, id) {
        return false;
    }
    if should_hold_draw_spell_for_creature(view, id) {
        return false;
    }
    true
}

#[allow(
    clippy::too_many_lines,
    reason = "Checks X affordability, ETB self-life loss, and spell effect/target preconditions"
)]
fn is_spell_worth_casting_now_inner(view: &PlayerView, id: ObjectId) -> bool {
    let actor = view.decision_player.unwrap_or(view.seat);
    let my_life = view.seat(actor).map_or(20, |s| s.life);
    let pool_total = view
        .seat(actor)
        .map_or(0, |s| u32::try_from(s.mana_pool.total()).unwrap_or(0));
    let total_reachable = pool_total.saturating_add(untapped_mana_production(view, actor));
    if let Some((cost, tax)) = lookup_spell_cost(view, id)
        && cost.has_variable()
        && cost.with_x(1).cmc().saturating_add(tax) > total_reachable
    {
        return false;
    }
    let Some(ci) = lookup_card_identity(view, id) else {
        return true;
    };
    let Some(def) = baylee_cards::by_index(ci.index) else {
        return true;
    };
    let abilities = def.abilities_for_face(usize::from(ci.face));

    // Check ETB triggers on permanent spells for lethal self-life-loss (e.g. Rathi Fiend)
    for ab in abilities {
        if let AbilityDef::Triggered {
            trigger: Trigger::ETB,
            effects,
            ..
        } = ab
        {
            let etb_self_loss: i32 = effects
                .iter()
                .filter_map(|e| match e {
                    Effect::LoseLife {
                        target: PlayerRel::You | PlayerRel::EachPlayer,
                        amount: Amount::Fixed(loss),
                    } => Some(*loss as i32),
                    _ => None,
                })
                .sum();
            if etb_self_loss > 0 && my_life <= etb_self_loss + 1 {
                return false;
            }
        }
    }

    let mut any_spell = false;
    let mut any_open = false;
    for ab in abilities {
        if let AbilityDef::Spell {
            condition,
            effects,
            targets,
            ..
        } = ab
        {
            any_spell = true;
            if condition
                .as_ref()
                .is_some_and(|c| condition_closed(view, actor, c))
            {
                continue;
            }
            if is_counterspell_effects(effects)
                && view.stack.iter().all(|s| s.controller == actor)
            {
                continue;
            }
            let self_loss: i32 = effects
                .iter()
                .filter_map(|e| match e {
                    Effect::LoseLife {
                        target: PlayerRel::You | PlayerRel::EachPlayer,
                        amount: Amount::Fixed(loss),
                    } => Some(*loss as i32),
                    _ => None,
                })
                .sum();
            if self_loss > 0 && my_life <= self_loss + 3 {
                continue;
            }
            let chosen_loss: i32 = effects
                .iter()
                .filter_map(|e| match e {
                    Effect::LoseLife {
                        target: PlayerRel::Chosen,
                        amount: Amount::Fixed(loss),
                    } => Some(*loss as i32),
                    _ => None,
                })
                .sum();
            let chosen_draw = effects.iter().any(|e| {
                matches!(
                    e,
                    Effect::DrawCardsFor {
                        who: PlayerRel::Chosen,
                        ..
                    }
                )
            });
            if chosen_draw && chosen_loss > 0 {
                let opp_lethal = view
                    .seats
                    .iter()
                    .any(|s| s.player != actor && s.life > 0 && s.life <= chosen_loss);
                if !opp_lethal && my_life <= chosen_loss + 3 {
                    continue;
                }
            }
            if let Some(req) = targets
                && (req.min >= 1 || req.count_is_x)
                && let TargetSpec::Object(filter) = req.spec
            {
                if is_pure_pump_effects(effects)
                    && !view
                        .battlefield_of(actor)
                        .any(|o| public_object_matches_basic_filter(actor, o, filter))
                {
                    continue;
                }
                if is_hostile_object_effects(effects)
                    && !view.battlefield.iter().any(|o| {
                        o.controller != actor
                            && !o.types.contains(TypeSet::LAND)
                            && public_object_matches_basic_filter(actor, o, filter)
                    })
                {
                    continue;
                }
            }
            any_open = true;
        }
    }
    !any_spell || any_open
}

#[allow(
    clippy::too_many_lines,
    reason = "Evaluates draw spell holding vs castable creatures in hand"
)]
fn should_hold_draw_spell_for_creature(view: &PlayerView, id: ObjectId) -> bool {
    let actor = view.decision_player.unwrap_or(view.seat);
    let Some(ci) = lookup_card_identity(view, id) else {
        return false;
    };
    let Some(def) = baylee_cards::by_index(ci.index) else {
        return false;
    };
    let Some(face) = def.faces.get(usize::from(ci.face)) else {
        return false;
    };
    let permanent_types = TypeSet::CREATURE
        .union(TypeSet::PLANESWALKER)
        .union(TypeSet::ARTIFACT)
        .union(TypeSet::ENCHANTMENT)
        .union(TypeSet::BATTLE)
        .union(TypeSet::LAND);
    if face.types.intersects(permanent_types) {
        return false;
    }
    let abilities = def.abilities_for_face(usize::from(ci.face));
    let mut has_draw = false;
    let mut has_chosen_loss = 0_i32;
    for ab in abilities {
        if let AbilityDef::Spell { effects, .. } = ab {
            if is_hostile_object_effects(effects)
                || is_pure_pump_effects(effects)
                || is_counterspell_effects(effects)
                || effects
                    .iter()
                    .any(|e| matches!(e, Effect::CreateToken { .. }))
            {
                return false;
            }
            for e in *effects {
                match e {
                    Effect::DrawCards { .. }
                    | Effect::DrawCardsFor {
                        who: PlayerRel::Chosen,
                        ..
                    } => {
                        has_draw = true;
                    }
                    Effect::LoseLife {
                        target: PlayerRel::Chosen,
                        amount: Amount::Fixed(loss),
                    } => {
                        has_chosen_loss += *loss as i32;
                    }
                    _ => {}
                }
            }
        }
    }
    if !has_draw {
        return false;
    }
    if has_chosen_loss > 0
        && view
            .seats
            .iter()
            .any(|s| s.player != actor && s.life > 0 && s.life <= has_chosen_loss)
    {
        return false;
    }

    let pool_total = view
        .seat(actor)
        .map_or(0, |s| u32::try_from(s.mana_pool.total()).unwrap_or(0));
    let total_reachable = pool_total.saturating_add(untapped_mana_production(view, actor));
    let draw_cmc =
        lookup_spell_cost(view, id).map_or(2, |(c, t)| c.with_x(0).cmc().saturating_add(t));
    if draw_cmc == 0 {
        return false;
    }

    let my_creatures = view
        .battlefield_of(actor)
        .filter(|o| o.types.contains(TypeSet::CREATURE))
        .count();
    if my_creatures > 2 && view.hand.len() < 7 {
        return false;
    }

    let board_colors = available_non_sac_colors(view, actor);
    for h in &view.hand {
        if h.id == id {
            continue;
        }
        let Some(odef) = baylee_cards::by_index(h.card.index) else {
            continue;
        };
        let Some(oface) = odef.faces.get(usize::from(h.card.face)) else {
            continue;
        };
        if !oface.types.contains(TypeSet::CREATURE) {
            continue;
        }
        if !is_spell_worth_casting_now_inner(view, h.id) {
            continue;
        }
        if let Some((ocost, otax)) = lookup_spell_cost(view, h.id) {
            let ocmc = ocost.with_x(0).cmc().saturating_add(otax);
            if ocmc <= total_reachable
                && cost_colors_reachable(&ocost, &board_colors)
                && draw_cmc.saturating_add(ocmc) > total_reachable
            {
                return true;
            }
        }
    }
    false
}

fn has_reachable_payable_spell(view: &PlayerView, legal: &LegalActions) -> bool {
    let actor = view.decision_player.unwrap_or(view.seat);
    let pool_total = view
        .seat(actor)
        .map_or(0, |s| u32::try_from(s.mana_pool.total()).unwrap_or(0));
    let total_reachable = pool_total.saturating_add(untapped_mana_production(view, actor));
    let board_colors = available_non_sac_colors(view, actor);
    for &id in &legal.payable {
        if legal.castable.contains(&id) || !is_spell_worth_casting_now(view, id) {
            continue;
        }
        if let Some((cost, tax)) = lookup_spell_cost(view, id) {
            let min_cmc = if cost.has_variable() {
                cost.with_x(1).cmc()
            } else {
                cost.with_x(0).cmc()
            };
            if min_cmc.saturating_add(tax) <= total_reachable
                && cost_colors_reachable(&cost, &board_colors)
            {
                return true;
            }
        }
    }
    false
}

#[allow(
    clippy::too_many_lines,
    reason = "Validates mana ability sacrifice, life, and filter restrictions against current sinks"
)]
fn is_mana_ability_usable_now(
    view: &PlayerView,
    legal: &LegalActions,
    source: ObjectId,
    ability_index: u32,
) -> bool {
    let actor = view.decision_player.unwrap_or(view.seat);
    let Some(obj) = view.object(source) else {
        return true;
    };
    if legal.castable.is_empty()
        && legal.payable.is_empty()
        && legal.suspendable.is_empty()
        && view.owed.is_none()
    {
        if obj.types.contains(TypeSet::CREATURE) {
            return false;
        }
        if !legal.unpaid_abilities.is_empty()
            && legal
                .unpaid_abilities
                .iter()
                .all(|&(id, _, _)| id == source)
        {
            return false;
        }
    }
    let rf = obj.rules.or_else(|| {
        obj.card.map(|c| RulesFace {
            card: c.index,
            face: c.face,
        })
    });
    let Some(rf) = rf else { return true };
    let Some(def) = baylee_cards::by_index(rf.card) else {
        return true;
    };
    let abilities = def.abilities_for_face(usize::from(rf.face));
    let Some(AbilityDef::Activated { cost, effects, .. }) = abilities.get(ability_index as usize)
    else {
        return true;
    };
    for part in cost.parts {
        if let CostPart::SacrificeX(filter) = part
            && !view
                .battlefield_of(actor)
                .any(|o| public_object_matches_basic_filter(actor, o, filter))
        {
            return false;
        }
    }
    if ability_index > 0 {
        let mut any_mana = false;
        let mut any_usable = false;
        for eff in *effects {
            if let Effect::AddMana { restriction, .. } = eff {
                any_mana = true;
                if let Some(rest) = restriction
                    && rest.restricts
                {
                    if payable_admits_mana_restriction(view, obj, legal, rest) {
                        any_usable = true;
                    }
                } else {
                    any_usable = true;
                }
            }
        }
        if any_mana && !any_usable {
            return false;
        }
        let filter_cmc = cost.mana.with_x(0).cmc();
        if filter_cmc > 0 {
            let has_zero_cost_mana_ab = abilities.iter().any(|ab| {
                if let AbilityDef::Activated {
                    cost: c0,
                    effects: e0,
                    targets: t0,
                    ..
                } = ab
                {
                    c0.mana == ManaCost::ZERO
                        && baylee_cards_dsl::mana_rule::activated_mana_ability(
                            c0,
                            e0,
                            t0.is_some(),
                        )
                } else {
                    false
                }
            });
            if has_zero_cost_mana_ab {
                let pool_total = view
                    .seat(actor)
                    .map_or(0, |s| u32::try_from(s.mana_pool.total()).unwrap_or(0));
                let total_reachable = pool_total
                    .saturating_add(untapped_mana_production(view, actor))
                    .saturating_sub(filter_cmc);
                let deficit = unmet_color_deficit(view, legal);
                if !deficit[..5].contains(&1) {
                    return false;
                }
                if view.owed.is_none() {
                    let any_payable_fits = legal.payable.iter().any(|&id| {
                        !legal.castable.contains(&id)
                            && is_spell_worth_casting_now(view, id)
                            && lookup_spell_cost(view, id).is_some_and(|(sc, tax)| {
                                sc.with_x(0).cmc().saturating_add(tax) <= total_reachable
                                    && (0..5).any(|c| {
                                        deficit[c] == 1
                                            && super::choices::count_color_pips(&sc, c) > 0
                                    })
                            })
                    });
                    if !any_payable_fits {
                        return false;
                    }
                }
            }
        }
    }
    true
}

#[allow(
    clippy::too_many_lines,
    reason = "Validates activated ability subtype filters, X costs, and target/sacrifice preconditions"
)]
fn is_activated_ability_worth_activating_now(
    view: &PlayerView,
    source: ObjectId,
    ability_index: u32,
) -> bool {
    let actor = view.decision_player.unwrap_or(view.seat);
    let Some(obj) = view.object(source) else {
        return true;
    };
    let rf = obj.rules.or_else(|| {
        obj.card.map(|c| RulesFace {
            card: c.index,
            face: c.face,
        })
    });
    let Some(rf) = rf else { return true };
    let Some(def) = baylee_cards::by_index(rf.card) else {
        return true;
    };
    let Some(ab) = def
        .abilities_for_face(usize::from(rf.face))
        .get(ability_index as usize)
    else {
        return true;
    };
    let (cost, effects, target_spec) = match ab {
        AbilityDef::Activated {
            cost,
            effects,
            targets,
            ..
        }
        | AbilityDef::ActivatedConditional {
            cost,
            effects,
            targets,
            ..
        } => (cost, *effects, targets.as_ref().map(|r| &r.spec)),
        _ => return true,
    };
    for eff in effects {
        if let Effect::LookAtTopMayTake {
            filter: Filter::HasSubtype(st),
            ..
        } = eff
        {
            let has_matching_subtype = view.battlefield_of(actor).any(|o| o.subtypes.contains(*st))
                || view.hand.iter().any(|h| {
                    baylee_cards::by_index(h.card.index)
                        .and_then(|d| d.faces.get(usize::from(h.card.face)))
                        .is_some_and(|f| f.subtypes.contains(st))
                })
                || view
                    .graveyards
                    .get(usize::from(actor.get()))
                    .is_some_and(|gy| gy.iter().any(|o| o.subtypes.contains(*st)));
            if !has_matching_subtype {
                return false;
            }
        }
    }
    let pool_total = view
        .seat(actor)
        .map_or(0, |s| u32::try_from(s.mana_pool.total()).unwrap_or(0));
    let total_mana = pool_total.saturating_add(untapped_mana_production(view, actor));
    let self_mana = u32::from(obj.types.contains(TypeSet::LAND) || obj.board_mana.is_some());
    let avail_mana = if cost.parts.contains(&CostPart::TapSelf) {
        total_mana.saturating_sub(self_mana)
    } else {
        total_mana
    };
    if cost.mana.has_variable() {
        let base_cmc = cost.mana.with_x(0).cmc();
        if base_cmc > avail_mana {
            return false;
        }
        let x_pips = cost.mana.with_x(1).cmc().saturating_sub(base_cmc).max(1);
        let max_x = avail_mana.saturating_sub(base_cmc) / x_pips;
        if let Some(TargetSpec::CardInGraveyard(filter, PlayerRel::You)) = target_spec {
            let Some(gy) = view.graveyards.get(usize::from(actor.get())) else {
                return false;
            };
            let has_match = gy.iter().any(|o| {
                if !public_object_matches_basic_filter(actor, o, filter) {
                    return false;
                }
                let Some(orf) = o.rules.or_else(|| {
                    o.card.map(|c| RulesFace {
                        card: c.index,
                        face: c.face,
                    })
                }) else {
                    return false;
                };
                let Some(odef) = baylee_cards::by_index(orf.card) else {
                    return false;
                };
                let Some(oface) = odef.faces.get(usize::from(orf.face)) else {
                    return false;
                };
                oface.mana_cost.cmc() <= max_x
            });
            if !has_match {
                return false;
            }
        }
    }
    if let Some(TargetSpec::Object(filter)) = target_spec
        && is_hostile_object_effects(effects)
        && !view.battlefield.iter().any(|o| {
            o.controller != actor
                && !o.types.contains(TypeSet::LAND)
                && public_object_matches_basic_filter(actor, o, filter)
        })
    {
        return false;
    }
    true
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TargetIntent {
    Hostile,
    Beneficial,
    Neutral,
}

pub(super) fn classify_active_target_intent(
    view: &PlayerView,
    actor: PlayerId,
    active_source: Option<(ObjectId, Option<u32>)>,
) -> TargetIntent {
    if let Some((oid, ab_opt)) = active_source
        && let Some(ci) = lookup_card_identity(view, oid)
        && let Some(def) = baylee_cards::by_index(ci.index)
    {
        let abilities = def.abilities_for_face(usize::from(ci.face));
        if let Some(ab_idx) = ab_opt {
            if let Some(
                AbilityDef::Activated { effects, .. }
                | AbilityDef::ActivatedConditional { effects, .. },
            ) = abilities.get(ab_idx as usize)
            {
                return classify_effects_intent(view, actor, effects);
            }
        } else {
            for ab in abilities {
                if let AbilityDef::Spell { effects, .. }
                | AbilityDef::Triggered { effects, .. } = ab
                {
                    let intent = classify_effects_intent(view, actor, effects);
                    if intent != TargetIntent::Neutral {
                        return intent;
                    }
                }
            }
        }
    }
    if let Some(top) = view.top_of_stack()
        && let Some(rf) = top.rules.or_else(|| {
            top.card.map(|c| RulesFace {
                card: c.index,
                face: c.face,
            })
        })
        && let Some(def) = baylee_cards::by_index(rf.card)
    {
        let abilities = def.abilities_for_face(usize::from(rf.face));
        for ab in abilities {
            let effects: &[Effect] = match ab {
                AbilityDef::Spell { effects, .. }
                | AbilityDef::Activated { effects, .. }
                | AbilityDef::ActivatedConditional { effects, .. }
                | AbilityDef::Triggered { effects, .. } => effects,
                _ => continue,
            };
            let intent = classify_effects_intent(view, actor, effects);
            if intent != TargetIntent::Neutral {
                return intent;
            }
        }
    }
    if let Some(newest_obj) = view
        .battlefield_of(actor)
        .max_by_key(|o| o.id)
        && let Some(rf) = newest_obj.rules.or_else(|| {
            newest_obj.card.map(|c| RulesFace {
                card: c.index,
                face: c.face,
            })
        })
        && let Some(def) = baylee_cards::by_index(rf.card)
    {
        for ab in def.abilities_for_face(usize::from(rf.face)) {
            if let AbilityDef::Triggered {
                trigger: Trigger::ETB,
                effects,
                ..
            } = ab
            {
                let intent = classify_effects_intent(view, actor, effects);
                if intent != TargetIntent::Neutral {
                    return intent;
                }
            }
        }
    }
    TargetIntent::Neutral
}

fn classify_effects_intent(view: &PlayerView, actor: PlayerId, effects: &[Effect]) -> TargetIntent {
    if effects.iter().any(|e| matches!(e, Effect::Fight { .. })) {
        return TargetIntent::Neutral;
    }
    let has_draw_chosen = effects.iter().any(|e| {
        matches!(
            e,
            Effect::DrawCardsFor {
                who: PlayerRel::Chosen,
                ..
            } | Effect::DrawCards { .. }
        )
    });
    let lose_life_fixed = effects.iter().find_map(|e| match e {
        Effect::LoseLife {
            amount: Amount::Fixed(n),
            target: PlayerRel::Chosen,
        } => Some(*n),
        _ => None,
    });
    if has_draw_chosen {
        if let Some(loss) = lose_life_fixed {
            let loss_i32 = loss as i32;
            let opp_lethal = view
                .seats
                .iter()
                .any(|s| s.player != actor && s.life > 0 && s.life <= loss_i32);
            if opp_lethal {
                return TargetIntent::Hostile;
            }
            if view.seat(actor).is_some_and(|s| s.life > loss_i32 + 2) {
                return TargetIntent::Beneficial;
            }
            return TargetIntent::Neutral;
        }
        return TargetIntent::Beneficial;
    }
    if is_hostile_object_effects(effects) {
        return TargetIntent::Hostile;
    }
    if is_pure_pump_effects(effects) {
        return TargetIntent::Beneficial;
    }
    TargetIntent::Neutral
}

fn is_friendly_target_action(view: &PlayerView, actor: PlayerId, action: &PlayerAction) -> bool {
    match action {
        PlayerAction::ChoosePlayer(p) => *p == actor,
        PlayerAction::ChooseTargets { objects, players } => {
            if players.contains(&actor) {
                return true;
            }
            objects
                .iter()
                .any(|&oid| view.object(oid).is_some_and(|o| o.controller == actor))
        }
        _ => false,
    }
}

fn is_hostile_target_action(view: &PlayerView, actor: PlayerId, action: &PlayerAction) -> bool {
    match action {
        PlayerAction::ChoosePlayer(p) => *p != actor,
        PlayerAction::ChooseTargets { objects, players } => {
            if players.iter().any(|&p| p != actor) {
                return true;
            }
            objects
                .iter()
                .any(|&oid| view.object(oid).is_some_and(|o| o.controller != actor))
        }
        _ => false,
    }
}

fn is_counterspell_effects(effects: &[Effect]) -> bool {
    effects.iter().any(|e| {
        matches!(
            e,
            Effect::CounterTargetSpell
                | Effect::CounterTargetSpellToExile
                | Effect::CounterTargetSpellOrAbility
                | Effect::CounterTargetAbility
        )
    })
}

fn is_pure_pump_effects(effects: &[Effect]) -> bool {
    !effects.is_empty()
        && effects.iter().any(|e| match e {
            Effect::PumpTarget {
                power, toughness, ..
            }
            | Effect::PumpTargetSlot {
                power, toughness, ..
            } => !power.is_negative() && !toughness.is_negative(),
            Effect::UntapTarget => true,
            _ => false,
        })
        && !is_hostile_object_effects(effects)
}

fn is_hostile_object_effects(effects: &[Effect]) -> bool {
    !effects.is_empty()
        && !effects.iter().any(|e| matches!(e, Effect::Fight { .. }))
        && effects.iter().any(|e| match e {
            Effect::Destroy { .. }
            | Effect::Exile { .. }
            | Effect::DealDamage { .. }
            | Effect::TapTarget => true,
            Effect::PumpTarget {
                power, toughness, ..
            }
            | Effect::PumpTargetSlot {
                power, toughness, ..
            } => power.is_negative() || toughness.is_negative(),
            _ => false,
        })
}

fn condition_closed(view: &PlayerView, actor: PlayerId, cond: &Condition) -> bool {
    match cond {
        Condition::MaxSpeed => view.seat(actor).is_none_or(|s| s.speed != 4),
        Condition::YourTurn => view.active != actor,
        Condition::DuringStep(step) => !match step {
            StepKind::Upkeep => view.step == Step::Upkeep,
            StepKind::Draw => view.step == Step::Draw,
            StepKind::CombatBegin => view.step == Step::CombatBegin,
            StepKind::DeclareAttackers => view.step == Step::DeclareAttackers,
            StepKind::DeclareBlockers => view.step == Step::DeclareBlockers,
            StepKind::CombatDamage => {
                matches!(view.step, Step::CombatDamage | Step::CombatDamageFirst)
            }
            StepKind::CombatEnd => view.step == Step::CombatEnd,
            StepKind::End => view.step == Step::End,
            StepKind::FirstMain => true,
        },
        Condition::All(parts) => parts.iter().any(|p| condition_closed(view, actor, p)),
        _ => false,
    }
}

pub(super) fn lookup_card_identity(view: &PlayerView, id: ObjectId) -> Option<CardIdentity> {
    if let Some(h) = view.hand.iter().find(|h| h.id == id) {
        return Some(h.card);
    }
    for sh in view.controlled_hands.iter().chain(view.shared_hands.iter()) {
        if let Some(h) = sh.cards.iter().find(|h| h.id == id) {
            return Some(h.card);
        }
    }
    view.object(id).and_then(|o| o.card)
}

pub(super) fn lookup_spell_cost(view: &PlayerView, id: ObjectId) -> Option<(ManaCost, u32)> {
    let actor = view.decision_player.unwrap_or(view.seat);
    if let Some(obj) = view.object(id)
        && let Some(fb) = obj.flashback
    {
        return Some((fb, 0));
    }
    let ci = lookup_card_identity(view, id)?;
    let def = baylee_cards::by_index(ci.index)?;
    let face = def.faces.get(usize::from(ci.face))?;
    let cost = face.mana_cost;
    let tax = view
        .seat(actor)
        .and_then(|s| s.commanders.iter().find(|c| c.object == id))
        .map_or(0, |c| c.casts.saturating_mul(2));
    Some((cost, tax))
}

fn redirect_mana_source_for_colors(
    cands: &EncodedCandidates4,
    logits: &[f32],
    view: &PlayerView,
    legal: &LegalActions,
    chosen_idx: usize,
) -> usize {
    let deficit = unmet_color_deficit(view, legal);
    if !deficit.iter().any(|&d| d > 0) {
        return chosen_idx;
    }
    let satisfies = |idx: usize| -> bool {
        let Some(ref act) = cands.candidates[idx].action else {
            return false;
        };
        let colors = action_produced_colors(view, legal, act);
        (0..6).any(|c| deficit[c] > 0 && colors[c])
    };
    if satisfies(chosen_idx) {
        return chosen_idx;
    }
    (0..cands.candidates.len())
        .filter(|&i| logits[i].is_finite())
        .filter(|&i| {
            cands.candidates[i]
                .action
                .as_ref()
                .is_some_and(|a| is_mana_action(view, a))
                && satisfies(i)
        })
        .max_by(|&a, &b| logits[a].total_cmp(&logits[b]))
        .unwrap_or(chosen_idx)
}

fn redirect_land_for_hand_colors(
    cands: &EncodedCandidates4,
    logits: &[f32],
    view: &PlayerView,
    chosen_idx: usize,
) -> usize {
    let actor = view.decision_player.unwrap_or(view.seat);
    let mut board_colors = [false; 5];
    for obj in view.battlefield_of(actor) {
        if obj.status.contains(ObjectStatus::PHASED_OUT) {
            continue;
        }
        let cols = non_sac_source_colors(view, obj.id);
        for c in 0..5 {
            if cols[c] {
                board_colors[c] = true;
            }
        }
    }
    let mut missing = [false; 5];
    for h in &view.hand {
        if is_true_land_card(view, h.id) {
            continue;
        }
        if let Some((cost, _)) = lookup_spell_cost(view, h.id) {
            for c in 0..5 {
                if super::choices::count_color_pips(&cost, c) > 0 && !board_colors[c] {
                    missing[c] = true;
                }
            }
        }
    }
    if !missing.iter().any(|&m| m) {
        return chosen_idx;
    }
    let land_covers_missing = |idx: usize| -> bool {
        let Some(PlayerAction::PlayLand { card }) = cands.candidates[idx].action else {
            return false;
        };
        let cols = land_card_colors(view, card);
        (0..5).any(|c| missing[c] && cols[c])
    };
    if land_covers_missing(chosen_idx) {
        return chosen_idx;
    }
    (0..cands.candidates.len())
        .filter(|&i| logits[i].is_finite() && land_covers_missing(i))
        .max_by(|&a, &b| logits[a].total_cmp(&logits[b]))
        .unwrap_or(chosen_idx)
}

fn unmet_color_deficit(view: &PlayerView, legal: &LegalActions) -> [u32; 6] {
    let actor = view.decision_player.unwrap_or(view.seat);
    let Some(seat) = view.seat(actor) else {
        return [0; 6];
    };
    let have = [
        seat.mana_pool
            .white
            .saturating_add(u32::try_from(seat.mana_pool.restricted[0]).unwrap_or(0)),
        seat.mana_pool
            .blue
            .saturating_add(u32::try_from(seat.mana_pool.restricted[1]).unwrap_or(0)),
        seat.mana_pool
            .black
            .saturating_add(u32::try_from(seat.mana_pool.restricted[2]).unwrap_or(0)),
        seat.mana_pool
            .red
            .saturating_add(u32::try_from(seat.mana_pool.restricted[3]).unwrap_or(0)),
        seat.mana_pool
            .green
            .saturating_add(u32::try_from(seat.mana_pool.restricted[4]).unwrap_or(0)),
        seat.mana_pool
            .colorless
            .saturating_add(u32::try_from(seat.mana_pool.restricted[5]).unwrap_or(0)),
    ];
    let pool_total = u32::try_from(seat.mana_pool.total()).unwrap_or(0);
    let total_reachable = pool_total.saturating_add(untapped_mana_production(view, actor));

    let mut max_need = [0_u32; 6];
    let mut accum_cost = |cost: &ManaCost| {
        let mut need = [0_u32; 6];
        for (sym, count) in cost.runs() {
            let n = u32::from(count);
            match sym {
                ManaSymbol::White | ManaSymbol::TwoOrColor(baylee_core::color::Color::White) => {
                    need[0] += n;
                }
                ManaSymbol::Blue | ManaSymbol::TwoOrColor(baylee_core::color::Color::Blue) => {
                    need[1] += n;
                }
                ManaSymbol::Black | ManaSymbol::TwoOrColor(baylee_core::color::Color::Black) => {
                    need[2] += n;
                }
                ManaSymbol::Red | ManaSymbol::TwoOrColor(baylee_core::color::Color::Red) => {
                    need[3] += n;
                }
                ManaSymbol::Green | ManaSymbol::TwoOrColor(baylee_core::color::Color::Green) => {
                    need[4] += n;
                }
                ManaSymbol::Colorless => {
                    need[5] += n;
                }
                ManaSymbol::Hybrid(pair) | ManaSymbol::HybridPhyrexian(pair) => {
                    let cset = ColorSet::of_pair(pair);
                    let mut indices = Vec::with_capacity(2);
                    for c in Color::ALL {
                        if cset.contains(c) {
                            indices.push(c as usize);
                        }
                    }
                    if indices.iter().all(|&idx| have[idx] <= need[idx]) {
                        for idx in indices {
                            need[idx] += n;
                        }
                    }
                }
                _ => {}
            }
        }
        for c in 0..6 {
            max_need[c] = max_need[c].max(need[c]);
        }
    };

    if let Some(ManaPayment::Fixed(ref cost)) = view.owed {
        accum_cost(cost);
    } else {
        let board_colors = available_non_sac_colors(view, actor);
        for &id in &legal.payable {
            if legal.castable.contains(&id) || !is_spell_worth_casting_now(view, id) {
                continue;
            }
            if let Some((cost, tax)) = lookup_spell_cost(view, id)
                && cost.with_x(0).cmc().saturating_add(tax) <= total_reachable
                && cost_colors_reachable(&cost, &board_colors)
            {
                accum_cost(&cost);
            }
        }
    }

    let mut deficit = [0_u32; 6];
    for c in 0..6 {
        deficit[c] = max_need[c].saturating_sub(have[c]);
    }
    deficit
}

fn available_non_sac_colors(view: &PlayerView, actor: PlayerId) -> [bool; 6] {
    let mut cols = [false; 6];
    if let Some(seat) = view.seat(actor) {
        cols[0] = seat.mana_pool.white > 0 || seat.mana_pool.restricted[0] > 0;
        cols[1] = seat.mana_pool.blue > 0 || seat.mana_pool.restricted[1] > 0;
        cols[2] = seat.mana_pool.black > 0 || seat.mana_pool.restricted[2] > 0;
        cols[3] = seat.mana_pool.red > 0 || seat.mana_pool.restricted[3] > 0;
        cols[4] = seat.mana_pool.green > 0 || seat.mana_pool.restricted[4] > 0;
        cols[5] = seat.mana_pool.colorless > 0 || seat.mana_pool.restricted[5] > 0;
    }
    for obj in view.battlefield_of(actor) {
        if obj.status.contains(ObjectStatus::TAPPED)
            || obj.status.contains(ObjectStatus::PHASED_OUT)
        {
            continue;
        }
        let sc = non_sac_source_colors(view, obj.id);
        for c in 0..6 {
            if sc[c] {
                cols[c] = true;
            }
        }
    }
    cols
}

#[allow(clippy::trivially_copy_pass_by_ref)]
fn cost_colors_reachable(cost: &ManaCost, avail_colors: &[bool; 6]) -> bool {
    for (sym, _) in cost.runs() {
        let ok = match sym {
            ManaSymbol::White | ManaSymbol::TwoOrColor(Color::White) => avail_colors[0],
            ManaSymbol::Blue | ManaSymbol::TwoOrColor(Color::Blue) => avail_colors[1],
            ManaSymbol::Black | ManaSymbol::TwoOrColor(Color::Black) => avail_colors[2],
            ManaSymbol::Red | ManaSymbol::TwoOrColor(Color::Red) => avail_colors[3],
            ManaSymbol::Green | ManaSymbol::TwoOrColor(Color::Green) => avail_colors[4],
            ManaSymbol::Colorless => avail_colors[5],
            ManaSymbol::Hybrid(pair) | ManaSymbol::HybridPhyrexian(pair) => {
                let cset = ColorSet::of_pair(pair);
                Color::ALL
                    .iter()
                    .any(|&c| cset.contains(c) && avail_colors[c as usize])
            }
            _ => true,
        };
        if !ok {
            return false;
        }
    }
    true
}

fn action_produced_colors(
    view: &PlayerView,
    legal: &LegalActions,
    action: &PlayerAction,
) -> [bool; 6] {
    let mut colors = [false; 6];
    let (source, ab_idx_opt) = match action {
        PlayerAction::ActivateManaAbility { source } => (*source, None),
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        } => (*source, Some(*ability_index)),
        _ => return colors,
    };
    let Some(obj) = view.object(source) else {
        return colors;
    };
    if ab_idx_opt.is_none() || ab_idx_opt == Some(0) {
        mark_intrinsic_land_colors(obj, &mut colors);
    }
    if let Some(ref bm) = obj.board_mana
        && ab_idx_opt.is_none_or(|idx| bm.index == idx)
    {
        for mc in &bm.colors {
            colors[mc.index()] = true;
        }
    }
    if let Some(idx) = ab_idx_opt
        && let Some(slot) = granted_slot(idx)
        && let Some(gm) = obj.granted_mana_for(slot)
    {
        for mc in &gm.colors {
            colors[mc.index()] = true;
        }
    }
    let target_ab_idx = ab_idx_opt.unwrap_or(0) as usize;
    let rf = obj.rules.or_else(|| {
        obj.card.map(|c| RulesFace {
            card: c.index,
            face: c.face,
        })
    });
    if let Some(rf) = rf
        && let Some(def) = baylee_cards::by_index(rf.card)
        && let Some(AbilityDef::Activated {
            cost,
            effects,
            targets,
            ..
        }) = def
            .abilities_for_face(usize::from(rf.face))
            .get(target_ab_idx)
        && baylee_cards_dsl::mana_rule::activated_mana_ability(cost, effects, targets.is_some())
    {
        if obj.types.contains(TypeSet::LAND)
            && cost.parts.iter().any(|p| {
                matches!(
                    p,
                    CostPart::Sacrifice(_) | CostPart::SacrificeSelf | CostPart::SacrificeX(_)
                )
            })
        {
            return [false; 6];
        }
        mark_effect_mana_colors_for_payable(view, obj, legal, effects, &mut colors);
    } else if let Some(tok_id) = obj.token
        && let Some(tdef) = baylee_cards::tokens::by_token_id(tok_id)
        && let Some(AbilityDef::Activated {
            cost,
            effects,
            targets,
            ..
        }) = tdef.abilities.get(target_ab_idx)
        && baylee_cards_dsl::mana_rule::activated_mana_ability(cost, effects, targets.is_some())
    {
        mark_effect_mana_colors_for_payable(view, obj, legal, effects, &mut colors);
    }
    colors
}

pub(super) fn non_sac_source_colors(view: &PlayerView, source: ObjectId) -> [bool; 6] {
    let mut colors = [false; 6];
    let Some(obj) = view.object(source) else {
        return colors;
    };
    mark_intrinsic_land_colors(obj, &mut colors);
    if let Some(ref bm) = obj.board_mana {
        for mc in &bm.colors {
            colors[mc.index()] = true;
        }
    }
    for gm in obj.granted_mana_options() {
        for mc in &gm.colors {
            colors[mc.index()] = true;
        }
    }
    let rf = obj.rules.or_else(|| {
        obj.card.map(|c| RulesFace {
            card: c.index,
            face: c.face,
        })
    });
    if let Some(rf) = rf
        && let Some(def) = baylee_cards::by_index(rf.card)
    {
        for ab in def.abilities_for_face(usize::from(rf.face)) {
            if let AbilityDef::Activated {
                cost,
                effects,
                targets,
                ..
            } = ab
                && baylee_cards_dsl::mana_rule::activated_mana_ability(
                    cost,
                    effects,
                    targets.is_some(),
                )
            {
                if obj.types.contains(TypeSet::LAND)
                    && (cost.mana != ManaCost::ZERO
                        || cost.parts.iter().any(|p| {
                            matches!(
                                p,
                                CostPart::Sacrifice(_)
                                    | CostPart::SacrificeSelf
                                    | CostPart::SacrificeX(_)
                            )
                        }))
                {
                    continue;
                }
                mark_unrestricted_effect_mana_colors(obj, effects, &mut colors);
            }
        }
    } else if let Some(tok_id) = obj.token
        && let Some(tdef) = baylee_cards::tokens::by_token_id(tok_id)
    {
        for ab in tdef.abilities {
            if let AbilityDef::Activated {
                cost,
                effects,
                targets,
                ..
            } = ab
                && baylee_cards_dsl::mana_rule::activated_mana_ability(
                    cost,
                    effects,
                    targets.is_some(),
                )
            {
                mark_effect_mana_colors(effects, &mut colors);
            }
        }
    }
    colors
}

pub(super) fn land_card_colors(view: &PlayerView, id: ObjectId) -> [bool; 5] {
    let mut colors = [false; 6];
    let Some(ci) = lookup_card_identity(view, id) else {
        return [false; 5];
    };
    let Some(def) = baylee_cards::by_index(ci.index) else {
        return [false; 5];
    };
    let Some(face) = def.faces.get(usize::from(ci.face)) else {
        return [false; 5];
    };
    if face.subtypes.contains(&land_subtypes::PLAINS) {
        colors[0] = true;
    }
    if face.subtypes.contains(&land_subtypes::ISLAND) {
        colors[1] = true;
    }
    if face.subtypes.contains(&land_subtypes::SWAMP) {
        colors[2] = true;
    }
    if face.subtypes.contains(&land_subtypes::MOUNTAIN) {
        colors[3] = true;
    }
    if face.subtypes.contains(&land_subtypes::FOREST) {
        colors[4] = true;
    }
    for ab in def.abilities_for_face(usize::from(ci.face)) {
        if let AbilityDef::Activated {
            cost,
            effects,
            targets,
            ..
        } = ab
            && baylee_cards_dsl::mana_rule::activated_mana_ability(cost, effects, targets.is_some())
        {
            if cost.parts.iter().any(|p| {
                matches!(
                    p,
                    CostPart::Sacrifice(_) | CostPart::SacrificeSelf | CostPart::SacrificeX(_)
                )
            }) {
                continue;
            }
            mark_effect_mana_colors(effects, &mut colors);
        }
    }
    [colors[0], colors[1], colors[2], colors[3], colors[4]]
}

fn mark_intrinsic_land_colors(obj: &PublicObject, colors: &mut [bool; 6]) {
    if obj.subtypes.contains(land_subtypes::PLAINS) {
        colors[ManaColor::White.index()] = true;
    }
    if obj.subtypes.contains(land_subtypes::ISLAND) {
        colors[ManaColor::Blue.index()] = true;
    }
    if obj.subtypes.contains(land_subtypes::SWAMP) {
        colors[ManaColor::Black.index()] = true;
    }
    if obj.subtypes.contains(land_subtypes::MOUNTAIN) {
        colors[ManaColor::Red.index()] = true;
    }
    if obj.subtypes.contains(land_subtypes::FOREST) {
        colors[ManaColor::Green.index()] = true;
    }
}

fn mark_effect_mana_colors_for_payable(
    view: &PlayerView,
    src_obj: &PublicObject,
    legal: &LegalActions,
    effects: &[Effect],
    colors: &mut [bool; 6],
) {
    for eff in effects {
        if let Effect::AddMana {
            source,
            restriction,
            ..
        } = eff
        {
            if let Some(rest) = restriction
                && rest.restricts
                && !payable_admits_mana_restriction(view, src_obj, legal, rest)
            {
                continue;
            }
            mark_single_mana_source(source, colors);
        }
    }
}

fn face_matches_mana_filter(
    src_obj: &PublicObject,
    types: TypeSet,
    subtypes: &[baylee_core::ids::SubtypeId],
    filter: &Filter,
) -> bool {
    match filter {
        Filter::HasType(ts) => types.intersects(*ts),
        Filter::HasSubtype(st) => subtypes.contains(st),
        Filter::MatchesChosenTypeOfSource => src_obj
            .chosen_subtype
            .is_some_and(|st| subtypes.contains(&st)),
        Filter::Or(list) => list
            .iter()
            .any(|f| face_matches_mana_filter(src_obj, types, subtypes, f)),
        Filter::And(list) => list
            .iter()
            .all(|f| face_matches_mana_filter(src_obj, types, subtypes, f)),
        _ => false,
    }
}

fn payable_admits_mana_restriction(
    view: &PlayerView,
    src_obj: &PublicObject,
    legal: &LegalActions,
    rest: &ManaRestriction,
) -> bool {
    let actor = view.decision_player.unwrap_or(view.seat);
    let pool_total = view
        .seat(actor)
        .map_or(0, |s| u32::try_from(s.mana_pool.total()).unwrap_or(0));
    let total_reachable = pool_total.saturating_add(untapped_mana_production(view, actor));
    for &id in &legal.payable {
        if legal.castable.contains(&id) || !is_spell_worth_casting_now(view, id) {
            continue;
        }
        if let Some((cost, tax)) = lookup_spell_cost(view, id) {
            let min_cmc = if cost.has_variable() {
                cost.with_x(1).cmc()
            } else {
                cost.with_x(0).cmc()
            };
            if min_cmc.saturating_add(tax) > total_reachable {
                continue;
            }
        }
        let Some(ci) = lookup_card_identity(view, id) else {
            continue;
        };
        let Some(def) = baylee_cards::by_index(ci.index) else {
            continue;
        };
        let Some(face) = def.faces.get(usize::from(ci.face)) else {
            continue;
        };
        if face_matches_mana_filter(src_obj, face.types, face.subtypes, rest.filter) {
            return true;
        }
    }
    false
}

fn mark_unrestricted_effect_mana_colors(
    src_obj: &PublicObject,
    effects: &[Effect],
    colors: &mut [bool; 6],
) {
    for eff in effects {
        if let Effect::AddMana {
            source,
            restriction,
            ..
        } = eff
        {
            if let Some(rest) = restriction
                && rest.restricts
                && !(matches!(rest.filter, Filter::MatchesChosenTypeOfSource)
                    && src_obj.chosen_subtype.is_some())
            {
                continue;
            }
            mark_single_mana_source(source, colors);
        }
    }
}

fn mark_effect_mana_colors(effects: &[Effect], colors: &mut [bool; 6]) {
    for eff in effects {
        if let Effect::AddMana { source, .. } = eff {
            mark_single_mana_source(source, colors);
        }
    }
}

fn mark_single_mana_source(source: &ManaSource, colors: &mut [bool; 6]) {
    match source {
        ManaSource::Fixed(mc) => {
            colors[mc.index()] = true;
        }
        ManaSource::Choice(list) | ManaSource::ChosenOr(list) => {
            for mc in *list {
                colors[mc.index()] = true;
            }
        }
        ManaSource::CommanderIdentity
        | ManaSource::LandColor { .. }
        | ManaSource::ColorsAmong { .. }
        | ManaSource::Chosen
        | ManaSource::ExiledWithSource
        | ManaSource::CircledColors => {
            for col in colors.iter_mut().take(5) {
                *col = true;
            }
        }
        ManaSource::IntrinsicBasicLandTypes => {}
    }
}
