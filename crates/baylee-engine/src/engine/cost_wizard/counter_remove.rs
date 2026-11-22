//! Multi-permanent and multi-kind counter removal costs (`CostPart::RemoveAnyCounter`,
//! `CostPart::RemoveAnyCounterX`, `CR 118.3`, `CR 601.2h`).

use crate::eval;
use crate::state::GameState;
use crate::text_changes::RuleContext;
use crate::zone::Zone;
use baylee_cards_dsl::{CostPart, CounterKind, FaceDef, Filter};
use baylee_core::ids::{ObjectId, PlayerId};
use smallvec::SmallVec;

type PickChoices = SmallVec<[(ObjectId, CounterKind); 8]>;

#[derive(Clone, Copy)]
struct CounterEnv<'a> {
    state: &'a GameState,
    player: PlayerId,
    context: RuleContext,
}

#[derive(Clone, Copy, Debug)]
struct CounterDemand {
    needed: u32,
    kind: Option<CounterKind>,
    filter: &'static Filter,
    exclude_source: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum CounterStep {
    Complete,
    Impossible,
    AutoPick(ObjectId, CounterKind),
    AskKind {
        object: ObjectId,
        kinds: SmallVec<[CounterKind; 4]>,
    },
    AskObject {
        options: Vec<ObjectId>,
    },
}

fn source_moved_before(parts: &[CostPart], idx: usize) -> bool {
    parts[..idx].iter().any(|p| {
        matches!(
            p,
            CostPart::SacrificeSelf
                | CostPart::ExileSelf
                | CostPart::ReturnSelfToHand
                | CostPart::DiscardSelf
        )
    })
}

fn self_reserved_for(parts: &[CostPart], x: u32, wanted: CounterKind) -> u32 {
    parts.iter().fold(0_u32, |acc, part| match part {
        CostPart::RemoveCounterSelf { kind, n } if *kind == wanted => {
            acc.saturating_add(u32::from(*n))
        }
        CostPart::RemoveCounterSelfX { kind } if *kind == wanted => acc.saturating_add(x),
        _ => acc,
    })
}

fn collect_demands_from(
    parts: &[CostPart],
    x: u32,
    skip_picks: usize,
) -> SmallVec<[CounterDemand; 4]> {
    let mut skipped = 0_usize;
    let mut out = SmallVec::new();
    for (idx, part) in parts.iter().enumerate() {
        let (total, kind, filter) = match part {
            CostPart::RemoveAnyCounter {
                count,
                kind,
                filter,
            } => (u32::from(*count), *kind, *filter),
            CostPart::RemoveAnyCounterX { kind, filter } => (x, *kind, *filter),
            _ => continue,
        };
        if total == 0 {
            continue;
        }
        let total_usize = usize::try_from(total).unwrap_or(usize::MAX);
        if skipped + total_usize <= skip_picks {
            skipped += total_usize;
            continue;
        }
        let take_off = skip_picks.saturating_sub(skipped);
        skipped = skip_picks;
        let needed = total.saturating_sub(u32::try_from(take_off).unwrap_or(u32::MAX));
        if needed > 0 {
            out.push(CounterDemand {
                needed,
                kind,
                filter,
                exclude_source: source_moved_before(parts, idx),
            });
        }
    }
    out
}

fn nth_pick_spec(
    parts: &[CostPart],
    x: u32,
    pick_index: usize,
) -> Option<(usize, Option<CounterKind>, &'static Filter)> {
    let mut cursor = 0_usize;
    for (idx, part) in parts.iter().enumerate() {
        let (count, kind, filter) = match part {
            CostPart::RemoveAnyCounter {
                count,
                kind,
                filter,
            } => (usize::from(*count), *kind, *filter),
            CostPart::RemoveAnyCounterX { kind, filter } => {
                (usize::try_from(x).unwrap_or(usize::MAX), *kind, *filter)
            }
            _ => continue,
        };
        if pick_index < cursor.saturating_add(count) {
            return Some((idx, kind, filter));
        }
        cursor = cursor.saturating_add(count);
    }
    None
}

fn available_buckets(
    state: &GameState,
    player: PlayerId,
    context: RuleContext,
    parts: &[CostPart],
    x: u32,
    excluded_objects: &[ObjectId],
    already_picked: &[(ObjectId, CounterKind)],
) -> SmallVec<[(ObjectId, CounterKind, u32); 8]> {
    let mut out = SmallVec::new();
    for id in state.battlefield_view() {
        if excluded_objects.contains(&id) {
            continue;
        }
        let Some(obj) = state.object(id) else {
            continue;
        };
        if obj.controller != player {
            continue;
        }
        for (ck, raw) in obj.counters.iter() {
            let mut rem = u32::from(raw);
            if id == context.source {
                rem = rem.saturating_sub(self_reserved_for(parts, x, ck));
            }
            let spent = u32::try_from(
                already_picked
                    .iter()
                    .filter(|&&(oid, okind)| oid == id && okind == ck)
                    .count(),
            )
            .unwrap_or(u32::MAX);
            rem = rem.saturating_sub(spent);
            if rem > 0 {
                out.push((id, ck, rem));
            }
        }
    }
    out
}

fn bucket_matches(
    state: &GameState,
    player: PlayerId,
    context: RuleContext,
    demand: &CounterDemand,
    id: ObjectId,
    ck: CounterKind,
) -> bool {
    if demand.exclude_source && id == context.source {
        return false;
    }
    if demand.kind.is_some_and(|wanted| wanted != ck) {
        return false;
    }
    state
        .object(id)
        .is_some_and(|o| eval::matches_with_context(demand.filter, state, o, player, context))
}

fn assign_demands(
    env: CounterEnv<'_>,
    demands: &[CounterDemand],
    demand_idx: usize,
    rem_needed: u32,
    start_bucket: usize,
    buckets: &mut [(ObjectId, CounterKind, u32)],
) -> bool {
    if demand_idx >= demands.len() {
        return true;
    }
    if rem_needed == 0 {
        let next = demand_idx + 1;
        let next_needed = demands.get(next).map_or(0, |d| d.needed);
        return assign_demands(env, demands, next, next_needed, 0, buckets);
    }
    let demand = &demands[demand_idx];
    // Fast check: total matching capacity across remaining buckets must cover `rem_needed`.
    let mut cap = 0_u32;
    for &(id, ck, amt) in &buckets[start_bucket..] {
        if amt > 0 && bucket_matches(env.state, env.player, env.context, demand, id, ck) {
            cap = cap.saturating_add(amt);
        }
    }
    if cap < rem_needed {
        return false;
    }
    if demand_idx + 1 == demands.len() {
        return true;
    }
    for b_idx in start_bucket..buckets.len() {
        let (id, ck, avail) = buckets[b_idx];
        if avail == 0 || !bucket_matches(env.state, env.player, env.context, demand, id, ck) {
            continue;
        }
        let max_take = avail.min(rem_needed);
        for take in (1..=max_take).rev() {
            buckets[b_idx].2 -= take;
            let ok = assign_demands(
                env,
                demands,
                demand_idx,
                rem_needed - take,
                b_idx + 1,
                buckets,
            );
            buckets[b_idx].2 += take;
            if ok {
                return true;
            }
        }
    }
    false
}

pub(crate) fn can_afford_counter_parts(
    state: &GameState,
    player: PlayerId,
    context: RuleContext,
    parts: &[CostPart],
    x: u32,
    excluded_objects: &[ObjectId],
) -> bool {
    if !parts.iter().any(|p| {
        matches!(
            p,
            CostPart::RemoveAnyCounter { .. } | CostPart::RemoveAnyCounterX { .. }
        )
    }) {
        return true;
    }
    let demands = collect_demands_from(parts, x, 0);
    if demands.is_empty() {
        return true;
    }
    let mut buckets = available_buckets(state, player, context, parts, x, excluded_objects, &[]);
    assign_demands(
        CounterEnv {
            state,
            player,
            context,
        },
        &demands,
        0,
        demands[0].needed,
        0,
        &mut buckets,
    )
}

pub(crate) fn spell_counter_costs_payable(
    state: &GameState,
    player: PlayerId,
    card: ObjectId,
    face: &'static FaceDef,
    choice: crate::engine::cast_wizard::CostChoice,
    x: u32,
    excluded_objects: &[ObjectId],
) -> bool {
    let parts: SmallVec<[CostPart; 4]> =
        crate::engine::cast_wizard::wizard_cost_parts(face, choice)
            .copied()
            .collect();
    can_afford_counter_parts(
        state,
        player,
        eval::live_context(state, card),
        &parts,
        x,
        excluded_objects,
    )
}

pub(crate) fn total_battlefield_counters(state: &GameState, player: PlayerId) -> u32 {
    state
        .battlefield_view()
        .into_iter()
        .filter_map(|id| state.object(id))
        .filter(|o| o.controller == player)
        .flat_map(|o| o.counters.iter().map(|(_, n)| u32::from(n)))
        .fold(0_u32, u32::saturating_add)
}

pub(crate) fn activation_counter_x_max(
    state: &GameState,
    player: PlayerId,
    context: RuleContext,
    parts: &[CostPart],
) -> Option<u32> {
    if !parts
        .iter()
        .any(|p| matches!(p, CostPart::RemoveAnyCounterX { .. }))
    {
        return None;
    }
    let upper = total_battlefield_counters(state, player);
    if !can_afford_counter_parts(state, player, context, parts, 0, &[]) {
        return Some(0);
    }
    Some(crate::casting::greatest_affordable(upper, |x| {
        can_afford_counter_parts(state, player, context, parts, x, &[])
    }))
}

fn legal_next_pairs(
    state: &GameState,
    player: PlayerId,
    context: RuleContext,
    parts: &[CostPart],
    x: u32,
    excluded_objects: &[ObjectId],
    already_picked: &[(ObjectId, CounterKind)],
) -> Option<(&'static Filter, PickChoices)> {
    let pick_idx = already_picked.len();
    let (part_idx, kind_req, filter) = nth_pick_spec(parts, x, pick_idx)?;
    let remaining_demands = collect_demands_from(parts, x, pick_idx + 1);
    let mut buckets = available_buckets(
        state,
        player,
        context,
        parts,
        x,
        excluded_objects,
        already_picked,
    );
    let current = CounterDemand {
        needed: 1,
        kind: kind_req,
        filter,
        exclude_source: source_moved_before(parts, part_idx),
    };
    let env = CounterEnv {
        state,
        player,
        context,
    };
    let mut valid = SmallVec::new();
    for b_idx in 0..buckets.len() {
        let (id, ck, avail) = buckets[b_idx];
        if avail == 0 || !bucket_matches(state, player, context, &current, id, ck) {
            continue;
        }
        buckets[b_idx].2 -= 1;
        let ok = if remaining_demands.is_empty() {
            true
        } else {
            assign_demands(
                env,
                &remaining_demands,
                0,
                remaining_demands[0].needed,
                0,
                &mut buckets,
            )
        };
        buckets[b_idx].2 += 1;
        if ok && !valid.contains(&(id, ck)) {
            valid.push((id, ck));
        }
    }
    Some((filter, valid))
}

pub(crate) fn next_counter_step(
    state: &GameState,
    player: PlayerId,
    context: RuleContext,
    parts: &[CostPart],
    x: u32,
    excluded_objects: &[ObjectId],
    already_picked: &[(ObjectId, CounterKind)],
) -> CounterStep {
    let Some((filter, valid)) = legal_next_pairs(
        state,
        player,
        context,
        parts,
        x,
        excluded_objects,
        already_picked,
    ) else {
        return CounterStep::Complete;
    };
    if valid.is_empty() {
        return CounterStep::Impossible;
    }
    if *filter == Filter::This {
        let mut kinds = SmallVec::new();
        for &(id, ck) in &valid {
            if id == context.source && !kinds.contains(&ck) {
                kinds.push(ck);
            }
        }
        return match kinds.as_slice() {
            [] => CounterStep::Impossible,
            [only] => CounterStep::AutoPick(context.source, *only),
            _ => CounterStep::AskKind {
                object: context.source,
                kinds,
            },
        };
    }
    let mut options = Vec::new();
    for &(id, _) in &valid {
        if !options.contains(&id) {
            options.push(id);
        }
    }
    CounterStep::AskObject { options }
}

pub(crate) fn valid_kinds_for_chosen_object(
    state: &GameState,
    player: PlayerId,
    context: RuleContext,
    cost_and_x: (&[CostPart], u32),
    excluded_objects: &[ObjectId],
    already_picked: &[(ObjectId, CounterKind)],
    chosen_object: ObjectId,
) -> SmallVec<[CounterKind; 4]> {
    let (parts, x) = cost_and_x;
    let mut kinds = SmallVec::new();
    if let Some((_, valid)) = legal_next_pairs(
        state,
        player,
        context,
        parts,
        x,
        excluded_objects,
        already_picked,
    ) {
        for (id, ck) in valid {
            if id == chosen_object && !kinds.contains(&ck) {
                kinds.push(ck);
            }
        }
    }
    kinds
}

pub(crate) fn pay_counter_removals(
    state: &mut GameState,
    player: PlayerId,
    context: RuleContext,
    part: &CostPart,
    x: u32,
    removals: &mut core::slice::Iter<'_, (ObjectId, CounterKind)>,
) -> Result<(), crate::engine::EngineError> {
    let (needed, kind_req, filter) = match part {
        CostPart::RemoveAnyCounter {
            count,
            kind,
            filter,
        } => (usize::from(*count), *kind, *filter),
        CostPart::RemoveAnyCounterX { kind, filter } => {
            (usize::try_from(x).unwrap_or(usize::MAX), *kind, *filter)
        }
        _ => return Ok(()),
    };
    if needed == 0 {
        return Ok(());
    }
    let mut grouped: SmallVec<[(ObjectId, CounterKind, u16); 4]> = SmallVec::new();
    for _ in 0..needed {
        let &(obj_id, ck) = removals
            .next()
            .ok_or(crate::engine::EngineError::IllegalAction(
                "unanswered counter removal cost",
            ))?;
        if kind_req.is_some_and(|wanted| wanted != ck) {
            return Err(crate::engine::EngineError::IllegalAction(
                "wrong counter kind for cost",
            ));
        }
        let valid_obj = state.object(obj_id).is_some_and(|o| {
            o.zone == Zone::Battlefield
                && o.controller == player
                && eval::matches_with_context(filter, state, o, player, context)
        });
        if !valid_obj {
            return Err(crate::engine::EngineError::IllegalAction(
                "selected permanent cannot pay counter removal cost",
            ));
        }
        if let Some(entry) = grouped
            .iter_mut()
            .find(|(gid, gck, _)| *gid == obj_id && *gck == ck)
        {
            entry.2 = entry.2.saturating_add(1);
        } else {
            grouped.push((obj_id, ck, 1));
        }
    }
    for (obj_id, ck, amt) in grouped {
        if crate::replacement::remove_counters_by(state, obj_id, ck, amt, Some(player)) < amt {
            return Err(crate::engine::EngineError::IllegalAction(
                "not enough counters to pay the cost",
            ));
        }
    }
    Ok(())
}
