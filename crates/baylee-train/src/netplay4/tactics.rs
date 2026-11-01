//! View-only tactical guards and color-aware mana selection for [`super::NetPlayer4`].

use baylee_cards_dsl::{
    AbilityDef, ActivationTiming, Amount, Condition, CostPart, CounterKind, Effect, Filter,
    ManaRestriction, ManaSource, PlayerRel, SearchDest, StepKind, TargetSpec, Trigger,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::generated::subtypes::land as land_subtypes;
use baylee_core::ids::{ObjectId, PlayerId};
use baylee_core::mana::{ManaColor, ManaCost, ManaPayment, ManaSymbol};
use baylee_core::types::{SupertypeSet, TypeSet};
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
                        | PlayerAction::ActivateAbility { source, .. } => {
                            *source == src && !cand.group_object_ids.iter().any(|&oid| oid != src)
                        }
                        _ => false,
                    })
                    || ((!has_sink || play_land_first) && is_mana_action(view, act))
                {
                    trial[i] = f32::NEG_INFINITY;
                } else if let PlayerAction::CastSpell { card } = act {
                    if !is_spell_worth_casting_now(view, *card)
                        || !is_x_spell_ready_from_castable(view, *card, pool_total)
                        || should_hold_spell_for_better_main_phase_creature(view, legal, *card)
                    {
                        trial[i] = f32::NEG_INFINITY;
                    }
                } else if let PlayerAction::ActivateManaAbility { source } = act {
                    if !is_mana_ability_usable_now(view, legal, *source, 0) {
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
                    ) || is_waiting_for_main_phase_trigger_with_floating_mana(
                        view, legal, pool_total,
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
                    let cand_obj = |cand: &crate::features4::Candidate4| -> Option<ObjectId> {
                        cand.object_id.or_else(|| match &cand.action {
                            Some(PlayerAction::ChooseTargets { objects, players })
                                if objects.len() == 1 && players.is_empty() =>
                            {
                                Some(objects[0])
                            }
                            _ => None,
                        })
                    };
                    let cand_pid = |cand: &crate::features4::Candidate4| -> Option<PlayerId> {
                        cand.player_id.or_else(|| match &cand.action {
                            Some(PlayerAction::ChooseTargets { objects, players })
                                if objects.is_empty() && players.len() == 1 =>
                            {
                                Some(players[0])
                            }
                            Some(PlayerAction::ChoosePlayer(p)) => Some(*p),
                            _ => None,
                        })
                    };
                    let fixed_dmg = super::choices::active_source_fixed_damage(view, active_source);
                    let any_opp_player_target = cands
                        .candidates
                        .iter()
                        .any(|cand| cand_pid(cand).is_some_and(|pid| pid != actor));
                    let any_lethal_creature_kill = fixed_dmg.is_some_and(|dmg| {
                        cands.candidates.iter().any(|cand| {
                            cand_obj(cand)
                                .and_then(|oid| view.object(oid))
                                .is_some_and(|o| {
                                    o.controller != actor
                                        && o.types.contains(TypeSet::CREATURE)
                                        && (o.keywords
                                            & baylee_cards_dsl::KeywordSet::INDESTRUCTIBLE.bits())
                                            == 0
                                        && !(any_opp_player_target
                                            && dmg >= 2
                                            && o.power.unwrap_or(0) <= 0
                                            && o.token.is_some())
                                        && (i32::from(o.toughness.unwrap_or(1))
                                            - i32::from(o.damage))
                                        .max(1)
                                            <= dmg
                                })
                        })
                    });
                    let any_damaged_opp_creature = fixed_dmg.is_some_and(|_| {
                        !any_lethal_creature_kill
                            && cands.candidates.iter().any(|cand| {
                                cand_obj(cand)
                                    .and_then(|oid| view.object(oid))
                                    .is_some_and(|o| {
                                        o.controller != actor
                                            && o.types.contains(TypeSet::CREATURE)
                                            && o.damage > 0
                                            && (o.keywords
                                                & baylee_cards_dsl::KeywordSet::INDESTRUCTIBLE
                                                    .bits())
                                                == 0
                                    })
                            })
                    });
                    let can_follow_up_ping = view.stack.len() >= 2
                        || matches!(
                            view.step,
                            Step::CombatBegin | Step::DeclareAttackers | Step::DeclareBlockers
                        )
                        || pool_total.saturating_add(untapped_mana_production(view, actor)) >= 4;
                    let any_two_hit_opp_creature = fixed_dmg.is_some_and(|dmg| {
                        !any_lethal_creature_kill
                            && can_follow_up_ping
                            && cands.candidates.iter().any(|cand| {
                                cand_obj(cand)
                                    .and_then(|oid| view.object(oid))
                                    .is_some_and(|o| {
                                        o.controller != actor
                                            && o.types.contains(TypeSet::CREATURE)
                                            && (o.keywords
                                                & baylee_cards_dsl::KeywordSet::INDESTRUCTIBLE
                                                    .bits())
                                                == 0
                                            && (i32::from(o.toughness.unwrap_or(1))
                                                - i32::from(o.damage))
                                            .max(1)
                                                <= dmg * 2
                                    })
                            })
                    });
                    let any_urgent_ench = cands.candidates.iter().any(|cand| {
                        cand_obj(cand)
                            .and_then(|oid| view.object(oid))
                            .is_some_and(|o| {
                                o.controller != actor && super::choices::is_urgent_enchantment(o)
                            })
                    });
                    let max_ben_tier = cands
                        .candidates
                        .iter()
                        .filter_map(|cand| cand_obj(cand).and_then(|oid| view.object(oid)))
                        .filter(|o| o.controller == actor && o.types.contains(TypeSet::CREATURE))
                        .map(|o| super::choices::beneficial_creature_target_tier(actor, o))
                        .max()
                        .unwrap_or(0);
                    let mut trial = masked_logits.clone();
                    for (i, cand) in cands.candidates.iter().enumerate() {
                        let Some(ref act) = cand.action else {
                            continue;
                        };
                        match intent {
                            TargetIntent::Hostile => {
                                if is_friendly_target_action(view, actor, act) {
                                    trial[i] = f32::NEG_INFINITY;
                                } else if any_urgent_ench {
                                    let is_urgent = cand_obj(cand)
                                        .and_then(|oid| view.object(oid))
                                        .is_some_and(|o| {
                                            o.controller != actor
                                                && super::choices::is_urgent_enchantment(o)
                                        });
                                    if !is_urgent {
                                        trial[i] = f32::NEG_INFINITY;
                                    }
                                } else if let Some(dmg) = fixed_dmg {
                                    if any_lethal_creature_kill {
                                        let is_lethal_target = cand_obj(cand)
                                            .and_then(|oid| view.object(oid))
                                            .is_some_and(|o| {
                                                o.controller != actor
                                                    && o.types.contains(TypeSet::CREATURE)
                                                    && (o.keywords
                                                        & baylee_cards_dsl::KeywordSet::INDESTRUCTIBLE
                                                            .bits())
                                                        == 0
                                                    && !(any_opp_player_target
                                                        && dmg >= 2
                                                        && o.power.unwrap_or(0) <= 0
                                                        && o.token.is_some())
                                                    && (i32::from(o.toughness.unwrap_or(1))
                                                        - i32::from(o.damage))
                                                    .max(1)
                                                        <= dmg
                                            })
                                            || cand_pid(cand).is_some_and(|pid| {
                                                pid != actor
                                                    && view.seat(pid).map_or(20, |s| s.life) <= dmg
                                            });
                                        if !is_lethal_target {
                                            trial[i] = f32::NEG_INFINITY;
                                        }
                                    } else if any_damaged_opp_creature {
                                        let is_damaged_creature = cand_obj(cand)
                                            .and_then(|oid| view.object(oid))
                                            .is_some_and(|o| {
                                                o.controller != actor
                                                    && o.types.contains(TypeSet::CREATURE)
                                                    && o.damage > 0
                                                    && (o.keywords
                                                        & baylee_cards_dsl::KeywordSet::INDESTRUCTIBLE
                                                            .bits())
                                                        == 0
                                            });
                                        if !is_damaged_creature {
                                            trial[i] = f32::NEG_INFINITY;
                                        }
                                    } else if any_two_hit_opp_creature {
                                        let is_two_hit_creature = cand_obj(cand)
                                            .and_then(|oid| view.object(oid))
                                            .is_some_and(|o| {
                                                o.controller != actor
                                                    && o.types.contains(TypeSet::CREATURE)
                                                    && (o.keywords
                                                        & baylee_cards_dsl::KeywordSet::INDESTRUCTIBLE
                                                            .bits())
                                                        == 0
                                                    && (i32::from(o.toughness.unwrap_or(1))
                                                        - i32::from(o.damage))
                                                    .max(1)
                                                        <= dmg * 2
                                            });
                                        if !is_two_hit_creature {
                                            trial[i] = f32::NEG_INFINITY;
                                        }
                                    } else if !can_follow_up_ping
                                        && any_opp_player_target
                                        && cand_pid(cand).is_none_or(|pid| pid == actor)
                                    {
                                        trial[i] = f32::NEG_INFINITY;
                                    }
                                }
                            }
                            TargetIntent::Beneficial => {
                                if is_hostile_target_action(view, actor, act) {
                                    trial[i] = f32::NEG_INFINITY;
                                } else if max_ben_tier > 0
                                    && let Some(o) = cand_obj(cand).and_then(|oid| view.object(oid))
                                    && o.controller == actor
                                    && o.types.contains(TypeSet::CREATURE)
                                    && super::choices::beneficial_creature_target_tier(actor, o) + 2
                                        < max_ben_tier
                                {
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

        if chosen_idx == 0 && matches!(cands.candidates[0].action, Some(PlayerAction::PassPriority))
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
                && (legal.castable.iter().any(|&id| {
                    is_spell_worth_casting_now(view, id)
                        && is_x_spell_ready_from_castable(view, id, pool_total)
                        && !should_hold_spell_for_better_main_phase_creature(view, legal, id)
                }) || legal.abilities.iter().any(|&(src, ab_idx)| {
                    is_paid_ability_ready_from_legal(view, src, ab_idx, pool_total)
                }));
            let floating_mid_tap = pool_total > 0
                && !floating_with_castable
                && has_sink
                && (view.stack.is_empty()
                    || view.active != actor
                    || !matches!(view.phase, Phase::FirstMain | Phase::SecondMain));
            let draw_ab_ready = ((view.active == actor
                && matches!(view.phase, Phase::FirstMain | Phase::SecondMain))
                || (view.active != actor && view.phase == Phase::Ending))
                && has_proactive_draw_ability_play(view, legal);
            let main_phase_proactive = view.owed.is_none()
                && view.stack.is_empty()
                && (draw_ab_ready
                    || (p_pass < 0.90
                        && ((view.active == actor
                            && matches!(view.phase, Phase::FirstMain | Phase::SecondMain)
                            && (play_land_first
                                || has_proactive_main_phase_play(view, legal)
                                || ((view.phase == Phase::SecondMain || view.hand.is_empty())
                                    && has_sink)))
                            || (view.phase == Phase::Combat
                                && ((view.active == actor
                                    && view.step == Step::DeclareAttackers)
                                    || (view.active != actor
                                        && view.step == Step::DeclareBlockers))
                                && has_proactive_combat_ability_play(view, legal)))));

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
            }
            | AbilityDef::ActivatedConditional {
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
            }
            | AbilityDef::ActivatedConditional {
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

pub(super) fn active_source_requires_tap_self(
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
        AbilityDef::Activated { cost, .. }
        | AbilityDef::ActivatedConditional { cost, .. }
        | AbilityDef::ModalActivated { cost, .. } => {
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
                pool_total < cost.cmc() || unmet_color_deficit(view, legal).iter().any(|&d| d > 0)
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
    if is_waiting_for_main_phase_trigger_with_floating_mana(view, legal, pool_total) {
        return false;
    }
    if !legal.suspendable.is_empty()
        || !legal.foretellable.is_empty()
        || !legal.granted_actions.is_empty()
    {
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
            if req <= total_reachable
                && spell_colors_reachable_inner(view, actor, Some(legal), Some(id), None, &cost)
            {
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
        let Some(abilities) = abilities_of_battlefield_object(obj) else {
            continue;
        };
        for (ab_idx, ab) in abilities.iter().enumerate() {
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
                && (obj.status.contains(ObjectStatus::TAPPED)
                    || (obj.types.contains(TypeSet::CREATURE) && obj.summoning_sick))
            {
                continue;
            }
            if matches!(timing, ActivationTiming::SorcerySpeed) && !own_main {
                continue;
            }
            if condition.is_some_and(|c| condition_closed(view, actor, c)) {
                continue;
            }
            // Do not tap lands just to sacrifice or exile a land unless we control >= 2 creatures in main/combat
            if obj.types.contains(TypeSet::LAND)
                && (cost.parts.contains(&CostPart::SacrificeSelf)
                    || cost.parts.contains(&CostPart::ExileSelf))
                && (my_creatures < 2 || !own_main)
            {
                continue;
            }
            let self_mana = u32::from(
                !obj.status.contains(ObjectStatus::TAPPED)
                    && (obj.types.contains(TypeSet::LAND) || obj.board_mana.is_some()),
            );
            let avail_for_ab = if cost.parts.contains(&CostPart::TapSelf) {
                total_reachable.saturating_sub(self_mana)
            } else {
                total_reachable
            };
            let req_cmc = if cost.mana.has_variable() {
                cost.mana.with_x(1).cmc()
            } else {
                cost.mana.with_x(0).cmc()
            };
            let exclude = cost.parts.contains(&CostPart::TapSelf).then_some(obj.id);
            if req_cmc <= avail_for_ab
                && spell_colors_reachable_inner(view, actor, Some(legal), None, exclude, &cost.mana)
                && (pool_total < req_cmc || cost.mana.has_variable())
            {
                return true;
            }
        }
    }

    false
}

pub(super) fn abilities_of_battlefield_object(obj: &PublicObject) -> Option<&'static [AbilityDef]> {
    if let Some(rf) = obj.rules.or_else(|| {
        obj.card.map(|c| RulesFace {
            card: c.index,
            face: c.face,
        })
    }) {
        return baylee_cards::by_index(rf.card)
            .map(|def| def.abilities_for_face(usize::from(rf.face)));
    }
    obj.token
        .and_then(baylee_cards::tokens::by_token_id)
        .map(|t| t.abilities)
}

fn filter_requires_opponent_controller(filter: &Filter) -> bool {
    match filter {
        Filter::ControlledByOpponent => true,
        Filter::And(list) => list.iter().any(filter_requires_opponent_controller),
        Filter::Or(list) => list.iter().all(filter_requires_opponent_controller),
        _ => false,
    }
}

fn creature_power_matches_etb_filter(power: i32, filter: &Filter) -> bool {
    match filter {
        Filter::PowerAtMost(max_p) => power <= i32::from(*max_p),
        Filter::PowerAtLeast(min_p) => power >= i32::from(*min_p),
        Filter::And(list) => list
            .iter()
            .all(|f| creature_power_matches_etb_filter(power, f)),
        Filter::Or(list) => list
            .iter()
            .any(|f| creature_power_matches_etb_filter(power, f)),
        _ => true,
    }
}

fn has_entered_this_turn_mana_ability(view: &PlayerView, legal: &LegalActions) -> bool {
    let actor = view.decision_player.unwrap_or(view.seat);
    for &(src, ab_idx) in &legal.abilities {
        let Some(obj) = view.object(src) else {
            continue;
        };
        if obj.controller != actor || obj.status.contains(ObjectStatus::TAPPED) {
            continue;
        }
        let Some(abilities) = abilities_of_battlefield_object(obj) else {
            continue;
        };
        if let Some(AbilityDef::ActivatedConditional {
            cost,
            effects,
            targets,
            condition: Condition::SourceMatches(Filter::EnteredThisTurn),
            ..
        }) = abilities.get(ab_idx as usize)
            && baylee_cards_dsl::mana_rule::activated_mana_ability(cost, effects, targets.is_some())
        {
            return true;
        }
    }
    false
}

fn creature_card_board_score(view: &PlayerView, id: ObjectId) -> i32 {
    let Some(ci) = lookup_card_identity(view, id) else {
        return 0;
    };
    let Some(def) = baylee_cards::by_index(ci.index) else {
        return 0;
    };
    let Some(face) = def.faces.get(usize::from(ci.face)) else {
        return 0;
    };
    if !face.types.contains(TypeSet::CREATURE) {
        return 0;
    }
    let p = i32::from(face.power.unwrap_or(0)).max(0);
    let t = i32::from(face.toughness.unwrap_or(0)).max(0);
    let mut bonus = 0_i32;
    for ab in def.abilities_for_face(usize::from(ci.face)) {
        if let AbilityDef::Triggered {
            trigger, effects, ..
        } = ab
        {
            let has_downside = effects.iter().any(|e| {
                matches!(
                    e,
                    Effect::LoseLife {
                        target: PlayerRel::You,
                        ..
                    } | Effect::DealDamage {
                        target: TargetSpec::Player(PlayerRel::You),
                        ..
                    }
                )
            });
            if has_downside {
                bonus -= 2;
            } else if matches!(
                *trigger,
                Trigger::ETB
                    | Trigger::StepBegin {
                        step: StepKind::CombatBegin | StepKind::Upkeep,
                        whose: PlayerRel::You
                    }
            ) {
                bonus += 3;
            }
        }
    }
    (p + t + bonus).max(1)
}

fn is_removal_or_board_affecting_noncreature(view: &PlayerView, id: ObjectId) -> bool {
    let Some(ci) = lookup_card_identity(view, id) else {
        return true;
    };
    let Some(def) = baylee_cards::by_index(ci.index) else {
        return true;
    };
    def.abilities_for_face(usize::from(ci.face))
        .iter()
        .any(|ab| match ab {
            AbilityDef::Spell { effects, .. }
            | AbilityDef::Triggered {
                trigger: Trigger::ETB,
                effects,
                ..
            } => effects.iter().any(|e| {
                matches!(
                    e,
                    Effect::Destroy { .. }
                        | Effect::DestroyAll { .. }
                        | Effect::Exile { .. }
                        | Effect::ExileAll { .. }
                        | Effect::DealDamage {
                            target: TargetSpec::Object(_) | TargetSpec::AnyTarget,
                            ..
                        }
                        | Effect::CreateToken { .. }
                        | Effect::Fight { .. }
                )
            }),
            _ => false,
        })
}

fn creature_has_immediate_or_engine_ability(view: &PlayerView, id: ObjectId) -> bool {
    let Some(ci) = lookup_card_identity(view, id) else {
        return false;
    };
    let Some(def) = baylee_cards::by_index(ci.index) else {
        return false;
    };
    let Some(face) = def.faces.get(usize::from(ci.face)) else {
        return false;
    };
    if (face.keywords.bits()
        & (baylee_cards_dsl::KeywordSet::HASTE.bits()
            | baylee_cards_dsl::KeywordSet::FLYING.bits()))
        != 0
    {
        return true;
    }
    def.abilities_for_face(usize::from(ci.face))
        .iter()
        .any(|ab| {
            matches!(
                ab,
                AbilityDef::Triggered {
                    trigger: Trigger::ETB
                        | Trigger::EntersBattlefield(_)
                        | Trigger::SpellCast(_)
                        | Trigger::StepBegin {
                            whose: PlayerRel::You,
                            ..
                        },
                    ..
                }
            )
        })
}

#[allow(
    clippy::too_many_lines,
    reason = "Compares main-phase creature curve and empty-board noncreature priority"
)]
fn should_hold_spell_for_better_main_phase_creature(
    view: &PlayerView,
    legal: &LegalActions,
    card: ObjectId,
) -> bool {
    let actor = view.decision_player.unwrap_or(view.seat);
    if view.active != actor
        || !matches!(view.phase, Phase::FirstMain | Phase::SecondMain)
        || !view.stack.is_empty()
        || view.owed.is_some()
    {
        return false;
    }
    let Some((cost_low, tax_low)) = lookup_spell_cost(view, card) else {
        return false;
    };
    let cmc_low = cost_low.with_x(0).cmc().saturating_add(tax_low);
    let pool_total = view
        .seat(actor)
        .map_or(0, |s| u32::try_from(s.mana_pool.total()).unwrap_or(0));
    let total_reachable = pool_total.saturating_add(untapped_mana_production(view, actor));
    let score_low = creature_card_board_score(view, card);
    let is_low_creature = score_low > 0;

    let my_creatures = view
        .battlefield_of(actor)
        .filter(|o| {
            o.types.contains(TypeSet::CREATURE) && !o.status.contains(ObjectStatus::PHASED_OUT)
        })
        .count();
    let opp_creatures = view
        .battlefield
        .iter()
        .filter(|o| {
            o.controller != actor
                && o.types.contains(TypeSet::CREATURE)
                && !o.status.contains(ObjectStatus::PHASED_OUT)
        })
        .count();

    let dup_noncreature_on_board = !is_low_creature
        && lookup_card_identity(view, card).is_some_and(|ci| {
            view.battlefield_of(actor).any(|o| {
                !o.status.contains(ObjectStatus::PHASED_OUT)
                    && o.card.is_some_and(|c| c.index == ci.index)
            })
        });
    if !is_low_creature
        && ((my_creatures == 0 && opp_creatures > 0) || dup_noncreature_on_board)
        && !is_removal_or_board_affecting_noncreature(view, card)
    {
        let blocks_creature = legal
            .castable
            .iter()
            .chain(legal.payable.iter())
            .copied()
            .any(|other| {
                other != card
                    && creature_card_board_score(view, other) > 0
                    && is_spell_worth_casting_now(view, other)
                    && lookup_spell_cost(view, other).is_some_and(|(c_hi, t_hi)| {
                        let cmc_hi = c_hi.with_x(0).cmc().saturating_add(t_hi);
                        cmc_hi <= total_reachable
                            && cmc_low + cmc_hi > total_reachable
                            && spell_colors_reachable_inner_opts(
                                view,
                                actor,
                                Some(legal),
                                Some(other),
                                None,
                                &c_hi,
                                false,
                            )
                    })
            });
        if blocks_creature {
            return true;
        }
    }

    if !is_low_creature {
        return false;
    }

    let mut color_supply = [0_u32; 5];
    if let Some(seat) = view.seat(actor) {
        color_supply[0] = seat.mana_pool.white;
        color_supply[1] = seat.mana_pool.blue;
        color_supply[2] = seat.mana_pool.black;
        color_supply[3] = seat.mana_pool.red;
        color_supply[4] = seat.mana_pool.green;
    }
    for o in view.battlefield_of(actor) {
        if o.status.contains(ObjectStatus::TAPPED) || o.status.contains(ObjectStatus::PHASED_OUT) {
            continue;
        }
        let sc = non_sac_source_colors(view, o.id);
        for c in 0..5 {
            if sc[c] {
                color_supply[c] += 1;
            }
        }
    }
    let can_double_spell = legal
        .castable
        .iter()
        .chain(legal.payable.iter())
        .copied()
        .any(|other| {
            other != card
                && is_spell_worth_casting_now(view, other)
                && lookup_spell_cost(view, other).is_some_and(|(c2, t2)| {
                    let cmc2 = c2.with_x(0).cmc().saturating_add(t2);
                    cmc2 > 0
                        && cmc_low + cmc2 <= total_reachable
                        && (0..5).all(|col| {
                            let pips = super::choices::count_color_pips(&cost_low, col)
                                + super::choices::count_color_pips(&c2, col);
                            pips <= color_supply[col]
                        })
                })
        });
    if can_double_spell {
        return false;
    }

    let low_has_engine = creature_has_immediate_or_engine_ability(view, card);
    let low_is_flying_3drop = cmc_low == 3
        && lookup_card_identity(view, card)
            .and_then(|ci| {
                baylee_cards::by_index(ci.index).and_then(|d| d.faces.get(usize::from(ci.face)))
            })
            .is_some_and(|f| f.keywords.contains(baylee_cards_dsl::KeywordSet::FLYING));
    if low_has_engine && !low_is_flying_3drop {
        return false;
    }

    legal
        .payable
        .iter()
        .chain(legal.castable.iter())
        .copied()
        .any(|other| {
            if other == card
                || !is_spell_worth_casting_now(view, other)
                || !creature_has_immediate_or_engine_ability(view, other)
            {
                return false;
            }
            let score_hi = creature_card_board_score(view, other);
            if score_hi < score_low + 2 {
                return false;
            }
            let Some((cost_hi, tax_hi)) = lookup_spell_cost(view, other) else {
                return false;
            };
            let cmc_hi = cost_hi.with_x(0).cmc().saturating_add(tax_hi);
            let better = if low_has_engine {
                cmc_hi > cmc_low && cmc_hi <= 5 && score_hi >= score_low + 3
            } else {
                (cmc_hi == cmc_low + 1) || (cmc_hi == cmc_low && legal.castable.contains(&other))
            };
            better
                && cmc_hi <= total_reachable
                && cmc_low + cmc_hi > total_reachable
                && spell_colors_reachable_inner_opts(
                    view,
                    actor,
                    Some(legal),
                    Some(other),
                    None,
                    &cost_hi,
                    false,
                )
        })
}

fn is_waiting_for_main_phase_trigger_with_floating_mana(
    view: &PlayerView,
    legal: &LegalActions,
    pool_total: u32,
) -> bool {
    let actor = view.decision_player.unwrap_or(view.seat);
    view.active == actor
        && matches!(view.phase, Phase::FirstMain | Phase::SecondMain)
        && !view.stack.is_empty()
        && pool_total > 0
        && legal.castable.is_empty()
        && legal.payable.is_empty()
        && view.hand.iter().any(|c| !is_true_land_card(view, c.id))
}

fn is_paid_ability_ready_from_legal(
    view: &PlayerView,
    source: ObjectId,
    ability_index: u32,
    pool_total: u32,
) -> bool {
    if is_mana_action(
        view,
        &PlayerAction::ActivateAbility {
            source,
            ability_index,
        },
    ) {
        return false;
    }
    if !is_activated_ability_worth_activating_now(view, source, ability_index) {
        return false;
    }
    let Some(obj) = view.object(source) else {
        return false;
    };
    let Some(abilities) = abilities_of_battlefield_object(obj) else {
        return false;
    };
    let Some(
        AbilityDef::Activated { cost, .. }
        | AbilityDef::ActivatedConditional { cost, .. }
        | AbilityDef::ModalActivated { cost, .. },
    ) = abilities.get(ability_index as usize)
    else {
        return false;
    };
    if cost.mana == ManaCost::ZERO {
        return false;
    }
    if !cost.mana.has_variable() {
        return true;
    }
    let actor = view.decision_player.unwrap_or(view.seat);
    pool_total >= cost.mana.with_x(1).cmc() && untapped_mana_production(view, actor) == 0
}

fn has_proactive_combat_ability_play(view: &PlayerView, legal: &LegalActions) -> bool {
    let actor = view.decision_player.unwrap_or(view.seat);
    let pool_total = view
        .seat(actor)
        .map_or(0, |s| u32::try_from(s.mana_pool.total()).unwrap_or(0));
    let total_reachable = pool_total.saturating_add(untapped_mana_production(view, actor));

    for atk in &view.combat.attackers {
        let Some(obj) = view.object(atk.creature) else {
            continue;
        };
        if obj.controller != actor {
            continue;
        }
        let Some(abilities) = abilities_of_battlefield_object(obj) else {
            continue;
        };
        for (ab_idx, ab) in abilities.iter().enumerate() {
            let idx_u32 = ab_idx as u32;
            let in_legal = legal.abilities.contains(&(obj.id, idx_u32))
                || legal
                    .unpaid_abilities
                    .iter()
                    .any(|&(id, idx, _)| id == obj.id && idx == idx_u32);
            if !in_legal || !is_activated_ability_worth_activating_now(view, obj.id, idx_u32) {
                continue;
            }
            if let AbilityDef::Activated { cost, .. }
            | AbilityDef::ActivatedConditional { cost, .. } = ab
                && cost.mana != ManaCost::ZERO
            {
                let req_cmc = cost.mana.with_x(0).cmc();
                let exclude = cost.parts.contains(&CostPart::TapSelf).then_some(obj.id);
                if req_cmc <= total_reachable
                    && spell_colors_reachable_inner(
                        view,
                        actor,
                        Some(legal),
                        None,
                        exclude,
                        &cost.mana,
                    )
                {
                    return true;
                }
            }
        }
    }
    if view.active != actor && view.phase == Phase::Combat && view.step == Step::DeclareBlockers {
        for obj in view.battlefield_of(actor) {
            let Some(abilities) = abilities_of_battlefield_object(obj) else {
                continue;
            };
            for (ab_idx, ab) in abilities.iter().enumerate() {
                let idx_u32 = ab_idx as u32;
                let in_legal = legal.abilities.contains(&(obj.id, idx_u32))
                    || legal
                        .unpaid_abilities
                        .iter()
                        .any(|&(id, idx, _)| id == obj.id && idx == idx_u32);
                if !in_legal || !is_activated_ability_worth_activating_now(view, obj.id, idx_u32) {
                    continue;
                }
                if let AbilityDef::Activated { cost, effects, .. }
                | AbilityDef::ActivatedConditional { cost, effects, .. } = ab
                    && effects
                        .iter()
                        .any(|e| matches!(e, Effect::PreventAllCombatDamageThisTurn))
                {
                    let req_cmc = cost.mana.with_x(0).cmc();
                    let exclude = cost.parts.contains(&CostPart::TapSelf).then_some(obj.id);
                    if req_cmc <= total_reachable
                        && (cost.mana == ManaCost::ZERO
                            || spell_colors_reachable_inner(
                                view,
                                actor,
                                Some(legal),
                                None,
                                exclude,
                                &cost.mana,
                            ))
                    {
                        return true;
                    }
                }
            }
        }
    }
    false
}

#[allow(
    clippy::too_many_lines,
    reason = "Checks reachable proactive main-phase spells and late-game card-draw abilities"
)]
fn has_proactive_main_phase_play(view: &PlayerView, legal: &LegalActions) -> bool {
    let actor = view.decision_player.unwrap_or(view.seat);
    let pool_total = view
        .seat(actor)
        .map_or(0, |s| u32::try_from(s.mana_pool.total()).unwrap_or(0));
    let total_reachable = pool_total.saturating_add(untapped_mana_production(view, actor));
    let has_temp_mana_land = has_entered_this_turn_mana_ability(view, legal);

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
            if min_cmc.saturating_add(tax) > total_reachable
                || !spell_colors_reachable_inner(view, actor, Some(legal), Some(id), None, &cost)
            {
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
        if face.types.contains(TypeSet::INSTANT)
            && (has_temp_mana_land || view.phase == Phase::SecondMain)
        {
            let is_tutor_or_draw = def
                .abilities_for_face(usize::from(ci.face))
                .iter()
                .any(|ab| match ab {
                    AbilityDef::Spell { effects, .. } => effects.iter().any(|e| {
                        matches!(
                            e,
                            Effect::SearchLibrary { .. }
                                | Effect::SearchLibraryBeforeShuffle { .. }
                                | Effect::SearchLibraryUpTo { .. }
                                | Effect::DrawCards { .. }
                        )
                    }),
                    _ => false,
                });
            if is_tutor_or_draw {
                return true;
            }
        }
    }
    false
}

#[allow(
    clippy::too_many_lines,
    reason = "Checks reachable card-draw activated abilities when hand is nearly empty or no main-phase play exists"
)]
fn has_proactive_draw_ability_play(view: &PlayerView, legal: &LegalActions) -> bool {
    let actor = view.decision_player.unwrap_or(view.seat);
    let pool_total = view
        .seat(actor)
        .map_or(0, |s| u32::try_from(s.mana_pool.total()).unwrap_or(0));
    let total_reachable = pool_total.saturating_add(untapped_mana_production(view, actor));

    let lands_in_hand = view
        .hand
        .iter()
        .filter(|h| is_true_land_card(view, h.id))
        .count();
    let max_hand_for_draw = if lands_in_hand == 0 && !view.hand.is_empty() {
        7
    } else {
        5
    };
    let allow_draw_ab = ((view.active == actor
        && (view.phase == Phase::SecondMain
            || view.hand.is_empty()
            || (view.phase == Phase::FirstMain
                && !should_play_land_first(view, legal)
                && !has_proactive_main_phase_play(view, legal))))
        || (view.active != actor && view.phase == Phase::Ending))
        && view.hand.len() <= max_hand_for_draw;
    if allow_draw_ab {
        for obj in view.battlefield_of(actor) {
            if obj.status.contains(ObjectStatus::PHASED_OUT) {
                continue;
            }
            let Some(abilities) = abilities_of_battlefield_object(obj) else {
                continue;
            };
            for (ab_idx, ab) in abilities.iter().enumerate() {
                let idx_u32 = ab_idx as u32;
                let in_legal = legal.abilities.contains(&(obj.id, idx_u32))
                    || legal
                        .unpaid_abilities
                        .iter()
                        .any(|&(id, idx, _)| id == obj.id && idx == idx_u32);
                if !in_legal || !is_activated_ability_worth_activating_now(view, obj.id, idx_u32) {
                    continue;
                }
                let (cost, effects) = match ab {
                    AbilityDef::Activated { cost, effects, .. }
                    | AbilityDef::ActivatedConditional { cost, effects, .. } => (cost, *effects),
                    _ => continue,
                };
                if cost.mana == ManaCost::ZERO {
                    continue;
                }
                if cost.parts.contains(&CostPart::TapSelf)
                    && (obj.status.contains(ObjectStatus::TAPPED)
                        || (obj.types.contains(TypeSet::CREATURE) && obj.summoning_sick))
                {
                    continue;
                }
                let draws_cards = effects.iter().any(|e| {
                    matches!(
                        e,
                        Effect::DrawCards { .. }
                            | Effect::DrawCardsFor {
                                who: PlayerRel::You | PlayerRel::EachPlayer,
                                ..
                            }
                    )
                });
                if !draws_cards {
                    continue;
                }
                let self_mana = u32::from(
                    !obj.status.contains(ObjectStatus::TAPPED)
                        && (obj.types.contains(TypeSet::LAND) || obj.board_mana.is_some()),
                );
                let avail_for_ab = if cost.parts.contains(&CostPart::TapSelf) {
                    total_reachable.saturating_sub(self_mana)
                } else {
                    total_reachable
                };
                let req_cmc = cost.mana.with_x(0).cmc();
                let exclude = cost.parts.contains(&CostPart::TapSelf).then_some(obj.id);
                if req_cmc <= avail_for_ab
                    && spell_colors_reachable_inner(
                        view,
                        actor,
                        Some(legal),
                        None,
                        exclude,
                        &cost.mana,
                    )
                {
                    return true;
                }
            }
        }
    }
    false
}

fn spell_matches_cast_filter(
    actor: PlayerId,
    perm_controller: PlayerId,
    spell_types: TypeSet,
    spell_cmc: u32,
    filter: &Filter,
) -> bool {
    match filter {
        Filter::Any => true,
        Filter::ControlledByYou => perm_controller == actor,
        Filter::ControlledByOpponent => perm_controller != actor,
        Filter::HasType(ts) => spell_types.intersects(*ts),
        Filter::LacksType(ts) => !spell_types.intersects(*ts),
        Filter::CmcAtMost(max_c) => spell_cmc <= *max_c,
        Filter::CmcAtLeast(min_c) => spell_cmc >= *min_c,
        Filter::And(list) => list
            .iter()
            .all(|f| spell_matches_cast_filter(actor, perm_controller, spell_types, spell_cmc, f)),
        Filter::Or(list) => list
            .iter()
            .any(|f| spell_matches_cast_filter(actor, perm_controller, spell_types, spell_cmc, f)),
        _ => false,
    }
}

fn cast_trigger_self_damage(
    view: &PlayerView,
    actor: PlayerId,
    spell_types: TypeSet,
    spell_cmc: u32,
) -> i32 {
    let mut total_loss = 0_i32;
    for obj in &view.battlefield {
        if obj.status.contains(ObjectStatus::PHASED_OUT) {
            continue;
        }
        let Some(rf) = obj.rules.or_else(|| {
            obj.card.map(|c| RulesFace {
                card: c.index,
                face: c.face,
            })
        }) else {
            continue;
        };
        let Some(def) = baylee_cards::by_index(rf.card) else {
            continue;
        };
        for ab in def.abilities_for_face(usize::from(rf.face)) {
            let (filter, effects) = match ab {
                AbilityDef::Triggered {
                    trigger: Trigger::SpellCast(f) | Trigger::SpellCastOrCopied(f),
                    effects,
                    ..
                } => (*f, *effects),
                _ => continue,
            };
            if !spell_matches_cast_filter(actor, obj.controller, spell_types, spell_cmc, filter) {
                continue;
            }
            for e in effects {
                match e {
                    Effect::DealDamage {
                        target: TargetSpec::Player(rel),
                        amount: Amount::Fixed(n),
                        ..
                    }
                    | Effect::LoseLife {
                        target: rel,
                        amount: Amount::Fixed(n),
                    } => {
                        let hits_actor =
                            matches!(rel, PlayerRel::EventPlayer | PlayerRel::EachPlayer)
                                || (matches!(rel, PlayerRel::You) && obj.controller == actor)
                                || (matches!(rel, PlayerRel::EachOpponent)
                                    && obj.controller != actor);
                        if hits_actor {
                            total_loss = total_loss.saturating_add(i32::try_from(*n).unwrap_or(99));
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    total_loss
}

pub(super) fn is_spell_worth_casting_now(view: &PlayerView, id: ObjectId) -> bool {
    if !is_spell_worth_casting_now_inner(view, id) {
        return false;
    }
    if should_hold_draw_spell_for_creature(view, id)
        || should_hold_small_creature_for_curve_topper(view, id)
    {
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
    let my_lib = view.seat(actor).map_or(99, |s| s.library_count);
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

    let face_opt = def.faces.get(usize::from(ci.face));
    let is_creature_spell = face_opt.is_some_and(|f| f.types.contains(TypeSet::CREATURE));
    let spell_power = face_opt.and_then(|f| f.power).map_or(0, i32::from);
    let count_self_draws = |effs: &[Effect]| -> u32 {
        effs.iter()
            .filter_map(|e| match e {
                Effect::DrawCards {
                    amount: Amount::Fixed(n),
                }
                | Effect::DrawCardsFor {
                    who: PlayerRel::You | PlayerRel::EachPlayer,
                    amount: Amount::Fixed(n),
                } => Some(*n),
                Effect::DrawCards { .. }
                | Effect::DrawCardsFor {
                    who: PlayerRel::You | PlayerRel::EachPlayer,
                    ..
                } => Some(1),
                _ => None,
            })
            .sum()
    };
    let mut mandatory_draws: u32 = abilities
        .iter()
        .map(|ab| match ab {
            AbilityDef::Spell { effects, .. }
            | AbilityDef::Triggered {
                trigger: Trigger::ETB,
                effects,
                ..
            } => count_self_draws(effects),
            _ => 0,
        })
        .sum();
    let mut board_etb_draw_engines = 0_usize;
    if is_creature_spell {
        for perm in view.battlefield_of(actor) {
            let Some(p_abs) = abilities_of_battlefield_object(perm) else {
                continue;
            };
            for pab in p_abs {
                if let AbilityDef::Triggered {
                    trigger: Trigger::EntersBattlefield(filter),
                    effects,
                    ..
                } = pab
                    && !matches!(*filter, Filter::This)
                {
                    let d = count_self_draws(effects);
                    if d > 0 {
                        board_etb_draw_engines += 1;
                        if creature_power_matches_etb_filter(spell_power, filter) {
                            mandatory_draws = mandatory_draws.saturating_add(d);
                        }
                    }
                }
            }
        }
    }
    let spell_cmc = lookup_spell_cost(view, id).map_or(0, |(c, _)| c.with_x(0).cmc());
    if let Some(face) = face_opt {
        for perm in &view.battlefield {
            if perm.status.contains(ObjectStatus::PHASED_OUT) {
                continue;
            }
            let Some(p_abs) = abilities_of_battlefield_object(perm) else {
                continue;
            };
            for pab in p_abs {
                if let AbilityDef::Triggered {
                    trigger: Trigger::SpellCast(filter),
                    effects,
                    ..
                } = pab
                    && spell_matches_cast_filter(
                        actor,
                        perm.controller,
                        face.types,
                        spell_cmc,
                        filter,
                    )
                {
                    if perm.controller == actor {
                        mandatory_draws = mandatory_draws.saturating_add(count_self_draws(effects));
                    } else {
                        let opp_forced: u32 = effects
                            .iter()
                            .filter_map(|e| match e {
                                Effect::DrawCardsFor {
                                    who:
                                        PlayerRel::EventPlayer
                                        | PlayerRel::Opponent
                                        | PlayerRel::EachPlayer,
                                    amount: Amount::Fixed(n),
                                } => Some(*n),
                                Effect::DrawCardsFor {
                                    who:
                                        PlayerRel::EventPlayer
                                        | PlayerRel::Opponent
                                        | PlayerRel::EachPlayer,
                                    ..
                                } => Some(1),
                                _ => None,
                            })
                            .sum();
                        mandatory_draws = mandatory_draws.saturating_add(opp_forced);
                        if opp_forced >= 4 && (my_lib <= 40 || view.hand.len() >= 7) {
                            let destroys_ench = abilities.iter().any(|ab| match ab {
                                AbilityDef::Spell { effects: se, .. } => se.iter().any(|e| {
                                    matches!(e, Effect::Destroy { .. } | Effect::Exile { .. })
                                }),
                                AbilityDef::ModalSpell { modes, .. } => {
                                    modes.iter().any(super::choices::mode_destroys_enchantment)
                                }
                                _ => false,
                            });
                            if !destroys_ench {
                                return false;
                            }
                        }
                    }
                }
            }
        }
    }
    if mandatory_draws > 0 && my_lib <= mandatory_draws.saturating_add(3) {
        return false;
    }
    let is_etb_draw_engine = abilities.iter().any(|ab| {
        matches!(
            ab,
            AbilityDef::Triggered {
                trigger: Trigger::EntersBattlefield(filter),
                effects,
                ..
            } if !matches!(*filter, Filter::This) && count_self_draws(effects) > 0
        )
    });
    if is_etb_draw_engine && board_etb_draw_engines >= 1 && (view.hand.len() >= 6 || my_lib <= 20) {
        return false;
    }

    // Check battlefield SpellCast punisher triggers (e.g. Eidolon of the Great Revel, Spellshock, Mai)
    let mut existing_cast_self_loss = 0_i32;
    if let Some(face) = face_opt {
        if face.supertypes.contains(SupertypeSet::LEGENDARY)
            && !face.types.contains(TypeSet::PLANESWALKER)
            && view.battlefield_of(actor).any(|o| {
                !o.status.contains(ObjectStatus::PHASED_OUT)
                    && o.damage == 0
                    && o.card
                        .is_some_and(|c| c.index == ci.index && c.face == ci.face)
            })
        {
            let opp_benefits_from_deaths = view.battlefield.iter().any(|o| {
                o.controller != actor
                    && !o.status.contains(ObjectStatus::PHASED_OUT)
                    && abilities_of_battlefield_object(o).is_some_and(|abs| {
                        abs.iter().any(|ab| {
                            matches!(
                                ab,
                                AbilityDef::Triggered {
                                    trigger: Trigger::Dies(f),
                                    ..
                                } if !matches!(f, Filter::This)
                            )
                        })
                    })
            });
            let has_other_castable_creature = view.hand.iter().any(|h| {
                h.id != id
                    && h.card.index != ci.index
                    && creature_card_board_score(view, h.id) > 0
                    && lookup_spell_cost(view, h.id).is_some_and(|(hc, htax)| {
                        hc.with_x(0).cmc().saturating_add(htax) <= total_reachable
                            && spell_colors_reachable(view, actor, h.id, &hc)
                    })
            });
            let has_value_etb_or_dies = abilities.iter().any(|ab| {
                matches!(
                    ab,
                    AbilityDef::Triggered {
                        trigger: Trigger::ETB
                            | Trigger::Dies(Filter::This)
                            | Trigger::LeavesBattlefield(Filter::This),
                        ..
                    }
                )
            });
            if !has_value_etb_or_dies || opp_benefits_from_deaths || has_other_castable_creature {
                return false;
            }
        }
        let cast_self_loss = cast_trigger_self_damage(view, actor, face.types, spell_cmc);
        existing_cast_self_loss = cast_self_loss;
        if cast_self_loss > 0 && my_life <= cast_self_loss {
            return false;
        }
        if cast_self_loss > 0 && my_life - cast_self_loss <= 4 && !is_creature_spell {
            let is_urgent_removal = abilities.iter().any(|ab| match ab {
                AbilityDef::Spell { effects, .. } => effects.iter().any(|e| {
                    matches!(
                        e,
                        Effect::Destroy { .. } | Effect::Exile { .. } | Effect::CounterTargetSpell
                    )
                }),
                _ => false,
            });
            if !is_urgent_removal {
                return false;
            }
        }
    }

    // Do not deploy symmetric SpellCast punishers when our life is already low or one is already active
    if (my_life <= 8 || existing_cast_self_loss >= 2) && abilities.iter().any(|ab| {
        matches!(
            ab,
            AbilityDef::Triggered {
                trigger: Trigger::SpellCast(filter),
                effects,
                ..
            } if !filter_requires_opponent_controller(filter)
                && effects.iter().any(|e| matches!(
                    e,
                    Effect::DealDamage {
                        target: TargetSpec::Player(PlayerRel::EventPlayer | PlayerRel::EachPlayer),
                        ..
                    } | Effect::LoseLife {
                        target: PlayerRel::EventPlayer | PlayerRel::EachPlayer,
                        ..
                    }
                ))
        )
    }) {
        return false;
    }

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

    if !abilities.is_empty()
        && abilities.iter().all(|ab| {
            matches!(
                ab,
                AbilityDef::Triggered {
                    condition: Some(Condition::HandSizeAtMost(0)),
                    ..
                }
            )
        })
        && view.hand.len() > 2
    {
        return false;
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
            if is_counterspell_effects(effects) && view.stack.iter().all(|s| s.controller == actor)
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
            if self_loss > 0
                && (my_life <= self_loss + 3 || (my_life <= self_loss + 8 && view.hand.len() >= 5))
            {
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
                if !opp_lethal
                    && (my_life <= chosen_loss + 3
                        || (my_life <= chosen_loss + 8 && view.hand.len() >= 5))
                {
                    continue;
                }
            }
            if let Some(req) = targets
                && (req.min >= 1 || req.count_is_x)
            {
                if let TargetSpec::Object(filter) = req.spec {
                    if is_pure_pump_effects(effects) {
                        if !view
                            .battlefield_of(actor)
                            .any(|o| public_object_matches_basic_filter(actor, o, filter))
                        {
                            continue;
                        }
                        let is_instant =
                            face_opt.is_some_and(|f| f.types.contains(TypeSet::INSTANT));
                        let grants_haste = effects.iter().any(|e| {
                            matches!(
                                e,
                                Effect::PumpTarget { keywords, .. }
                                    if keywords.contains(baylee_cards_dsl::KeywordSet::HASTE)
                            )
                        });
                        let has_sick_creature = view.battlefield_of(actor).any(|o| {
                            o.types.contains(TypeSet::CREATURE)
                                && o.summoning_sick
                                && !o.status.contains(ObjectStatus::TAPPED)
                                && !o.status.contains(ObjectStatus::PHASED_OUT)
                                && (o.keywords & baylee_cards_dsl::KeywordSet::HASTE.bits()) == 0
                                && public_object_matches_basic_filter(actor, o, filter)
                        });
                        if is_instant
                            && matches!(view.phase, Phase::FirstMain | Phase::SecondMain)
                            && view.stack.is_empty()
                            && !(grants_haste && has_sick_creature)
                        {
                            continue;
                        }
                    }
                    if is_hostile_object_effects(effects)
                        && !view.battlefield.iter().any(|o| {
                            o.controller != actor
                                && (!o.types.contains(TypeSet::LAND)
                                    || filter_targets_lands(filter))
                                && public_object_matches_basic_filter(actor, o, filter)
                        })
                    {
                        continue;
                    }
                } else if matches!(req.spec, TargetSpec::AnyTarget)
                    && effects.len() == 1
                    && let Effect::DealDamage {
                        amount: Amount::Fixed(dmg_u),
                        ..
                    } = effects[0]
                    && dmg_u <= 2
                    && view.phase != Phase::Combat
                {
                    let dmg = i32::try_from(dmg_u).unwrap_or(2);
                    let opp_low_life = view
                        .seats
                        .iter()
                        .any(|s| s.player != actor && s.life > 0 && s.life <= dmg * 2 + 1);
                    let opp_has_killable_target = view.battlefield.iter().any(|o| {
                        o.controller != actor
                            && !o.status.contains(ObjectStatus::PHASED_OUT)
                            && ((o.types.contains(TypeSet::CREATURE)
                                && (o.keywords
                                    & baylee_cards_dsl::KeywordSet::INDESTRUCTIBLE.bits())
                                    == 0
                                && ((i32::from(o.toughness.unwrap_or(1)) - i32::from(o.damage))
                                    .max(1)
                                    <= dmg
                                    || o.damage > 0))
                                || (o.types.contains(TypeSet::PLANESWALKER)
                                    && i32::from(o.loyalty.unwrap_or(99)) <= dmg))
                    });
                    let has_spellcast_payoff = view.battlefield_of(actor).any(|o| {
                        !o.status.contains(ObjectStatus::PHASED_OUT)
                            && abilities_of_battlefield_object(o).is_some_and(|abs| {
                                abs.iter().any(|ab| {
                                    matches!(
                                        ab,
                                        AbilityDef::Triggered {
                                            trigger: Trigger::SpellCast(_),
                                            ..
                                        }
                                    )
                                })
                            })
                    });
                    if !opp_low_life && !opp_has_killable_target && !has_spellcast_payoff {
                        continue;
                    }
                }
            }
            // Do not cast self-loot spells (e.g. Faithless Looting: draw N, discard M with M >= N)
            // when we have fewer than M other cards in hand, or no excess lands/uncastables to pitch.
            let draw_fixed: u32 = effects
                .iter()
                .filter_map(|e| match e {
                    Effect::DrawCards {
                        amount: Amount::Fixed(n),
                    } => Some(*n),
                    _ => None,
                })
                .sum();
            let disc_self: u32 = effects
                .iter()
                .filter_map(|e| match e {
                    Effect::DiscardForPlayers {
                        who: PlayerRel::You,
                        count,
                    } => Some(u32::from(*count)),
                    _ => None,
                })
                .sum();
            if disc_self > 0 && disc_self >= draw_fixed {
                let other_hand = view.hand.iter().filter(|h| h.id != id).count() as u32;
                if other_hand < disc_self {
                    continue;
                }
                let pitchable = view
                    .hand
                    .iter()
                    .filter(|h| {
                        h.id != id
                            && (is_true_land_card(view, h.id)
                                || lookup_spell_cost(view, h.id).is_some_and(|(c, t)| {
                                    c.with_x(0).cmc().saturating_add(t) > total_reachable + 1
                                }))
                    })
                    .count() as u32;
                if pitchable < disc_self && view.hand.len() < 5 {
                    continue;
                }
            }
            any_open = true;
        } else if let AbilityDef::ModalSpell { modes, .. } = ab {
            any_spell = true;
            for mode in *modes {
                if is_modal_mode_worth_casting(view, actor, mode) {
                    any_open = true;
                    break;
                }
            }
        }
    }
    !any_spell || any_open
}

pub(super) fn is_modal_mode_worth_casting(
    view: &PlayerView,
    actor: PlayerId,
    mode: &baylee_cards_dsl::SpellMode,
) -> bool {
    if is_counterspell_effects(mode.effects) && view.stack.iter().all(|s| s.controller == actor) {
        return false;
    }
    if let Some(ref req) = mode.targets
        && (req.min >= 1 || req.count_is_x)
        && let TargetSpec::Object(filter) = req.spec
    {
        if is_hostile_object_effects(mode.effects) {
            let fixed_dmg_only = mode.effects.iter().find_map(|e| match e {
                Effect::DealDamage {
                    amount: Amount::Fixed(d),
                    ..
                } => Some(i32::try_from(*d).unwrap_or(99)),
                _ => None,
            });
            let has_non_dmg_hostile = mode.effects.iter().any(|e| {
                matches!(
                    e,
                    Effect::Destroy { .. } | Effect::Exile { .. } | Effect::Fight { .. }
                )
            });
            if !view.battlefield.iter().any(|o| {
                if o.controller == actor
                    || (o.types.contains(TypeSet::LAND) && !filter_targets_lands(filter))
                    || !public_object_matches_basic_filter(actor, o, filter)
                {
                    return false;
                }
                if let Some(dmg) = fixed_dmg_only
                    && !has_non_dmg_hostile
                    && view.phase != Phase::Combat
                {
                    let rem_t = (i32::from(o.toughness.unwrap_or(1)) - i32::from(o.damage)).max(1);
                    return rem_t <= dmg;
                }
                true
            }) {
                return false;
            }
        }
        if mode.effects.iter().all(|e| matches!(e, Effect::TapTarget))
            && view.phase != Phase::Combat
            && !view
                .battlefield_of(actor)
                .any(|o| o.types.contains(TypeSet::CREATURE) && !o.summoning_sick)
        {
            return false;
        }
        if is_pure_pump_effects(mode.effects) {
            if !view
                .battlefield_of(actor)
                .any(|o| public_object_matches_basic_filter(actor, o, filter))
            {
                return false;
            }
            let is_zero_pt_keyword_pump = mode.effects.iter().all(|e| {
                matches!(
                    e,
                    Effect::PumpTarget {
                        power: Amount::Fixed(0),
                        toughness: Amount::Fixed(0),
                        ..
                    }
                )
            });
            if is_zero_pt_keyword_pump && !matches!(view.phase, Phase::Combat) {
                return false;
            }
            let grants_haste = mode.effects.iter().any(|e| {
                matches!(
                    e,
                    Effect::PumpTarget { keywords, .. }
                        if keywords.contains(baylee_cards_dsl::KeywordSet::HASTE)
                )
            });
            let has_sick_creature = view.battlefield_of(actor).any(|o| {
                o.types.contains(TypeSet::CREATURE)
                    && o.summoning_sick
                    && !o.status.contains(ObjectStatus::TAPPED)
                    && !o.status.contains(ObjectStatus::PHASED_OUT)
                    && (o.keywords & baylee_cards_dsl::KeywordSet::HASTE.bits()) == 0
                    && public_object_matches_basic_filter(actor, o, filter)
            });
            if matches!(view.phase, Phase::SecondMain | Phase::Ending) && view.stack.is_empty() {
                return false;
            }
            if grants_haste
                && !has_sick_creature
                && view.phase == Phase::FirstMain
                && view.stack.is_empty()
            {
                return false;
            }
        }
    }
    true
}

#[allow(
    clippy::too_many_lines,
    reason = "Evaluates draw and ramp spell holding vs castable creatures in hand"
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
    let pool_total = view
        .seat(actor)
        .map_or(0, |s| u32::try_from(s.mana_pool.total()).unwrap_or(0));
    let total_reachable = pool_total.saturating_add(untapped_mana_production(view, actor));

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
                    .any(|e| matches!(e, Effect::CreateToken { .. } | Effect::AddMana { .. }))
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
                    Effect::SearchLibrary { finds, .. }
                    | Effect::SearchLibraryBeforeShuffle { finds, .. }
                        if !finds.is_empty()
                            && finds.iter().all(|f| {
                                (matches!(f.dest, SearchDest::Hand | SearchDest::TopOfLibrary)
                                    || total_reachable >= 5)
                                    && f.instead_if.is_none()
                            }) =>
                    {
                        has_draw = true;
                    }
                    Effect::SearchLibraryUpTo { find, .. }
                        if (matches!(find.dest, SearchDest::Hand | SearchDest::TopOfLibrary)
                            || total_reachable >= 5)
                            && find.instead_if.is_none() =>
                    {
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
        } else if let AbilityDef::ModalSpell { modes, .. } = ab {
            if modes.iter().any(|m| {
                is_hostile_object_effects(m.effects)
                    || is_pure_pump_effects(m.effects)
                    || is_counterspell_effects(m.effects)
            }) {
                return false;
            }
            if total_reachable >= 4
                && modes.iter().any(|m| {
                    m.effects.iter().any(|e| {
                        matches!(
                            e,
                            Effect::SearchLibrary { .. }
                                | Effect::SearchLibraryUpTo { .. }
                                | Effect::DrawCards { .. }
                        )
                    })
                })
            {
                has_draw = true;
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

    let draw_cmc =
        lookup_spell_cost(view, id).map_or(2, |(c, t)| c.with_x(0).cmc().saturating_add(t));
    if draw_cmc == 0 {
        return false;
    }

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
                && spell_colors_reachable(view, actor, h.id, &ocost)
                && draw_cmc.saturating_add(ocmc) > total_reachable
            {
                return true;
            }
        }
    }
    false
}

fn should_hold_small_creature_for_curve_topper(view: &PlayerView, id: ObjectId) -> bool {
    let actor = view.decision_player.unwrap_or(view.seat);
    if view.active != actor
        || !view.stack.is_empty()
        || !matches!(view.phase, Phase::FirstMain | Phase::SecondMain)
    {
        return false;
    }
    let Some(ci) = lookup_card_identity(view, id) else {
        return false;
    };
    let Some(def) = baylee_cards::by_index(ci.index) else {
        return false;
    };
    let Some(face) = def.faces.get(usize::from(ci.face)) else {
        return false;
    };
    if !face.types.contains(TypeSet::CREATURE) {
        return false;
    }
    let Some((cost, tax)) = lookup_spell_cost(view, id) else {
        return false;
    };
    let my_cmc = cost.with_x(0).cmc().saturating_add(tax);
    if my_cmc == 0 || my_cmc > 2 {
        return false;
    }
    let produces_mana = def
        .abilities_for_face(usize::from(ci.face))
        .iter()
        .any(|ab| match ab {
            AbilityDef::Activated {
                cost: c,
                effects: e,
                targets: t,
                ..
            }
            | AbilityDef::ActivatedConditional {
                cost: c,
                effects: e,
                targets: t,
                ..
            } => baylee_cards_dsl::mana_rule::activated_mana_ability(c, e, t.is_some()),
            _ => false,
        });
    if produces_mana {
        return false;
    }
    let pool_total = view
        .seat(actor)
        .map_or(0, |s| u32::try_from(s.mana_pool.total()).unwrap_or(0));
    let total_reachable = pool_total.saturating_add(untapped_mana_production(view, actor));

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
            if ocmc >= my_cmc + 2
                && ocmc >= 4
                && ocmc <= total_reachable
                && spell_colors_reachable(view, actor, h.id, &ocost)
                && my_cmc.saturating_add(ocmc) > total_reachable
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
                && spell_colors_reachable_inner(view, actor, Some(legal), Some(id), None, &cost)
            {
                return true;
            }
        }
    }
    false
}

fn permanent_tap_self_damage(abilities: &[AbilityDef]) -> i32 {
    let mut dmg = 0_i32;
    for ab in abilities {
        if let AbilityDef::Triggered {
            trigger: Trigger::BecomesTapped(_) | Trigger::TappedForMana { .. },
            effects,
            ..
        } = ab
        {
            for e in *effects {
                match e {
                    Effect::DealDamage {
                        target: TargetSpec::Player(PlayerRel::You | PlayerRel::EachPlayer),
                        amount: Amount::Fixed(n),
                        ..
                    }
                    | Effect::LoseLife {
                        target: PlayerRel::You | PlayerRel::EachPlayer,
                        amount: Amount::Fixed(n),
                    } => {
                        dmg = dmg.saturating_add(i32::try_from(*n).unwrap_or(99));
                    }
                    _ => {}
                }
            }
        }
    }
    dmg
}

#[allow(
    clippy::too_many_lines,
    reason = "Validates mana ability sacrifice, life, tap, and filter restrictions against current sinks"
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
        && legal.foretellable.is_empty()
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
    let Some(abilities) = abilities_of_battlefield_object(obj) else {
        return true;
    };
    let Some(
        AbilityDef::Activated { cost, effects, .. }
        | AbilityDef::ActivatedConditional { cost, effects, .. },
    ) = abilities.get(ability_index as usize)
    else {
        return true;
    };
    for part in cost.parts {
        match part {
            CostPart::SacrificeX(filter)
                if !view
                    .battlefield_of(actor)
                    .any(|o| public_object_matches_basic_filter(actor, o, filter)) =>
            {
                return false;
            }
            CostPart::TapOther(filter) | CostPart::TapX(filter)
                if !view.battlefield_of(actor).any(|o| {
                    o.id != source
                        && !o.status.contains(ObjectStatus::TAPPED)
                        && !o.status.contains(ObjectStatus::PHASED_OUT)
                        && public_object_matches_basic_filter(actor, o, filter)
                }) =>
            {
                return false;
            }
            CostPart::PayLife(n) if view.seat(actor).map_or(20, |s| s.life) <= i32::from(*n) => {
                return false;
            }
            _ => {}
        }
    }
    let my_life = view.seat(actor).map_or(20, |s| s.life);
    if cost.parts.contains(&CostPart::TapSelf) && my_life <= permanent_tap_self_damage(abilities) {
        return false;
    }
    if effects.iter().any(|e| match e {
        Effect::DealDamage {
            target: TargetSpec::Player(PlayerRel::You | PlayerRel::EachPlayer),
            amount: Amount::Fixed(n),
            ..
        }
        | Effect::LoseLife {
            target: PlayerRel::You | PlayerRel::EachPlayer,
            amount: Amount::Fixed(n),
        } => my_life <= i32::try_from(*n).unwrap_or(99),
        _ => false,
    }) {
        return false;
    }
    let mut any_mana = false;
    let mut any_usable = false;
    for eff in *effects {
        if let Effect::AddMana {
            source: m_src,
            restriction,
            ..
        } = eff
        {
            any_mana = true;
            if let ManaSource::ColorsAmong { filter, .. } = m_src
                && colors_among_battlefield(view, actor, filter) == [false; 5]
            {
                continue;
            }
            if matches!(m_src, ManaSource::CommanderIdentity)
                && view.seat(actor).is_none_or(|s| s.commanders.is_empty())
            {
                continue;
            }
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
    if ability_index > 0 {
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
                        && baylee_cards_dsl::mana_rule::activated_mana_ability(c0, e0, t0.is_some())
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
                let mut free_supply = [0_u32; 5];
                for other in view.battlefield_of(actor) {
                    if other.id == source
                        || other.status.contains(ObjectStatus::TAPPED)
                        || other.status.contains(ObjectStatus::PHASED_OUT)
                    {
                        continue;
                    }
                    let sc = non_sac_source_colors(view, other.id);
                    for c in 0..5 {
                        if sc[c] {
                            free_supply[c] += 1;
                        }
                    }
                }
                let needs_filter = (0..5).any(|c| deficit[c] > free_supply[c]);
                if !needs_filter {
                    return false;
                }
                if view.owed.is_none() {
                    let any_payable_fits = legal.payable.iter().any(|&id| {
                        !legal.castable.contains(&id)
                            && is_spell_worth_casting_now(view, id)
                            && lookup_spell_cost(view, id).is_some_and(|(sc, tax)| {
                                sc.with_x(0).cmc().saturating_add(tax) <= total_reachable
                                    && (0..5).any(|c| {
                                        deficit[c] > free_supply[c]
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
    let my_lib = view.seat(actor).map_or(99, |s| s.library_count);
    let Some(obj) = view.object(source) else {
        let Some(ci) = lookup_card_identity(view, source) else {
            return true;
        };
        let Some(def) = baylee_cards::by_index(ci.index) else {
            return true;
        };
        let Some(
            AbilityDef::Activated { effects, .. }
            | AbilityDef::ActivatedConditional { effects, .. },
        ) = def
            .abilities_for_face(usize::from(ci.face))
            .get(ability_index as usize)
        else {
            return true;
        };
        let hand_draws: u32 = effects
            .iter()
            .filter_map(|e| match e {
                Effect::DrawCards {
                    amount: Amount::Fixed(n),
                }
                | Effect::DrawCardsFor {
                    who: PlayerRel::You | PlayerRel::EachPlayer,
                    amount: Amount::Fixed(n),
                } => Some(*n),
                Effect::DrawCards { .. }
                | Effect::DrawCardsFor {
                    who: PlayerRel::You | PlayerRel::EachPlayer,
                    ..
                } => Some(1),
                _ => None,
            })
            .sum();
        if hand_draws > 0 && my_lib <= hand_draws.saturating_add(4) {
            return false;
        }
        return true;
    };
    let Some(abilities) = abilities_of_battlefield_object(obj) else {
        return true;
    };
    let Some(ab) = abilities.get(ability_index as usize) else {
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
    let is_counter_ab = effects.iter().any(|e| {
        matches!(
            e,
            Effect::CounterTargetSpell
                | Effect::CounterTargetAbility
                | Effect::CounterTargetSpellOrAbility
        )
    });
    if !is_counter_ab
        && !view.stack.is_empty()
        && view.stack.iter().any(|item| item.controller == actor)
    {
        return false;
    }
    if effects
        .iter()
        .any(|e| matches!(e, Effect::PreventAllCombatDamageThisTurn))
    {
        if view.active == actor
            || view.phase != Phase::Combat
            || view.step != Step::DeclareBlockers
            || view.stack.iter().any(|s| s.controller == actor)
        {
            return false;
        }
        let my_life = view.seat(actor).map_or(20, |s| s.life);
        let unblocked_dmg: i32 = view
            .combat
            .attackers
            .iter()
            .filter(|a| a.defending == baylee_core::ids::Defender::Player(actor))
            .filter(|a| {
                !view
                    .combat
                    .blockers
                    .iter()
                    .any(|b| b.attacker == a.creature)
            })
            .filter_map(|a| view.object(a.creature))
            .map(|o| i32::from(o.power.unwrap_or(0)).max(0))
            .sum();
        if unblocked_dmg < 4 && unblocked_dmg < my_life {
            return false;
        }
    }
    let gives_opponent_cards = effects.iter().any(|e| {
        matches!(
            e,
            Effect::DrawCardsFor {
                who: PlayerRel::Chosen
                    | PlayerRel::EachPlayer
                    | PlayerRel::Opponent
                    | PlayerRel::EachOpponent,
                ..
            }
        )
    });
    if gives_opponent_cards
        && (!view.hand.is_empty() || (view.active == actor && view.phase != Phase::SecondMain))
    {
        return false;
    }
    let is_rummage_sac = cost.parts.contains(&CostPart::SacrificeSelf)
        && cost.parts.iter().any(|p| matches!(p, CostPart::Discard(_)));
    if is_rummage_sac {
        if view.hand.is_empty() {
            return false;
        }
        if view.active == actor && view.phase == Phase::FirstMain && view.hand.len() >= 4 {
            let pool_total = view
                .seat(actor)
                .map_or(0, |s| u32::try_from(s.mana_pool.total()).unwrap_or(0));
            if pool_total < cost.mana.with_x(0).cmc() {
                return false;
            }
        }
    }
    for eff in effects {
        let filter_opt = match eff {
            Effect::LookAtTopMayTake { filter, .. }
            | Effect::SearchLibrary { filter, .. }
            | Effect::SearchLibraryBeforeShuffle { filter, .. }
            | Effect::SearchLibraryUpTo { filter, .. } => Some(*filter),
            _ => None,
        };
        if let Some(filter) = filter_opt
            && let Some((st, max_cmc)) = filter_subtype_and_max_cmc(filter)
        {
            let obj_matches = |o: &PublicObject| -> bool {
                if o.token.is_some() || !o.subtypes.contains(st) {
                    return false;
                }
                if let Some(mc) = max_cmc {
                    let o_cmc = o
                        .rules
                        .or_else(|| {
                            o.card.map(|c| RulesFace {
                                card: c.index,
                                face: c.face,
                            })
                        })
                        .and_then(|rf| baylee_cards::by_index(rf.card).map(|d| (d, rf.face)))
                        .and_then(|(d, f)| d.faces.get(usize::from(f)))
                        .map_or(0, |f| f.mana_cost.with_x(0).cmc());
                    if o_cmc > mc {
                        return false;
                    }
                }
                true
            };
            let has_matching_subtype = view.battlefield_of(actor).any(obj_matches)
                || view.hand.iter().any(|h| {
                    baylee_cards::by_index(h.card.index)
                        .and_then(|d| d.faces.get(usize::from(h.card.face)))
                        .is_some_and(|f| {
                            f.subtypes.contains(&st)
                                && max_cmc.is_none_or(|mc| f.mana_cost.with_x(0).cmc() <= mc)
                        })
                })
                || view
                    .graveyards
                    .get(usize::from(actor.get()))
                    .is_some_and(|gy| gy.iter().any(obj_matches));
            if !has_matching_subtype {
                return false;
            }
            if cost.parts.contains(&CostPart::SacrificeSelf)
                && obj.types.contains(TypeSet::ENCHANTMENT)
                && abilities.iter().any(|a| {
                    matches!(
                        a,
                        AbilityDef::Triggered {
                            trigger: Trigger::StepBegin {
                                step: StepKind::Upkeep,
                                whose: PlayerRel::You,
                            },
                            ..
                        }
                    )
                })
            {
                let pool_now = view
                    .seat(actor)
                    .map_or(0, |s| u32::try_from(s.mana_pool.total()).unwrap_or(0));
                let mana_now = pool_now.saturating_add(untapped_mana_production(view, actor));
                if mana_now < 7 || view.hand.len() >= 2 {
                    return false;
                }
            }
        }
    }
    let pool_total = view
        .seat(actor)
        .map_or(0, |s| u32::try_from(s.mana_pool.total()).unwrap_or(0));
    let total_mana = pool_total.saturating_add(untapped_mana_production(view, actor));
    let self_mana = u32::from(
        !obj.status.contains(ObjectStatus::TAPPED)
            && (obj.types.contains(TypeSet::LAND) || obj.board_mana.is_some()),
    );
    let avail_mana = if cost.parts.contains(&CostPart::TapSelf) {
        total_mana.saturating_sub(self_mana)
    } else {
        total_mana
    };
    if !cost.mana.has_variable()
        && cost.mana != ManaCost::ZERO
        && cost.mana.with_x(0).cmc() > avail_mana
    {
        return false;
    }
    let mut self_draw_ab: u32 = effects
        .iter()
        .filter_map(|e| match e {
            Effect::DrawCards {
                amount: Amount::Fixed(n),
            }
            | Effect::DrawCardsFor {
                who: PlayerRel::You | PlayerRel::EachPlayer,
                amount: Amount::Fixed(n),
            } => Some(*n),
            Effect::DrawCards { .. }
            | Effect::DrawCardsFor {
                who: PlayerRel::You | PlayerRel::EachPlayer,
                ..
            } => Some(1),
            _ => None,
        })
        .sum();
    if effects
        .iter()
        .any(|e| matches!(e, Effect::CreateToken { .. }))
    {
        for perm in view.battlefield_of(actor) {
            let Some(p_abs) = abilities_of_battlefield_object(perm) else {
                continue;
            };
            for pab in p_abs {
                if let AbilityDef::Triggered {
                    trigger: Trigger::EntersBattlefield(filter),
                    effects: peffs,
                    ..
                } = pab
                    && !matches!(*filter, Filter::This)
                {
                    let d: u32 = peffs
                        .iter()
                        .filter_map(|e| match e {
                            Effect::DrawCards {
                                amount: Amount::Fixed(n),
                            }
                            | Effect::DrawCardsFor {
                                who: PlayerRel::You | PlayerRel::EachPlayer,
                                amount: Amount::Fixed(n),
                            } => Some(*n),
                            Effect::DrawCards { .. } => Some(1),
                            _ => None,
                        })
                        .sum();
                    self_draw_ab = self_draw_ab.saturating_add(d);
                }
            }
        }
    }
    if self_draw_ab > 0 && my_lib <= self_draw_ab.saturating_add(4) {
        return false;
    }
    if self_draw_ab > 0
        && (cost.parts.contains(&CostPart::SacrificeSelf)
            || cost.parts.contains(&CostPart::ExileSelf))
        && (obj.board_mana.is_some() || obj.types.contains(TypeSet::LAND))
        && (view.hand.len() > 2 || total_mana < 5)
    {
        return false;
    }
    if !effects.is_empty()
        && effects
            .iter()
            .all(|e| matches!(e, Effect::Regenerate { .. }))
    {
        let sac_creature_cost = cost
            .parts
            .iter()
            .any(|p| matches!(p, CostPart::Sacrifice(_) | CostPart::SacrificeX(_)));
        if sac_creature_cost
            && !view.battlefield_of(actor).any(|o| {
                o.id != source
                    && o.types.contains(TypeSet::CREATURE)
                    && !o.status.contains(ObjectStatus::PHASED_OUT)
            })
        {
            return false;
        }
        let stack_threatens_source = view.top_of_stack().is_some_and(|top| {
            top.controller != actor
                && abilities_of_battlefield_object(top).is_some_and(|abs| {
                    abs.iter().any(|ab| match ab {
                        AbilityDef::Spell { effects: se, .. }
                        | AbilityDef::Activated { effects: se, .. }
                        | AbilityDef::ActivatedConditional { effects: se, .. }
                        | AbilityDef::Triggered { effects: se, .. } => se.iter().any(|e| {
                            matches!(
                                e,
                                Effect::Destroy { .. }
                                    | Effect::DestroyAll { .. }
                                    | Effect::DealDamage { .. }
                            )
                        }),
                        _ => false,
                    })
                })
        });
        let in_lethal_combat = view.phase == Phase::Combat
            && matches!(view.step, Step::DeclareBlockers | Step::CombatDamageFirst)
            && (view.combat.attackers.iter().any(|a| {
                a.creature == source && view.combat.blockers.iter().any(|b| b.attacker == source)
            }) || view.combat.blockers.iter().any(|b| b.blocker == source));
        if !stack_threatens_source && !in_lethal_combat {
            return false;
        }
    }
    let creates_only_non_creature_token = effects.iter().any(|e| {
        matches!(e, Effect::CreateToken { token, .. } if !token.types.contains(TypeSet::CREATURE))
    }) && !effects.iter().any(|e| {
        matches!(e, Effect::CreateToken { token, .. } if token.types.contains(TypeSet::CREATURE))
    });
    if creates_only_non_creature_token
        && ((view.active == actor && view.phase == Phase::FirstMain)
            || (cost.mana.with_x(0).cmc() >= 4 && view.hand.len() >= 2))
    {
        return false;
    }
    let is_pure_lifegain_sac = cost.parts.contains(&CostPart::SacrificeSelf)
        && !effects.is_empty()
        && effects.iter().all(|e| matches!(e, Effect::GainLife { .. }));
    if is_pure_lifegain_sac {
        let my_life = view.seat(actor).map_or(20, |s| s.life);
        if my_life > 10 || (view.active == actor && view.phase == Phase::FirstMain && my_life > 5) {
            return false;
        }
    }
    for eff in effects {
        if let Effect::PumpFilter {
            filter: Filter::This,
            power,
            toughness,
            duration: baylee_cards_dsl::Duration::UntilEndOfTurn,
            keywords,
            ..
        } = eff
            && keywords.is_empty()
        {
            if let Amount::CountOf { filter: f, .. } = power
                && !view.battlefield.iter().any(|o| {
                    !o.status.contains(ObjectStatus::PHASED_OUT)
                        && public_object_matches_basic_filter(actor, o, f)
                })
            {
                return false;
            }
            if let Amount::CountOf { filter: f, .. } = toughness
                && !view.battlefield.iter().any(|o| {
                    !o.status.contains(ObjectStatus::PHASED_OUT)
                        && public_object_matches_basic_filter(actor, o, f)
                })
            {
                return false;
            }
            let in_combat = view.phase == Phase::Combat
                && matches!(view.step, Step::DeclareAttackers | Step::DeclareBlockers)
                && (view.combat.attackers.iter().any(|a| a.creature == source)
                    || view.combat.blockers.iter().any(|b| b.blocker == source));
            if !in_combat {
                return false;
            }
        }
        if let Effect::PumpFilter {
            filter: Filter::This,
            power: Amount::Fixed(0),
            toughness: Amount::Fixed(0),
            keywords,
            ..
        } = eff
            && !keywords.is_empty()
        {
            if (obj.keywords & keywords.bits()) == keywords.bits()
                || view.stack.iter().any(|s| {
                    matches!(
                        s.stack_item,
                        Some(baylee_view::StackItem::Ability { source: src, .. }) if src == source
                    )
                })
            {
                return false;
            }
            let menace_bit = baylee_cards_dsl::KeywordSet::MENACE.bits();
            let trample_bit = baylee_cards_dsl::KeywordSet::TRAMPLE.bits();
            let vigilance_bit = baylee_cards_dsl::KeywordSet::VIGILANCE.bits();
            let atk_only_bits = menace_bit | trample_bit | vigilance_bit;
            if (keywords.bits() & !atk_only_bits) == 0 {
                if view.active != actor {
                    return false;
                }
                let is_attacking = view.combat.attackers.iter().any(|a| a.creature == source);
                if (keywords.bits() & vigilance_bit) == 0 {
                    if view.phase != Phase::Combat || !is_attacking {
                        return false;
                    }
                    if (keywords.bits() & !menace_bit) == 0 {
                        let opp_has_untapped_creature = view.battlefield.iter().any(|o| {
                            o.controller != actor
                                && o.types.contains(TypeSet::CREATURE)
                                && !o.status.contains(ObjectStatus::TAPPED)
                                && !o.status.contains(ObjectStatus::PHASED_OUT)
                        });
                        if view.step != Step::DeclareAttackers || !opp_has_untapped_creature {
                            return false;
                        }
                    } else if !matches!(view.step, Step::DeclareAttackers | Step::DeclareBlockers) {
                        return false;
                    }
                } else if matches!(view.phase, Phase::SecondMain | Phase::Ending)
                    || (view.phase == Phase::Combat && view.step != Step::CombatBegin)
                    || obj.status.contains(ObjectStatus::TAPPED)
                    || obj.summoning_sick
                {
                    return false;
                }
            }
        } else if let Effect::PumpTarget {
            power: Amount::Fixed(0),
            toughness: Amount::Fixed(0),
            keywords,
            ..
        } = eff
            && !keywords.is_empty()
        {
            let menace_bit = baylee_cards_dsl::KeywordSet::MENACE.bits();
            let trample_bit = baylee_cards_dsl::KeywordSet::TRAMPLE.bits();
            let vigilance_bit = baylee_cards_dsl::KeywordSet::VIGILANCE.bits();
            let atk_only_bits = menace_bit | trample_bit | vigilance_bit;
            if (keywords.bits() & !atk_only_bits) == 0 {
                if view.active != actor
                    || !(view.phase == Phase::FirstMain
                        || (view.phase == Phase::Combat && view.step == Step::CombatBegin))
                {
                    return false;
                }
                let has_eligible_attacker = view.battlefield_of(actor).any(|o| {
                    o.types.contains(TypeSet::CREATURE)
                        && !o.status.contains(ObjectStatus::TAPPED)
                        && !o.status.contains(ObjectStatus::PHASED_OUT)
                        && !o.summoning_sick
                        && (o.keywords & baylee_cards_dsl::KeywordSet::DEFENDER.bits()) == 0
                        && (o.keywords & keywords.bits()) != keywords.bits()
                        && target_spec.is_none_or(|ts| match ts {
                            TargetSpec::Object(f) => {
                                public_object_matches_basic_filter(actor, o, f)
                            }
                            _ => true,
                        })
                });
                if !has_eligible_attacker {
                    return false;
                }
            }
        }
    }
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
    if let Some(TargetSpec::CardInGraveyard(filter, PlayerRel::You)) = target_spec {
        let has_gy_match = view
            .graveyards
            .get(usize::from(actor.get()))
            .is_some_and(|gy| {
                gy.iter()
                    .any(|o| public_object_matches_basic_filter(actor, o, filter))
            });
        if !has_gy_match {
            return false;
        }
    }
    if effects
        .iter()
        .any(|e| matches!(e, Effect::GraveyardToHand { .. }))
    {
        let has_gy_any = view
            .graveyards
            .get(usize::from(actor.get()))
            .is_some_and(|gy| !gy.is_empty());
        if !has_gy_any {
            return false;
        }
    }
    if let Some(TargetSpec::Object(filter)) = target_spec
        && is_hostile_object_effects(effects)
        && !view.battlefield.iter().any(|o| {
            o.controller != actor
                && (!o.types.contains(TypeSet::LAND) || filter_targets_lands(filter))
                && public_object_matches_basic_filter(actor, o, filter)
        })
    {
        return false;
    }
    true
}

fn filter_targets_lands(filter: &Filter) -> bool {
    match filter {
        Filter::HasType(ts) => *ts == TypeSet::LAND,
        Filter::And(parts) | Filter::Or(parts) => parts.iter().any(filter_targets_lands),
        _ => false,
    }
}

fn filter_subtype_and_max_cmc(
    filter: &Filter,
) -> Option<(baylee_core::ids::SubtypeId, Option<u32>)> {
    let is_basic_land_sub = |st: &baylee_core::ids::SubtypeId| {
        [
            land_subtypes::PLAINS,
            land_subtypes::ISLAND,
            land_subtypes::SWAMP,
            land_subtypes::MOUNTAIN,
            land_subtypes::FOREST,
        ]
        .contains(st)
    };
    match filter {
        Filter::HasSubtype(st) if !is_basic_land_sub(st) => Some((*st, None)),
        Filter::And(parts) => {
            let mut sub = None;
            let mut max_cmc = None;
            for p in *parts {
                match p {
                    Filter::HasSubtype(st) if !is_basic_land_sub(st) => {
                        sub = Some(*st);
                    }
                    Filter::CmcAtMost(c) => {
                        max_cmc = Some(*c);
                    }
                    _ => {}
                }
            }
            sub.map(|s| (s, max_cmc))
        }
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum TargetIntent {
    Hostile,
    Beneficial,
    Neutral,
}

fn classify_ability_intent(view: &PlayerView, actor: PlayerId, ab: &AbilityDef) -> TargetIntent {
    match ab {
        AbilityDef::Spell { effects, .. }
        | AbilityDef::Activated { effects, .. }
        | AbilityDef::ActivatedConditional { effects, .. }
        | AbilityDef::Triggered { effects, .. } => classify_effects_intent(view, actor, effects),
        AbilityDef::ModalSpell { modes, .. }
        | AbilityDef::ModalActivated { modes, .. }
        | AbilityDef::ModalTriggered { modes, .. } => {
            for m in *modes {
                let intent = classify_effects_intent(view, actor, m.effects);
                if intent != TargetIntent::Neutral {
                    return intent;
                }
            }
            TargetIntent::Neutral
        }
        _ => TargetIntent::Neutral,
    }
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
            if let Some(ab) = abilities.get(ab_idx as usize) {
                let intent = classify_ability_intent(view, actor, ab);
                if intent != TargetIntent::Neutral {
                    return intent;
                }
            }
        } else {
            for ab in abilities {
                let intent = classify_ability_intent(view, actor, ab);
                if intent != TargetIntent::Neutral {
                    return intent;
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
            let intent = classify_ability_intent(view, actor, ab);
            if intent != TargetIntent::Neutral {
                return intent;
            }
        }
    }
    if let Some(gy) = view.graveyards.get(usize::from(actor.get())) {
        for obj in gy.iter().rev() {
            let Some(abs) = abilities_of_battlefield_object(obj) else {
                continue;
            };
            for ab in abs {
                if let AbilityDef::Triggered {
                    trigger: Trigger::Dies(_) | Trigger::LeavesBattlefield(_),
                    effects,
                    targets: Some(_),
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
    }
    let mut bf_objs: Vec<&PublicObject> = view.battlefield_of(actor).collect();
    bf_objs.sort_by_key(|o| std::cmp::Reverse(o.id));
    for obj in bf_objs {
        let Some(rf) = obj.rules.or_else(|| {
            obj.card.map(|c| RulesFace {
                card: c.index,
                face: c.face,
            })
        }) else {
            continue;
        };
        let Some(def) = baylee_cards::by_index(rf.card) else {
            continue;
        };
        for ab in def.abilities_for_face(usize::from(rf.face)) {
            if let AbilityDef::Triggered {
                effects,
                targets: Some(_),
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
    if effects.iter().any(|e| {
        matches!(
            e,
            Effect::Fight {
                fighter: baylee_cards_dsl::TargetSlot::First,
                ..
            }
        )
    }) {
        return TargetIntent::Neutral;
    }
    if is_hostile_object_effects(effects) {
        return TargetIntent::Hostile;
    }
    let has_draw_chosen = effects.iter().any(|e| {
        matches!(
            e,
            Effect::DrawCardsFor {
                who: PlayerRel::Chosen,
                ..
            }
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
            Effect::AddCounter {
                kind: CounterKind::P1P1,
                ..
            }
            | Effect::UntapTarget => true,
            _ => false,
        })
        && !is_hostile_object_effects(effects)
}

fn is_hostile_object_effects(effects: &[Effect]) -> bool {
    !effects.is_empty()
        && !effects.iter().any(|e| {
            matches!(
                e,
                Effect::Fight {
                    fighter: baylee_cards_dsl::TargetSlot::First,
                    ..
                }
            )
        })
        && effects.iter().any(|e| match e {
            Effect::Destroy { .. }
            | Effect::Exile { .. }
            | Effect::DealDamage { .. }
            | Effect::TapTarget
            | Effect::Fight {
                fighter: baylee_cards_dsl::TargetSlot::This,
                ..
            } => true,
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
        Condition::ControlCount(filter, min) => {
            let count = view
                .battlefield_of(actor)
                .filter(|o| {
                    !o.status.contains(ObjectStatus::PHASED_OUT)
                        && public_object_matches_basic_filter(actor, o, filter)
                })
                .count();
            count < usize::from(*min)
        }
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
    for gy in &view.graveyards {
        if let Some(obj) = gy.iter().find(|o| o.id == id)
            && let Some(ci) = obj.card
        {
            return Some(ci);
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

#[allow(
    clippy::too_many_lines,
    reason = "Tiered mana pain and opponent-token drawback inspection across triggered and activated abilities"
)]
fn mana_action_pain_tier(view: &PlayerView, action: &PlayerAction) -> u8 {
    let (source, ab_idx_opt) = match action {
        PlayerAction::ActivateManaAbility { source } => (*source, None),
        PlayerAction::ActivateAbility {
            source,
            ability_index,
        } => (*source, Some(*ability_index)),
        _ => return 0,
    };
    let Some(obj) = view.object(source) else {
        return 0;
    };
    let rf = obj.rules.or_else(|| {
        obj.card.map(|c| RulesFace {
            card: c.index,
            face: c.face,
        })
    });
    let Some(rf) = rf else { return 0 };
    let Some(def) = baylee_cards::by_index(rf.card) else {
        return 0;
    };
    let abilities = def.abilities_for_face(usize::from(rf.face));
    if abilities.iter().any(|ab| {
        matches!(
            ab,
            AbilityDef::Triggered {
                trigger: Trigger::BecomesTapped(_) | Trigger::TappedForMana { .. },
                effects,
                targets,
                ..
            } if effects.iter().any(|e| matches!(
                e,
                Effect::CreateTokensFor {
                    player: PlayerRel::EachOpponent,
                    ..
                }
            ) || (matches!(e, Effect::CreateTokensFor { player: PlayerRel::Chosen, .. })
                && targets.as_ref().is_some_and(|t| matches!(
                    t.spec,
                    TargetSpec::AnyOpponent | TargetSpec::Player(PlayerRel::EachOpponent)
                ))))
        )
    }) {
        return 3;
    }
    let actor = view.decision_player.unwrap_or(view.seat);
    let my_life = view.seat(actor).map_or(20, |s| s.life);
    let tap_self_dmg = permanent_tap_self_damage(abilities);
    if tap_self_dmg > 0 {
        return if my_life <= tap_self_dmg { 99 } else { 1 };
    }
    let idx = ab_idx_opt.unwrap_or(0);
    if let Some(
        AbilityDef::Activated { cost, effects, .. }
        | AbilityDef::ActivatedConditional { cost, effects, .. },
    ) = abilities.get(idx as usize)
    {
        for p in cost.parts {
            match p {
                CostPart::PayLife(n) if my_life <= i32::from(*n) => return 99,
                CostPart::PayLife(n) if *n >= 3 => return 4,
                CostPart::PayLife(n) if *n == 2 => return 2,
                CostPart::PayLife(_) | CostPart::PayLifeX => return 1,
                CostPart::Sacrifice(_)
                | CostPart::SacrificeSelf
                | CostPart::SacrificeX(_)
                | CostPart::RemoveCounterSelf { .. } => return 2,
                _ => {}
            }
        }
        for e in *effects {
            match e {
                Effect::DealDamage {
                    target: TargetSpec::Player(PlayerRel::You | PlayerRel::EachPlayer),
                    amount: Amount::Fixed(n),
                    ..
                }
                | Effect::LoseLife {
                    target: PlayerRel::You | PlayerRel::EachPlayer,
                    amount: Amount::Fixed(n),
                } => {
                    return if my_life <= i32::try_from(*n).unwrap_or(99) {
                        99
                    } else if *n >= 3 {
                        4
                    } else if *n == 2 {
                        2
                    } else {
                        1
                    };
                }
                Effect::DealDamage {
                    target: TargetSpec::Player(PlayerRel::You | PlayerRel::EachPlayer),
                    ..
                }
                | Effect::LoseLife {
                    target: PlayerRel::You | PlayerRel::EachPlayer,
                    ..
                } => return 1,
                Effect::ReturnSelfDuringNextUntap => return 2,
                _ => {}
            }
        }
    }
    0
}

#[allow(
    clippy::too_many_lines,
    reason = "Mana source ordering by pain tier, non-mana utility, color flexibility, and unmet color deficit"
)]
fn redirect_mana_source_for_colors(
    cands: &EncodedCandidates4,
    logits: &[f32],
    view: &PlayerView,
    legal: &LegalActions,
    chosen_idx: usize,
) -> usize {
    let actor = view.decision_player.unwrap_or(view.seat);
    let deficit = unmet_color_deficit(view, legal);
    let any_deficit = deficit.iter().any(|&d| d > 0);
    let pain_tier = |idx: usize| -> u8 {
        cands.candidates[idx]
            .action
            .as_ref()
            .map_or(9, |a| mana_action_pain_tier(view, a))
    };
    let source_utility_and_flex = |idx: usize| -> (bool, usize) {
        let Some(ref act) = cands.candidates[idx].action else {
            return (true, 9);
        };
        let src = match act {
            PlayerAction::ActivateManaAbility { source }
            | PlayerAction::ActivateAbility { source, .. } => *source,
            _ => return (true, 9),
        };
        let Some(obj) = view.object(src) else {
            return (true, 9);
        };
        let has_non_mana = obj.types.contains(TypeSet::CREATURE)
            || obj
                .rules
                .or_else(|| {
                    obj.card.map(|c| RulesFace {
                        card: c.index,
                        face: c.face,
                    })
                })
                .and_then(|rf| {
                    baylee_cards::by_index(rf.card)
                        .map(|d| d.abilities_for_face(usize::from(rf.face)))
                })
                .is_some_and(|abs| {
                    abs.iter().any(|ab| {
                        matches!(
                            ab,
                            AbilityDef::Activated { .. }
                                | AbilityDef::ActivatedConditional { .. }
                                | AbilityDef::ModalActivated { .. }
                        ) && !ab.is_mana_ability()
                    })
                });
        let cols = non_sac_source_colors(view, src);
        let flex = cols[0..5].iter().filter(|&&c| c).count();
        (has_non_mana, flex)
    };
    if !any_deficit {
        if pain_tier(chosen_idx) == 0 {
            return chosen_idx;
        }
        return (0..cands.candidates.len())
            .filter(|&i| logits[i].is_finite())
            .filter(|&i| {
                cands.candidates[i]
                    .action
                    .as_ref()
                    .is_some_and(|a| is_mana_action(view, a))
            })
            .max_by(|&a, &b| {
                let (ua, fa) = source_utility_and_flex(a);
                let (ub, fb) = source_utility_and_flex(b);
                pain_tier(b)
                    .cmp(&pain_tier(a))
                    .then_with(|| ub.cmp(&ua))
                    .then_with(|| fb.cmp(&fa))
                    .then_with(|| logits[a].total_cmp(&logits[b]))
            })
            .unwrap_or(chosen_idx);
    }
    let satisfies = |idx: usize| -> bool {
        let Some(ref act) = cands.candidates[idx].action else {
            return false;
        };
        let colors = action_produced_colors(view, legal, act);
        (0..6).any(|c| deficit[c] > 0 && colors[c])
    };
    if let Some(best_direct) = (0..cands.candidates.len())
        .filter(|&i| logits[i].is_finite())
        .filter(|&i| {
            cands.candidates[i]
                .action
                .as_ref()
                .is_some_and(|a| is_mana_action(view, a))
                && satisfies(i)
        })
        .max_by(|&a, &b| {
            let (ua, fa) = source_utility_and_flex(a);
            let (ub, fb) = source_utility_and_flex(b);
            pain_tier(b)
                .cmp(&pain_tier(a))
                .then_with(|| ub.cmp(&ua))
                .then_with(|| fb.cmp(&fa))
                .then_with(|| logits[a].total_cmp(&logits[b]))
        })
    {
        return best_direct;
    }
    // No currently payable mana ability produces the deficit color directly; check if an
    // untapped filter land on our battlefield can produce the deficit color once we generate
    // generic mana from another source.
    let filter_sources: Vec<ObjectId> = view
        .battlefield_of(actor)
        .filter(|o| {
            !o.status.contains(ObjectStatus::TAPPED)
                && !o.status.contains(ObjectStatus::PHASED_OUT)
                && filter_land_min_cost_and_colors(view, o.id)
                    .is_some_and(|(_, cols)| (0..5).any(|c| deficit[c] > 0 && cols[c]))
        })
        .map(|o| o.id)
        .collect();
    if !filter_sources.is_empty()
        && let Some(best_feeder) = (0..cands.candidates.len())
            .filter(|&i| logits[i].is_finite())
            .filter(|&i| {
                cands.candidates[i].action.as_ref().is_some_and(|a| {
                    is_mana_action(view, a)
                        && match a {
                            PlayerAction::ActivateManaAbility { source }
                            | PlayerAction::ActivateAbility { source, .. } => {
                                !filter_sources.contains(source)
                            }
                            _ => false,
                        }
                })
            })
            .max_by(|&a, &b| {
                let (ua, fa) = source_utility_and_flex(a);
                let (ub, fb) = source_utility_and_flex(b);
                pain_tier(b)
                    .cmp(&pain_tier(a))
                    .then_with(|| ub.cmp(&ua))
                    .then_with(|| fb.cmp(&fa))
                    .then_with(|| logits[a].total_cmp(&logits[b]))
            })
    {
        return best_feeder;
    }
    chosen_idx
}

fn land_is_bouncer_or_dup_legend(view: &PlayerView, actor: PlayerId, card: ObjectId) -> bool {
    let Some(ci) = lookup_card_identity(view, card) else {
        return false;
    };
    let Some(def) = baylee_cards::by_index(ci.index) else {
        return false;
    };
    let Some(face) = def.faces.get(usize::from(ci.face)) else {
        return false;
    };
    if face.supertypes.contains(SupertypeSet::LEGENDARY)
        && view.battlefield_of(actor).any(|o| {
            !o.status.contains(ObjectStatus::PHASED_OUT)
                && o.card.is_some_and(|c| c.index == ci.index)
        })
    {
        return true;
    }
    def.abilities_for_face(usize::from(ci.face))
        .iter()
        .any(|ab| match ab {
            AbilityDef::Activated { effects, .. }
            | AbilityDef::ActivatedConditional { effects, .. }
            | AbilityDef::Triggered { effects, .. } => effects
                .iter()
                .any(|e| matches!(e, Effect::ReturnSelfDuringNextUntap)),
            _ => false,
        })
}

#[allow(
    clippy::too_many_lines,
    reason = "Prioritizes permanent lands that cover missing hand colors while avoiding bounce-loop and duplicate legendary lands"
)]
fn redirect_land_for_hand_colors(
    cands: &EncodedCandidates4,
    logits: &[f32],
    view: &PlayerView,
    chosen_idx: usize,
) -> usize {
    let actor = view.decision_player.unwrap_or(view.seat);
    let pool_total = view
        .seat(actor)
        .map_or(0, |s| u32::try_from(s.mana_pool.total()).unwrap_or(0));
    let avail_after_land = pool_total
        .saturating_add(untapped_mana_production(view, actor))
        .saturating_add(1);
    let mut board_color_counts = [0_u32; 5];
    for obj in view.battlefield_of(actor) {
        if obj.status.contains(ObjectStatus::PHASED_OUT) {
            continue;
        }
        let cols = non_sac_source_colors(view, obj.id);
        for c in 0..5 {
            if cols[c] {
                board_color_counts[c] += 1;
            }
        }
    }
    let mut missing = [false; 5];
    let mut missing_castable_now = [false; 5];
    for h in &view.hand {
        if is_true_land_card(view, h.id) {
            continue;
        }
        if let Some((cost, tax)) = lookup_spell_cost(view, h.id) {
            let cmc = cost.with_x(0).cmc().saturating_add(tax);
            for c in 0..5 {
                let pips = super::choices::count_color_pips(&cost, c);
                if pips > board_color_counts[c] {
                    missing[c] = true;
                    if cmc <= avail_after_land {
                        missing_castable_now[c] = true;
                    }
                }
            }
        }
    }
    let chosen_is_bouncer_or_dup = match cands.candidates[chosen_idx].action {
        Some(PlayerAction::PlayLand { card }) => {
            land_is_bouncer_or_dup_legend(view, actor, card)
                && !{
                    let cols = land_card_colors(view, card);
                    (0..5).any(|c| missing_castable_now[c] && cols[c])
                }
        }
        _ => false,
    };
    let land_covers_missing = |idx: usize| -> bool {
        let Some(PlayerAction::PlayLand { card }) = cands.candidates[idx].action else {
            return false;
        };
        let is_bad = land_is_bouncer_or_dup_legend(view, actor, card);
        let cols = land_card_colors(view, card);
        if is_bad {
            (0..5).any(|c| missing_castable_now[c] && cols[c])
        } else {
            (0..5).any(|c| missing[c] && cols[c])
        }
    };
    if missing.iter().any(|&m| m) {
        if land_covers_missing(chosen_idx) && !chosen_is_bouncer_or_dup {
            return chosen_idx;
        }
        if let Some(best_missing) = (0..cands.candidates.len())
            .filter(|&i| logits[i].is_finite() && land_covers_missing(i))
            .max_by(|&a, &b| {
                let bad_a = match cands.candidates[a].action {
                    Some(PlayerAction::PlayLand { card }) => {
                        land_is_bouncer_or_dup_legend(view, actor, card)
                    }
                    _ => true,
                };
                let bad_b = match cands.candidates[b].action {
                    Some(PlayerAction::PlayLand { card }) => {
                        land_is_bouncer_or_dup_legend(view, actor, card)
                    }
                    _ => true,
                };
                bad_b
                    .cmp(&bad_a)
                    .then_with(|| logits[a].total_cmp(&logits[b]))
            })
        {
            return best_missing;
        }
    }
    if chosen_is_bouncer_or_dup
        && let Some(best_perm) = (0..cands.candidates.len())
            .filter(|&i| {
                logits[i].is_finite()
                    && matches!(
                        cands.candidates[i].action,
                        Some(PlayerAction::PlayLand { card })
                            if !land_is_bouncer_or_dup_legend(view, actor, card)
                    )
            })
            .max_by(|&a, &b| logits[a].total_cmp(&logits[b]))
    {
        return best_perm;
    }
    chosen_idx
}

#[allow(
    clippy::too_many_lines,
    reason = "Computes unmet mana color deficits across owed costs, payable spells, and unpaid abilities"
)]
fn unmet_color_deficit(view: &PlayerView, legal: &LegalActions) -> [u32; 6] {
    let actor = view.decision_player.unwrap_or(view.seat);
    let Some(seat) = view.seat(actor) else {
        return [0; 6];
    };
    let have_unrestricted = [
        seat.mana_pool.white,
        seat.mana_pool.blue,
        seat.mana_pool.black,
        seat.mana_pool.red,
        seat.mana_pool.green,
        seat.mana_pool.colorless,
    ];
    let have_creature = [
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

    let mut deficit = [0_u32; 6];
    let mut accum_cost = |cost: &ManaCost, is_creature: bool| {
        let have = if is_creature {
            &have_creature
        } else {
            &have_unrestricted
        };
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
            deficit[c] = deficit[c].max(need[c].saturating_sub(have[c]));
        }
    };

    if let Some(ManaPayment::Fixed(ref cost)) = view.owed {
        let owed_is_creature = view
            .top_of_stack()
            .is_some_and(|top| top.types.contains(TypeSet::CREATURE));
        accum_cost(cost, owed_is_creature);
    } else {
        let mut any_payable_accum = false;
        for &id in &legal.payable {
            if legal.castable.contains(&id)
                || !is_spell_worth_casting_now(view, id)
                || should_hold_spell_for_better_main_phase_creature(view, legal, id)
            {
                continue;
            }
            if let Some((cost, tax)) = lookup_spell_cost(view, id)
                && cost.with_x(0).cmc().saturating_add(tax) <= total_reachable
                && spell_colors_reachable_inner_opts(
                    view,
                    actor,
                    Some(legal),
                    Some(id),
                    None,
                    &cost,
                    false,
                )
            {
                accum_cost(&cost, creature_card_board_score(view, id) > 0);
                any_payable_accum = true;
            }
        }
        if !any_payable_accum {
            for &id in &legal.payable {
                if legal.castable.contains(&id)
                    || !is_spell_worth_casting_now(view, id)
                    || should_hold_spell_for_better_main_phase_creature(view, legal, id)
                {
                    continue;
                }
                if let Some((cost, tax)) = lookup_spell_cost(view, id)
                    && cost.with_x(0).cmc().saturating_add(tax) <= total_reachable
                    && spell_colors_reachable_inner(view, actor, Some(legal), Some(id), None, &cost)
                {
                    accum_cost(&cost, creature_card_board_score(view, id) > 0);
                    any_payable_accum = true;
                }
            }
        }
        if !any_payable_accum {
            for &(src, ab_idx, ref cost) in &legal.unpaid_abilities {
                if legal.abilities.contains(&(src, ab_idx))
                    || !is_activated_ability_worth_activating_now(view, src, ab_idx)
                {
                    continue;
                }
                let Some(obj) = view.object(src) else {
                    continue;
                };
                let tap_self = obj
                    .rules
                    .or_else(|| {
                        obj.card.map(|c| RulesFace {
                            card: c.index,
                            face: c.face,
                        })
                    })
                    .and_then(|rf| baylee_cards::by_index(rf.card).map(|d| (d, rf.face)))
                    .and_then(|(d, f)| d.abilities_for_face(usize::from(f)).get(ab_idx as usize))
                    .is_some_and(|ab| match ab {
                        AbilityDef::Activated { cost: c, .. }
                        | AbilityDef::ActivatedConditional { cost: c, .. }
                        | AbilityDef::ModalActivated { cost: c, .. } => {
                            c.parts.contains(&CostPart::TapSelf)
                        }
                        _ => false,
                    });
                let self_mana = u32::from(
                    !obj.status.contains(ObjectStatus::TAPPED)
                        && (obj.types.contains(TypeSet::LAND) || obj.board_mana.is_some()),
                );
                let avail_for_ab = if tap_self {
                    total_reachable.saturating_sub(self_mana)
                } else {
                    total_reachable
                };
                let exclude = tap_self.then_some(src);
                if cost.with_x(0).cmc() <= avail_for_ab
                    && spell_colors_reachable_inner(view, actor, Some(legal), None, exclude, cost)
                {
                    accum_cost(cost, false);
                }
            }
        }
    }

    deficit
}

pub(super) fn spell_colors_reachable(
    view: &PlayerView,
    actor: PlayerId,
    spell_id: ObjectId,
    cost: &ManaCost,
) -> bool {
    spell_colors_reachable_inner(view, actor, None, Some(spell_id), None, cost)
}

fn spell_colors_reachable_inner(
    view: &PlayerView,
    actor: PlayerId,
    legal: Option<&LegalActions>,
    spell_id: Option<ObjectId>,
    exclude_source: Option<ObjectId>,
    cost: &ManaCost,
) -> bool {
    spell_colors_reachable_inner_opts(view, actor, legal, spell_id, exclude_source, cost, true)
}

#[allow(
    clippy::too_many_lines,
    reason = "Checks unrestricted, filter-land, and tribal mana sources against spell color pips"
)]
fn spell_colors_reachable_inner_opts(
    view: &PlayerView,
    actor: PlayerId,
    legal: Option<&LegalActions>,
    spell_id: Option<ObjectId>,
    exclude_source: Option<ObjectId>,
    cost: &ManaCost,
    allow_filter_lands: bool,
) -> bool {
    let spell_face = spell_id
        .and_then(|sid| lookup_card_identity(view, sid))
        .and_then(|ci| {
            baylee_cards::by_index(ci.index).and_then(|d| d.faces.get(usize::from(ci.face)))
        });
    let allow_restricted = spell_face.is_some_and(|f| f.types.contains(TypeSet::CREATURE));
    let mut counts = [0_u32; 6];
    if let Some(seat) = view.seat(actor) {
        counts[0] = seat.mana_pool.white;
        counts[1] = seat.mana_pool.blue;
        counts[2] = seat.mana_pool.black;
        counts[3] = seat.mana_pool.red;
        counts[4] = seat.mana_pool.green;
        counts[5] = seat.mana_pool.colorless;
        if allow_restricted {
            for (cnt, &rest) in counts.iter_mut().zip(seat.mana_pool.restricted.iter()) {
                *cnt = cnt.saturating_add(u32::try_from(rest).unwrap_or(0));
            }
        }
    }
    let pool_total = view
        .seat(actor)
        .map_or(0, |s| u32::try_from(s.mana_pool.total()).unwrap_or(0));
    let total_reach = pool_total.saturating_add(untapped_mana_production(view, actor));
    let spell_cmc = spell_id
        .and_then(|sid| lookup_spell_cost(view, sid))
        .map_or_else(
            || cost.with_x(0).cmc(),
            |(c, tax)| c.with_x(0).cmc().saturating_add(tax),
        );
    let mut rem_filter_slack = total_reach.saturating_sub(spell_cmc);
    for obj in view.battlefield_of(actor) {
        if Some(obj.id) == exclude_source
            || obj.status.contains(ObjectStatus::TAPPED)
            || obj.status.contains(ObjectStatus::PHASED_OUT)
        {
            continue;
        }
        let mut sc = non_sac_source_colors(view, obj.id);
        if allow_filter_lands
            && !sc[0..5].iter().all(|&b| b)
            && let Some((f_cmc, f_cols)) = filter_land_min_cost_and_colors(view, obj.id)
            && spell_cmc <= total_reach
            && f_cmc <= rem_filter_slack
        {
            rem_filter_slack -= f_cmc;
            for c in 0..5 {
                sc[c] |= f_cols[c];
            }
        }
        if let Some(rf) = obj.rules.or_else(|| {
            obj.card.map(|c| RulesFace {
                card: c.index,
                face: c.face,
            })
        }) && let Some(def) = baylee_cards::by_index(rf.card)
        {
            for (ab_idx, ab) in def
                .abilities_for_face(usize::from(rf.face))
                .iter()
                .enumerate()
            {
                if let AbilityDef::ActivatedConditional {
                    cost: ab_cost,
                    effects,
                    targets,
                    condition: Condition::SourceMatches(Filter::EnteredThisTurn),
                    ..
                } = ab
                    && ab_cost.mana == ManaCost::ZERO
                    && baylee_cards_dsl::mana_rule::activated_mana_ability(
                        ab_cost,
                        effects,
                        targets.is_some(),
                    )
                    && legal.is_some_and(|l| l.abilities.contains(&(obj.id, ab_idx as u32)))
                {
                    mark_unrestricted_effect_mana_colors(Some((view, actor)), effects, &mut sc);
                }
                if let Some(face) = spell_face
                    && let AbilityDef::Activated {
                        cost: ab_cost,
                        effects,
                        targets,
                        ..
                    }
                    | AbilityDef::ActivatedConditional {
                        cost: ab_cost,
                        effects,
                        targets,
                        ..
                    } = ab
                    && ab_cost.mana == ManaCost::ZERO
                    && baylee_cards_dsl::mana_rule::activated_mana_ability(
                        ab_cost,
                        effects,
                        targets.is_some(),
                    )
                {
                    for eff in *effects {
                        if let Effect::AddMana {
                            source,
                            restriction: Some(rest),
                            ..
                        } = eff
                            && rest.restricts
                            && face_matches_mana_filter(
                                obj,
                                face.supertypes,
                                face.types,
                                face.subtypes,
                                rest.filter,
                            )
                        {
                            mark_single_mana_source_with_view(Some((view, actor)), source, &mut sc);
                        }
                    }
                }
            }
        }
        for c in 0..6 {
            if sc[c] {
                counts[c] += 1;
            }
        }
    }
    let avail_bools = [
        counts[0] > 0,
        counts[1] > 0,
        counts[2] > 0,
        counts[3] > 0,
        counts[4] > 0,
        counts[5] > 0,
    ];
    if !cost_colors_reachable(cost, &avail_bools) {
        return false;
    }
    for (sym, count) in cost.runs() {
        let req = u32::from(count);
        let ok = match sym {
            ManaSymbol::White | ManaSymbol::TwoOrColor(Color::White) => counts[0] >= req,
            ManaSymbol::Blue | ManaSymbol::TwoOrColor(Color::Blue) => counts[1] >= req,
            ManaSymbol::Black | ManaSymbol::TwoOrColor(Color::Black) => counts[2] >= req,
            ManaSymbol::Red | ManaSymbol::TwoOrColor(Color::Red) => counts[3] >= req,
            ManaSymbol::Green | ManaSymbol::TwoOrColor(Color::Green) => counts[4] >= req,
            ManaSymbol::Colorless => counts[5] >= req,
            _ => true,
        };
        if !ok {
            return false;
        }
    }
    true
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
        && let Some(
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
            },
        ) = def
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
        && let Some(
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
            },
        ) = tdef.abilities.get(target_ab_idx)
        && baylee_cards_dsl::mana_rule::activated_mana_ability(cost, effects, targets.is_some())
    {
        mark_effect_mana_colors_for_payable(view, obj, legal, effects, &mut colors);
    }
    colors
}

#[allow(
    clippy::too_many_lines,
    reason = "Inspects non-sacrifice mana production across intrinsic, board, granted, and card/token abilities"
)]
pub(super) fn non_sac_source_colors(view: &PlayerView, source: ObjectId) -> [bool; 6] {
    let actor = view.decision_player.unwrap_or(view.seat);
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
            let (cost, effects, targets, cond_opt) = match ab {
                AbilityDef::Activated {
                    cost,
                    effects,
                    targets,
                    ..
                } => (cost, *effects, targets.is_some(), None),
                AbilityDef::ActivatedConditional {
                    cost,
                    effects,
                    targets,
                    condition,
                    ..
                } => (cost, *effects, targets.is_some(), Some(condition)),
                _ => continue,
            };
            if !baylee_cards_dsl::mana_rule::activated_mana_ability(cost, effects, targets) {
                continue;
            }
            if let Some(cond) = cond_opt
                && (condition_closed(view, actor, cond)
                    || matches!(cond, Condition::SourceMatches(Filter::EnteredThisTurn)))
            {
                continue;
            }
            let my_life = view.seat(actor).map_or(20, |s| s.life);
            if (cost.parts.contains(&CostPart::TapSelf)
                && my_life
                    <= permanent_tap_self_damage(def.abilities_for_face(usize::from(rf.face))))
                || cost.parts.iter().any(|p| match p {
                    CostPart::PayEnergy(n) => view.seat(actor).map_or(0, |s| s.energy) < *n,
                    CostPart::PayLife(n) => my_life <= i32::from(*n),
                    CostPart::TapOther(filter) | CostPart::TapX(filter) => {
                        !view.battlefield_of(actor).any(|o| {
                            o.id != source
                                && !o.status.contains(ObjectStatus::TAPPED)
                                && !o.status.contains(ObjectStatus::PHASED_OUT)
                                && public_object_matches_basic_filter(actor, o, filter)
                        })
                    }
                    _ => false,
                })
                || effects.iter().any(|e| match e {
                    Effect::DealDamage {
                        target: TargetSpec::Player(PlayerRel::You | PlayerRel::EachPlayer),
                        amount: Amount::Fixed(n),
                        ..
                    }
                    | Effect::LoseLife {
                        target: PlayerRel::You | PlayerRel::EachPlayer,
                        amount: Amount::Fixed(n),
                    } => my_life <= i32::try_from(*n).unwrap_or(99),
                    _ => false,
                })
            {
                continue;
            }
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
            mark_unrestricted_effect_mana_colors(Some((view, actor)), effects, &mut colors);
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
            }
            | AbilityDef::ActivatedConditional {
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
    let actor = view.decision_player.unwrap_or(view.seat);
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
        let (cost, effects, targets, cond_opt) = match ab {
            AbilityDef::Activated {
                cost,
                effects,
                targets,
                ..
            } => (cost, *effects, targets.is_some(), None),
            AbilityDef::ActivatedConditional {
                cost,
                effects,
                targets,
                condition,
                ..
            } => (cost, *effects, targets.is_some(), Some(condition)),
            _ => continue,
        };
        if !baylee_cards_dsl::mana_rule::activated_mana_ability(cost, effects, targets) {
            continue;
        }
        if let Some(cond) = cond_opt
            && (condition_closed(view, actor, cond)
                || matches!(cond, Condition::SourceMatches(Filter::EnteredThisTurn)))
        {
            continue;
        }
        if cost.mana != ManaCost::ZERO
            || cost
                .parts
                .iter()
                .any(|p| !matches!(p, CostPart::TapSelf | CostPart::PayLife(_)))
        {
            continue;
        }
        mark_unrestricted_effect_mana_colors(Some((view, actor)), effects, &mut colors);
    }
    [colors[0], colors[1], colors[2], colors[3], colors[4]]
}

pub(super) fn filter_land_min_cost_and_colors(
    view: &PlayerView,
    id: ObjectId,
) -> Option<(u32, [bool; 5])> {
    let actor = view.decision_player.unwrap_or(view.seat);
    let abilities = if let Some(obj) = view.object(id) {
        if !obj.types.contains(TypeSet::LAND) {
            return None;
        }
        abilities_of_battlefield_object(obj)?
    } else {
        let ci = lookup_card_identity(view, id)?;
        let def = baylee_cards::by_index(ci.index)?;
        let face = def.faces.get(usize::from(ci.face))?;
        if !face.types.contains(TypeSet::LAND) {
            return None;
        }
        def.abilities_for_face(usize::from(ci.face))
    };
    let mut best_cmc: Option<u32> = None;
    let mut colors = [false; 6];
    for ab in abilities {
        if let AbilityDef::Activated {
            cost,
            effects,
            targets,
            ..
        } = ab
            && cost.mana != ManaCost::ZERO
            && !cost.parts.iter().any(|p| {
                matches!(
                    p,
                    CostPart::Sacrifice(_) | CostPart::SacrificeSelf | CostPart::SacrificeX(_)
                )
            })
            && baylee_cards_dsl::mana_rule::activated_mana_ability(cost, effects, targets.is_some())
        {
            let cmc = cost.mana.with_x(0).cmc();
            let mut ab_cols = [false; 6];
            mark_unrestricted_effect_mana_colors(Some((view, actor)), effects, &mut ab_cols);
            if ab_cols[0..5].iter().any(|&b| b) {
                best_cmc = Some(best_cmc.map_or(cmc, |old| old.min(cmc)));
                for c in 0..5 {
                    colors[c] |= ab_cols[c];
                }
            }
        }
    }
    best_cmc.map(|cmc| (cmc, [colors[0], colors[1], colors[2], colors[3], colors[4]]))
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
    let actor = view.decision_player.unwrap_or(view.seat);
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
            mark_single_mana_source_with_view(Some((view, actor)), source, colors);
        }
    }
}

fn face_matches_mana_filter(
    src_obj: &PublicObject,
    supertypes: baylee_core::types::SupertypeSet,
    types: TypeSet,
    subtypes: &[baylee_core::ids::SubtypeId],
    filter: &Filter,
) -> bool {
    match filter {
        Filter::HasSupertype(st) => supertypes.contains(*st),
        Filter::HasType(ts) => types.intersects(*ts),
        Filter::HasSubtype(st) => subtypes.contains(st),
        Filter::MatchesChosenTypeOfSource => src_obj
            .chosen_subtype
            .is_some_and(|st| subtypes.contains(&st)),
        Filter::Or(list) => list
            .iter()
            .any(|f| face_matches_mana_filter(src_obj, supertypes, types, subtypes, f)),
        Filter::And(list) => list
            .iter()
            .all(|f| face_matches_mana_filter(src_obj, supertypes, types, subtypes, f)),
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
        if face_matches_mana_filter(
            src_obj,
            face.supertypes,
            face.types,
            face.subtypes,
            rest.filter,
        ) {
            return true;
        }
    }
    false
}

fn colors_among_battlefield(view: &PlayerView, actor: PlayerId, filter: &Filter) -> [bool; 5] {
    let mut cols = [false; 5];
    for obj in view.battlefield_of(actor) {
        if obj.status.contains(ObjectStatus::PHASED_OUT) {
            continue;
        }
        if public_object_matches_basic_filter(actor, obj, filter) {
            for c in Color::ALL {
                if obj.colors.contains(c) {
                    cols[c as usize] = true;
                }
            }
        }
    }
    cols
}

fn mark_unrestricted_effect_mana_colors(
    ctx: Option<(&PlayerView, PlayerId)>,
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
            {
                continue;
            }
            mark_single_mana_source_with_view(ctx, source, colors);
        }
    }
}

fn mark_effect_mana_colors(effects: &[Effect], colors: &mut [bool; 6]) {
    for eff in effects {
        if let Effect::AddMana { source, .. } = eff {
            mark_single_mana_source_with_view(None, source, colors);
        }
    }
}

fn mark_single_mana_source_with_view(
    ctx: Option<(&PlayerView, PlayerId)>,
    source: &ManaSource,
    colors: &mut [bool; 6],
) {
    match source {
        ManaSource::Fixed(mc) => {
            colors[mc.index()] = true;
        }
        ManaSource::Choice(list) | ManaSource::ChosenOr(list) => {
            for mc in *list {
                colors[mc.index()] = true;
            }
        }
        ManaSource::ColorsAmong { filter, .. } => {
            if let Some((view, actor)) = ctx {
                let among = colors_among_battlefield(view, actor, filter);
                for (c, &ok) in among.iter().enumerate() {
                    if ok {
                        colors[c] = true;
                    }
                }
            } else {
                for col in colors.iter_mut().take(5) {
                    *col = true;
                }
            }
        }
        ManaSource::CommanderIdentity => {
            if let Some((view, actor)) = ctx
                && view.seat(actor).is_some_and(|s| !s.commanders.is_empty())
            {
                for col in colors.iter_mut().take(5) {
                    *col = true;
                }
            }
        }
        ManaSource::LandColor { .. }
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
