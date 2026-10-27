//! Non-priority choice tactics (`ChooseColor`, `ChooseSubtype`, `ChooseCards`,
//! `DiscardChoice`, `MulliganBottom`, `ChooseAttackers`, and `ChooseBlockers`) for [`super::NetPlayer4`].

use std::collections::{BTreeMap, BTreeSet};

use baylee_cards_dsl::{
    AbilityDef, Amount, CostPart, Effect, Filter, KeywordSet, ManaSource, PlayerRel, StepKind,
    TargetSpec, Trigger, counters,
};
use baylee_core::color::{Color, ColorSet};
use baylee_core::ids::{AbilityRef, Defender, ObjectId, PlayerId, SubtypeId};
use baylee_core::mana::{ManaColor, ManaCost, ManaPayment, ManaSymbol};
use baylee_core::types::{SupertypeSet, TypeSet};
use baylee_engine::choice::{ArrangePlace, ChoicePrompt, Pending, PlayerAction, YesNoPrompt};
use baylee_view::{ObjectStatus, PlayerView, PublicObject, RulesFace};

use crate::features4::{EncodedCandidates4, untapped_mana_production};

use super::tactics::{
    TargetIntent, classify_active_target_intent, is_modal_mode_worth_casting,
    is_spell_worth_casting_now, is_true_land_card, land_card_colors, lookup_card_identity,
    lookup_spell_cost, non_sac_source_colors,
};

/// Applies view-only tactical masking to non-priority `SingleChoice` decisions
/// (`ChooseColor`, `ChooseSubtype`, `Mulligan`, `ChooseCastMode`, and single-pick `ChooseCards`).
#[allow(
    clippy::too_many_lines,
    reason = "Applies view-only masking across ChooseColor, ChooseSubtype, Mulligan, ChooseCastMode, and ChooseCards prompts"
)]
pub(super) fn mask_non_priority_single_choice(
    cands: &EncodedCandidates4,
    masked_logits: &mut [f32],
    view: &PlayerView,
    pending: &Pending,
    active_source: Option<(ObjectId, Option<u32>)>,
) {
    let actor = view.decision_player.unwrap_or(view.seat);
    match pending {
        Pending::Mulligan {
            taken,
            next_is_free,
            can_take: true,
            ..
        } => {
            if *taken <= 2 {
                let mut land_count = 0_usize;
                let mut hand_colors = [false; 5];
                let mut has_choose_subtype_land = false;
                for h in &view.hand {
                    if is_true_land_card(view, h.id) {
                        land_count += 1;
                        let cols = land_card_colors(view, h.id);
                        for (c, &ok) in cols.iter().enumerate() {
                            if ok {
                                hand_colors[c] = true;
                            }
                        }
                        if baylee_cards::by_index(h.card.index)
                            .and_then(|d| d.faces.get(usize::from(h.card.face)))
                            .is_some_and(|f| {
                                f.enter_modifiers
                                    .contains(&baylee_cards_dsl::EnterModifier::ChooseSubtype)
                            })
                        {
                            has_choose_subtype_land = true;
                        }
                    }
                }
                let early_castable_count = view
                    .hand
                    .iter()
                    .filter(|h| {
                        !is_true_land_card(view, h.id)
                            && lookup_spell_cost(view, h.id).is_some_and(|(cost, tax)| {
                                let max_early = if land_count == 1 { 1 } else { 3 };
                                cost.with_x(0).cmc().saturating_add(tax) <= max_early
                                    && (0..5)
                                        .all(|c| count_color_pips(&cost, c) == 0 || hand_colors[c])
                            })
                    })
                    .count();
                let any_color_castable_up_to_4 = view.hand.iter().any(|h| {
                    !is_true_land_card(view, h.id)
                        && lookup_spell_cost(view, h.id).is_some_and(|(cost, tax)| {
                            cost.with_x(0).cmc().saturating_add(tax) <= 4
                                && (0..5).all(|c| count_color_pips(&cost, c) == 0 || hand_colors[c])
                        })
                });
                let has_castable_early_spell = early_castable_count >= 1;
                let should_take = land_count == 0
                    || land_count >= 6
                    || (!has_choose_subtype_land && *next_is_free && !has_castable_early_spell)
                    || (!has_choose_subtype_land && *taken <= 1 && !any_color_castable_up_to_4)
                    || (land_count == 1
                        && *next_is_free
                        && *taken == 0
                        && early_castable_count < 2)
                    || (land_count == 1
                        && (*next_is_free || *taken == 0)
                        && !has_castable_early_spell);
                let should_keep = (2..=5).contains(&land_count) && has_castable_early_spell;
                if should_take || should_keep {
                    let mut trial = masked_logits.to_vec();
                    for (i, c) in cands.candidates.iter().enumerate() {
                        if (should_take && matches!(c.action, Some(PlayerAction::MulliganKeep)))
                            || (should_keep && matches!(c.action, Some(PlayerAction::MulliganTake)))
                        {
                            trial[i] = f32::NEG_INFINITY;
                        }
                    }
                    if trial.iter().any(|l| l.is_finite()) {
                        masked_logits.copy_from_slice(&trial);
                    }
                }
            }
        }
        Pending::ChooseCastMode {
            object, options, ..
        } => {
            if let Some(ci) = lookup_card_identity(view, *object)
                && let Some(def) = baylee_cards::by_index(ci.index)
            {
                let abilities = def.abilities_for_face(usize::from(ci.face));
                let modal_modes = abilities.iter().find_map(|ab| match ab {
                    AbilityDef::ModalSpell { modes, .. } => Some(*modes),
                    _ => None,
                });
                if let Some(modes) = modal_modes {
                    let mut trial = masked_logits.to_vec();
                    let urgent_opp_ench = opp_has_urgent_enchantment(view, actor);
                    let mut any_ench_removal = false;
                    for (i, c) in cands.candidates.iter().enumerate() {
                        if let Some(PlayerAction::ChooseMode(opt_idx)) = c.action
                            && let Some(opt) = options.get(opt_idx)
                            && let baylee_engine::choice::CastModeKind::Mode(m_idx) = opt.kind
                            && let Some(mode) = modes.get(m_idx)
                        {
                            if !is_modal_mode_worth_casting(view, actor, mode) {
                                trial[i] = f32::NEG_INFINITY;
                            } else if urgent_opp_ench && mode_destroys_enchantment(mode) {
                                any_ench_removal = true;
                            }
                        }
                    }
                    if urgent_opp_ench && any_ench_removal {
                        for (i, c) in cands.candidates.iter().enumerate() {
                            if let Some(PlayerAction::ChooseMode(opt_idx)) = c.action
                                && let Some(opt) = options.get(opt_idx)
                                && let baylee_engine::choice::CastModeKind::Mode(m_idx) = opt.kind
                                && let Some(mode) = modes.get(m_idx)
                                && !mode_destroys_enchantment(mode)
                            {
                                trial[i] = f32::NEG_INFINITY;
                            }
                        }
                    }
                    if trial.iter().any(|l| l.is_finite()) {
                        masked_logits.copy_from_slice(&trial);
                    }
                }
            }
        }
        Pending::ChooseColor { .. } => {
            let scores: Vec<u32> = cands
                .candidates
                .iter()
                .map(|c| match c.action {
                    Some(PlayerAction::ChooseColor(mc)) => color_demand_score(view, actor, mc),
                    _ => 0,
                })
                .collect();
            let max_score = scores.iter().copied().max().unwrap_or(0);
            if max_score > 0 {
                let mut trial = masked_logits.to_vec();
                for (i, &s) in scores.iter().enumerate() {
                    if s < max_score {
                        trial[i] = f32::NEG_INFINITY;
                    }
                }
                if trial.iter().any(|l| l.is_finite()) {
                    masked_logits.copy_from_slice(&trial);
                }
            }
        }
        Pending::ChooseSubtype { .. } => {
            let scores: Vec<u32> = cands
                .candidates
                .iter()
                .map(|c| match c.action {
                    Some(PlayerAction::ChooseSubtype(st)) => subtype_demand_score(view, actor, st),
                    _ => 0,
                })
                .collect();
            let max_score = scores.iter().copied().max().unwrap_or(0);
            if max_score > 0 {
                let mut trial = masked_logits.to_vec();
                for (i, &s) in scores.iter().enumerate() {
                    if s < max_score {
                        trial[i] = f32::NEG_INFINITY;
                    }
                }
                if trial.iter().any(|l| l.is_finite()) {
                    masked_logits.copy_from_slice(&trial);
                }
            }
        }
        Pending::YesNo { prompt, source, .. } => {
            let req_mana = match prompt {
                YesNoPrompt::PayTax { mana } => Some(u32::from(*mana)),
                YesNoPrompt::PayMana { cost } => Some(cost.with_x(0).cmc()),
                _ => None,
            };
            let decline_shock = if let YesNoPrompt::PayLifeOrEnterTapped { amount } = prompt {
                let my_life = view.seat(actor).map_or(20, |s| s.life);
                let amt = i32::from(*amount);
                let opp_power: i32 = view
                    .battlefield
                    .iter()
                    .filter(|o| {
                        o.controller != actor
                            && o.types.contains(TypeSet::CREATURE)
                            && !o.status.contains(ObjectStatus::PHASED_OUT)
                    })
                    .map(|o| i32::from(o.power.unwrap_or(0)).max(0))
                    .sum();
                let pool_total = view
                    .seat(actor)
                    .map_or(0, |s| u32::try_from(s.mana_pool.total()).unwrap_or(0));
                let avail_now = pool_total.saturating_add(untapped_mana_production(view, actor));
                let needs_one_more_mana = view.hand.iter().any(|h| {
                    !is_true_land_card(view, h.id)
                        && lookup_spell_cost(view, h.id).is_some_and(|(c, t)| {
                            let req = c.with_x(0).cmc().saturating_add(t);
                            req == avail_now || req == avail_now.saturating_add(1)
                        })
                });
                my_life <= amt + 5 || my_life - amt <= opp_power || !needs_one_more_mana
            } else {
                false
            };
            let decline_draw = should_decline_yes_no_for_library(view, actor, *source);
            let decline_tax =
                req_mana.is_some_and(|req| !should_pay_tax(view, actor, *source, req));
            let force_pay_tax =
                matches!(prompt, YesNoPrompt::PayTax { .. }) && !decline_draw && !decline_tax;
            if decline_draw || decline_tax || decline_shock {
                let mut trial = masked_logits.to_vec();
                for (i, c) in cands.candidates.iter().enumerate() {
                    if matches!(c.action, Some(PlayerAction::YesNo(true))) {
                        trial[i] = f32::NEG_INFINITY;
                    }
                }
                if trial.iter().any(|l| l.is_finite()) {
                    masked_logits.copy_from_slice(&trial);
                }
            } else if force_pay_tax {
                let mut trial = masked_logits.to_vec();
                for (i, c) in cands.candidates.iter().enumerate() {
                    if matches!(c.action, Some(PlayerAction::YesNo(false))) {
                        trial[i] = f32::NEG_INFINITY;
                    }
                }
                if trial.iter().any(|l| l.is_finite()) {
                    masked_logits.copy_from_slice(&trial);
                }
            }
        }
        Pending::ChooseCards {
            options, prompt, ..
        } => {
            if matches!(
                prompt,
                ChoicePrompt::SearchLibrary | ChoicePrompt::PutIntoHand | ChoicePrompt::Wish
            ) {
                let tiers: Vec<i32> = cands
                    .candidates
                    .iter()
                    .map(|c| match &c.action {
                        Some(PlayerAction::ChooseObjects { objects }) if objects.len() == 1 => {
                            tutor_card_tier(view, actor, objects[0])
                        }
                        _ => 0,
                    })
                    .collect();
                if let Some(max_tier) = tiers.iter().copied().max() {
                    let mut trial = masked_logits.to_vec();
                    for (i, &t) in tiers.iter().enumerate() {
                        if t < max_tier {
                            trial[i] = f32::NEG_INFINITY;
                        }
                    }
                    if trial.iter().any(|l| l.is_finite()) {
                        masked_logits.copy_from_slice(&trial);
                    }
                }
            } else if is_discard_choose_cards(view, *prompt, options, active_source) {
                let tiers: Vec<i32> = cands
                    .candidates
                    .iter()
                    .map(|c| match &c.action {
                        Some(PlayerAction::ChooseObjects { objects }) if objects.len() == 1 => {
                            discard_card_tier(view, actor, objects[0]).0
                        }
                        _ => 0,
                    })
                    .collect();
                if let Some(max_tier) = tiers.iter().copied().max() {
                    let mut trial = masked_logits.to_vec();
                    for (i, &t) in tiers.iter().enumerate() {
                        if t < max_tier {
                            trial[i] = f32::NEG_INFINITY;
                        }
                    }
                    if trial.iter().any(|l| l.is_finite()) {
                        masked_logits.copy_from_slice(&trial);
                    }
                }
            } else if matches!(prompt, ChoicePrompt::CostSacrifice) {
                let src_id = active_source.map(|s| s.0);
                let tiers: Vec<i32> = cands
                    .candidates
                    .iter()
                    .map(|c| match &c.action {
                        Some(PlayerAction::ChooseObjects { objects }) if objects.len() == 1 => {
                            sacrifice_card_tier(view, actor, objects[0], src_id)
                        }
                        _ => 0,
                    })
                    .collect();
                if let Some(max_tier) = tiers.iter().copied().max() {
                    let mut trial = masked_logits.to_vec();
                    for (i, &t) in tiers.iter().enumerate() {
                        if t < max_tier {
                            trial[i] = f32::NEG_INFINITY;
                        }
                    }
                    if trial.iter().any(|l| l.is_finite()) {
                        masked_logits.copy_from_slice(&trial);
                    }
                }
            } else if is_self_bounce_choose_cards(view, actor, *prompt, options, active_source) {
                let tiers: Vec<i32> = cands
                    .candidates
                    .iter()
                    .map(|c| match &c.action {
                        Some(PlayerAction::ChooseObjects { objects }) if objects.len() == 1 => {
                            self_bounce_card_tier(view, actor, objects[0])
                        }
                        _ => 0,
                    })
                    .collect();
                if let Some(max_tier) = tiers.iter().copied().max() {
                    let mut trial = masked_logits.to_vec();
                    for (i, &t) in tiers.iter().enumerate() {
                        if t < max_tier {
                            trial[i] = f32::NEG_INFINITY;
                        }
                    }
                    if trial.iter().any(|l| l.is_finite()) {
                        masked_logits.copy_from_slice(&trial);
                    }
                }
            }
        }
        Pending::Arrange { piles, .. }
            if piles.len() == 2
                && piles.iter().all(|p| p.min == 0)
                && piles.iter().any(|p| p.place == ArrangePlace::LibraryTop)
                && piles.iter().any(|p| {
                    matches!(
                        p.place,
                        ArrangePlace::LibraryBottom | ArrangePlace::Graveyard
                    )
                }) =>
        {
            let my_lands_board = view
                .battlefield_of(actor)
                .filter(|o| {
                    o.types.contains(TypeSet::LAND) && !o.status.contains(ObjectStatus::PHASED_OUT)
                })
                .count();
            let my_lands_hand = view
                .hand
                .iter()
                .filter(|h| is_true_land_card(view, h.id))
                .count();
            let total_lands = my_lands_board + my_lands_hand;
            if total_lands >= 5 || total_lands <= 2 {
                let want_land_on_top = total_lands <= 2;
                let n = cands.candidates.len().min(masked_logits.len());
                let score_cand = |idx: usize| -> i32 {
                    let Some(PlayerAction::Arrange { piles: ref ap }) =
                        cands.candidates[idx].action
                    else {
                        return i32::MIN;
                    };
                    if ap.len() != 2 {
                        return i32::MIN;
                    }
                    let mut s = 0_i32;
                    for (p_idx, pile_cards) in ap.iter().enumerate() {
                        let is_top = piles[p_idx].place == ArrangePlace::LibraryTop;
                        for &cid in pile_cards {
                            let is_land = is_true_land_card(view, cid);
                            let good = if is_top {
                                is_land == want_land_on_top
                            } else {
                                is_land != want_land_on_top
                            };
                            if good {
                                s += 2;
                            } else {
                                s -= 2;
                            }
                        }
                    }
                    s
                };
                let best_s = (0..n)
                    .filter(|&i| masked_logits[i].is_finite())
                    .map(&score_cand)
                    .max()
                    .unwrap_or(i32::MIN);
                if best_s > i32::MIN {
                    let mut trial = masked_logits.to_vec();
                    for (i, t) in trial.iter_mut().enumerate().take(n) {
                        if t.is_finite() && score_cand(i) < best_s {
                            *t = f32::NEG_INFINITY;
                        }
                    }
                    if trial.iter().any(|l| l.is_finite()) {
                        masked_logits.copy_from_slice(&trial);
                    }
                }
            }
        }
        _ => {}
    }
}

/// Adjusts `SubsetPick` candidate logits before constrained subset sampling
/// (`ChooseAttackers`, `ChooseTargets`, `DiscardChoice`, `MulliganBottom`, `ChooseCards`).
#[allow(
    clippy::too_many_lines,
    clippy::cast_precision_loss,
    reason = "Adjusts SubsetPick logits across attackers, targets, discards, and tutors"
)]
pub(super) fn adjust_subset_logits(
    cands: &EncodedCandidates4,
    logits: &[f32],
    view: &PlayerView,
    pending: &Pending,
    active_source: Option<(ObjectId, Option<u32>)>,
) -> Vec<f32> {
    let n = cands.candidates.len().min(logits.len());
    let mut out = logits[..n].to_vec();
    let actor = view.decision_player.unwrap_or(view.seat);

    match pending {
        Pending::ChooseAttackers {
            defenders,
            required,
            ..
        } => {
            let mut pos_attacker_ids = Vec::new();
            let mut non_engine_powers = Vec::new();
            let first_def_pid = defenders.iter().find_map(|d| match d {
                Defender::Player(p) => Some(*p),
                _ => None,
            });
            let mut unblockable_dmg = 0_i32;
            for cand in &cands.candidates[..n] {
                if let Some(oid) = cand.object_id
                    && !pos_attacker_ids.contains(&oid)
                    && let Some(o) = view.object(oid)
                    && o.power.unwrap_or(0) > 0
                {
                    pos_attacker_ids.push(oid);
                    if !is_protected_engine_or_catastrophic_ltb(view, o) {
                        let p = i32::from(o.power.unwrap_or(0));
                        non_engine_powers.push(p);
                        let min_blks = if has_kw(o, KeywordSet::MENACE) { 2 } else { 1 };
                        if let Some(def_pid) = first_def_pid
                            && view
                                .battlefield_of(def_pid)
                                .filter(|b| {
                                    b.types.contains(TypeSet::CREATURE)
                                        && !b.status.contains(ObjectStatus::TAPPED)
                                        && !b.status.contains(ObjectStatus::PHASED_OUT)
                                        && !has_kw(b, KeywordSet::CANT_BLOCK)
                                        && can_legally_block(o, b)
                                })
                                .count()
                                < min_blks
                        {
                            unblockable_dmg += p;
                        }
                    }
                }
            }
            non_engine_powers.sort_unstable();
            let total_pos_attackers = pos_attacker_ids.len();
            let my_life = view.seat(actor).map_or(20, |s| s.life);
            let my_lib = view.seat(actor).map_or(99, |s| s.library_count);

            let mut hold_back_for_crackback: BTreeSet<ObjectId> = BTreeSet::new();
            if let Some(def_pid) = first_def_pid {
                let def_life = view.seat(def_pid).map_or(20, |p| p.life);
                let total_def_blockers = view
                    .battlefield_of(def_pid)
                    .filter(|b| {
                        b.types.contains(TypeSet::CREATURE)
                            && !b.status.contains(ObjectStatus::TAPPED)
                            && !b.status.contains(ObjectStatus::PHASED_OUT)
                            && !has_kw(b, KeywordSet::CANT_BLOCK)
                    })
                    .count();
                let guaranteed_unblocked_count =
                    non_engine_powers.len().saturating_sub(total_def_blockers);
                let guaranteed_unblocked_dmg: i32 = non_engine_powers
                    .iter()
                    .take(guaranteed_unblocked_count)
                    .sum();
                if unblockable_dmg < def_life && guaranteed_unblocked_dmg < def_life {
                    let my_untapped_blockers: Vec<&PublicObject> = view
                        .battlefield_of(actor)
                        .filter(|o| {
                            o.types.contains(TypeSet::CREATURE)
                                && !o.status.contains(ObjectStatus::TAPPED)
                                && !o.status.contains(ObjectStatus::PHASED_OUT)
                                && !has_kw(o, KeywordSet::CANT_BLOCK)
                        })
                        .collect();
                    let mut opp_blockable_crackback_objs: Vec<&PublicObject> = Vec::new();
                    let mut opp_unblockable_crackback = 0_i32;
                    for b in view.battlefield_of(def_pid).filter(|b| {
                        b.types.contains(TypeSet::CREATURE)
                            && !b.status.contains(ObjectStatus::PHASED_OUT)
                            && !has_kw(b, KeywordSet::DEFENDER)
                            && b.power.unwrap_or(0) > 0
                    }) {
                        let min_b = if has_kw(b, KeywordSet::MENACE) { 2 } else { 1 };
                        let can_blk_cnt = my_untapped_blockers
                            .iter()
                            .filter(|&&o| can_legally_block(b, o))
                            .count();
                        if can_blk_cnt >= min_b {
                            opp_blockable_crackback_objs.push(b);
                        } else {
                            opp_unblockable_crackback += i32::from(b.power.unwrap_or(0));
                        }
                    }
                    let mut opp_crackback_powers: Vec<i32> = opp_blockable_crackback_objs
                        .iter()
                        .map(|b| i32::from(b.power.unwrap_or(0)))
                        .collect();
                    opp_crackback_powers.sort_unstable_by(|a, b| b.cmp(a));
                    let already_holding_objs: Vec<&PublicObject> = my_untapped_blockers
                        .iter()
                        .copied()
                        .filter(|o| {
                            (!pos_attacker_ids.contains(&o.id) || has_kw(o, KeywordSet::VIGILANCE))
                                && dies_self_damage_amount(o) < my_life
                                && opp_blockable_crackback_objs
                                    .iter()
                                    .any(|b| can_legally_block(b, o))
                        })
                        .collect();
                    let already_holding = already_holding_objs.len();
                    let mut held_self_dmg: i32 = if opp_crackback_powers.is_empty() {
                        0
                    } else {
                        already_holding_objs
                            .iter()
                            .copied()
                            .map(dies_self_damage_amount)
                            .sum()
                    };
                    let rem_crackback: i32 = opp_unblockable_crackback
                        + opp_crackback_powers
                            .iter()
                            .skip(already_holding)
                            .sum::<i32>()
                        + held_self_dmg;
                    if opp_unblockable_crackback < my_life && rem_crackback >= my_life {
                        let mut hold_candidates: Vec<(ObjectId, i32, i32, i32)> = pos_attacker_ids
                            .iter()
                            .copied()
                            .filter(|id| !required.contains(id))
                            .filter_map(|id| {
                                let o = view.object(id)?;
                                let self_dmg = dies_self_damage_amount(o);
                                if has_kw(o, KeywordSet::VIGILANCE)
                                    || has_kw(o, KeywordSet::CANT_BLOCK)
                                    || self_dmg >= my_life
                                    || !opp_blockable_crackback_objs
                                        .iter()
                                        .any(|b| can_legally_block(b, o))
                                {
                                    return None;
                                }
                                Some((
                                    id,
                                    i32::from(o.power.unwrap_or(0)),
                                    i32::from(o.toughness.unwrap_or(1)),
                                    self_dmg,
                                ))
                            })
                            .collect();
                        hold_candidates.sort_by(|a, b| a.1.cmp(&b.1).then_with(|| b.2.cmp(&a.2)));
                        let mut cur_skip = already_holding;
                        for (cid, _, _, self_dmg) in hold_candidates {
                            let blocked_p =
                                opp_crackback_powers.get(cur_skip).copied().unwrap_or(0);
                            if blocked_p <= self_dmg {
                                continue;
                            }
                            hold_back_for_crackback.insert(cid);
                            cur_skip += 1;
                            held_self_dmg += self_dmg;
                            let next_rem: i32 = opp_unblockable_crackback
                                + opp_crackback_powers.iter().skip(cur_skip).sum::<i32>()
                                + held_self_dmg;
                            if next_rem < my_life {
                                break;
                            }
                        }
                    }
                }
            }

            for (i, cand) in cands.candidates[..n].iter().enumerate() {
                let Some(att_id) = cand.object_id else {
                    continue;
                };
                if required.contains(&att_id) {
                    continue;
                }
                if hold_back_for_crackback.contains(&att_id) {
                    out[i] = -20.0;
                    continue;
                }
                let Some(att_obj) = view.object(att_id) else {
                    continue;
                };
                let att_power = att_obj.power.unwrap_or(0);
                if att_power <= 0 {
                    out[i] = -20.0;
                    continue;
                }
                let def_target = cand.defender.or_else(|| defenders.first().copied());
                let Some(Defender::Player(def_pid)) = def_target else {
                    continue;
                };
                let is_fragile_engine = is_protected_engine_or_catastrophic_ltb(view, att_obj);
                let total_def_blockers = view
                    .battlefield_of(def_pid)
                    .filter(|b| {
                        b.types.contains(TypeSet::CREATURE)
                            && !b.status.contains(ObjectStatus::TAPPED)
                            && !b.status.contains(ObjectStatus::PHASED_OUT)
                            && !has_kw(b, KeywordSet::CANT_BLOCK)
                    })
                    .count();
                let min_blks = if has_kw(att_obj, KeywordSet::MENACE) {
                    2
                } else {
                    1
                };
                let is_unblockable = !is_fragile_engine
                    && view
                        .battlefield_of(def_pid)
                        .filter(|b| {
                            b.types.contains(TypeSet::CREATURE)
                                && !b.status.contains(ObjectStatus::TAPPED)
                                && !b.status.contains(ObjectStatus::PHASED_OUT)
                                && !has_kw(b, KeywordSet::CANT_BLOCK)
                                && can_legally_block(att_obj, b)
                        })
                        .count()
                        < min_blks;
                let def_life = view.seat(def_pid).map_or(20, |p| p.life);
                let guaranteed_unblocked_count =
                    non_engine_powers.len().saturating_sub(total_def_blockers);
                let guaranteed_unblocked_dmg: i32 = non_engine_powers
                    .iter()
                    .take(guaranteed_unblocked_count)
                    .sum();
                let has_vigilance = has_kw(att_obj, KeywordSet::VIGILANCE);
                if is_unblockable && unblockable_dmg >= def_life {
                    out[i] = out[i].max(4.0);
                    continue;
                }
                let opp_total_crackback_power: i32 = view
                    .battlefield_of(def_pid)
                    .filter(|b| {
                        b.types.contains(TypeSet::CREATURE)
                            && !b.status.contains(ObjectStatus::PHASED_OUT)
                            && !has_kw(b, KeywordSet::DEFENDER)
                    })
                    .map(|b| i32::from(b.power.unwrap_or(0)).max(0))
                    .sum();
                let mut sum_legal_blk_power = 0_i32;
                let mut any_dt_blk = false;
                let mut any_stonewall_blk = false;
                for b in view.battlefield_of(def_pid) {
                    if b.types.contains(TypeSet::CREATURE)
                        && !b.status.contains(ObjectStatus::TAPPED)
                        && !b.status.contains(ObjectStatus::PHASED_OUT)
                        && !has_kw(b, KeywordSet::CANT_BLOCK)
                        && can_legally_block(att_obj, b)
                    {
                        sum_legal_blk_power += i32::from(b.power.unwrap_or(0)).max(0);
                        if has_kw(b, KeywordSet::DEATHTOUCH) {
                            any_dt_blk = true;
                        }
                        let b_rem_t =
                            (i32::from(b.toughness.unwrap_or(1)) - i32::from(b.damage)).max(1);
                        if b_rem_t > i32::from(att_power) || has_kw(b, KeywordSet::INDESTRUCTIBLE) {
                            any_stonewall_blk = true;
                        }
                    }
                }
                let att_rem_t =
                    (i32::from(att_obj.toughness.unwrap_or(1)) - i32::from(att_obj.damage)).max(1);
                let is_unkillable_juggernaut = !is_fragile_engine
                    && att_power >= 4
                    && !any_dt_blk
                    && (!any_stonewall_blk || has_kw(att_obj, KeywordSet::TRAMPLE))
                    && sum_legal_blk_power < att_rem_t
                    && (has_vigilance
                        || (my_life >= 10 && my_life - opp_total_crackback_power >= 5));
                if is_unkillable_juggernaut {
                    out[i] = out[i].max(2.0);
                    continue;
                }
                let death_payoff = friendly_creature_death_payoff(view, actor, att_obj);
                let go_wide_alpha = !is_fragile_engine
                    && ((guaranteed_unblocked_count > 0 && guaranteed_unblocked_dmg >= def_life)
                        || (non_engine_powers.len() >= 8
                            && non_engine_powers.len() >= total_def_blockers + 4
                            && (total_def_blockers <= 5 || guaranteed_unblocked_dmg >= 4))
                        || (my_lib <= 12
                            && non_engine_powers.len() > total_def_blockers
                            && guaranteed_unblocked_dmg >= 2)
                        || (has_vigilance && non_engine_powers.len() >= total_def_blockers + 4)
                        || (total_def_blockers <= 1
                            && non_engine_powers.len() >= 3
                            && guaranteed_unblocked_dmg >= 2
                            && (death_payoff >= 1 || my_life >= 12))
                        || (is_unblockable
                            && (has_vigilance
                                || has_kw(att_obj, KeywordSet::CANT_BLOCK)
                                || total_pos_attackers >= total_def_blockers + 2
                                || my_life >= 12
                                || my_life - opp_total_crackback_power >= 5)));
                if go_wide_alpha {
                    out[i] = out[i].max(2.0);
                    continue;
                }
                if is_suicide_attack_into_blockers(view, att_obj, def_pid, total_pos_attackers) {
                    out[i] = -20.0;
                }
            }
        }
        Pending::DiscardChoice { .. } | Pending::MulliganBottom { .. } => {
            for (i, cand) in cands.candidates[..n].iter().enumerate() {
                if let Some(oid) = cand.object_id {
                    let (tier, bonus) = discard_card_tier(view, actor, oid);
                    out[i] += (tier as f32) * 10.0 + bonus;
                }
            }
        }
        Pending::ChooseCards {
            options, prompt, ..
        } => {
            if matches!(
                prompt,
                ChoicePrompt::SearchLibrary | ChoicePrompt::PutIntoHand | ChoicePrompt::Wish
            ) {
                for (i, cand) in cands.candidates[..n].iter().enumerate() {
                    if let Some(oid) = cand.object_id {
                        let tier = tutor_card_tier(view, actor, oid);
                        out[i] += (tier as f32) * 10.0;
                    }
                }
            } else if is_discard_choose_cards(view, *prompt, options, active_source) {
                for (i, cand) in cands.candidates[..n].iter().enumerate() {
                    if let Some(oid) = cand.object_id {
                        let (tier, bonus) = discard_card_tier(view, actor, oid);
                        out[i] += (tier as f32) * 10.0 + bonus;
                    }
                }
            } else if matches!(prompt, ChoicePrompt::CostSacrifice) {
                let src_id = active_source.map(|s| s.0);
                for (i, cand) in cands.candidates[..n].iter().enumerate() {
                    if let Some(oid) = cand.object_id {
                        let tier = sacrifice_card_tier(view, actor, oid, src_id);
                        out[i] += (tier as f32) * 10.0;
                    }
                }
            } else if is_self_bounce_choose_cards(view, actor, *prompt, options, active_source) {
                for (i, cand) in cands.candidates[..n].iter().enumerate() {
                    if let Some(oid) = cand.object_id {
                        let tier = self_bounce_card_tier(view, actor, oid);
                        out[i] += (tier as f32) * 10.0;
                    }
                }
            }
        }
        Pending::ChooseTargets { min, .. } => {
            let intent = classify_active_target_intent(view, actor, active_source);
            let fixed_dmg = active_source_fixed_damage(view, active_source);
            let any_opp_player_target = cands.candidates[..n]
                .iter()
                .any(|cand| cand.player_id.is_some_and(|p| p != actor));
            let any_lethal_creature_kill = fixed_dmg.is_some_and(|dmg| {
                cands.candidates[..n].iter().any(|cand| {
                    cand.object_id
                        .and_then(|oid| view.object(oid))
                        .is_some_and(|o| {
                            o.controller != actor
                                && o.types.contains(TypeSet::CREATURE)
                                && !has_kw(o, KeywordSet::INDESTRUCTIBLE)
                                && !(any_opp_player_target
                                    && dmg >= 2
                                    && o.power.unwrap_or(0) <= 0
                                    && o.token.is_some())
                                && (i32::from(o.toughness.unwrap_or(1)) - i32::from(o.damage))
                                    .max(1)
                                    <= dmg
                        })
                })
            });
            let any_urgent_ench_target = cands.candidates[..n].iter().any(|cand| {
                cand.object_id
                    .and_then(|oid| view.object(oid))
                    .is_some_and(|o| o.controller != actor && is_urgent_enchantment(o))
            });
            if intent != TargetIntent::Neutral {
                for (i, cand) in cands.candidates[..n].iter().enumerate() {
                    let obj_opt = cand.object_id.and_then(|oid| view.object(oid));
                    let is_friendly = obj_opt.is_some_and(|o| o.controller == actor)
                        || cand.player_id == Some(actor);
                    let is_hostile = obj_opt.is_some_and(|o| o.controller != actor)
                        || cand.player_id.is_some_and(|p| p != actor);
                    match intent {
                        TargetIntent::Hostile => {
                            if is_friendly {
                                out[i] -= 15.0;
                            } else if is_hostile {
                                out[i] += 2.0;
                                if let Some(o) = obj_opt {
                                    if any_urgent_ench_target && is_urgent_enchantment(o) {
                                        out[i] += 10.0;
                                    }
                                    if let Some(dmg) = fixed_dmg
                                        && any_lethal_creature_kill
                                        && o.types.contains(TypeSet::CREATURE)
                                        && !has_kw(o, KeywordSet::INDESTRUCTIBLE)
                                        && !(any_opp_player_target
                                            && dmg >= 2
                                            && o.power.unwrap_or(0) <= 0
                                            && o.token.is_some())
                                    {
                                        let rem_t = (i32::from(o.toughness.unwrap_or(1))
                                            - i32::from(o.damage))
                                        .max(1);
                                        if rem_t <= dmg {
                                            out[i] += 8.0
                                                + (i32::from(o.power.unwrap_or(0)).max(0) as f32)
                                                    * 0.5;
                                        }
                                    }
                                }
                            }
                        }
                        TargetIntent::Beneficial => {
                            if is_hostile {
                                out[i] -= 15.0;
                            } else if is_friendly {
                                let tier = cand
                                    .object_id
                                    .and_then(|oid| view.object(oid))
                                    .map_or(0, |o| beneficial_creature_target_tier(actor, o));
                                out[i] += 2.0 + (tier as f32) * 0.5;
                            }
                        }
                        TargetIntent::Neutral => {}
                    }
                }
            }
            if *min <= 1 && is_single_target_concentrated_source(view, active_source) && n > 0 {
                let mut best_i = 0;
                for i in 1..n {
                    if out[i] > out[best_i] {
                        best_i = i;
                    }
                }
                let best_val = out[best_i].max(2.0) + 5.0;
                for (i, v) in out.iter_mut().enumerate() {
                    if i == best_i {
                        *v = best_val;
                    } else {
                        *v = -15.0;
                    }
                }
            }
        }
        _ => {}
    }

    out
}

fn is_single_target_concentrated_source(
    view: &PlayerView,
    active_source: Option<(ObjectId, Option<u32>)>,
) -> bool {
    let src_oid = active_source.map(|(oid, _)| oid).or(view.casting);
    if let Some(oid) = src_oid
        && let Some(ci) = lookup_card_identity(view, oid)
        && let Some(def) = baylee_cards::by_index(ci.index)
    {
        let face_idx = usize::from(ci.face);
        if def
            .faces
            .get(face_idx)
            .is_some_and(|f| f.extra_target_cost > 0)
        {
            return true;
        }
        for ab in def.abilities_for_face(face_idx) {
            let effects: &[Effect] = match ab {
                AbilityDef::Spell { effects, .. }
                | AbilityDef::Activated { effects, .. }
                | AbilityDef::ActivatedConditional { effects, .. }
                | AbilityDef::Triggered { effects, .. } => effects,
                _ => continue,
            };
            if effects
                .iter()
                .any(|e| matches!(e, Effect::DealDamageEvenly { .. }))
            {
                return true;
            }
        }
    }
    false
}

pub(super) fn beneficial_creature_target_tier(actor: PlayerId, o: &PublicObject) -> i32 {
    if o.controller != actor || !o.types.contains(TypeSet::CREATURE) {
        return 0;
    }
    let p = i32::from(o.power.unwrap_or(0)).max(0);
    let t = (i32::from(o.toughness.unwrap_or(1)) - i32::from(o.damage)).max(1);
    let kw = o.keywords;
    let has_evasion_or_combat_kw = has_kw(o, KeywordSet::FLYING)
        || has_kw(o, KeywordSet::MENACE)
        || has_kw(o, KeywordSet::TRAMPLE)
        || has_kw(o, KeywordSet::LIFELINK)
        || has_kw(o, KeywordSet::DOUBLE_STRIKE)
        || has_kw(o, KeywordSet::FIRST_STRIKE)
        || has_kw(o, KeywordSet::VIGILANCE);
    let can_attack_now = !o.status.contains(ObjectStatus::TAPPED)
        && !o.summoning_sick
        && (kw & KeywordSet::DEFENDER.bits()) == 0;
    let is_step_begin_engine =
        super::tactics::abilities_of_battlefield_object(o).is_some_and(|abs| {
            abs.iter().any(|ab| {
                matches!(
                    ab,
                    AbilityDef::Triggered {
                        trigger: Trigger::StepBegin {
                            step: StepKind::CombatBegin | StepKind::Upkeep,
                            whose: PlayerRel::You
                        },
                        ..
                    }
                )
            })
        });
    let death_payoff = self_creature_death_payoff(o);
    let mut tier = (p + t).min(8);
    if is_step_begin_engine && p <= 3 && t <= 3 {
        tier += 6;
    }
    if has_evasion_or_combat_kw {
        tier += 5;
    }
    if can_attack_now {
        tier += 2;
    }
    if death_payoff > 0 && !has_evasion_or_combat_kw {
        tier -= 4;
    }
    tier
}

fn has_kw(obj: &PublicObject, kw: KeywordSet) -> bool {
    (obj.keywords & kw.bits()) != 0
}

#[allow(
    clippy::too_many_lines,
    reason = "Evaluates surviving blockers, go-wide thresholds, death triggers, and trading-down prevention"
)]
fn is_suicide_attack_into_blockers(
    view: &PlayerView,
    att_obj: &PublicObject,
    def_pid: PlayerId,
    total_pos_attackers: usize,
) -> bool {
    if has_kw(att_obj, KeywordSet::INDESTRUCTIBLE) || has_kw(att_obj, KeywordSet::UNBLOCKABLE) {
        return false;
    }
    let att_power = i32::from(att_obj.power.unwrap_or(0)).max(0);
    let att_rem_tough =
        (i32::from(att_obj.toughness.unwrap_or(1)) - i32::from(att_obj.damage)).max(1);
    let att_deathtouch = has_kw(att_obj, KeywordSet::DEATHTOUCH) && att_power > 0;

    let legal_blockers: Vec<&PublicObject> = view
        .battlefield_of(def_pid)
        .filter(|b| {
            b.types.contains(TypeSet::CREATURE)
                && !b.status.contains(ObjectStatus::TAPPED)
                && !b.status.contains(ObjectStatus::PHASED_OUT)
                && !has_kw(b, KeywordSet::CANT_BLOCK)
                && can_legally_block(att_obj, b)
        })
        .collect();

    if legal_blockers.is_empty() {
        return false;
    }

    let self_death_payoff = self_creature_death_payoff(att_obj);
    let death_payoff = friendly_creature_death_payoff(view, att_obj.controller, att_obj);
    if att_power <= 2
        && self_death_payoff >= 2
        && total_pos_attackers >= legal_blockers.len()
        && legal_blockers.len() <= 2
        && !has_kw(att_obj, KeywordSet::FLYING)
    {
        return false;
    }

    // 1. Check blockers that strictly survive combat with `att_obj` (rem_tough > att_power or First Strike kill)
    if !att_deathtouch && total_pos_attackers < legal_blockers.len() + 4 {
        let att_first =
            has_kw(att_obj, KeywordSet::FIRST_STRIKE) || has_kw(att_obj, KeywordSet::DOUBLE_STRIKE);
        let surviving: Vec<&&PublicObject> = legal_blockers
            .iter()
            .filter(|b| {
                let b_rem = (i32::from(b.toughness.unwrap_or(1)) - i32::from(b.damage)).max(1);
                let bp = i32::from(b.power.unwrap_or(0));
                let b_first_kills = !att_first
                    && (has_kw(b, KeywordSet::FIRST_STRIKE)
                        || has_kw(b, KeywordSet::DOUBLE_STRIKE))
                    && (bp >= att_rem_tough || (has_kw(b, KeywordSet::DEATHTOUCH) && bp > 0));
                b_rem > att_power || has_kw(b, KeywordSet::INDESTRUCTIBLE) || b_first_kills
            })
            .collect();

        let min_blockers = if has_kw(att_obj, KeywordSet::MENACE) {
            2
        } else {
            1
        };
        if surviving.len() >= min_blockers {
            let free_eaters = surviving
                .iter()
                .filter(|b| {
                    let bp = i32::from(b.power.unwrap_or(0));
                    bp >= att_rem_tough || (has_kw(b, KeywordSet::DEATHTOUCH) && bp > 0)
                })
                .count();
            let allow_go_wide_into_single_blocker =
                legal_blockers.len() == 1 && total_pos_attackers >= 3 && death_payoff >= 1;
            let single_free_eater =
                min_blockers == 1 && free_eaters >= 1 && !allow_go_wide_into_single_blocker;
            let gang_free_eat = !allow_go_wide_into_single_blocker
                && total_pos_attackers < legal_blockers.len() + 4
                && {
                    let sum_p: i32 = surviving
                        .iter()
                        .map(|b| i32::from(b.power.unwrap_or(0)).max(0))
                        .sum();
                    let any_dt = surviving
                        .iter()
                        .any(|b| has_kw(b, KeywordSet::DEATHTOUCH) && b.power.unwrap_or(0) > 0);
                    sum_p >= att_rem_tough || any_dt
                };
            if single_free_eater || gang_free_eat {
                return true;
            }
            if min_blockers == 1 && total_pos_attackers <= legal_blockers.len() {
                for &&b_surv in &surviving {
                    let b_surv_p = i32::from(b_surv.power.unwrap_or(0)).max(0);
                    for &b_chump in &legal_blockers {
                        if b_chump.id == b_surv.id {
                            continue;
                        }
                        let b_chump_p = i32::from(b_chump.power.unwrap_or(0)).max(0);
                        let b_chump_rem = (i32::from(b_chump.toughness.unwrap_or(1))
                            - i32::from(b_chump.damage))
                        .max(1);
                        let chump_dt = has_kw(b_chump, KeywordSet::DEATHTOUCH) && b_chump_p > 0;
                        if (b_surv_p + b_chump_p >= att_rem_tough || chump_dt)
                            && b_chump_p + b_chump_rem + 1 < att_power + att_rem_tough
                        {
                            return true;
                        }
                    }
                }
                if !has_kw(att_obj, KeywordSet::TRAMPLE) && !allow_go_wide_into_single_blocker {
                    let surv_power: i32 = surviving
                        .iter()
                        .map(|b| i32::from(b.power.unwrap_or(0)).max(0))
                        .sum();
                    let expendable_token_power: i32 = legal_blockers
                        .iter()
                        .filter(|b| {
                            b.token.is_some()
                                && !surviving.iter().any(|s| s.id == b.id)
                                && i32::from(b.power.unwrap_or(0)).max(0)
                                    + (i32::from(b.toughness.unwrap_or(1)) - i32::from(b.damage))
                                        .max(1)
                                    <= 2
                        })
                        .map(|b| i32::from(b.power.unwrap_or(0)).max(0))
                        .sum();
                    if surv_power + expendable_token_power >= att_rem_tough {
                        return true;
                    }
                }
            }
        }
        if min_blockers == 2
            && !surviving.is_empty()
            && legal_blockers.len() >= 2
            && total_pos_attackers <= legal_blockers.len()
        {
            for &&b_surv in &surviving {
                let b_surv_p = i32::from(b_surv.power.unwrap_or(0)).max(0);
                let b_surv_rem =
                    (i32::from(b_surv.toughness.unwrap_or(1)) - i32::from(b_surv.damage)).max(1);
                for &b_other in &legal_blockers {
                    if b_other.id == b_surv.id {
                        continue;
                    }
                    let b_other_p = i32::from(b_other.power.unwrap_or(0)).max(0);
                    let b_other_rem = (i32::from(b_other.toughness.unwrap_or(1))
                        - i32::from(b_other.damage))
                    .max(1);
                    let kills_att = b_surv_p + b_other_p >= att_rem_tough
                        || (has_kw(b_surv, KeywordSet::DEATHTOUCH) && b_surv_p > 0)
                        || (has_kw(b_other, KeywordSet::DEATHTOUCH) && b_other_p > 0);
                    if kills_att
                        && ((b_surv_rem > att_power && att_power <= 1)
                            || b_other_p + b_other_rem < att_power + att_rem_tough)
                    {
                        return true;
                    }
                }
            }
        }
    }

    // 1b. Avoid trading down a higher-stat fragile attacker (e.g. 3/2 or 4/2) into a smaller blocker (e.g. 2/1 or two 1/1s)
    let def_life = view.seat(def_pid).map_or(20, |s| s.life);
    if !att_deathtouch
        && !has_kw(att_obj, KeywordSet::FIRST_STRIKE)
        && !has_kw(att_obj, KeywordSet::DOUBLE_STRIKE)
        && !has_kw(att_obj, KeywordSet::TRAMPLE)
        && !has_kw(att_obj, KeywordSet::MENACE)
        && att_power >= 3
        && att_rem_tough <= 3
        && death_payoff == 0
        && total_pos_attackers <= legal_blockers.len()
        && def_life > att_power
    {
        let trades_down = att_rem_tough <= 2
            && legal_blockers.iter().any(|b| {
                let bp = i32::from(b.power.unwrap_or(0)).max(0);
                let b_rem = (i32::from(b.toughness.unwrap_or(1)) - i32::from(b.damage)).max(1);
                let b_kills = bp >= att_rem_tough || (has_kw(b, KeywordSet::DEATHTOUCH) && bp > 0);
                b_kills
                    && bp < att_power
                    && ((b_rem <= att_rem_tough && bp + b_rem + 2 <= att_power + att_rem_tough)
                        || (att_rem_tough == 1 && legal_blockers.len() >= 2))
            });
        let small_one_power_blks = legal_blockers
            .iter()
            .filter(|b| {
                let bp = i32::from(b.power.unwrap_or(0)).max(0);
                let b_rem = (i32::from(b.toughness.unwrap_or(1)) - i32::from(b.damage)).max(1);
                bp == 1 && b_rem == 1
            })
            .count();
        let any_small_token = legal_blockers.iter().any(|b| {
            b.token.is_some()
                && i32::from(b.power.unwrap_or(0)).max(0) == 1
                && (i32::from(b.toughness.unwrap_or(1)) - i32::from(b.damage)).max(1) == 1
        });
        let double_one_one_trade = (total_pos_attackers == 1
            || total_pos_attackers < legal_blockers.len())
            && att_rem_tough == 2
            && small_one_power_blks >= 2
            && any_small_token;
        let gang_surviving_trade_down = total_pos_attackers <= legal_blockers.len()
            && (0..legal_blockers.len()).any(|i| {
                let b1 = legal_blockers[i];
                let b1_p = i32::from(b1.power.unwrap_or(0)).max(0);
                let b1_rem = (i32::from(b1.toughness.unwrap_or(1)) - i32::from(b1.damage)).max(1);
                b1_p + b1_rem < att_power + att_rem_tough
                    && ((i + 1)..legal_blockers.len()).any(|j| {
                        let b2 = legal_blockers[j];
                        let b2_p = i32::from(b2.power.unwrap_or(0)).max(0);
                        let b2_rem =
                            (i32::from(b2.toughness.unwrap_or(1)) - i32::from(b2.damage)).max(1);
                        b2_p + b2_rem < att_power + att_rem_tough
                            && b1_p + b2_p >= att_rem_tough
                            && b1_rem + b2_rem > att_power
                            && (b1_rem.min(b2_rem) + b1_p.min(b2_p) + 2
                                <= att_power + att_rem_tough)
                    })
            });
        if trades_down || double_one_one_trade || gang_surviving_trade_down {
            return true;
        }
    }

    // 2. Protect fragile tap-ability, combat-begin/upkeep/spellcast/lord/fog engine, or catastrophic-LTB creatures
    let is_small_spellcast_engine = total_pos_attackers <= legal_blockers.len() + 1
        && att_power <= 2
        && att_rem_tough <= 2
        && super::tactics::abilities_of_battlefield_object(att_obj).is_some_and(|abs| {
            abs.iter().any(|ab| {
                matches!(
                    ab,
                    AbilityDef::Triggered {
                        trigger: Trigger::SpellCast(_),
                        ..
                    }
                )
            })
        });
    let sum_blocker_powers: i32 = legal_blockers
        .iter()
        .map(|b| i32::from(b.power.unwrap_or(0)).max(0))
        .sum();
    if (is_protected_engine_or_catastrophic_ltb(view, att_obj) || is_small_spellcast_engine)
        && (sum_blocker_powers >= att_rem_tough
            || legal_blockers
                .iter()
                .any(|b| has_kw(b, KeywordSet::DEATHTOUCH) && b.power.unwrap_or(0) > 0))
    {
        return true;
    }

    false
}

fn dies_self_damage_amount(obj: &PublicObject) -> i32 {
    let Some(abs) = super::tactics::abilities_of_battlefield_object(obj) else {
        return 0;
    };
    let mut dmg = 0_i32;
    for ab in abs {
        if let AbilityDef::Triggered {
            trigger: Trigger::Dies(Filter::This) | Trigger::LeavesBattlefield(Filter::This),
            effects,
            ..
        } = ab
        {
            for eff in *effects {
                match eff {
                    Effect::DealDamage {
                        target: TargetSpec::Player(PlayerRel::You | PlayerRel::EachPlayer),
                        amount: Amount::Fixed(n),
                        ..
                    }
                    | Effect::LoseLife {
                        target: PlayerRel::You | PlayerRel::EachPlayer,
                        amount: Amount::Fixed(n),
                    } => {
                        dmg += i32::try_from(*n).unwrap_or(0);
                    }
                    _ => {}
                }
            }
        }
    }
    dmg
}

fn is_protected_engine_or_catastrophic_ltb(view: &PlayerView, obj: &PublicObject) -> bool {
    let rem_t = (i32::from(obj.toughness.unwrap_or(1)) - i32::from(obj.damage)).max(1);
    if rem_t <= 1 && has_non_mana_tap_ability(view, obj.id) {
        return true;
    }
    let Some(abs) = super::tactics::abilities_of_battlefield_object(obj) else {
        return false;
    };
    let p = i32::from(obj.power.unwrap_or(0)).max(0);
    abs.iter().any(|ab| match ab {
        AbilityDef::Triggered {
            trigger: Trigger::LeavesBattlefield(Filter::This) | Trigger::Dies(Filter::This),
            effects,
            ..
        } => effects.iter().any(|e| {
            matches!(
                e,
                Effect::ExileAll { .. } | Effect::DestroyAll { .. } | Effect::SacrificeAll { .. }
            )
        }),
        AbilityDef::Triggered {
            trigger:
                Trigger::StepBegin {
                    step: StepKind::CombatBegin | StepKind::Upkeep,
                    whose: PlayerRel::You,
                },
            ..
        } => p <= 2 && rem_t <= 2,
        AbilityDef::Static(_) => p <= 1 && rem_t <= 1,
        AbilityDef::Activated { effects, .. }
        | AbilityDef::ActivatedConditional { effects, .. } => {
            p <= 2
                && rem_t <= 2
                && effects
                    .iter()
                    .any(|e| matches!(e, Effect::PreventAllCombatDamageThisTurn))
        }
        _ => false,
    })
}

/// Adjusts `PairPick` (`Pending::ChooseBlockers`) logits so that blockers do not
/// redundantly pile onto a single attacker when other attackers go unblocked,
/// free kills are taken, and lethal combat damage is blocked.
#[allow(
    clippy::too_many_lines,
    reason = "Explicit combat assignment passes over view-visible attacker/blocker pairs"
)]
pub(super) fn adjust_pair_logits(
    cands: &EncodedCandidates4,
    raw_logits: &[f32],
    view: &PlayerView,
    pending: &Pending,
) -> Vec<f32> {
    let n = raw_logits.len().min(cands.pair_objects.len());
    let mut out = raw_logits[..n].to_vec();
    let Pending::ChooseBlockers {
        bounds, obeying, ..
    } = pending
    else {
        return out;
    };
    if n == 0 {
        return out;
    }
    let actor = view.decision_player.unwrap_or(view.seat);
    let my_life = view.seat(actor).map_or(20, |s| s.life);

    let atk_min_blockers = |atk_id: ObjectId| -> usize {
        let from_bounds = bounds
            .iter()
            .filter(|b| b.attacker == atk_id)
            .map(|b| usize::try_from(b.min_blockers).unwrap_or(usize::MAX))
            .max()
            .unwrap_or(1);
        let has_menace = view
            .object(atk_id)
            .is_some_and(|o| has_kw(o, KeywordSet::MENACE));
        from_bounds.max(if has_menace { 2 } else { 1 })
    };
    let atk_max_blockers = |atk_id: ObjectId| -> usize {
        bounds
            .iter()
            .filter(|b| b.attacker == atk_id)
            .map(|b| usize::try_from(b.max_blockers).unwrap_or(usize::MAX))
            .min()
            .unwrap_or(usize::MAX)
    };

    // 1. Inspect what raw greedy selection (`sample_pairs_constrained` at tau <= 0.35) would do
    let mut raw_blk_best: BTreeMap<ObjectId, (usize, f32, f32)> = BTreeMap::new();
    let mut attackers: BTreeSet<ObjectId> = BTreeSet::new();
    let mut blockers: BTreeSet<ObjectId> = BTreeSet::new();
    for (i, &l) in raw_logits[..n].iter().enumerate() {
        let [b_idx, a_idx] = cands.pairs[i];
        if b_idx < 0 || a_idx < 0 {
            continue;
        }
        let (blk, atk) = cands.pair_objects[i];
        blockers.insert(blk);
        attackers.insert(atk);
        let p = 1.0 / (1.0 + (-l).exp());
        let entry = raw_blk_best
            .entry(blk)
            .or_insert((i, f32::NEG_INFINITY, 0.0));
        if l > entry.1 {
            entry.0 = i;
            entry.1 = l;
        }
        entry.2 += p;
    }
    if attackers.is_empty() || blockers.is_empty() {
        return out;
    }

    let mut raw_atk_blks: BTreeMap<ObjectId, Vec<ObjectId>> = BTreeMap::new();
    for (&b_key, &(best_i, best_l, p_sum)) in &raw_blk_best {
        if best_l > 0.0 || (p_sum > 0.50 && best_l > -0.65) {
            let (blk, atk) = cands.pair_objects[best_i];
            debug_assert_eq!(blk, b_key);
            raw_atk_blks.entry(atk).or_default().push(blk);
        }
    }

    let atk_stats = |atk_id: ObjectId| -> (i32, i32, bool, bool, bool, bool, bool) {
        let Some(o) = view.object(atk_id) else {
            return (0, 1, false, false, false, false, false);
        };
        let ap = i32::from(o.power.unwrap_or(0)).max(0);
        let at = (i32::from(o.toughness.unwrap_or(1)) - i32::from(o.damage)).max(1);
        let dt = has_kw(o, KeywordSet::DEATHTOUCH) && ap > 0;
        let fs = has_kw(o, KeywordSet::FIRST_STRIKE) || has_kw(o, KeywordSet::DOUBLE_STRIKE);
        let tr = has_kw(o, KeywordSet::TRAMPLE);
        let mn = atk_min_blockers(atk_id) >= 2;
        let ind = has_kw(o, KeywordSet::INDESTRUCTIBLE);
        (ap, at, dt, fs, tr, mn, ind)
    };
    let blk_stats = |blk_id: ObjectId| -> (i32, i32, bool, bool, bool) {
        let Some(o) = view.object(blk_id) else {
            return (0, 1, false, false, false);
        };
        let bp = i32::from(o.power.unwrap_or(0)).max(0);
        let bt = (i32::from(o.toughness.unwrap_or(1)) - i32::from(o.damage)).max(1);
        let dt = has_kw(o, KeywordSet::DEATHTOUCH) && bp > 0;
        let fs = has_kw(o, KeywordSet::FIRST_STRIKE) || has_kw(o, KeywordSet::DOUBLE_STRIKE);
        let ind = has_kw(o, KeywordSet::INDESTRUCTIBLE);
        (bp, bt, dt, fs, ind)
    };

    let attacks_me = |atk_id: ObjectId| -> bool {
        view.combat
            .attackers
            .iter()
            .find(|a| a.creature == atk_id)
            .is_none_or(|a| a.defending == Defender::Player(actor))
    };

    // Check if raw greedy selection has a flaw:
    // (a) redundant or non-killing multi-block on the same attacker,
    // (b) bounds/menace or `obeying` requirement violation,
    // (c) unblocked damage is lethal while unused blockers or multi-blockers remain.
    let mut has_redundant_multiblock = false;
    let mut has_non_menace_multiblock = false;
    let mut has_illegal_menace = obeying.iter().any(|&(req_b, req_a)| {
        !raw_atk_blks
            .get(&req_a)
            .is_some_and(|blks| blks.contains(&req_b))
    });
    let unblockable_atk_dmg: i32 = view
        .combat
        .attackers
        .iter()
        .filter(|a| a.defending == Defender::Player(actor) && !attackers.contains(&a.creature))
        .map(|a| atk_stats(a.creature).0)
        .sum();
    let mut raw_unblocked_dmg = unblockable_atk_dmg;
    let unblocked_atks_exist = attackers
        .iter()
        .any(|k| raw_atk_blks.get(k).is_none_or(Vec::is_empty));

    for &atk_id in &attackers {
        let (ap, at, _, _, tr, mn, ind) = atk_stats(atk_id);
        let min_b = atk_min_blockers(atk_id);
        let max_b = atk_max_blockers(atk_id);
        let hits_me = attacks_me(atk_id);
        let assigned = raw_atk_blks.get(&atk_id).map_or(&[][..], Vec::as_slice);
        if assigned.is_empty() {
            if hits_me {
                raw_unblocked_dmg += ap;
            }
        } else {
            if assigned.len() < min_b || assigned.len() > max_b {
                has_illegal_menace = true;
            }
            let mut total_bp = 0_i32;
            let mut min_bp = i32::MAX;
            let mut any_dt = false;
            let mut total_bt = 0_i32;
            for &b_id in assigned {
                let (bp, bt, dt, _, _) = blk_stats(b_id);
                total_bp += bp;
                min_bp = min_bp.min(bp);
                if dt {
                    any_dt = true;
                }
                total_bt += bt;
            }
            if hits_me && tr && ap > total_bt {
                raw_unblocked_dmg += ap - total_bt;
            }
            if assigned.len() >= 2 {
                let kills = !ind && (total_bp >= at || any_dt);
                let kills_without_smallest =
                    !ind && (total_bp - min_bp >= at || (any_dt && assigned.len() >= 2));
                if !kills || kills_without_smallest {
                    has_redundant_multiblock = true;
                }
                if !mn {
                    has_non_menace_multiblock = true;
                }
            }
        }
    }
    if has_non_menace_multiblock && raw_unblocked_dmg >= my_life && unblocked_atks_exist {
        has_redundant_multiblock = true;
    }

    let raw_used_blks: usize = raw_atk_blks.values().map(Vec::len).sum();
    let raw_used_blk_set: BTreeSet<ObjectId> = raw_atk_blks
        .values()
        .flat_map(|v| v.iter().copied())
        .collect();
    let has_missed_kill = (0..n).any(|i| {
        let [b_idx, a_idx] = cands.pairs[i];
        if b_idx < 0 || a_idx < 0 {
            return false;
        }
        let (blk_id, atk_id) = cands.pair_objects[i];
        if raw_used_blk_set.contains(&blk_id)
            || raw_atk_blks.contains_key(&atk_id)
            || !attacks_me(atk_id)
            || atk_max_blockers(atk_id) == 0
        {
            return false;
        }
        let (ap, at, a_dt, a_fs, _, a_mn, a_ind) = atk_stats(atk_id);
        if a_mn || a_ind || ap <= 0 {
            return false;
        }
        let (bp, bt, b_dt, b_fs, b_ind) = blk_stats(blk_id);
        let kills = bp >= at || b_dt;
        if !kills {
            return false;
        }
        let survives = b_ind || (b_fs && !a_fs) || (!a_dt && bt > ap);
        let favorable_trade_at_low_life =
            my_life - raw_unblocked_dmg <= 4 && ap >= 2 && ap >= bp && !(a_fs && !b_fs && ap >= bt);
        survives || favorable_trade_at_low_life
    });
    let has_missed_gang_kill = attackers.iter().any(|&atk_id| {
        if !attacks_me(atk_id) || atk_max_blockers(atk_id) < 2 {
            return false;
        }
        let (ap, at, a_dt, a_fs, _, _, a_ind) = atk_stats(atk_id);
        if a_ind || a_dt || a_fs || ap < 2 || at <= 0 {
            return false;
        }
        let assigned = raw_atk_blks.get(&atk_id).map_or(&[][..], Vec::as_slice);
        let raw_kills = assigned.iter().map(|&b| blk_stats(b).0).sum::<i32>() >= at
            || assigned.iter().any(|&b| blk_stats(b).2);
        if raw_kills {
            return false;
        }
        let cand_blks: Vec<ObjectId> = (0..n)
            .filter_map(|i| {
                let [b_idx, a_idx] = cands.pairs[i];
                if b_idx < 0 || a_idx < 0 {
                    return None;
                }
                let (b_id, a_id) = cands.pair_objects[i];
                (a_id == atk_id
                    && (!raw_used_blk_set.contains(&b_id) || assigned.contains(&b_id))
                    && blk_stats(b_id).0 > 0)
                    .then_some(b_id)
            })
            .collect();
        (0..cand_blks.len()).any(|i| {
            let (bp1, bt1, ..) = blk_stats(cand_blks[i]);
            ((i + 1)..cand_blks.len()).any(|j| {
                let (bp2, bt2, ..) = blk_stats(cand_blks[j]);
                bp1 + bp2 >= at && bt1 + bt2 > ap && ap >= bp1.min(bp2)
            })
        })
    });
    let has_wasted_zero_power_block = raw_atk_blks
        .iter()
        .any(|(&atk_id, blks)| !blks.is_empty() && atk_stats(atk_id).0 <= 0);
    let has_smaller_block_than_unblocked = raw_atk_blks.iter().any(|(&atk_id, blks)| {
        if blks.is_empty() {
            return false;
        }
        let ap = atk_stats(atk_id).0;
        attackers.iter().any(|&other_atk| {
            raw_atk_blks.get(&other_atk).is_none_or(Vec::is_empty)
                && attacks_me(other_atk)
                && atk_stats(other_atk).0 > ap
                && atk_min_blockers(other_atk) <= blockers.len()
                && blks.iter().any(|&b_id| {
                    (0..n).any(|i| {
                        cands.pairs[i][0] >= 0
                            && cands.pairs[i][1] >= 0
                            && cands.pair_objects[i] == (b_id, other_atk)
                    })
                })
        })
    });
    let need_survival_blocks = raw_unblocked_dmg >= my_life
        || (raw_unblocked_dmg > 0 && my_life - raw_unblocked_dmg <= 3)
        || (raw_unblocked_dmg >= 6 && my_life - raw_unblocked_dmg <= 7)
        || (raw_unblocked_dmg >= 2 && my_life <= 10 && blockers.len() >= 4 && raw_used_blks == 0);
    let lethal_unblocked = need_survival_blocks
        && (raw_used_blks < blockers.len()
            || has_wasted_zero_power_block
            || has_smaller_block_than_unblocked
            || has_non_menace_multiblock)
        && unblocked_atks_exist;

    if (!attackers.iter().any(|&a| attacks_me(a))
        || (!has_redundant_multiblock
            && !has_illegal_menace
            && !lethal_unblocked
            && !has_missed_kill
            && !has_missed_gang_kill
            && !has_wasted_zero_power_block))
        && !has_illegal_menace
    {
        for (i, out_l) in out.iter_mut().enumerate().take(n) {
            let [b_idx, a_idx] = cands.pairs[i];
            if b_idx < 0 || a_idx < 0 {
                continue;
            }
            let (blk_id, atk_id) = cands.pair_objects[i];
            let assigned = raw_atk_blks.get(&atk_id).map_or(&[][..], Vec::as_slice);
            if assigned.contains(&blk_id)
                && assigned.len() >= atk_min_blockers(atk_id)
                && assigned.len() <= atk_max_blockers(atk_id)
            {
                let (ap, at, a_dt, a_fs, _, _, a_ind) = atk_stats(atk_id);
                let (bp, bt, b_dt, b_fs, b_ind) = blk_stats(blk_id);
                let kills = !a_ind && (bp >= at || b_dt);
                let survives = b_ind || (kills && b_fs && !a_fs) || (!a_dt && bt > ap);
                let atk_has_active_engine = view
                    .object(atk_id)
                    .and_then(super::tactics::abilities_of_battlefield_object)
                    .is_some_and(|abs| {
                        abs.iter().any(|ab| {
                            matches!(
                                ab,
                                AbilityDef::Activated { .. }
                                    | AbilityDef::ActivatedConditional { .. }
                                    | AbilityDef::Triggered {
                                        trigger: Trigger::StepBegin { .. } | Trigger::SpellCast(_),
                                        ..
                                    }
                            )
                        })
                    });
                let trades_favorably = kills
                    && !(a_fs && !b_fs && ap >= bt)
                    && (ap > bp
                        || (atk_has_active_engine && bt == 1 && ap >= 1)
                        || (ap == bp
                            && ap >= 2
                            && (my_life <= 12 || my_life - raw_unblocked_dmg <= 8)));
                let pointless_solo_chump =
                    assigned.len() == 1 && !kills && !survives && !need_survival_blocks;
                if !pointless_solo_chump
                    && (raw_logits[i] > 0.0 || need_survival_blocks || survives || trades_favorably)
                {
                    *out_l = (*out_l).max(0.5);
                } else {
                    *out_l = -20.0;
                }
            } else {
                *out_l = -20.0;
            }
        }
        return out;
    }

    // Build tactical blocker assignment:
    let mut sorted_atks: Vec<ObjectId> = attackers.iter().copied().collect();
    sorted_atks.sort_by(|&a, &b| {
        let (ap_a, at_a, ..) = atk_stats(a);
        let (ap_b, at_b, ..) = atk_stats(b);
        ap_b.cmp(&ap_a)
            .then_with(|| at_b.cmp(&at_a))
            .then_with(|| a.cmp(&b))
    });

    let mut used_blks: BTreeSet<ObjectId> = BTreeSet::new();
    let mut chosen_pairs: BTreeSet<usize> = BTreeSet::new();
    let mut blocked_atks: BTreeSet<ObjectId> = BTreeSet::new();
    for &(req_b, req_a) in obeying {
        if let Some(p_idx) = (0..n).find(|&i| {
            cands.pairs[i][0] >= 0
                && cands.pairs[i][1] >= 0
                && cands.pair_objects[i] == (req_b, req_a)
        }) {
            used_blks.insert(req_b);
            blocked_atks.insert(req_a);
            chosen_pairs.insert(p_idx);
        }
    }

    let total_atk_dmg: i32 = unblockable_atk_dmg
        + sorted_atks
            .iter()
            .filter(|&&a| attacks_me(a))
            .map(|&a| atk_stats(a).0)
            .sum::<i32>();
    let can_pair = |b_id: ObjectId, a_id: ObjectId| -> bool {
        (0..n).any(|i| {
            cands.pairs[i][0] >= 0
                && cands.pairs[i][1] >= 0
                && cands.pair_objects[i] == (b_id, a_id)
        })
    };
    let leaves_lethal_unblocked = |atk_id: ObjectId,
                                   k_blks: usize,
                                   cand_blks: &[ObjectId],
                                   used_blks: &BTreeSet<ObjectId>,
                                   blocked_atks: &BTreeSet<ObjectId>|
     -> bool {
        let ap = atk_stats(atk_id).0;
        let rem_blks_after = blockers.len().saturating_sub(used_blks.len() + k_blks);
        let mut other_powers: Vec<i32> = sorted_atks
            .iter()
            .copied()
            .filter(|&other| other != atk_id && !blocked_atks.contains(&other) && attacks_me(other))
            .map(|other| atk_stats(other).0)
            .collect();
        other_powers.sort_unstable_by(|a, b| b.cmp(a));
        let unavoidable_dmg: i32 =
            unblockable_atk_dmg + other_powers.iter().skip(rem_blks_after).sum::<i32>();
        if unavoidable_dmg < my_life
            && !(total_atk_dmg >= my_life && unavoidable_dmg + 2 >= my_life)
        {
            return false;
        }
        let larger_blockable_count = sorted_atks
            .iter()
            .copied()
            .filter(|&other| {
                other != atk_id
                    && !blocked_atks.contains(&other)
                    && attacks_me(other)
                    && atk_stats(other).0 > ap
                    && atk_min_blockers(other) <= blockers.len().saturating_sub(used_blks.len())
                    && cand_blks.iter().any(|&b_id| can_pair(b_id, other))
            })
            .count();
        if k_blks == 1 {
            larger_blockable_count > rem_blks_after
        } else {
            larger_blockable_count > rem_blks_after || other_powers.len() > rem_blks_after
        }
    };

    // Pass 1: Free kills (`blk` kills `atk` solo and survives)
    for &atk_id in &sorted_atks {
        if blocked_atks.contains(&atk_id) || atk_max_blockers(atk_id) == 0 {
            continue;
        }
        let (ap, at, a_dt, a_fs, _, a_mn, a_ind) = atk_stats(atk_id);
        if a_mn || a_ind || ap <= 0 {
            continue;
        }
        let best_pair = (0..n)
            .filter(|&i| {
                let [b_idx, a_idx] = cands.pairs[i];
                if b_idx < 0 || a_idx < 0 {
                    return false;
                }
                let (blk_id, a_id) = cands.pair_objects[i];
                if a_id != atk_id
                    || used_blks.contains(&blk_id)
                    || leaves_lethal_unblocked(atk_id, 1, &[blk_id], &used_blks, &blocked_atks)
                {
                    return false;
                }
                let (bp, bt, b_dt, b_fs, b_ind) = blk_stats(blk_id);
                let kills = bp >= at || b_dt;
                let survives = b_ind || (kills && b_fs && !a_fs) || (!a_dt && bt > ap);
                kills && survives
            })
            .min_by(|&i, &j| {
                let (b_i, _) = cands.pair_objects[i];
                let (b_j, _) = cands.pair_objects[j];
                let (bp_i, bt_i, ..) = blk_stats(b_i);
                let (bp_j, bt_j, ..) = blk_stats(b_j);
                (bp_i + bt_i)
                    .cmp(&(bp_j + bt_j))
                    .then_with(|| raw_logits[j].total_cmp(&raw_logits[i]))
            });
        if let Some(p_idx) = best_pair {
            let (blk_id, _) = cands.pair_objects[p_idx];
            used_blks.insert(blk_id);
            blocked_atks.insert(atk_id);
            chosen_pairs.insert(p_idx);
        }
    }

    // Pass 2: Free bounce / wall blocks (`blk` survives solo without dying and prevents damage)
    for &atk_id in &sorted_atks {
        if blocked_atks.contains(&atk_id) || atk_max_blockers(atk_id) == 0 {
            continue;
        }
        let (ap, _at, a_dt, _a_fs, _, a_mn, _) = atk_stats(atk_id);
        if a_mn || ap <= 0 {
            continue;
        }
        let best_pair = (0..n)
            .filter(|&i| {
                let [b_idx, a_idx] = cands.pairs[i];
                if b_idx < 0 || a_idx < 0 {
                    return false;
                }
                let (blk_id, a_id) = cands.pair_objects[i];
                if a_id != atk_id
                    || used_blks.contains(&blk_id)
                    || leaves_lethal_unblocked(atk_id, 1, &[blk_id], &used_blks, &blocked_atks)
                {
                    return false;
                }
                let (_bp, bt, _b_dt, _b_fs, b_ind) = blk_stats(blk_id);
                b_ind || (!a_dt && bt > ap)
            })
            .max_by(|&i, &j| raw_logits[i].total_cmp(&raw_logits[j]));
        if let Some(p_idx) = best_pair {
            let (blk_id, _) = cands.pair_objects[p_idx];
            used_blks.insert(blk_id);
            blocked_atks.insert(atk_id);
            chosen_pairs.insert(p_idx);
        }
    }

    // Pass 3: Solo 1-for-1 trades (`blk` kills `atk` solo)
    for &atk_id in &sorted_atks {
        if blocked_atks.contains(&atk_id) || atk_max_blockers(atk_id) == 0 {
            continue;
        }
        let (ap, at, _a_dt, a_fs, _, a_mn, a_ind) = atk_stats(atk_id);
        if a_mn || a_ind || ap <= 0 {
            continue;
        }
        let best_pair = (0..n)
            .filter(|&i| {
                let [b_idx, a_idx] = cands.pairs[i];
                if b_idx < 0 || a_idx < 0 {
                    return false;
                }
                let (blk_id, a_id) = cands.pair_objects[i];
                if a_id != atk_id
                    || used_blks.contains(&blk_id)
                    || leaves_lethal_unblocked(atk_id, 1, &[blk_id], &used_blks, &blocked_atks)
                {
                    return false;
                }
                let (bp, bt, b_dt, b_fs, _b_ind) = blk_stats(blk_id);
                if a_fs && !b_fs && ap >= bt {
                    return false;
                }
                let kills = bp >= at || b_dt;
                let favorable_or_wanted =
                    ap >= bp || raw_logits[i] > 0.0 || raw_unblocked_dmg >= my_life;
                kills && favorable_or_wanted
            })
            .min_by(|&i, &j| {
                let (b_i, _) = cands.pair_objects[i];
                let (b_j, _) = cands.pair_objects[j];
                let (bp_i, bt_i, ..) = blk_stats(b_i);
                let (bp_j, bt_j, ..) = blk_stats(b_j);
                (bp_i + bt_i)
                    .cmp(&(bp_j + bt_j))
                    .then_with(|| raw_logits[j].total_cmp(&raw_logits[i]))
            });
        if let Some(p_idx) = best_pair {
            let (blk_id, _) = cands.pair_objects[p_idx];
            used_blks.insert(blk_id);
            blocked_atks.insert(atk_id);
            chosen_pairs.insert(p_idx);
        }
    }

    // Pass 3b: Minimal gang-block kills on attackers where the raw network wanted to multi-block
    // or when we have 6+ blockers at low life facing a >=3 power attacker.
    for &atk_id in &sorted_atks {
        if blocked_atks.contains(&atk_id) {
            continue;
        }
        let min_b = atk_min_blockers(atk_id).max(2);
        let max_b = atk_max_blockers(atk_id).min(3);
        if min_b > max_b {
            continue;
        }
        let (ap, at, a_dt, a_fs, _, _a_mn, a_ind) = atk_stats(atk_id);
        if a_ind || a_dt || a_fs || at <= 0 {
            continue;
        }
        let mut avail: Vec<usize> = (0..n)
            .filter(|&i| {
                let [b_idx, a_idx] = cands.pairs[i];
                if b_idx < 0 || a_idx < 0 {
                    return false;
                }
                let (blk_id, a_id) = cands.pair_objects[i];
                a_id == atk_id && !used_blks.contains(&blk_id) && blk_stats(blk_id).0 > 0
            })
            .collect();
        avail.sort_by(|&i, &j| {
            let (b_i, _) = cands.pair_objects[i];
            let (b_j, _) = cands.pair_objects[j];
            let (bp_i, bt_i, ..) = blk_stats(b_i);
            let (bp_j, bt_j, ..) = blk_stats(b_j);
            (bp_i + bt_i)
                .cmp(&(bp_j + bt_j))
                .then_with(|| raw_logits[j].total_cmp(&raw_logits[i]))
        });
        let mut gang = Vec::new();
        let mut sum_bp = 0_i32;
        if min_b <= 2 && avail.len() >= 2 {
            let mut best_two: Option<(bool, i32, usize, usize)> = None;
            for ia in 0..avail.len() {
                let pa = avail[ia];
                let (ba, _) = cands.pair_objects[pa];
                let (bpa, bta, ..) = blk_stats(ba);
                for &pb in &avail[(ia + 1)..] {
                    let (bb, _) = cands.pair_objects[pb];
                    let (bpb, btb, ..) = blk_stats(bb);
                    if bpa + bpb >= at {
                        let one_surv = bta + btb > ap && ap >= bpa.min(bpb);
                        let cost_stat = bpa + bta + bpb + btb;
                        let cand_key = (one_surv, -cost_stat, pa, pb);
                        if best_two.is_none_or(|cur| (cand_key.0, cand_key.1) > (cur.0, cur.1)) {
                            best_two = Some(cand_key);
                        }
                    }
                }
            }
            if let Some((_, _, pa, pb)) = best_two {
                let (ba, _) = cands.pair_objects[pa];
                let (bb, _) = cands.pair_objects[pb];
                sum_bp = blk_stats(ba).0 + blk_stats(bb).0;
                gang.push(pa);
                gang.push(pb);
            }
        }
        if gang.is_empty() {
            for &p_idx in &avail {
                let (blk_id, _) = cands.pair_objects[p_idx];
                let (bp, ..) = blk_stats(blk_id);
                gang.push(p_idx);
                sum_bp += bp;
                if sum_bp >= at && gang.len() >= min_b {
                    break;
                }
            }
        }
        let mut idx = 0;
        while idx < gang.len() && gang.len() > min_b {
            let (blk_id, _) = cands.pair_objects[gang[idx]];
            let (bp, ..) = blk_stats(blk_id);
            if sum_bp - bp >= at {
                sum_bp -= bp;
                gang.remove(idx);
            } else {
                idx += 1;
            }
        }
        let sum_bt: i32 = gang
            .iter()
            .map(|&p_idx| blk_stats(cands.pair_objects[p_idx].0).1)
            .sum();
        let min_gang_bp: i32 = gang
            .iter()
            .map(|&p_idx| blk_stats(cands.pair_objects[p_idx].0).0)
            .min()
            .unwrap_or(0);
        let one_survives_two_block = gang.len() == 2 && sum_bt > ap && ap >= min_gang_bp;
        let raw_assigned = raw_atk_blks.get(&atk_id).map_or(0, Vec::len);
        let allow_gang = raw_assigned >= 2
            || (attacks_me(atk_id) && ap >= 3 && my_life <= 10 && blockers.len() >= 6)
            || (attacks_me(atk_id) && ap >= 2 && one_survives_two_block);
        let gang_blks: Vec<ObjectId> = gang.iter().map(|&idx| cands.pair_objects[idx].0).collect();
        if allow_gang
            && sum_bp >= at
            && gang.len() >= min_b
            && gang.len() <= max_b
            && ap >= 2
            && !leaves_lethal_unblocked(atk_id, gang.len(), &gang_blks, &used_blks, &blocked_atks)
        {
            for p_idx in gang {
                let (blk_id, _) = cands.pair_objects[p_idx];
                used_blks.insert(blk_id);
                chosen_pairs.insert(p_idx);
            }
            blocked_atks.insert(atk_id);
        }
    }

    // Pass 4: Chump block largest remaining unblocked attackers if unblocked damage is lethal/near-lethal
    // or if the model strongly wanted to block with a remaining blocker (`best_l > 0.0`)
    let mut rem_unblocked_dmg: i32 = unblockable_atk_dmg
        + sorted_atks
            .iter()
            .filter(|&&a| !blocked_atks.contains(&a) && attacks_me(a))
            .map(|&a| atk_stats(a).0)
            .sum::<i32>();

    for &atk_id in &sorted_atks {
        if blocked_atks.contains(&atk_id) || !attacks_me(atk_id) || atk_max_blockers(atk_id) == 0 {
            continue;
        }
        let min_b = atk_min_blockers(atk_id);
        let (ap, _at, _, _, _, a_mn, _) = atk_stats(atk_id);
        if ap <= 0 {
            continue;
        }
        let need_lethal_chump = rem_unblocked_dmg >= my_life
            || my_life - rem_unblocked_dmg <= 3
            || (rem_unblocked_dmg >= 6 && my_life - rem_unblocked_dmg <= 7 && ap >= 3);
        if a_mn || min_b >= 2 {
            if !need_lethal_chump || min_b > atk_max_blockers(atk_id) {
                continue;
            }
            let mut avail: Vec<usize> = (0..n)
                .filter(|&i| {
                    let [b_idx, a_idx] = cands.pairs[i];
                    if b_idx < 0 || a_idx < 0 {
                        return false;
                    }
                    let (blk_id, a_id) = cands.pair_objects[i];
                    a_id == atk_id && !used_blks.contains(&blk_id)
                })
                .collect();
            avail.sort_by(|&i, &j| {
                let (b_i, _) = cands.pair_objects[i];
                let (b_j, _) = cands.pair_objects[j];
                let (bp_i, bt_i, ..) = blk_stats(b_i);
                let (bp_j, bt_j, ..) = blk_stats(b_j);
                (bp_i + bt_i)
                    .cmp(&(bp_j + bt_j))
                    .then_with(|| raw_logits[j].total_cmp(&raw_logits[i]))
            });
            if avail.len() >= min_b {
                for &p_idx in &avail[..min_b] {
                    let (blk_id, _) = cands.pair_objects[p_idx];
                    used_blks.insert(blk_id);
                    chosen_pairs.insert(p_idx);
                }
                blocked_atks.insert(atk_id);
                rem_unblocked_dmg -= ap;
            }
            continue;
        }
        let best_pair = (0..n)
            .filter(|&i| {
                let [b_idx, a_idx] = cands.pairs[i];
                if b_idx < 0 || a_idx < 0 {
                    return false;
                }
                let (blk_id, a_id) = cands.pair_objects[i];
                if a_id != atk_id || used_blks.contains(&blk_id) {
                    return false;
                }
                let (bp, bt, ..) = blk_stats(blk_id);
                let value_death_block = ap >= 3
                    && bp <= 1
                    && bt <= 1
                    && view
                        .object(blk_id)
                        .is_some_and(|bo| friendly_creature_death_payoff(view, actor, bo) >= 2);
                need_lethal_chump
                    || value_death_block
                    || (raw_logits[i] > 0.0 && ap >= 3 && my_life <= 10)
            })
            .min_by(|&i, &j| {
                let (b_i, _) = cands.pair_objects[i];
                let (b_j, _) = cands.pair_objects[j];
                let (bp_i, bt_i, ..) = blk_stats(b_i);
                let (bp_j, bt_j, ..) = blk_stats(b_j);
                (bp_i + bt_i)
                    .cmp(&(bp_j + bt_j))
                    .then_with(|| raw_logits[j].total_cmp(&raw_logits[i]))
            });
        if let Some(p_idx) = best_pair {
            let (blk_id, _) = cands.pair_objects[p_idx];
            used_blks.insert(blk_id);
            blocked_atks.insert(atk_id);
            chosen_pairs.insert(p_idx);
            rem_unblocked_dmg -= ap;
        }
    }

    // Final bounds validation on `chosen_pairs` (e.g. if `obeying` placed 1 blocker on a menace attacker)
    for &atk_id in &sorted_atks {
        let min_b = atk_min_blockers(atk_id);
        let max_b = atk_max_blockers(atk_id);
        let mut cur: Vec<usize> = chosen_pairs
            .iter()
            .copied()
            .filter(|&i| cands.pair_objects[i].1 == atk_id)
            .collect();
        if cur.is_empty() {
            continue;
        }
        if cur.len() > max_b {
            for &drop_i in &cur[max_b..] {
                chosen_pairs.remove(&drop_i);
            }
        } else if cur.len() < min_b {
            let need = min_b - cur.len();
            let mut spare: Vec<usize> = (0..n)
                .filter(|&i| {
                    cands.pairs[i][0] >= 0
                        && cands.pairs[i][1] >= 0
                        && cands.pair_objects[i].1 == atk_id
                        && !used_blks.contains(&cands.pair_objects[i].0)
                })
                .collect();
            spare.sort_by(|&i, &j| {
                let (bp_i, bt_i, ..) = blk_stats(cands.pair_objects[i].0);
                let (bp_j, bt_j, ..) = blk_stats(cands.pair_objects[j].0);
                (bp_i + bt_i).cmp(&(bp_j + bt_j))
            });
            if spare.len() >= need {
                for &add_i in &spare[..need] {
                    used_blks.insert(cands.pair_objects[add_i].0);
                    chosen_pairs.insert(add_i);
                }
            } else {
                for drop_i in cur.drain(..) {
                    chosen_pairs.remove(&drop_i);
                }
            }
        }
    }

    for i in 0..n {
        if chosen_pairs.contains(&i) {
            out[i] = raw_logits[i].max(0.5) + 10.0;
        } else {
            out[i] = -20.0;
        }
    }
    out
}

fn can_legally_block(att: &PublicObject, blk: &PublicObject) -> bool {
    if has_kw(att, KeywordSet::FLYING)
        && !has_kw(blk, KeywordSet::FLYING)
        && !has_kw(blk, KeywordSet::REACH)
    {
        return false;
    }
    if has_kw(att, KeywordSet::FEAR)
        && !blk.types.contains(TypeSet::ARTIFACT)
        && !blk.colors.contains(Color::Black)
    {
        return false;
    }
    if has_kw(att, KeywordSet::INTIMIDATE)
        && !blk.types.contains(TypeSet::ARTIFACT)
        && !att.colors.intersects(blk.colors)
    {
        return false;
    }
    true
}

fn has_non_mana_tap_ability(view: &PlayerView, id: ObjectId) -> bool {
    let Some(ci) = lookup_card_identity(view, id) else {
        return false;
    };
    let Some(def) = baylee_cards::by_index(ci.index) else {
        return false;
    };
    def.abilities_for_face(usize::from(ci.face))
        .iter()
        .any(|ab| {
            !ab.is_mana_ability()
                && match ab {
                    AbilityDef::Activated { cost, .. }
                    | AbilityDef::ActivatedConditional { cost, .. }
                    | AbilityDef::ModalActivated { cost, .. } => {
                        cost.parts.contains(&CostPart::TapSelf)
                    }
                    _ => false,
                }
        })
}

#[allow(
    clippy::too_many_lines,
    reason = "Scores color demand across owed payment, hand spells, and permanent abilities"
)]
fn color_demand_score(view: &PlayerView, actor: PlayerId, mc: ManaColor) -> u32 {
    let idx = mc.index();
    if idx >= 5 {
        return 1;
    }
    let Some(seat) = view.seat(actor) else {
        return 0;
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
    let avail = pool_total
        .saturating_add(1)
        .saturating_add(untapped_mana_production(view, actor));

    // Count untapped single-color sources on battlefield
    let mut single_untapped = [0_u32; 5];
    for obj in view.battlefield_of(actor) {
        if obj.status.contains(ObjectStatus::TAPPED)
            || obj.status.contains(ObjectStatus::PHASED_OUT)
        {
            continue;
        }
        let cols = non_sac_source_colors(view, obj.id);
        let color_count = (0..5).filter(|&c| cols[c]).count();
        if color_count == 1 {
            for c in 0..5 {
                if cols[c] {
                    single_untapped[c] += 1;
                }
            }
        }
    }

    if let Some(ManaPayment::Fixed(ref cost)) = view.owed {
        let pips = count_color_pips(cost, idx);
        let unmet = pips.saturating_sub(have[idx]);
        if unmet > 0 {
            return 10_000 + unmet * 100;
        }
    }

    let mut score = 0_u32;
    for h in &view.hand {
        if is_true_land_card(view, h.id) {
            continue;
        }
        let Some((cost, tax)) = lookup_spell_cost(view, h.id) else {
            continue;
        };
        let pips = count_color_pips(&cost, idx);
        if pips == 0 {
            continue;
        }
        let cmc = cost.with_x(0).cmc().saturating_add(tax);
        let unmet_pool = pips.saturating_sub(have[idx]);
        let unmet_board = unmet_pool.saturating_sub(single_untapped[idx]);
        if cmc <= avail && is_spell_worth_casting_now(view, h.id) {
            if unmet_pool > 0 {
                score += 1000 * unmet_pool + 500 * unmet_board;
            } else {
                score += 50 * pips;
            }
        } else {
            score += 20 * unmet_board + 5 * pips;
        }
    }

    for obj in view.battlefield_of(actor) {
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
        for ab in def.abilities_for_face(usize::from(rf.face)) {
            if ab.is_mana_ability() {
                continue;
            }
            if let AbilityDef::Activated { cost, .. }
            | AbilityDef::ActivatedConditional { cost, .. }
            | AbilityDef::ModalActivated { cost, .. } = ab
            {
                let pips = count_color_pips(&cost.mana, idx);
                score += 5 * pips;
            }
        }
    }

    score
}

pub(super) fn count_color_pips(cost: &ManaCost, color_idx: usize) -> u32 {
    let mut pips = 0_u32;
    for (sym, count) in cost.runs() {
        let n = u32::from(count);
        let matches = match (color_idx, sym) {
            (0, ManaSymbol::White | ManaSymbol::TwoOrColor(Color::White))
            | (1, ManaSymbol::Blue | ManaSymbol::TwoOrColor(Color::Blue))
            | (2, ManaSymbol::Black | ManaSymbol::TwoOrColor(Color::Black))
            | (3, ManaSymbol::Red | ManaSymbol::TwoOrColor(Color::Red))
            | (4, ManaSymbol::Green | ManaSymbol::TwoOrColor(Color::Green)) => true,
            (idx, ManaSymbol::Hybrid(pair) | ManaSymbol::HybridPhyrexian(pair)) if idx < 5 => {
                ColorSet::of_pair(pair).contains(Color::ALL[idx])
            }
            _ => false,
        };
        if matches {
            pips += n;
        }
    }
    pips
}

fn subtype_demand_score(view: &PlayerView, actor: PlayerId, st: SubtypeId) -> u32 {
    let mut score = 0_u32;
    for h in &view.hand {
        if let Some(def) = baylee_cards::by_index(h.card.index)
            && def.faces.iter().any(|f| f.subtypes.contains(&st))
        {
            score += 3;
        }
    }
    for obj in view.battlefield_of(actor) {
        if obj.subtypes.contains(st) {
            score += 2;
        } else if let Some(ci) = obj.card
            && let Some(def) = baylee_cards::by_index(ci.index)
            && def.faces.iter().any(|f| f.subtypes.contains(&st))
        {
            score += 2;
        }
    }
    if let Some(gy) = view.graveyards.get(usize::from(actor.get())) {
        for obj in gy {
            if obj.subtypes.contains(st) {
                score += 1;
            }
        }
    }
    score
}

#[allow(
    clippy::too_many_lines,
    reason = "Ranks tutor candidates by mana curve, color fixing, and hand duplicates"
)]
fn tutor_card_tier(view: &PlayerView, actor: PlayerId, oid: ObjectId) -> i32 {
    let my_mana = view
        .battlefield_of(actor)
        .filter(|o| o.types.contains(TypeSet::LAND) || o.board_mana.is_some())
        .count() as u32;
    let my_creatures = view
        .battlefield_of(actor)
        .filter(|o| o.types.contains(TypeSet::CREATURE))
        .count();
    let lands_in_hand = view
        .hand
        .iter()
        .filter(|h| is_true_land_card(view, h.id))
        .count() as u32;
    let need_land = my_mana < 4 && lands_in_hand == 0;
    let max_castable_cmc = (my_mana + u32::from(lands_in_hand > 0)).max(2);

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
    let mut hand_land_color_counts = [0_u32; 5];
    for h in &view.hand {
        if is_true_land_card(view, h.id) {
            let cols = land_card_colors(view, h.id);
            for c in 0..5 {
                if cols[c] {
                    hand_land_color_counts[c] += 1;
                }
            }
        }
    }
    let mut missing_hand_colors = [false; 5];
    for h in &view.hand {
        if is_true_land_card(view, h.id) {
            continue;
        }
        if let Some((cost, _)) = lookup_spell_cost(view, h.id) {
            for c in 0..5 {
                if count_color_pips(&cost, c) > board_color_counts[c] + hand_land_color_counts[c] {
                    missing_hand_colors[c] = true;
                }
            }
        }
    }

    if is_pure_search_library_card(view, oid) {
        return 0;
    }
    if is_true_land_card(view, oid) {
        let cols = land_card_colors(view, oid);
        let fixes_missing = (0..5).any(|c| cols[c] && missing_hand_colors[c]);
        let produces_new_color =
            (0..5).any(|c| cols[c] && board_color_counts[c] + hand_land_color_counts[c] == 0);
        if fixes_missing {
            if my_mana + lands_in_hand <= 3 {
                return 6;
            }
            return 4;
        }
        if need_land {
            if produces_new_color {
                return 5;
            }
            if cols.iter().any(|&c| c) {
                return 4;
            }
            return 3;
        }
        if produces_new_color {
            return 3;
        }
        if my_mana + lands_in_hand >= 5 {
            return 1;
        }
        return 2;
    }

    if let Some((cost, _)) = lookup_spell_cost(view, oid) {
        for c in 0..5 {
            if count_color_pips(&cost, c) > 0
                && board_color_counts[c] + hand_land_color_counts[c] == 0
            {
                return 1;
            }
        }
    }
    let duplicate_penalty = if let Some(ci) = lookup_card_identity(view, oid)
        && view.hand.iter().any(|h| h.card.index == ci.index)
    {
        2
    } else {
        0
    };

    let cmc = lookup_spell_cost(view, oid).map_or(3, |(c, t)| c.with_x(0).cmc().saturating_add(t));
    let is_creature = is_creature_card(view, oid);
    let base_tier =
        if is_creature && my_creatures == 0 && cmc <= (my_mana + lands_in_hand.min(2)).max(4) {
            5
        } else if cmc <= max_castable_cmc {
            if !need_land {
                if is_draw_spell_card(view, oid) && view.hand.len() >= 5 && my_creatures == 0 {
                    3
                } else {
                    4
                }
            } else if is_draw_spell_card(view, oid) && cmc <= my_mana {
                4
            } else {
                2
            }
        } else if cmc == max_castable_cmc + 1 {
            2
        } else {
            1
        };
    (base_tier - duplicate_penalty).max(1)
}

fn effects_draw_cards_for_actor(effects: &[Effect]) -> bool {
    effects.iter().any(|e| match e {
        Effect::DrawCards { .. }
        | Effect::DrawCardsFor {
            who: PlayerRel::You | PlayerRel::EventPlayer | PlayerRel::EachPlayer,
            ..
        } => true,
        Effect::MayDo { effects: inner }
        | Effect::PlayerMayPayThen { effects: inner, .. }
        | Effect::PlayerMayPayManaThen { effects: inner, .. } => {
            effects_draw_cards_for_actor(inner)
        }
        _ => false,
    })
}

fn should_decline_yes_no_for_library(
    view: &PlayerView,
    actor: PlayerId,
    source: Option<AbilityRef>,
) -> bool {
    let my_lib = view.seat(actor).map_or(99, |s| s.library_count);
    if my_lib > 5 {
        return false;
    }
    let Some(aref) = source else {
        return false;
    };
    if aref.index == AbilityRef::SYNTHETIC {
        return false;
    }
    let Some(def) = baylee_cards::by_index(aref.card) else {
        return false;
    };
    let abilities = def.abilities_for_face(0);
    if let Some(
        AbilityDef::Spell { effects, .. }
        | AbilityDef::Activated { effects, .. }
        | AbilityDef::ActivatedConditional { effects, .. }
        | AbilityDef::Triggered { effects, .. },
    ) = abilities.get(aref.index as usize)
    {
        return effects_draw_cards_for_actor(effects);
    }
    abilities.iter().any(|ab| match ab {
        AbilityDef::Spell { effects, .. }
        | AbilityDef::Activated { effects, .. }
        | AbilityDef::ActivatedConditional { effects, .. }
        | AbilityDef::Triggered { effects, .. } => effects_draw_cards_for_actor(effects),
        _ => false,
    })
}

fn should_pay_tax(
    view: &PlayerView,
    actor: PlayerId,
    source: Option<AbilityRef>,
    req_mana: u32,
) -> bool {
    let pool_total = view
        .seat(actor)
        .map_or(0, |s| u32::try_from(s.mana_pool.total()).unwrap_or(0));
    let avail_mana = pool_total.saturating_add(untapped_mana_production(view, actor));
    if avail_mana < req_mana {
        return false;
    }
    let Some(aref) = source else {
        return true;
    };
    if aref.index == AbilityRef::SYNTHETIC {
        return true;
    }
    let Some(def) = baylee_cards::by_index(aref.card) else {
        return true;
    };
    let abilities = def.abilities_for_face(0);
    let check_effects = |effects: &[Effect]| -> bool {
        if effects.iter().any(|e| {
            matches!(
                e,
                Effect::PlayerMayPayThen { .. }
                    | Effect::PlayerMayPayManaThen { .. }
                    | Effect::PlayerMayPayThenElse { .. }
                    | Effect::PlayerMayPayManaEachThenElse { .. }
            )
        }) {
            return true;
        }
        let refusal_costs_card = effects.iter().any(|e| match e {
            Effect::PlayerMayPayOr { effect, .. } | Effect::PlayerMayPayManaOr { effect, .. } => {
                matches!(
                    effect,
                    Effect::CounterTargetSpell
                        | Effect::CounterTargetSpellToExile
                        | Effect::CounterTargetAbility
                        | Effect::CounterTargetSpellOrAbility
                        | Effect::SacrificeSelf
                        | Effect::DrawCards { .. }
                        | Effect::DrawCardsFor { .. }
                        | Effect::DealDamage {
                            target: TargetSpec::Player(PlayerRel::You),
                            ..
                        }
                )
            }
            _ => false,
        });
        if !refusal_costs_card {
            return false;
        }
        let is_cumulative = effects.iter().any(|e| {
            matches!(
                e,
                Effect::PlayerMayPayOr {
                    mana: Amount::CountersOnSource(counters::AGE),
                    effect: Effect::SacrificeSelf,
                    ..
                }
            )
        });
        if is_cumulative && !view.hand.is_empty() && req_mana >= 3 {
            return false;
        }
        true
    };

    if let Ok(idx) = usize::try_from(aref.index)
        && let Some(ab) = abilities.get(idx)
    {
        return match ab {
            AbilityDef::Spell { effects, .. }
            | AbilityDef::Activated { effects, .. }
            | AbilityDef::ActivatedConditional { effects, .. }
            | AbilityDef::Triggered { effects, .. } => check_effects(effects),
            _ => true,
        };
    }
    for ab in abilities {
        if let AbilityDef::Spell { effects, .. }
        | AbilityDef::Activated { effects, .. }
        | AbilityDef::ActivatedConditional { effects, .. }
        | AbilityDef::Triggered { effects, .. } = ab
            && check_effects(effects)
        {
            return true;
        }
    }
    false
}

fn is_discard_choose_cards(
    view: &PlayerView,
    prompt: ChoicePrompt,
    options: &[ObjectId],
    active_source: Option<(ObjectId, Option<u32>)>,
) -> bool {
    if matches!(prompt, ChoicePrompt::CostDiscard | ChoicePrompt::Discard) {
        return true;
    }
    if !matches!(prompt, ChoicePrompt::Generic)
        || options.is_empty()
        || !options
            .iter()
            .all(|&oid| view.hand.iter().any(|h| h.id == oid))
    {
        return false;
    }
    let has_discard_effect = |effects: &[Effect]| -> bool {
        effects.iter().any(|e| {
            matches!(
                e,
                Effect::DiscardForPlayers { .. }
                    | Effect::DiscardEachOrDraw { .. }
                    | Effect::DiscardUpToThenDraw { .. }
                    | Effect::DiscardUpToForPlayers { .. }
                    | Effect::DiscardAnyThenDraw
                    | Effect::DiscardCount { .. }
                    | Effect::DiscardMatching { .. }
                    | Effect::Connive { .. }
                    | Effect::Recruit { .. }
            )
        })
    };
    let check_oid = |oid: ObjectId, ab_idx: Option<u32>| -> bool {
        let Some(ci) = lookup_card_identity(view, oid) else {
            return false;
        };
        let Some(def) = baylee_cards::by_index(ci.index) else {
            return false;
        };
        let abilities = def.abilities_for_face(usize::from(ci.face));
        if let Some(idx) = ab_idx.and_then(|i| usize::try_from(i).ok())
            && let Some(ab) = abilities.get(idx)
        {
            return match ab {
                AbilityDef::Spell { effects, .. }
                | AbilityDef::Activated { effects, .. }
                | AbilityDef::ActivatedConditional { effects, .. }
                | AbilityDef::Triggered { effects, .. } => has_discard_effect(effects),
                _ => false,
            };
        }
        abilities.iter().any(|ab| match ab {
            AbilityDef::Spell { effects, .. }
            | AbilityDef::Activated { effects, .. }
            | AbilityDef::ActivatedConditional { effects, .. }
            | AbilityDef::Triggered { effects, .. } => has_discard_effect(effects),
            _ => false,
        })
    };
    if let Some((oid, ab_idx)) = active_source
        && check_oid(oid, ab_idx)
    {
        return true;
    }
    if let Some(oid) = view.casting
        && check_oid(oid, None)
    {
        return true;
    }
    view.battlefield.iter().any(|o| check_oid(o.id, None))
}

#[allow(clippy::cast_precision_loss)]
fn discard_card_tier(view: &PlayerView, actor: PlayerId, oid: ObjectId) -> (i32, f32) {
    let my_mana = view
        .battlefield_of(actor)
        .filter(|o| o.types.contains(TypeSet::LAND) || o.board_mana.is_some())
        .count() as u32;
    let lands_in_hand = view
        .hand
        .iter()
        .filter(|h| is_true_land_card(view, h.id))
        .count() as u32;
    let max_hand_spell_cmc = view
        .hand
        .iter()
        .filter(|h| !is_true_land_card(view, h.id))
        .filter_map(|h| lookup_spell_cost(view, h.id).map(|(c, t)| c.with_x(0).cmc() + t))
        .max()
        .unwrap_or(0);

    if is_true_land_card(view, oid) {
        if my_mana >= 6 && (lands_in_hand >= 1 || my_mana >= max_hand_spell_cmc) {
            return (4, 0.5);
        }
        if my_mana >= 5 && lands_in_hand >= 2 {
            return (4, 0.2);
        }
        if my_mana + lands_in_hand >= 6 && lands_in_hand >= 3 {
            return (4, 0.0);
        }
        return (0, 0.0);
    }

    let cmc = lookup_spell_cost(view, oid).map_or(2, |(c, t)| c.with_x(0).cmc().saturating_add(t));
    let cmc_bonus = (cmc.min(12) as f32) * 0.1;
    let is_off_color = lookup_spell_cost(view, oid).is_some_and(|(cost, _)| {
        !super::tactics::spell_colors_reachable(view, actor, oid, &cost)
            && (0..5).any(|c| {
                count_color_pips(&cost, c) > 0
                    && !view.battlefield_of(actor).any(|o| {
                        !o.status.contains(ObjectStatus::PHASED_OUT)
                            && (super::tactics::non_sac_source_colors(view, o.id)[c]
                                || super::tactics::filter_land_min_cost_and_colors(view, o.id)
                                    .is_some_and(|(_, cols)| cols[c]))
                    })
                    && !view.hand.iter().any(|h| {
                        is_true_land_card(view, h.id)
                            && (super::tactics::land_card_colors(view, h.id)[c]
                                || super::tactics::filter_land_min_cost_and_colors(view, h.id)
                                    .is_some_and(|(_, cols)| cols[c]))
                    })
            })
    });
    if is_off_color && cmc >= my_mana + 2 {
        return (5, cmc_bonus);
    }
    if is_off_color && cmc >= 4 {
        return (4, cmc_bonus + 0.5);
    }
    if let Some(ci) = lookup_card_identity(view, oid)
        && let Some(def) = baylee_cards::by_index(ci.index)
        && let Some(face) = def.faces.get(usize::from(ci.face))
        && face.supertypes.contains(SupertypeSet::LEGENDARY)
        && !face.types.contains(TypeSet::PLANESWALKER)
    {
        let on_board = view.battlefield_of(actor).any(|o| {
            !o.status.contains(ObjectStatus::PHASED_OUT)
                && o.card.is_some_and(|c| c.index == ci.index)
        });
        let copies_in_hand = view
            .hand
            .iter()
            .filter(|h| lookup_card_identity(view, h.id).is_some_and(|hc| hc.index == ci.index))
            .count();
        if on_board || copies_in_hand >= 2 {
            return (3, 0.8 + cmc_bonus);
        }
    }
    if cmc >= my_mana + 3 {
        return (4, cmc_bonus);
    }
    if cmc == my_mana + 2 {
        return (3, cmc_bonus);
    }
    let keep_high_impact_bonus = -((cmc.min(8) as f32) * 0.05);
    (1, keep_high_impact_bonus)
}

fn has_etb_self_bounce(obj: &PublicObject) -> bool {
    let Some(abs) = super::tactics::abilities_of_battlefield_object(obj) else {
        return false;
    };
    abs.iter().any(|ab| {
        matches!(
            ab,
            AbilityDef::Triggered {
                trigger: Trigger::ETB,
                effects,
                ..
            } if effects.iter().any(|e| matches!(e, Effect::ReturnChosenToHand { .. }))
        )
    })
}

fn is_self_bounce_choose_cards(
    view: &PlayerView,
    actor: PlayerId,
    prompt: ChoicePrompt,
    options: &[ObjectId],
    active_source: Option<(ObjectId, Option<u32>)>,
) -> bool {
    if matches!(prompt, ChoicePrompt::CostReturn) {
        return true;
    }
    if !matches!(prompt, ChoicePrompt::Generic)
        || options.is_empty()
        || !options
            .iter()
            .all(|&oid| view.battlefield_of(actor).any(|o| o.id == oid))
    {
        return false;
    }
    if let Some((src_id, _)) = active_source
        && view.object(src_id).is_some_and(has_etb_self_bounce)
    {
        return true;
    }
    if let Some(top) = view.top_of_stack()
        && let Some(baylee_view::StackItem::Ability { source, .. }) = top.stack_item
        && view.object(source).is_some_and(has_etb_self_bounce)
    {
        return true;
    }
    view.battlefield_of(actor).any(has_etb_self_bounce)
}

fn self_bounce_card_tier(view: &PlayerView, actor: PlayerId, oid: ObjectId) -> i32 {
    let Some(obj) = view.object(oid) else {
        return 1;
    };
    if has_etb_self_bounce(obj) || obj.token.is_some() {
        return 0;
    }
    let my_lands = view
        .battlefield_of(actor)
        .filter(|o| o.types.contains(TypeSet::LAND))
        .count();
    let has_positive_etb =
        super::tactics::abilities_of_battlefield_object(obj).is_some_and(|abs| {
            abs.iter().any(|ab| {
                matches!(
                    ab,
                    AbilityDef::Triggered {
                        trigger: Trigger::ETB,
                        effects,
                        ..
                    } if !effects.iter().any(|e| matches!(
                        e,
                        Effect::ReturnChosenToHand { .. }
                            | Effect::SacrificeAll { .. }
                            | Effect::LoseLife { target: PlayerRel::You, .. }
                    ))
                )
            })
        });
    let cmc = lookup_spell_cost(view, oid).map_or(2, |(c, _)| c.with_x(0).cmc());
    if has_positive_etb && cmc <= 2 {
        return 5;
    }
    if has_positive_etb && cmc <= 3 {
        return 4;
    }
    if obj.types.contains(TypeSet::LAND) {
        let tapped = obj.status.contains(ObjectStatus::TAPPED);
        if my_lands >= 4 && tapped {
            return 4;
        }
        if my_lands >= 4 {
            return 3;
        }
        if tapped {
            return 2;
        }
        return 1;
    }
    if cmc <= 2 {
        return 3;
    }
    2
}

fn is_positive_death_effect(eff: &Effect, require_creature_token: bool) -> bool {
    match eff {
        Effect::DrawCards { .. } | Effect::GainLife { .. } => true,
        Effect::DealDamage { target, .. } => !matches!(target, TargetSpec::Player(PlayerRel::You)),
        Effect::LoseLife { target, .. } => !matches!(target, PlayerRel::You),
        Effect::CreateToken { token, .. } => {
            !require_creature_token || token.types.contains(TypeSet::CREATURE)
        }
        _ => false,
    }
}

fn self_creature_death_payoff(obj: &PublicObject) -> i32 {
    let mut score = 0_i32;
    if let Some(abs) = super::tactics::abilities_of_battlefield_object(obj) {
        for ab in abs {
            if let AbilityDef::Triggered {
                trigger: Trigger::Dies(Filter::This),
                effects,
                ..
            } = ab
                && effects.iter().any(|e| is_positive_death_effect(e, true))
            {
                score += 2;
            }
        }
    }
    score
}

fn friendly_creature_death_payoff(
    view: &PlayerView,
    controller: PlayerId,
    obj: &PublicObject,
) -> i32 {
    let mut score = self_creature_death_payoff(obj);
    for perm in view.battlefield_of(controller) {
        if perm.id == obj.id || perm.status.contains(ObjectStatus::PHASED_OUT) {
            continue;
        }
        let Some(abs) = super::tactics::abilities_of_battlefield_object(perm) else {
            continue;
        };
        for ab in abs {
            if let AbilityDef::Triggered {
                trigger: Trigger::Dies(filter),
                effects,
                ..
            } = ab
                && !matches!(*filter, Filter::This)
                && effects.iter().any(|e| is_positive_death_effect(e, false))
            {
                score += 1;
            }
        }
    }
    score
}

fn sacrifice_card_tier(
    view: &PlayerView,
    actor: PlayerId,
    oid: ObjectId,
    src_id: Option<ObjectId>,
) -> i32 {
    let Some(obj) = view.object(oid) else {
        return 2;
    };
    if src_id == Some(oid) && obj.types.contains(TypeSet::CREATURE) {
        return 0;
    }
    let pt =
        i32::from(obj.power.unwrap_or(0)).max(0) + i32::from(obj.toughness.unwrap_or(0)).max(0);
    if pt <= 3 && friendly_creature_death_payoff(view, actor, obj) >= 2 {
        return 5;
    }
    if obj.token.is_some() && pt <= 3 {
        return 4;
    }
    if let Some(abs) = super::tactics::abilities_of_battlefield_object(obj) {
        let is_engine = abs.iter().any(|ab| match ab {
            AbilityDef::Activated {
                cost, mana_ability, ..
            }
            | AbilityDef::ActivatedConditional {
                cost, mana_ability, ..
            } => !*mana_ability && !cost.parts.contains(&CostPart::SacrificeSelf),
            AbilityDef::Triggered { trigger, .. } => {
                !matches!(*trigger, Trigger::ETB | Trigger::Dies(Filter::This))
            }
            _ => false,
        });
        if is_engine {
            return 1;
        }
    }
    if pt <= 3 {
        return 3;
    }
    2
}

pub(super) fn is_urgent_enchantment(obj: &PublicObject) -> bool {
    if !obj.types.contains(TypeSet::ENCHANTMENT) || obj.status.contains(ObjectStatus::PHASED_OUT) {
        return false;
    }
    let Some(abs) = super::tactics::abilities_of_battlefield_object(obj) else {
        return false;
    };
    abs.iter().any(|ab| match ab {
        AbilityDef::Triggered {
            trigger: Trigger::SpellCast(_) | Trigger::StepBegin { .. } | Trigger::Dies(_),
            effects,
            ..
        } => !effects
            .iter()
            .all(|e| matches!(e, Effect::AddMana { .. } | Effect::GainLife { .. })),
        AbilityDef::Static(_) => true,
        _ => false,
    })
}

fn opp_has_urgent_enchantment(view: &PlayerView, actor: PlayerId) -> bool {
    view.battlefield
        .iter()
        .any(|o| o.controller != actor && is_urgent_enchantment(o))
}

pub(super) fn mode_destroys_enchantment(mode: &baylee_cards_dsl::SpellMode) -> bool {
    let Some(ref t) = mode.targets else {
        return false;
    };
    let TargetSpec::Object(ref filter) = t.spec else {
        return false;
    };
    let matches_ench = match filter {
        Filter::HasType(ts) => ts.contains(TypeSet::ENCHANTMENT),
        Filter::And(list) | Filter::Or(list) => list
            .iter()
            .any(|f| matches!(f, Filter::HasType(ts) if ts.contains(TypeSet::ENCHANTMENT))),
        _ => false,
    };
    matches_ench
        && mode
            .effects
            .iter()
            .any(|e| matches!(e, Effect::Destroy { .. } | Effect::Exile { .. }))
}

#[allow(
    clippy::too_many_lines,
    reason = "Resolves fixed-damage amount across active_source, stack abilities, graveyard death triggers, and battlefield triggers"
)]
pub(super) fn active_source_fixed_damage(
    view: &PlayerView,
    active_source: Option<(ObjectId, Option<u32>)>,
) -> Option<i32> {
    let src_self_power: Option<i32> = active_source
        .map(|(oid, _)| oid)
        .or_else(|| {
            view.top_of_stack().map(|top| match top.stack_item {
                Some(baylee_view::StackItem::Ability { source, .. }) => source,
                _ => top.id,
            })
        })
        .and_then(|oid| view.object(oid))
        .and_then(|o| o.power)
        .map(|p| i32::from(p).max(0));
    let check_effects = |effects: &[Effect]| -> Option<i32> {
        for eff in effects {
            match eff {
                Effect::DealDamage {
                    amount: Amount::Fixed(dmg),
                    ..
                } => {
                    return i32::try_from(*dmg).ok();
                }
                Effect::Fight {
                    fighter: baylee_cards_dsl::TargetSlot::This,
                    ..
                } => {
                    if let Some(p) = src_self_power.filter(|&p| p > 0) {
                        return Some(p);
                    }
                }
                _ => {}
            }
        }
        None
    };
    let check_abilities = |abilities: &[AbilityDef], ab_idx: Option<u32>| -> Option<i32> {
        if let Some(idx) = ab_idx.and_then(|i| usize::try_from(i).ok())
            && let Some(ab) = abilities.get(idx)
        {
            return match ab {
                AbilityDef::Spell { effects, .. }
                | AbilityDef::Activated { effects, .. }
                | AbilityDef::ActivatedConditional { effects, .. }
                | AbilityDef::Triggered { effects, .. } => check_effects(effects),
                _ => None,
            };
        }
        for ab in abilities {
            if let AbilityDef::Spell { effects, .. }
            | AbilityDef::Activated { effects, .. }
            | AbilityDef::ActivatedConditional { effects, .. }
            | AbilityDef::Triggered { effects, .. } = ab
                && let Some(d) = check_effects(effects)
            {
                return Some(d);
            }
        }
        None
    };

    if let Some((oid, ab_idx)) = active_source {
        if let Some(obj) = view.object(oid)
            && let Some(abs) = super::tactics::abilities_of_battlefield_object(obj)
            && let Some(d) = check_abilities(abs, ab_idx)
        {
            return Some(d);
        }
        if let Some(ci) = lookup_card_identity(view, oid)
            && let Some(def) = baylee_cards::by_index(ci.index)
            && let Some(d) = check_abilities(def.abilities_for_face(usize::from(ci.face)), ab_idx)
        {
            return Some(d);
        }
    }

    if let Some(top) = view.top_of_stack() {
        if let Some(baylee_view::StackItem::Ability {
            source,
            ability,
            rules,
            token,
            ..
        }) = top.stack_item
        {
            let ab_idx = ability.map(|a| a.index);
            if let Some(rf) = rules
                && let Some(def) = baylee_cards::by_index(rf.card)
                && let Some(d) =
                    check_abilities(def.abilities_for_face(usize::from(rf.face)), ab_idx)
            {
                return Some(d);
            }
            if let Some(ta) = token
                && let Some(tdef) = baylee_cards::tokens::by_token_id(ta.token)
                && let Some(d) = check_abilities(tdef.abilities, Some(ta.index))
            {
                return Some(d);
            }
            if let Some(ci) = lookup_card_identity(view, source)
                && let Some(def) = baylee_cards::by_index(ci.index)
                && let Some(d) =
                    check_abilities(def.abilities_for_face(usize::from(ci.face)), ab_idx)
            {
                return Some(d);
            }
        }
        if let Some(abs) = super::tactics::abilities_of_battlefield_object(top)
            && let Some(d) = check_abilities(abs, None)
        {
            return Some(d);
        }
    }

    let actor = view.decision_player.unwrap_or(view.seat);
    if let Some(gy) = view.graveyards.get(usize::from(actor.get())) {
        for obj in gy.iter().rev() {
            let Some(abs) = super::tactics::abilities_of_battlefield_object(obj) else {
                continue;
            };
            for ab in abs {
                if let AbilityDef::Triggered {
                    trigger: Trigger::Dies(_) | Trigger::LeavesBattlefield(_),
                    effects,
                    targets: Some(_),
                    ..
                } = ab
                    && let Some(d) = check_effects(effects)
                {
                    return Some(d);
                }
            }
        }
    }

    let mut bf_objs: Vec<&PublicObject> = view.battlefield_of(actor).collect();
    bf_objs.sort_by_key(|o| std::cmp::Reverse(o.id));
    for obj in bf_objs {
        let Some(abs) = super::tactics::abilities_of_battlefield_object(obj) else {
            continue;
        };
        for ab in abs {
            if let AbilityDef::Triggered {
                effects,
                targets: Some(_),
                ..
            } = ab
                && let Some(d) = check_effects(effects)
            {
                return Some(d);
            }
        }
    }
    None
}

fn is_creature_card(view: &PlayerView, oid: ObjectId) -> bool {
    let Some(ci) = lookup_card_identity(view, oid) else {
        return false;
    };
    let Some(def) = baylee_cards::by_index(ci.index) else {
        return false;
    };
    def.faces
        .get(usize::from(ci.face))
        .is_some_and(|f| f.types.contains(TypeSet::CREATURE))
}

fn is_pure_search_library_card(view: &PlayerView, oid: ObjectId) -> bool {
    let Some(ci) = lookup_card_identity(view, oid) else {
        return false;
    };
    let Some(def) = baylee_cards::by_index(ci.index) else {
        return false;
    };
    let Some(face) = def.faces.get(usize::from(ci.face)) else {
        return false;
    };
    if face.types.contains(TypeSet::CREATURE) {
        return false;
    }
    def.abilities_for_face(usize::from(ci.face))
        .iter()
        .any(|ab| match ab {
            AbilityDef::Spell { effects, .. } => effects.iter().any(|e| {
                matches!(
                    e,
                    Effect::SearchLibrary { .. } | Effect::SearchLibraryUpTo { .. }
                )
            }),
            _ => false,
        })
}

fn is_draw_spell_card(view: &PlayerView, oid: ObjectId) -> bool {
    let Some(ci) = lookup_card_identity(view, oid) else {
        return false;
    };
    let Some(def) = baylee_cards::by_index(ci.index) else {
        return false;
    };
    def.abilities_for_face(usize::from(ci.face))
        .iter()
        .any(|ab| match ab {
            AbilityDef::Spell { effects, .. } => effects
                .iter()
                .any(|e| matches!(e, Effect::DrawCards { .. })),
            _ => false,
        })
}

#[allow(dead_code)]
fn _unused_mana_source_marker(_: ManaSource) {}
