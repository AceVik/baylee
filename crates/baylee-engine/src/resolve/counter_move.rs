//! Moving counters between battlefield permanents (`CR 122.5`).
//!
//! Covers single-counter moves (`Effect::MoveCounter`) and generalized counter
//! movement (`Effect::MoveCounters`): fixed counts, all counters, any number
//! (optionally at least one), one of each kind not already on the destination,
//! filtered sources, and distribution among filtered destinations.

use super::{
    AwaitingOp, Flow, GameState, Pending, Resolution, eval, run, source_attachment_lki,
    source_version, subjects,
};
use crate::choice::{CastModeDesc, CastModeKind, NumberPrompt};
use baylee_cards_dsl::{
    CounterKind, MoveCounterCount, MoveCounterDest, MoveCounterSource, TargetSlot,
};
use baylee_core::{
    ids::{DamageSourceRef, ObjectId},
    mana::ManaCost,
};

/// One `(source, destination, counter kind)` move candidate in a multi-step
/// [`Effect::MoveCounters`] prompt sequence.
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub struct MoveQueueEntry {
    /// Permanent losing counters.
    pub from: DamageSourceRef,
    /// Permanent receiving counters.
    pub to: DamageSourceRef,
    /// Counter kind being considered.
    pub kind: CounterKind,
}

/// Active continuation state for [`AwaitingOp::MoveCounters`].
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub enum MoveCountersStep {
    /// Choosing one counter kind to move via `Pending::ChooseCastMode`.
    PickSingleKind {
        /// Source permanent.
        from: DamageSourceRef,
        /// Destination permanent.
        to: DamageSourceRef,
        /// Distinct movable counter kinds offered.
        kinds: Vec<CounterKind>,
    },
    /// Choosing counts across one or more `(from, to, kind)` entries via
    /// `Pending::ChooseNumber`.
    ChooseCount {
        /// Remaining and visited move entries in deterministic order.
        queue: Vec<MoveQueueEntry>,
        /// Index in `queue` currently being prompted.
        index: usize,
        /// Remaining total budget when exact or capped, else `None` (unlimited).
        remaining_budget: Option<u16>,
        /// Whether `remaining_budget` must be spent completely if legal.
        exact_required: bool,
        /// Whether at least one counter must be moved overall if legal.
        min_one: bool,
        /// Total counters selected so far across earlier entries.
        moved_so_far: u16,
        /// Accumulated `(from, to, kind, count)` moves committed at completion.
        pending_moves: Vec<(DamageSourceRef, DamageSourceRef, CounterKind, u16)>,
    },
}

/// Resumable payload for [`AwaitingOp::MoveCounters`].
#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct MoveCountersPlan {
    /// Current interactive step.
    pub step: MoveCountersStep,
}

fn subject(state: &GameState, res: &Resolution, slot: TargetSlot) -> Option<DamageSourceRef> {
    match slot {
        TargetSlot::This => subjects::source(state, res),
        other => subjects::target(state, res, *res.slot_targets(other).first()?),
    }
}

pub(super) fn start(
    state: &mut GameState,
    res: &mut Resolution,
    from: TargetSlot,
    to: TargetSlot,
    kind: Option<CounterKind>,
) -> Option<Pending> {
    let from = subject(state, res, from)?;
    let to = subject(state, res, to)?;
    if let Some(kind) = kind {
        let moved = crate::replacement::move_counters(state, res.controller, from, to, kind, 1);
        res.subject.counters_moved = res.subject.counters_moved.saturating_add(u32::from(moved));
        return None;
    }
    let kinds: Vec<_> = state
        .object(from.object)?
        .counters
        .iter()
        .map(|(kind, _)| kind)
        .filter(|kind| crate::replacement::can_move_counter(state, from, to, *kind))
        .collect();
    match kinds.as_slice() {
        [] => None,
        [kind] => {
            let moved =
                crate::replacement::move_counters(state, res.controller, from, to, *kind, 1);
            res.subject.counters_moved =
                res.subject.counters_moved.saturating_add(u32::from(moved));
            None
        }
        _ => {
            let options = kinds
                .iter()
                .enumerate()
                .map(|(index, kind)| CastModeDesc {
                    index: u32::try_from(index).expect("finite counter-kind vocabulary"),
                    kind: CastModeKind::Counter(*kind),
                    cost: ManaCost::ZERO,
                })
                .collect();
            res.awaiting = Some(AwaitingOp::MoveCounter { from, to, kinds });
            Some(Pending::ChooseCastMode {
                player: res.controller,
                object: res.source,
                options,
            })
        }
    }
}

pub(super) fn resume(state: &mut GameState, res: &mut Resolution, index: usize) -> Flow {
    let Some(AwaitingOp::MoveCounter { from, to, kinds }) = res.awaiting.take() else {
        unreachable!("counter-kind choice")
    };
    let kind = *kinds.get(index).expect("validated counter-kind choice");
    let moved = crate::replacement::move_counters(state, res.controller, from, to, kind, 1);
    res.subject.counters_moved = res.subject.counters_moved.saturating_add(u32::from(moved));
    res.pc += 1;
    run(state, res)
}

fn resolve_sources(
    state: &GameState,
    res: &Resolution,
    from: MoveCounterSource,
) -> Vec<DamageSourceRef> {
    let you = res.controller;
    match from {
        MoveCounterSource::Slot(slot) => subject(state, res, slot)
            .filter(|r| subjects::on_battlefield(state, *r))
            .into_iter()
            .collect(),
        MoveCounterSource::FirstTarget => res
            .targets
            .first()
            .and_then(|&id| subjects::target(state, res, id))
            .filter(|r| subjects::on_battlefield(state, *r))
            .into_iter()
            .collect(),
        MoveCounterSource::Source => subjects::source(state, res)
            .filter(|r| subjects::on_battlefield(state, *r))
            .into_iter()
            .collect(),
        MoveCounterSource::Filter(filter) => state
            .battlefield_seen()
            .filter_map(|id| {
                let obj = state.object(id)?;
                eval::matches_with_context(filter, state, obj, you, res.rule_context()).then_some(
                    DamageSourceRef {
                        object: id,
                        version: obj.version,
                    },
                )
            })
            .collect(),
    }
}

fn resolve_dests(state: &GameState, res: &Resolution, to: MoveCounterDest) -> Vec<DamageSourceRef> {
    let you = res.controller;
    match to {
        MoveCounterDest::Slot(slot) => subject(state, res, slot)
            .filter(|r| subjects::on_battlefield(state, *r))
            .into_iter()
            .collect(),
        MoveCounterDest::SecondTarget => res
            .second_targets
            .first()
            .or_else(|| res.targets.get(1))
            .and_then(|&id| subjects::target(state, res, id))
            .filter(|r| subjects::on_battlefield(state, *r))
            .into_iter()
            .collect(),
        MoveCounterDest::FirstTarget => res
            .targets
            .first()
            .and_then(|&id| subjects::target(state, res, id))
            .filter(|r| subjects::on_battlefield(state, *r))
            .into_iter()
            .collect(),
        MoveCounterDest::Source => subjects::source(state, res)
            .filter(|r| subjects::on_battlefield(state, *r))
            .into_iter()
            .collect(),
        MoveCounterDest::Attached => eval::attached_for_ability(
            state,
            res.source,
            source_version(state, res),
            source_attachment_lki(state, res.on_stack),
        )
        .map(|host| DamageSourceRef {
            object: host.id,
            version: host.version,
        })
        .filter(|r| subjects::on_battlefield(state, *r))
        .into_iter()
        .collect(),
        MoveCounterDest::EventObject => res
            .event_object
            .and_then(|object| {
                super::event_object_identity(state, res)
                    .map(|(version, _)| DamageSourceRef { object, version })
            })
            .filter(|r| subjects::on_battlefield(state, *r))
            .into_iter()
            .collect(),
        MoveCounterDest::DistributeAmong(filter) => state
            .battlefield_seen()
            .filter_map(|id| {
                let obj = state.object(id)?;
                eval::matches_with_context(filter, state, obj, you, res.rule_context()).then_some(
                    DamageSourceRef {
                        object: id,
                        version: obj.version,
                    },
                )
            })
            .collect(),
    }
}

fn movable_kinds(
    state: &GameState,
    from: DamageSourceRef,
    to: DamageSourceRef,
    filter_kind: Option<CounterKind>,
) -> Vec<(CounterKind, u16)> {
    let Some(obj) = state.object(from.object) else {
        return Vec::new();
    };
    obj.counters
        .iter()
        .filter(|&(k, count)| {
            count > 0
                && filter_kind.is_none_or(|wanted| wanted == k)
                && crate::replacement::can_move_counter(state, from, to, k)
        })
        .collect()
}

/// Commits all scheduled `(from, to, kind, count)` counter moves simultaneously
/// (`CR 122.5`), routing destination placements through `counter_order` so
/// replacement multipliers apply once per destination batch.
fn commit_moves(
    state: &mut GameState,
    res: &mut Resolution,
    moves: &[(DamageSourceRef, DamageSourceRef, CounterKind, u16)],
) {
    let mut per_dest: Vec<(ObjectId, Vec<(CounterKind, u16)>)> = Vec::new();
    let mut total_moved: u32 = 0;
    for &(from, to, kind, requested) in moves {
        if requested == 0 || !crate::replacement::can_move_counter(state, from, to, kind) {
            continue;
        }
        let room = crate::replacement::counter_limits::room(state, to.object, kind);
        let already_dest: u16 =
            per_dest
                .iter()
                .find(|(id, _)| *id == to.object)
                .map_or(0, |(_, list)| {
                    list.iter()
                        .filter(|(k, _)| *k == kind)
                        .map(|(_, n)| *n)
                        .fold(0, u16::saturating_add)
                });
        let avail_room = room.map_or(requested, |r| requested.min(r.saturating_sub(already_dest)));
        if avail_room == 0 {
            continue;
        }
        let taken = crate::replacement::remove_counters_by(
            state,
            from.object,
            kind,
            avail_room,
            Some(res.controller),
        );
        if taken == 0 {
            continue;
        }
        total_moved = total_moved.saturating_add(u32::from(taken));
        if let Some((_, list)) = per_dest.iter_mut().find(|(id, _)| *id == to.object) {
            if let Some((_, existing)) = list.iter_mut().find(|(k, _)| *k == kind) {
                *existing = existing.saturating_add(taken);
            } else {
                list.push((kind, taken));
            }
        } else {
            per_dest.push((to.object, vec![(kind, taken)]));
        }
    }
    res.subject.counters_moved = res.subject.counters_moved.saturating_add(total_moved);
    let plans = per_dest
        .into_iter()
        .filter_map(|(dest, additions)| {
            crate::replacement::counter_order::plan(
                state,
                res.controller,
                dest,
                additions,
                true,
                None,
            )
        })
        .collect();
    crate::replacement::counter_order::enqueue_batch(state, plans, Vec::new());
}

/// Starts executing [`baylee_cards_dsl::Effect::MoveCounters`].
#[allow(clippy::too_many_lines)]
pub(super) fn start_multi(
    state: &mut GameState,
    res: &mut Resolution,
    from: MoveCounterSource,
    to: MoveCounterDest,
    kind: Option<CounterKind>,
    count: MoveCounterCount,
) -> Option<Pending> {
    let sources = resolve_sources(state, res, from);
    let dests = resolve_dests(state, res, to);
    if sources.is_empty() || dests.is_empty() {
        return None;
    }

    match count {
        MoveCounterCount::OneOfEachKindNotOnDest => {
            let mut batch = Vec::new();
            for &src in &sources {
                for &dst in &dests {
                    if src.object == dst.object {
                        continue;
                    }
                    let Some(dst_obj) = state.object(dst.object) else {
                        continue;
                    };
                    for (k, _) in movable_kinds(state, src, dst, kind) {
                        if dst_obj.counters.get(k) == 0 {
                            batch.push((src, dst, k, 1));
                        }
                    }
                }
            }
            commit_moves(state, res, &batch);
            None
        }
        MoveCounterCount::All if dests.len() == 1 => {
            let dst = dests[0];
            let mut batch = Vec::new();
            for &src in &sources {
                if src.object == dst.object {
                    continue;
                }
                for (k, avail) in movable_kinds(state, src, dst, kind) {
                    batch.push((src, dst, k, avail));
                }
            }
            commit_moves(state, res, &batch);
            None
        }
        MoveCounterCount::Exact(0) => None,
        MoveCounterCount::Exact(1) if sources.len() == 1 && dests.len() == 1 => {
            let (src, dst) = (sources[0], dests[0]);
            if src.object == dst.object {
                return None;
            }
            let kinds: Vec<CounterKind> = movable_kinds(state, src, dst, kind)
                .into_iter()
                .map(|(k, _)| k)
                .collect();
            match kinds.as_slice() {
                [] => None,
                [only] => {
                    commit_moves(state, res, &[(src, dst, *only, 1)]);
                    None
                }
                _ => {
                    let options = kinds
                        .iter()
                        .enumerate()
                        .map(|(index, k)| CastModeDesc {
                            index: u32::try_from(index).expect("finite counter-kind vocabulary"),
                            kind: CastModeKind::Counter(*k),
                            cost: ManaCost::ZERO,
                        })
                        .collect();
                    res.awaiting = Some(AwaitingOp::MoveCounters(Box::new(MoveCountersPlan {
                        step: MoveCountersStep::PickSingleKind {
                            from: src,
                            to: dst,
                            kinds,
                        },
                    })));
                    Some(Pending::ChooseCastMode {
                        player: res.controller,
                        object: res.source,
                        options,
                    })
                }
            }
        }
        _ => {
            let mut queue = Vec::new();
            for &src in &sources {
                for &dst in &dests {
                    if src.object == dst.object {
                        continue;
                    }
                    for (k, _) in movable_kinds(state, src, dst, kind) {
                        queue.push(MoveQueueEntry {
                            from: src,
                            to: dst,
                            kind: k,
                        });
                    }
                }
            }
            if queue.is_empty() {
                return None;
            }
            let (remaining_budget, exact_required, min_one) = match count {
                MoveCounterCount::Exact(n) => (Some(n), true, false),
                MoveCounterCount::AnyNumber { min_one } => (None, false, min_one),
                MoveCounterCount::All => {
                    // Distribute all counters among multiple destinations.
                    let total_avail: u16 = sources
                        .iter()
                        .filter_map(|src| state.object(src.object))
                        .flat_map(|obj| obj.counters.iter())
                        .filter(|&(k, n)| n > 0 && kind.is_none_or(|w| w == k))
                        .map(|(_, n)| n)
                        .fold(0, u16::saturating_add);
                    (Some(total_avail), true, false)
                }
                MoveCounterCount::OneOfEachKindNotOnDest => unreachable!("handled above"),
            };
            advance_choose_count(
                state,
                res,
                queue,
                0,
                remaining_budget,
                exact_required,
                min_one,
                0,
                Vec::new(),
            )
        }
    }
}

fn entry_bounds(
    state: &GameState,
    entry: MoveQueueEntry,
    pending_moves: &[(DamageSourceRef, DamageSourceRef, CounterKind, u16)],
) -> u16 {
    if !crate::replacement::can_move_counter(state, entry.from, entry.to, entry.kind) {
        return 0;
    }
    let already_from: u16 = pending_moves
        .iter()
        .filter(|&&(f, _, k, _)| f == entry.from && k == entry.kind)
        .map(|&(_, _, _, n)| n)
        .fold(0, u16::saturating_add);
    let avail_from = state
        .object(entry.from.object)
        .map_or(0, |o| o.counters.get(entry.kind))
        .saturating_sub(already_from);
    if avail_from == 0 {
        return 0;
    }
    let already_to: u16 = pending_moves
        .iter()
        .filter(|&&(_, t, k, _)| t == entry.to && k == entry.kind)
        .map(|&(_, _, _, n)| n)
        .fold(0, u16::saturating_add);
    let room_to = crate::replacement::counter_limits::room(state, entry.to.object, entry.kind)
        .map_or(u16::MAX, |r| r.saturating_sub(already_to));
    avail_from.min(room_to)
}

fn later_capacity(
    state: &GameState,
    later: &[MoveQueueEntry],
    pending_moves: &[(DamageSourceRef, DamageSourceRef, CounterKind, u16)],
) -> u16 {
    // Upper bound on counters movable by `later`, bounded per `(from, kind)` so
    // multiple destinations sharing one source never double-count the source's pool.
    let mut per_source_kind: Vec<(DamageSourceRef, CounterKind, u16)> = Vec::new();
    for &e in later {
        let cap = entry_bounds(state, e, pending_moves);
        if cap == 0 {
            continue;
        }
        let already_from: u16 = pending_moves
            .iter()
            .filter(|&&(f, _, k, _)| f == e.from && k == e.kind)
            .map(|&(_, _, _, n)| n)
            .fold(0, u16::saturating_add);
        let src_pool = state
            .object(e.from.object)
            .map_or(0, |o| o.counters.get(e.kind))
            .saturating_sub(already_from);
        if let Some((_, _, total)) = per_source_kind
            .iter_mut()
            .find(|(f, k, _)| *f == e.from && *k == e.kind)
        {
            *total = total.saturating_add(cap).min(src_pool);
        } else {
            per_source_kind.push((e.from, e.kind, cap.min(src_pool)));
        }
    }
    per_source_kind
        .into_iter()
        .map(|(_, _, n)| n)
        .fold(0, u16::saturating_add)
}

#[allow(clippy::too_many_arguments)]
fn advance_choose_count(
    state: &mut GameState,
    res: &mut Resolution,
    queue: Vec<MoveQueueEntry>,
    mut index: usize,
    mut remaining_budget: Option<u16>,
    exact_required: bool,
    min_one: bool,
    mut moved_so_far: u16,
    mut pending_moves: Vec<(DamageSourceRef, DamageSourceRef, CounterKind, u16)>,
) -> Option<Pending> {
    while index < queue.len() {
        if remaining_budget == Some(0) {
            break;
        }
        let entry = queue[index];
        let avail = entry_bounds(state, entry, &pending_moves);
        let max_here = avail.min(remaining_budget.unwrap_or(u16::MAX));
        if max_here == 0 {
            index += 1;
            continue;
        }
        let later_cap = later_capacity(state, &queue[index + 1..], &pending_moves);
        let min_here = if exact_required {
            remaining_budget
                .unwrap_or(0)
                .saturating_sub(later_cap)
                .min(max_here)
        } else if min_one && moved_so_far == 0 && later_cap == 0 {
            1_u16.min(max_here)
        } else {
            0
        };
        if min_here == max_here {
            if min_here > 0 {
                pending_moves.push((entry.from, entry.to, entry.kind, min_here));
                moved_so_far = moved_so_far.saturating_add(min_here);
                if let Some(rem) = &mut remaining_budget {
                    *rem = rem.saturating_sub(min_here);
                }
            }
            index += 1;
            continue;
        }
        res.awaiting = Some(AwaitingOp::MoveCounters(Box::new(MoveCountersPlan {
            step: MoveCountersStep::ChooseCount {
                queue,
                index,
                remaining_budget,
                exact_required,
                min_one,
                moved_so_far,
                pending_moves,
            },
        })));
        return Some(Pending::ChooseNumber {
            player: res.controller,
            min: u32::from(min_here),
            max: u32::from(max_here),
            reason: NumberPrompt::Counters {
                target: entry.to.object,
                kind: entry.kind,
            },
        });
    }
    commit_moves(state, res, &pending_moves);
    None
}

/// Resumes [`AwaitingOp::MoveCounters`] after a `Pending::ChooseCastMode` answer.
pub(super) fn resume_multi_mode(
    state: &mut GameState,
    res: &mut Resolution,
    plan: MoveCountersPlan,
    index: usize,
) -> Flow {
    let MoveCountersStep::PickSingleKind { from, to, kinds } = plan.step else {
        unreachable!("ChooseCastMode only answers PickSingleKind")
    };
    let kind = *kinds.get(index).expect("validated counter-kind choice");
    commit_moves(state, res, &[(from, to, kind, 1)]);
    res.pc += 1;
    run(state, res)
}

/// Resumes [`AwaitingOp::MoveCounters`] after a `Pending::ChooseNumber` answer.
pub(super) fn resume_multi_number(
    state: &mut GameState,
    res: &mut Resolution,
    plan: MoveCountersPlan,
    number: u32,
) -> Flow {
    let MoveCountersStep::ChooseCount {
        queue,
        index,
        mut remaining_budget,
        exact_required,
        min_one,
        mut moved_so_far,
        mut pending_moves,
    } = plan.step
    else {
        unreachable!("ChooseNumber only answers ChooseCount")
    };
    let entry = queue[index];
    let avail = entry_bounds(state, entry, &pending_moves);
    let max_here = avail.min(remaining_budget.unwrap_or(u16::MAX));
    let chosen = u16::try_from(number).unwrap_or(u16::MAX).min(max_here);
    if chosen > 0 {
        pending_moves.push((entry.from, entry.to, entry.kind, chosen));
        moved_so_far = moved_so_far.saturating_add(chosen);
        if let Some(rem) = &mut remaining_budget {
            *rem = rem.saturating_sub(chosen);
        }
    }
    if let Some(pending) = advance_choose_count(
        state,
        res,
        queue,
        index + 1,
        remaining_budget,
        exact_required,
        min_one,
        moved_so_far,
        pending_moves,
    ) {
        return Flow::Wait(pending);
    }
    res.pc += 1;
    run(state, res)
}
